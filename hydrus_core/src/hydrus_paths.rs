//! Rust port of `hydrus/core/HydrusPaths.py`.
//!
//! Notes on platform deviations:
//!
//! * The Python module reaches into `HG.controller.get_db_dir()` when no base
//!   directory is given. The core crate has no controller, so
//!   [`crate::hydrus_globals::db_dir`] is used, falling back to
//!   [`crate::hydrus_constants::base_dir`] during early boot/tests.
//! * `send2trash` is not available. [`recycle_path`] is therefore a retrying
//!   *permanent* delete; putting files in the platform trash belongs to a
//!   higher layer (the client UI).
//! * `psutil` is not available. [`get_free_space`] / [`get_total_space`] use
//!   direct Win32 calls on Windows, `statvfs` on 64-bit Linux, and `df -P -k`
//!   elsewhere on Unix. [`get_partition_info`] parses `/proc/mounts` on Linux,
//!   `mount` on other Unixes, and asks Win32 for the filesystem name.
//! * `shutil.copy2`/`copystat` also copy xattrs and flags on some systems.
//!   Rust's [`std::fs::copy`] and [`std::fs::set_permissions`] cover contents,
//!   permission bits and (via `copy_times`) timestamps; everything else is
//!   best-effort.
//! * Python's `BigJobPauser` is a threading concept with no core-crate
//!   equivalent, so tree operations do not pause. They still call the text and
//!   cancellation hooks.
//! * `launch_directory` and `open_file_location` spawn their helper process in
//!   a detached thread, like Python. Failures are ignored/logged, as the
//!   spawning API returns before the child reports anything.
//! * Paths crossing the FFI boundary use the platform encoding (`OsStrExt`).

use std::collections::HashSet;
use std::fs::{self, Metadata};
use std::io::{self, Read, Write};
use std::path::{Component, Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::LazyLock;
use std::time::{Duration, UNIX_EPOCH};

use crate::hydrus_constants::{
    PLATFORM_HAIKU, PLATFORM_LINUX, PLATFORM_MACOS, PLATFORM_WINDOWS, READ_BLOCK_SIZE,
};
use crate::hydrus_data::safe_print;
use crate::hydrus_exceptions::{ErrorKind, HydrusException, HydrusResult};
use crate::hydrus_globals;

#[cfg(all(target_os = "linux", target_pointer_width = "64"))]
use std::ffi::CString;

// ============================================================================
// Platform FFI
// ============================================================================

#[cfg(windows)]
#[allow(non_snake_case)]
unsafe extern "system" {
    fn GetDiskFreeSpaceExW(
        lpDirectoryName: *const u16,
        lpFreeBytesAvailableToCaller: *mut u64,
        lpTotalNumberOfBytes: *mut u64,
        lpTotalNumberOfFreeBytes: *mut u64,
    ) -> i32;

    fn GetVolumeInformationW(
        lpRootPathName: *const u16,
        lpVolumeNameBuffer: *mut u16,
        nVolumeNameSize: u32,
        lpVolumeSerialNumber: *mut u32,
        lpMaximumComponentLength: *mut u32,
        lpFileSystemFlags: *mut u32,
        lpFileSystemNameBuffer: *mut u16,
        nFileSystemNameSize: u32,
    ) -> i32;

    fn GetFileInformationByHandle(
        hFile: *mut std::ffi::c_void,
        lpFileInformation: *mut ByHandleFileInformation,
    ) -> i32;
}

#[cfg(windows)]
#[repr(C)]
#[allow(non_snake_case, dead_code)]
struct WinFileTime {
    dwLowDateTime: u32,
    dwHighDateTime: u32,
}

/// `BY_HANDLE_FILE_INFORMATION`
#[cfg(windows)]
#[repr(C)]
#[allow(non_snake_case, dead_code)]
struct ByHandleFileInformation {
    dwFileAttributes: u32,
    ftCreationTime: WinFileTime,
    ftLastAccessTime: WinFileTime,
    ftLastWriteTime: WinFileTime,
    dwVolumeSerialNumber: u32,
    nFileSizeHigh: u32,
    nFileSizeLow: u32,
    nNumberOfLinks: u32,
    nFileIndexHigh: u32,
    nFileIndexLow: u32,
}

#[cfg(windows)]
const FILE_FLAG_BACKUP_SEMANTICS: u32 = 0x0200_0000;

#[cfg(windows)]
const FILE_READ_ATTRIBUTES: u32 = 0x0000_0080;

/// Stable alternative to the still-unstable `MetadataExt::volume_serial_number`
/// and `MetadataExt::file_index`. Works for both files and directories.
#[cfg(windows)]
fn windows_file_id(path: &Path) -> Option<(u32, u64)> {
    use std::os::windows::fs::OpenOptionsExt;
    use std::os::windows::io::AsRawHandle;

    let file = fs::OpenOptions::new()
        .access_mode(FILE_READ_ATTRIBUTES)
        .custom_flags(FILE_FLAG_BACKUP_SEMANTICS)
        .open(path)
        .ok()?;

    // SAFETY: `info` is a plain C struct and the handle is valid for `file`.
    let mut info: ByHandleFileInformation = unsafe { std::mem::zeroed() };

    // SAFETY: the handle comes from an open `File`, and `info` is a valid
    // out-parameter of the correct size.
    let result = unsafe { GetFileInformationByHandle(file.as_raw_handle(), &mut info) };

    if result == 0 {
        return None;
    }

    let file_index = (u64::from(info.nFileIndexHigh) << 32) | u64::from(info.nFileIndexLow);

    Some((info.dwVolumeSerialNumber, file_index))
}

#[cfg(unix)]
#[cfg(any(
    target_os = "macos",
    target_os = "ios",
    target_os = "freebsd",
    target_os = "dragonfly",
    target_os = "netbsd",
    target_os = "openbsd"
))]
type ModeT = u16;

/// `mode_t` is 32-bit on Linux, Android, Solaris and Haiku.
#[cfg(unix)]
#[cfg(not(any(
    target_os = "macos",
    target_os = "ios",
    target_os = "freebsd",
    target_os = "dragonfly",
    target_os = "netbsd",
    target_os = "openbsd"
)))]
type ModeT = u32;

#[cfg(unix)]
unsafe extern "C" {
    fn umask(mask: ModeT) -> ModeT;

    fn flock(fd: std::os::raw::c_int, operation: std::os::raw::c_int) -> std::os::raw::c_int;
}

/// `struct statvfs` from glibc, 64-bit layout. We only read the first five
/// fields, but the struct is declared at full size so glibc cannot write past
/// the end of our buffer.
#[cfg(all(target_os = "linux", target_pointer_width = "64"))]
#[repr(C)]
#[allow(dead_code)]
struct LinuxStatVfs {
    f_bsize: u64,
    f_frsize: u64,
    f_blocks: u64,
    f_bfree: u64,
    f_bavail: u64,
    f_files: u64,
    f_ffree: u64,
    f_favail: u64,
    f_fsid: u64,
    f_flag: u64,
    f_namemax: u64,
    f_type: u32,
    _f_spare: [i32; 5],
}

#[cfg(all(target_os = "linux", target_pointer_width = "64"))]
unsafe extern "C" {
    fn statvfs(path: *const std::os::raw::c_char, buf: *mut LinuxStatVfs) -> std::os::raw::c_int;
}

#[cfg(unix)]
const LOCK_EX: std::os::raw::c_int = 2;
#[cfg(unix)]
const LOCK_NB: std::os::raw::c_int = 4;

/// `DO_NOT_DO_CHMOD_MODE`
pub static DO_NOT_DO_CHMOD_MODE: AtomicBool = AtomicBool::new(false);

/// `DO_FLOCK_ALREADY_IN_USE_TEST_IN_POSIX`
pub static DO_FLOCK_ALREADY_IN_USE_TEST_IN_POSIX: AtomicBool = AtomicBool::new(true);

/// `PROCESS_UMASK`, read once at first use like the Python import-time snippet.
pub static PROCESS_UMASK: LazyLock<u32> = LazyLock::new(|| {
    #[cfg(unix)]
    {
        // SAFETY: `umask` only reads and writes the process umask.
        unsafe {
            let old = umask(0o022);
            umask(old);
            u32::from(old)
        }
    }

    #[cfg(not(unix))]
    {
        0o022
    }
});

/// `NTFS_disallowed_names_case_insensitive`
pub static NTFS_DISALLOWED_NAMES_CASE_INSENSITIVE: LazyLock<HashSet<String>> = LazyLock::new(|| {
    let mut names = HashSet::new();

    for name in ["con", "prn", "aux", "nul"] {
        names.insert(name.to_string());
    }

    for i in 1..10 {
        names.insert(format!("com{i}"));
        names.insert(format!("lpt{i}"));
    }

    names
});

static WE_SWITCHED_TO_USERPATH: AtomicBool = AtomicBool::new(false);

/// `HC.WE_SWITCHED_TO_USERPATH`
pub fn we_switched_to_userpath() -> bool {
    WE_SWITCHED_TO_USERPATH.load(Ordering::Relaxed)
}

/// `HC.WE_SWITCHED_TO_USERPATH = ...`
pub fn set_we_switched_to_userpath(value: bool) {
    WE_SWITCHED_TO_USERPATH.store(value, Ordering::Relaxed);
}

// ============================================================================
// Small helpers
// ============================================================================

fn io_failure(kind: ErrorKind, context: impl std::fmt::Display, error: &io::Error) -> HydrusException {
    HydrusException::new(kind, format!("{context}: {error}"))
}

fn metadata_mtime_float(metadata: &Metadata) -> f64 {
    match metadata.modified() {
        Ok(modified) => match modified.duration_since(UNIX_EPOCH) {
            Ok(duration) => duration.as_secs_f64(),
            Err(error) => -(error.duration().as_secs_f64()),
        },
        Err(_) => 0.0,
    }
}

fn path_metadata(path: &Path) -> io::Result<Option<Metadata>> {
    match fs::metadata(path) {
        Ok(metadata) => Ok(Some(metadata)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

fn normalize_lexical(path: &Path) -> PathBuf {
    let mut stack: Vec<Component<'_>> = Vec::new();

    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => match stack.last() {
                Some(Component::Normal(_)) => {
                    stack.pop();
                }
                Some(Component::RootDir) | Some(Component::Prefix(_)) => {
                    // Cannot go above the root.
                }
                _ => stack.push(component),
            },
            Component::Prefix(_) | Component::RootDir | Component::Normal(_) => {
                stack.push(component);
            }
        }
    }

    if stack.is_empty() {
        return PathBuf::from(".");
    }

    let mut result = PathBuf::new();

    for component in stack {
        result.push(component.as_os_str());
    }

    result
}

fn components_equal(first: &Component<'_>, second: &Component<'_>) -> bool {
    if PLATFORM_WINDOWS {
        first
            .as_os_str()
            .to_string_lossy()
            .eq_ignore_ascii_case(&second.as_os_str().to_string_lossy())
    } else {
        first == second
    }
}

/// Lexical version of `os.path.relpath`. Returns `None` when the paths live on
/// different roots (e.g. different Windows drives), which is where Python
/// raises `ValueError`.
fn relative_path_lexical(path: &Path, base: &Path) -> Option<PathBuf> {
    if path.is_absolute() != base.is_absolute() {
        return None;
    }

    let path_components: Vec<Component<'_>> = path.components().collect();
    let base_components: Vec<Component<'_>> = base.components().collect();

    let mut common = 0;

    while common < path_components.len()
        && common < base_components.len()
        && components_equal(&path_components[common], &base_components[common])
    {
        common += 1;
    }

    let mut result = PathBuf::new();

    for component in &base_components[common..] {
        match component {
            Component::Normal(_) => result.push(".."),
            Component::CurDir => {}
            _ => return None,
        }
    }

    for component in &path_components[common..] {
        result.push(component.as_os_str());
    }

    if result.as_os_str().is_empty() {
        result.push(".");
    }

    Some(result)
}

/// The db dir is the base for portable/absolute path conversion, exactly as
/// `HG.controller.get_db_dir()` is in Python. Early boot and unit tests may not
/// have set it, so fall back to the install dir.
fn default_base_dir() -> PathBuf {
    crate::hydrus_globals::db_dir()
        .unwrap_or_else(|| crate::hydrus_constants::base_dir().to_path_buf())
}

fn expand_user(path: &str) -> PathBuf {
    if path == "~" {
        if let Some(home) = dirs::home_dir() {
            return home;
        }
    }

    for prefix in ["~/", "~\\"] {
        if let Some(rest) = path.strip_prefix(prefix) {
            if let Some(home) = dirs::home_dir() {
                return home.join(rest);
            }
        }
    }

    PathBuf::from(path)
}

// ============================================================================
// Stat helpers
// ============================================================================

/// `oserror_is_device_access_trouble( e )`
///
/// The POSIX errno values come straight from Python. On Windows the numeric
/// codes are Win32 errors, plus the HRESULT hydrus sees for a locked BitLocker
/// drive.
pub fn oserror_is_device_access_trouble(error: &io::Error) -> bool {
    let Some(code) = error.raw_os_error() else {
        return false;
    };

    if PLATFORM_WINDOWS {
        matches!(
            code,
            -2144272384 // locked BitLocker drive
                | 21        // ERROR_NOT_READY
                | 53        // ERROR_BAD_NETPATH
                | 54        // ERROR_NETWORK_BUSY
                | 55        // ERROR_DEV_NOT_EXIST
                | 59        // ERROR_UNEXP_NET_ERR
                | 64        // ERROR_NETNAME_DELETED
                | 65        // ERROR_NETWORK_ACCESS_DENIED
                | 1167      // ERROR_DEVICE_NOT_CONNECTED
        )
    } else {
        matches!(
            code,
            112 // EHOSTDOWN
                | 113 // EHOSTUNREACH
                | 104 // ECONNRESET
                | 110 // ETIMEDOUT
                | 116 // ESTALE
                | 107 // ENOTCONN
                | 19 // ENODEV
        )
    }
}

/// `stat_is_file( path_stat )`
pub fn stat_is_file(path_stat: &Metadata) -> bool {
    path_stat.file_type().is_file()
}

/// `stat_is_symlink( path_stat )`
///
/// Pass metadata from [`fs::symlink_metadata`] to inspect the link itself.
pub fn stat_is_symlink(path_stat: &Metadata) -> bool {
    path_stat.file_type().is_symlink()
}

/// `stat_is_dir( path_stat )`
pub fn stat_is_dir(path_stat: &Metadata) -> bool {
    path_stat.file_type().is_dir()
}

/// `stat_is_samefile( source_stat, dest_stat )`
pub fn stat_is_samefile(source_stat: &Metadata, dest_stat: &Metadata) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;

        source_stat.dev() == dest_stat.dev() && source_stat.ino() == dest_stat.ino()
    }

    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;

        // `volume_serial_number`/`file_index` are still unstable in std, so
        // compare the stable identity fields. This identifies the same file;
        // two distinct files sharing all four fields is vanishingly unlikely.
        source_stat.file_size() == dest_stat.file_size()
            && source_stat.creation_time() == dest_stat.creation_time()
            && source_stat.last_write_time() == dest_stat.last_write_time()
            && source_stat.file_attributes() == dest_stat.file_attributes()
    }

    #[cfg(not(any(unix, windows)))]
    {
        false
    }
}

/// Return whether two paths refer to the same file or directory.
///
/// This is the path-based equivalent of [`stat_is_samefile`]. On Windows it
/// uses `GetFileInformationByHandle`, which gives an exact volume serial and
/// file index without relying on the still-unstable `MetadataExt` helpers.
/// Missing or inaccessible paths simply return `false`.
pub fn paths_are_same_file(source: &Path, dest: &Path) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;

        let (Ok(source_stat), Ok(dest_stat)) = (fs::metadata(source), fs::metadata(dest)) else {
            return false;
        };

        source_stat.dev() == dest_stat.dev() && source_stat.ino() == dest_stat.ino()
    }

    #[cfg(windows)]
    {
        let (Some(source_id), Some(dest_id)) = (windows_file_id(source), windows_file_id(dest))
        else {
            return false;
        };

        source_id == dest_id
    }

    #[cfg(not(any(unix, windows)))]
    {
        let _ = (source, dest);
        false
    }
}

/// `Dest_stat_has_same_size_and_date_as_source( source_stat, dest_stat )`
pub fn dest_stat_has_same_size_and_date_as_source(
    source_stat: &Metadata,
    dest_stat: &Metadata,
) -> bool {
    source_stat.len() == dest_stat.len()
        && metadata_mtime_float(source_stat) as i64 == metadata_mtime_float(dest_stat) as i64
}

/// `append_path_until_no_conflicts( path )`
pub fn append_path_until_no_conflicts(path: &Path) -> PathBuf {
    let parent = path.parent().unwrap_or_else(|| Path::new(""));
    let file_name = path.file_name().map(|name| name.to_string_lossy().into_owned());

    let Some(file_name) = file_name else {
        return path.to_path_buf();
    };

    let (stem, extension) = match file_name.rfind('.') {
        Some(index) if index > 0 => (&file_name[..index], &file_name[index..]),
        _ => (file_name.as_str(), ""),
    };

    let mut candidate_stem = stem.to_string();
    let mut i = 0;

    loop {
        let candidate = parent.join(format!("{candidate_stem}{extension}"));

        if !candidate.exists() {
            return candidate;
        }

        candidate_stem = format!("{stem}_{i}");
        i += 1;
    }
}

// ============================================================================
// Path conversion
// ============================================================================

/// `convert_abs_path_to_portable_path( abs_path, base_dir_override = None )`
///
/// Returns a `/`-separated string on Windows, exactly like Python.
pub fn convert_abs_path_to_portable_path(
    abs_path: &Path,
    base_dir_override: Option<&Path>,
) -> String {
    let base_dir = base_dir_override
        .map(Path::to_path_buf)
        .unwrap_or_else(default_base_dir);

    let portable_path = relative_path_lexical(abs_path, &base_dir)
        .filter(|relative| !relative.starts_with(".."))
        .unwrap_or_else(|| abs_path.to_path_buf());

    let mut portable = portable_path.to_string_lossy().into_owned();

    if PLATFORM_WINDOWS {
        portable = portable.replace('\\', "/");
    }

    portable
}

/// `convert_abs_path_to_real_path( abs_path )`
///
/// Python uses `os.path.realpath( strict = False )`. We canonicalise as much of
/// the path as exists and fall back to the input otherwise.
pub fn convert_abs_path_to_real_path(abs_path: &Path) -> PathBuf {
    if let Ok(real_path) = fs::canonicalize(abs_path) {
        return real_path;
    }

    if let (Some(parent), Some(name)) = (abs_path.parent(), abs_path.file_name()) {
        if let Ok(real_parent) = fs::canonicalize(parent) {
            return real_parent.join(name);
        }
    }

    abs_path.to_path_buf()
}

/// `convert_portable_path_to_abs_path( portable_path, base_dir_override = None )`
pub fn convert_portable_path_to_abs_path(
    portable_path: &Path,
    base_dir_override: Option<&Path>,
) -> PathBuf {
    let portable_path = normalize_lexical(portable_path);

    let mut abs_path = if portable_path.is_absolute() {
        portable_path
    } else {
        let base_dir = base_dir_override
            .map(Path::to_path_buf)
            .unwrap_or_else(default_base_dir);

        normalize_lexical(&base_dir.join(portable_path))
    };

    // Legacy platform-migration recovery: a non-Windows path that does not
    // exist may still contain backslash separators.
    if !PLATFORM_WINDOWS && !abs_path.exists() {
        abs_path = PathBuf::from(abs_path.to_string_lossy().replace('\\', "/"));
    }

    abs_path
}

// ============================================================================
// Block IO
// ============================================================================

/// Iterator returned by [`read_file_like_as_blocks`].
pub struct BlockReader<R> {
    reader: R,
    block_size: usize,
}

impl<R: Read> Iterator for BlockReader<R> {
    type Item = io::Result<Vec<u8>>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let mut buffer = vec![0_u8; self.block_size];

            match self.reader.read(&mut buffer) {
                Ok(0) => return None,
                Ok(bytes_read) => {
                    buffer.truncate(bytes_read);

                    return Some(Ok(buffer));
                }
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                Err(error) => return Some(Err(error)),
            }
        }
    }
}

/// `read_file_like_as_blocks( f ) -> Iterator[bytes]`
pub fn read_file_like_as_blocks<R: Read>(reader: R) -> BlockReader<R> {
    BlockReader {
        reader,
        block_size: READ_BLOCK_SIZE,
    }
}

/// `CopyFileLikeToFileLike( f_source, f_dest )`
pub fn copy_file_like_to_file_like<R: Read, W: Write>(
    source: R,
    dest: &mut W,
) -> io::Result<()> {
    for block in read_file_like_as_blocks(source) {
        dest.write_all(&block?)?;
    }

    Ok(())
}

// ============================================================================
// Timestamps and copies
// ============================================================================

/// `file_modified_time_is_ok( mtime )`
pub fn file_modified_time_is_ok(mtime: f64) -> bool {
    if PLATFORM_WINDOWS {
        // 1980-01-01 UTC, the DOS epoch. Windows has trouble before this.
        mtime >= 31_553_280.0
    } else {
        mtime >= 0.0
    }
}

/// `retry_blocking_io_call( func, *args, **kwargs )`
///
/// Python passes the call target and arguments; the Rust port takes a closure.
pub fn retry_blocking_io_call<T, F>(mut func: F) -> io::Result<T>
where
    F: FnMut() -> io::Result<T>,
{
    const NUM_ATTEMPTS: usize = 5;

    let mut delay = 1.0_f64;
    let mut attempt = 0;

    loop {
        match func() {
            Ok(value) => return Ok(value),
            Err(error)
                if error.kind() == io::ErrorKind::WouldBlock && attempt + 1 < NUM_ATTEMPTS =>
            {
                std::thread::sleep(Duration::from_secs_f64(delay));

                delay *= 1.5;
                attempt += 1;
            }
            Err(error) => return Err(error),
        }
    }
}

fn copy_times_io(source: &Path, dest: &Path) -> io::Result<()> {
    let metadata = fs::metadata(source)?;

    let accessed = metadata.accessed()?;
    let modified = metadata.modified()?;

    let file = fs::OpenOptions::new().write(true).open(dest)?;
    let times = fs::FileTimes::new()
        .set_accessed(accessed)
        .set_modified(modified);

    file.set_times(times)
}

/// `copy_times( source, dest ) -> bool`
pub fn copy_times(source: &Path, dest: &Path) -> bool {
    copy_times_io(source, dest).is_ok()
}

fn copy_kind(error: &io::Error) -> ErrorKind {
    if error.kind() == io::ErrorKind::NotFound {
        ErrorKind::FileMissing
    } else {
        ErrorKind::Veto
    }
}

/// Copy file contents only, leaving the destination's existing permission bits
/// and timestamps alone. This is the `shutil.copyfile` half of `copy2`.
fn copy_file_contents_io(source: &Path, dest: &Path) -> io::Result<()> {
    let mut source_file = fs::File::open(source)?;
    let mut dest_file = fs::File::create(dest)?;

    io::copy(&mut source_file, &mut dest_file)?;

    Ok(())
}

fn copy_file_io(source: &Path, dest: &Path) -> io::Result<()> {
    let metadata = fs::metadata(source)?;

    let try_to_copy_modified_time = file_modified_time_is_ok(metadata_mtime_float(&metadata));

    if DO_NOT_DO_CHMOD_MODE.load(Ordering::Relaxed) {
        // Python uses `shutil.copyfile` here, which does not touch mode bits.
        retry_blocking_io_call(|| copy_file_contents_io(source, dest))?;
    } else if try_to_copy_modified_time {
        // `std::fs::copy` overwrites the destination and copies permission
        // bits, matching the useful part of `shutil.copy2`. Timestamps are
        // applied separately by `copy_times_io`.
        match retry_blocking_io_call(|| fs::copy(source, dest).map(|_| ())) {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::PermissionDenied => {
                retry_blocking_io_call(|| copy_file_contents_io(source, dest))?;
            }
            Err(error) => return Err(error),
        }
    } else {
        // `shutil.copy`: contents and permission bits, but no mtime copy.
        retry_blocking_io_call(|| fs::copy(source, dest).map(|_| ()))?;
    }

    if try_to_copy_modified_time {
        // `copy_times` is best-effort in Python too; it reports a bool but the
        // callers ignore it.
        let _ = retry_blocking_io_call(|| copy_times_io(source, dest));
    }

    Ok(())
}

/// `safe_copy2( source_path, dest_path )`
pub fn safe_copy2(source_path: &Path, dest_path: &Path) -> HydrusResult<()> {
    copy_file_io(source_path, dest_path).map_err(|error| {
        io_failure(
            copy_kind(&error),
            format!(
                "Could not copy \"{}\" to \"{}\"",
                source_path.display(),
                dest_path.display()
            ),
            &error,
        )
    })
}

/// `safe_copystat( source_path, dest_path )`
pub fn safe_copystat(source_path: &Path, dest_path: &Path) -> HydrusResult<()> {
    if DO_NOT_DO_CHMOD_MODE.load(Ordering::Relaxed) {
        let _ = retry_blocking_io_call(|| copy_times_io(source_path, dest_path));

        return Ok(());
    }

    let metadata = fs::metadata(source_path).map_err(|error| {
        io_failure(
            copy_kind(&error),
            format!("Could not read metadata for \"{}\"", source_path.display()),
            &error,
        )
    })?;

    fs::set_permissions(dest_path, metadata.permissions()).map_err(|error| {
        io_failure(
            ErrorKind::Veto,
            format!(
                "Could not copy permission bits from \"{}\" to \"{}\"",
                source_path.display(),
                dest_path.display()
            ),
            &error,
        )
    })?;

    let _ = retry_blocking_io_call(|| copy_times_io(source_path, dest_path));

    Ok(())
}

fn copy_dir_io(source: &Path, dest: &Path) -> io::Result<()> {
    fs::create_dir_all(dest)?;

    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let source_path = entry.path();
        let dest_path = dest.join(entry.file_name());
        let file_type = entry.file_type()?;

        if file_type.is_dir() {
            copy_dir_io(&source_path, &dest_path)?;
        } else {
            copy_file_io(&source_path, &dest_path)?;
        }
    }

    if let Ok(metadata) = fs::metadata(source) {
        let _ = fs::set_permissions(dest, metadata.permissions());
    }

    let _ = copy_times_io(source, dest);

    Ok(())
}

/// Resolve the same final destination `shutil.move` would choose.
fn move_final_dest(source: &Path, dest: &Path) -> PathBuf {
    if dest.is_dir() {
        match source.file_name() {
            Some(name) => dest.join(name),
            None => dest.to_path_buf(),
        }
    } else {
        dest.to_path_buf()
    }
}

fn move_path_io(source: &Path, dest: &Path) -> io::Result<()> {
    let final_dest = move_final_dest(source, dest);

    if fs::rename(source, &final_dest).is_ok() {
        return Ok(());
    }

    // Cross-device move (or Windows refusing to overwrite): copy, then delete.
    let source_is_dir = fs::symlink_metadata(source)
        .map(|metadata| metadata.is_dir())
        .unwrap_or(false);

    if source_is_dir {
        copy_dir_io(source, &final_dest)?;
        fs::remove_dir_all(source)?;
    } else {
        copy_file_io(source, &final_dest)?;
        fs::remove_file(source)?;
    }

    Ok(())
}

fn permission_recovering_move_io(source: &Path, dest: &Path) -> io::Result<()> {
    match move_path_io(source, dest) {
        Ok(()) => Ok(()),
        Err(first_error) => {
            safe_print(format!(
                "While trying to move \"{}\" to \"{}\", we had an error. Trying to set nicer permission bits now: {}",
                source.display(),
                dest.display(),
                first_error
            ));

            try_to_give_file_nice_permission_bits(source);

            move_path_io(source, dest)
        }
    }
}

/// `permission_recovering_shutil_move( source, dest )`
pub fn permission_recovering_shutil_move(source: &Path, dest: &Path) -> HydrusResult<()> {
    retry_blocking_io_call(|| permission_recovering_move_io(source, dest)).map_err(|error| {
        io_failure(
            copy_kind(&error),
            format!(
                "Could not move \"{}\" to \"{}\"",
                source.display(),
                dest.display()
            ),
            &error,
        )
    })
}

/// `delete_path( path ) -> bool`
///
/// Symlinks are unlinked, never followed. Missing paths count as success.
pub fn delete_path(path: &Path) -> bool {
    if hydrus_globals::file_report_mode() {
        safe_print(format!("Deleting {}", path.display()));
    }

    let path_stat_nofollow = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return true,
        Err(error) => {
            safe_print(format!(
                "Trying to delete \"{}\" failed: {}",
                path.display(),
                error
            ));

            return false;
        }
    };

    if stat_is_symlink(&path_stat_nofollow) {
        // Remove the link itself, whether it points at a file or a directory.
        if fs::remove_file(path).is_ok() || fs::remove_dir(path).is_ok() {
            return true;
        }

        safe_print(format!(
            "Trying to delete the symlink \"{}\" failed.",
            path.display()
        ));

        return false;
    }

    #[cfg(windows)]
    try_to_make_file_writeable(path, &path_stat_nofollow);

    let result = if stat_is_dir(&path_stat_nofollow) {
        fs::remove_dir_all(path)
    } else {
        fs::remove_file(path)
    };

    match result {
        Ok(()) => true,
        Err(error) => {
            if error.raw_os_error() == Some(32) {
                // ERROR_SHARING_VIOLATION
                safe_print(format!(
                    "Trying to delete \"{}\" failed because it was in use by another process.",
                    path.display()
                ));
            } else {
                safe_print(format!(
                    "Trying to delete \"{}\" caused the following error: {}",
                    path.display(),
                    error
                ));
            }

            false
        }
    }
}

// ============================================================================
// Merge / mirror
// ============================================================================

/// `merge_file( source, dest ) -> bool`
///
/// Moves a file unless the destination already has the same size and modified
/// date, in which case only the source is deleted.
pub fn merge_file(source: &Path, dest: &Path) -> HydrusResult<bool> {
    let source_stat = match fs::metadata(source) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return Err(HydrusException::new(
                ErrorKind::FileMissing,
                format!(
                    "Cannot file-merge \"{}\" to \"{}\"--the source does not exist!",
                    source.display(),
                    dest.display()
                ),
            ));
        }
        Err(error) if oserror_is_device_access_trouble(&error) => {
            return Err(io_failure(
                ErrorKind::DirectoryMissing,
                format!(
                    "Cannot file-merge \"{}\" to \"{}\"--the source device is having trouble",
                    source.display(),
                    dest.display()
                ),
                &error,
            ));
        }
        Err(error) => {
            return Err(io_failure(
                ErrorKind::Veto,
                format!(
                    "Cannot file-merge \"{}\" to \"{}\"",
                    source.display(),
                    dest.display()
                ),
                &error,
            ));
        }
    };

    if stat_is_dir(&source_stat) {
        return Err(HydrusException::new(
            ErrorKind::Veto,
            format!(
                "Cannot file-merge \"{}\" to \"{}\"--the source is a directory, not a file!",
                source.display(),
                dest.display()
            ),
        ));
    }

    let dest_stat = path_metadata(dest).map_err(|error| {
        io_failure(
            ErrorKind::Veto,
            format!(
                "Cannot file-merge \"{}\" to \"{}\"",
                source.display(),
                dest.display()
            ),
            &error,
        )
    })?;

    if let Some(dest_stat) = dest_stat {
        if stat_is_dir(&dest_stat) {
            return Err(HydrusException::new(
                ErrorKind::Veto,
                format!(
                    "Cannot file-merge \"{}\" to \"{}\"--the destination is a directory, not a file!",
                    source.display(),
                    dest.display()
                ),
            ));
        }

        if paths_are_same_file(source, dest) {
            return Err(HydrusException::new(
                ErrorKind::Veto,
                format!(
                    "Woah, \"{}\" and \"{}\" are the same file!",
                    source.display(),
                    dest.display()
                ),
            ));
        }

        if dest_stat_has_same_size_and_date_as_source(&source_stat, &dest_stat) {
            let _ = delete_path(source);

            return Ok(false);
        }

        #[cfg(windows)]
        try_to_make_file_writeable(dest, &dest_stat);
    }

    retry_blocking_io_call(|| permission_recovering_move_io(source, dest)).map_err(|error| {
        io_failure(
            ErrorKind::Veto,
            format!(
                "Could not file-merge \"{}\" to \"{}\"",
                source.display(),
                dest.display()
            ),
            &error,
        )
    })?;

    Ok(true)
}

fn merge_tree_walk(
    source_root: &Path,
    current_dir: &Path,
    dest_root: &Path,
    text_update_hook: &mut dyn FnMut(&str),
) -> HydrusResult<()> {
    text_update_hook(&format!("Copying {}.", current_dir.display()));

    let relative_path = current_dir
        .strip_prefix(source_root)
        .unwrap_or_else(|_| Path::new(""));
    let dest_dir = if relative_path.as_os_str().is_empty() {
        dest_root.to_path_buf()
    } else {
        dest_root.join(relative_path)
    };

    let mut subdirs = Vec::new();
    let mut symlink_dirs = Vec::new();
    let mut files = Vec::new();

    let entries = fs::read_dir(current_dir).map_err(|error| {
        io_failure(
            ErrorKind::DirectoryMissing,
            format!("Could not read \"{}\"", current_dir.display()),
            &error,
        )
    })?;

    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(file_type) = entry.file_type() else {
            continue;
        };

        if file_type.is_dir() {
            subdirs.push(path);
        } else if file_type.is_symlink() && path.is_dir() {
            // os.walk lists symlinked directories but does not follow them.
            symlink_dirs.push(path);
        } else {
            files.push(path);
        }
    }

    for source_path in subdirs {
        let Some(name) = source_path.file_name() else {
            continue;
        };
        let dest_path = dest_dir.join(name);

        make_sure_directory_exists(&dest_path)?;
        safe_copystat(&source_path, &dest_path)?;
        merge_tree_walk(source_root, &source_path, dest_root, text_update_hook)?;
    }

    for source_path in symlink_dirs {
        let Some(name) = source_path.file_name() else {
            continue;
        };
        let dest_path = dest_dir.join(name);

        make_sure_directory_exists(&dest_path)?;
        safe_copystat(&source_path, &dest_path)?;
    }

    for source_path in files {
        let Some(name) = source_path.file_name() else {
            continue;
        };
        let dest_path = dest_dir.join(name);

        mirror_file(&source_path, &dest_path).map_err(|error| {
            HydrusException::new(
                ErrorKind::Veto,
                format!(
                    "While trying to merge \"{}\" into \"{}\", mirroring \"{}\" to \"{}\" failed: {}",
                    source_root.display(),
                    dest_root.display(),
                    source_path.display(),
                    dest_path.display(),
                    error
                ),
            )
        })?;
    }

    Ok(())
}

fn merge_tree_impl(
    source: &Path,
    dest: &Path,
    text_update_hook: &mut dyn FnMut(&str),
) -> HydrusResult<()> {
    let source_stat = match fs::metadata(source) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return Err(HydrusException::new(
                ErrorKind::DirectoryMissing,
                format!(
                    "Cannot directory-merge \"{}\" to \"{}\"--the source does not exist!",
                    source.display(),
                    dest.display()
                ),
            ));
        }
        Err(error) if oserror_is_device_access_trouble(&error) => {
            return Err(io_failure(
                ErrorKind::DirectoryMissing,
                format!(
                    "Cannot directory-merge \"{}\" to \"{}\"--the source device is having trouble",
                    source.display(),
                    dest.display()
                ),
                &error,
            ));
        }
        Err(error) => {
            return Err(io_failure(
                ErrorKind::Veto,
                format!(
                    "Cannot directory-merge \"{}\" to \"{}\"",
                    source.display(),
                    dest.display()
                ),
                &error,
            ));
        }
    };

    if stat_is_file(&source_stat) {
        return Err(HydrusException::new(
            ErrorKind::Veto,
            format!(
                "Cannot directory-merge \"{}\" to \"{}\"--the source is a file, not a directory!",
                source.display(),
                dest.display()
            ),
        ));
    }

    let dest_stat = path_metadata(dest).map_err(|error| {
        io_failure(
            ErrorKind::Veto,
            format!(
                "Cannot directory-merge \"{}\" to \"{}\"",
                source.display(),
                dest.display()
            ),
            &error,
        )
    })?;

    if let Some(dest_stat) = dest_stat {
        if stat_is_file(&dest_stat) {
            return Err(HydrusException::new(
                ErrorKind::Veto,
                format!(
                    "Cannot directory-merge \"{}\" to \"{}\"--the destination is a file, not a directory!",
                    source.display(),
                    dest.display()
                ),
            ));
        }

        if paths_are_same_file(source, dest) {
            return Err(HydrusException::new(
                ErrorKind::Veto,
                format!(
                    "Woah, \"{}\" and \"{}\" are the same directory!",
                    source.display(),
                    dest.display()
                ),
            ));
        }

        merge_tree_walk(source, source, dest, text_update_hook)?;

        if !delete_path(source) {
            return Err(HydrusException::new(
                ErrorKind::Veto,
                format!(
                    "Merged \"{}\" into \"{}\", but could not delete the source.",
                    source.display(),
                    dest.display()
                ),
            ));
        }
    } else {
        match retry_blocking_io_call(|| permission_recovering_move_io(source, dest)) {
            Ok(()) => {}
            Err(error) if dest.exists() => {
                // A partial move can leave a destination behind; fall back to
                // the walking merge, exactly like Python.
                let _ = error;

                merge_tree_impl(source, dest, text_update_hook)?;
            }
            Err(error) => {
                return Err(io_failure(
                    ErrorKind::Veto,
                    format!(
                        "Could not directory-merge \"{}\" to \"{}\"",
                        source.display(),
                        dest.display()
                    ),
                    &error,
                ));
            }
        }
    }

    Ok(())
}

/// `merge_tree( source, dest, text_update_hook = None )`
pub fn merge_tree(
    source: &Path,
    dest: &Path,
    text_update_hook: Option<&mut dyn FnMut(&str)>,
) -> HydrusResult<()> {
    let mut no_op = |_: &str| {};

    let hook: &mut dyn FnMut(&str) = match text_update_hook {
        Some(hook) => hook,
        None => &mut no_op,
    };

    merge_tree_impl(source, dest, hook)
}

/// `mirror_file( source, dest ) -> bool`
///
/// Copies a file unless it already exists with the same size and modified
/// date. Returns whether an actual copy/overwrite happened.
pub fn mirror_file(source: &Path, dest: &Path) -> HydrusResult<bool> {
    let source_stat = match fs::metadata(source) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return Err(HydrusException::new(
                ErrorKind::FileMissing,
                format!(
                    "Cannot file-mirror \"{}\" to \"{}\"--the source does not exist!",
                    source.display(),
                    dest.display()
                ),
            ));
        }
        Err(error) if oserror_is_device_access_trouble(&error) => {
            return Err(io_failure(
                ErrorKind::DirectoryMissing,
                format!(
                    "Cannot file-mirror \"{}\" to \"{}\"--the source device is having trouble",
                    source.display(),
                    dest.display()
                ),
                &error,
            ));
        }
        Err(error) => {
            return Err(io_failure(
                ErrorKind::Veto,
                format!(
                    "Cannot file-mirror \"{}\" to \"{}\"",
                    source.display(),
                    dest.display()
                ),
                &error,
            ));
        }
    };

    if stat_is_dir(&source_stat) {
        return Err(HydrusException::new(
            ErrorKind::Veto,
            format!(
                "Cannot file-mirror \"{}\" to \"{}\"--the source is a directory, not a file!",
                source.display(),
                dest.display()
            ),
        ));
    }

    let dest_stat = path_metadata(dest).map_err(|error| {
        io_failure(
            ErrorKind::Veto,
            format!(
                "Cannot file-mirror \"{}\" to \"{}\"",
                source.display(),
                dest.display()
            ),
            &error,
        )
    })?;

    if let Some(dest_stat) = dest_stat {
        if stat_is_dir(&dest_stat) {
            return Err(HydrusException::new(
                ErrorKind::Veto,
                format!(
                    "Cannot file-mirror \"{}\" to \"{}\"--the destination is a directory, not a file!",
                    source.display(),
                    dest.display()
                ),
            ));
        }

        if paths_are_same_file(source, dest) {
            return Ok(false);
        }

        if dest_stat_has_same_size_and_date_as_source(&source_stat, &dest_stat) {
            return Ok(false);
        }

        #[cfg(windows)]
        try_to_make_file_writeable(dest, &dest_stat);
    }

    safe_copy2(source, dest).map_err(|error| {
        HydrusException::new(
            ErrorKind::Veto,
            format!(
                "Could not file-mirror \"{}\" to \"{}\": {}",
                source.display(),
                dest.display(),
                error
            ),
        )
    })?;

    Ok(true)
}

fn mirror_tree_walk(
    source_root: &Path,
    current_dir: &Path,
    dest_root: &Path,
    text_update_hook: &mut dyn FnMut(&str),
    is_cancelled_hook: &mut dyn FnMut() -> bool,
    deletee_paths: &mut HashSet<PathBuf>,
) -> HydrusResult<bool> {
    if is_cancelled_hook() {
        return Ok(true);
    }

    text_update_hook(&format!("Copying {}.", current_dir.display()));

    let relative_path = current_dir
        .strip_prefix(source_root)
        .unwrap_or_else(|_| Path::new(""));
    let dest_dir = if relative_path.as_os_str().is_empty() {
        dest_root.to_path_buf()
    } else {
        dest_root.join(relative_path)
    };

    make_sure_directory_exists(&dest_dir)?;

    let mut surplus_dest_paths: HashSet<PathBuf> = HashSet::new();

    for entry in fs::read_dir(&dest_dir).map_err(|error| {
        io_failure(
            ErrorKind::DirectoryMissing,
            format!("Could not read \"{}\"", dest_dir.display()),
            &error,
        )
    })? {
        if let Ok(entry) = entry {
            surplus_dest_paths.insert(entry.path());
        }
    }

    let mut subdirs = Vec::new();
    let mut symlink_dirs = Vec::new();
    let mut files = Vec::new();

    for entry in fs::read_dir(current_dir).map_err(|error| {
        io_failure(
            ErrorKind::DirectoryMissing,
            format!("Could not read \"{}\"", current_dir.display()),
            &error,
        )
    })? {
        let Ok(entry) = entry else {
            continue;
        };

        let path = entry.path();
        let Ok(file_type) = entry.file_type() else {
            continue;
        };

        if file_type.is_dir() {
            subdirs.push(path);
        } else if file_type.is_symlink() && path.is_dir() {
            symlink_dirs.push(path);
        } else {
            files.push(path);
        }
    }

    for source_path in subdirs {
        let Some(name) = source_path.file_name() else {
            continue;
        };
        let dest_path = dest_dir.join(name);

        surplus_dest_paths.remove(&dest_path);

        make_sure_directory_exists(&dest_path)?;
        safe_copystat(&source_path, &dest_path)?;

        if mirror_tree_walk(
            source_root,
            &source_path,
            dest_root,
            text_update_hook,
            is_cancelled_hook,
            deletee_paths,
        )? {
            return Ok(true);
        }
    }

    for source_path in symlink_dirs {
        let Some(name) = source_path.file_name() else {
            continue;
        };
        let dest_path = dest_dir.join(name);

        surplus_dest_paths.remove(&dest_path);

        make_sure_directory_exists(&dest_path)?;
        safe_copystat(&source_path, &dest_path)?;
    }

    for source_path in files {
        let Some(name) = source_path.file_name() else {
            continue;
        };
        let dest_path = dest_dir.join(name);

        surplus_dest_paths.remove(&dest_path);

        mirror_file(&source_path, &dest_path).map_err(|error| {
            HydrusException::new(
                ErrorKind::Veto,
                format!(
                    "While trying to mirror \"{}\" into \"{}\", copying \"{}\" to \"{}\" failed: {}",
                    source_root.display(),
                    dest_root.display(),
                    source_path.display(),
                    dest_path.display(),
                    error
                ),
            )
        })?;
    }

    deletee_paths.extend(surplus_dest_paths);

    Ok(false)
}

/// `mirror_tree( source, dest, text_update_hook = None, is_cancelled_hook = None )`
///
/// Makes `dest` look exactly like `source`, deleting surplus entries in `dest`.
pub fn mirror_tree(
    source: &Path,
    dest: &Path,
    text_update_hook: Option<&mut dyn FnMut(&str)>,
    is_cancelled_hook: Option<&mut dyn FnMut() -> bool>,
) -> HydrusResult<()> {
    let source_stat = match fs::metadata(source) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return Err(HydrusException::new(
                ErrorKind::DirectoryMissing,
                format!(
                    "Cannot directory-mirror \"{}\" to \"{}\"--the source does not exist!",
                    source.display(),
                    dest.display()
                ),
            ));
        }
        Err(error) if oserror_is_device_access_trouble(&error) => {
            return Err(io_failure(
                ErrorKind::DirectoryMissing,
                format!(
                    "Cannot directory-mirror \"{}\" to \"{}\"--the source device is having trouble",
                    source.display(),
                    dest.display()
                ),
                &error,
            ));
        }
        Err(error) => {
            return Err(io_failure(
                ErrorKind::Veto,
                format!(
                    "Cannot directory-mirror \"{}\" to \"{}\"",
                    source.display(),
                    dest.display()
                ),
                &error,
            ));
        }
    };

    if stat_is_file(&source_stat) {
        return Err(HydrusException::new(
            ErrorKind::Veto,
            format!(
                "Cannot directory-mirror \"{}\" to \"{}\"--the source is a file, not a directory!",
                source.display(),
                dest.display()
            ),
        ));
    }

    let dest_stat = path_metadata(dest).map_err(|error| {
        io_failure(
            ErrorKind::Veto,
            format!(
                "Cannot directory-mirror \"{}\" to \"{}\"",
                source.display(),
                dest.display()
            ),
            &error,
        )
    })?;

    if let Some(dest_stat) = dest_stat {
        if stat_is_file(&dest_stat) {
            return Err(HydrusException::new(
                ErrorKind::Veto,
                format!(
                    "Cannot directory-mirror \"{}\" to \"{}\"--the destination is a file, not a directory!",
                    source.display(),
                    dest.display()
                ),
            ));
        }

        if paths_are_same_file(source, dest) {
            return Ok(());
        }
    }

    let mut no_op_text = |_: &str| {};
    let mut no_op_cancel = || false;

    let text_hook: &mut dyn FnMut(&str) = match text_update_hook {
        Some(hook) => hook,
        None => &mut no_op_text,
    };
    let cancel_hook: &mut dyn FnMut() -> bool = match is_cancelled_hook {
        Some(hook) => hook,
        None => &mut no_op_cancel,
    };

    make_sure_directory_exists(dest)?;

    let mut deletee_paths = HashSet::new();

    let cancelled = mirror_tree_walk(
        source,
        source,
        dest,
        text_hook,
        cancel_hook,
        &mut deletee_paths,
    )?;

    if cancelled {
        return Ok(());
    }

    for deletee_path in deletee_paths {
        let _ = delete_path(&deletee_path);
    }

    Ok(())
}

// ============================================================================
// Directories, permissions and device state
// ============================================================================

/// `make_sure_directory_exists( path )`
pub fn make_sure_directory_exists(path: &Path) -> HydrusResult<()> {
    match fs::metadata(path) {
        Ok(path_stat) => {
            if stat_is_dir(&path_stat) {
                return Ok(());
            }

            return Err(HydrusException::new(
                ErrorKind::Veto,
                format!(
                    "Cannot create the directory \"{}\" because it already exists as a normal file!",
                    path.display()
                ),
            ));
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) if oserror_is_device_access_trouble(&error) => {
            return Err(io_failure(
                ErrorKind::DirectoryMissing,
                format!(
                    "While trying to ensure the directory \"{}\" exists, there appeared to be a device access issue. Is something not plugged in?",
                    path.display()
                ),
                &error,
            ));
        }
        Err(error) => {
            return Err(io_failure(
                ErrorKind::DirectoryMissing,
                format!("Could not examine \"{}\"", path.display()),
                &error,
            ));
        }
    }

    match fs::create_dir_all(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Err(io_failure(
            ErrorKind::DirectoryMissing,
            format!(
                "While trying to ensure the directory \"{}\" exists, none of the possible parent folders seem to exist either! Is this device having trouble?",
                path.display()
            ),
            &error,
        )),
        Err(error) if oserror_is_device_access_trouble(&error) => Err(io_failure(
            ErrorKind::DirectoryMissing,
            format!(
                "While trying to ensure the directory \"{}\" exists, there appeared to be a device access issue. Is something not plugged in?",
                path.display()
            ),
            &error,
        )),
        Err(error) => Err(io_failure(
            ErrorKind::Veto,
            format!("Could not create the directory \"{}\"", path.display()),
            &error,
        )),
    }
}

/// `file_is_writeable( path ) -> bool`
///
/// Python uses `os.access( path, os.W_OK )`. This checks the mode/read-only
/// bit; ACLs and effective uid are not modelled.
pub fn file_is_writeable(path: &Path) -> bool {
    let Ok(metadata) = fs::metadata(path) else {
        return false;
    };

    if metadata.permissions().readonly() {
        return false;
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;

        if metadata.mode() & 0o222 == 0 {
            return false;
        }
    }

    true
}

/// `directory_is_writeable( path ) -> bool`
///
/// Actually creates and removes a small probe file, because Windows
/// `Program Files` passes simpler permission checks.
pub fn directory_is_writeable(path: &Path) -> HydrusResult<bool> {
    if !potential_path_device_is_connected(path) {
        return Err(HydrusException::new(
            ErrorKind::DirectoryMissing,
            format!(
                "Cannot figure out if \"{}\" is writeable-to because its device does not seem to be mounted!",
                path.display()
            ),
        ));
    }

    if make_sure_directory_exists(path).is_err() {
        return Ok(false);
    }

    if !path.exists() || !path.is_dir() {
        return Ok(false);
    }

    let test_path = path.join(format!(
        "hpt_{:016x}{:016x}.txt",
        rand::random::<u64>(),
        rand::random::<u64>()
    ));

    let probe = b"Hydrus tested this directory for write permissions. If this file still exists, this directory can be written to but not deleted from.";

    let written = fs::write(&test_path, probe).is_ok();

    if !written {
        return Ok(false);
    }

    if fs::remove_file(&test_path).is_err() {
        return Ok(false);
    }

    Ok(true)
}

/// `try_to_give_File_nice_permission_bits( path )`
pub fn try_to_give_file_nice_permission_bits(path: &Path) {
    if DO_NOT_DO_CHMOD_MODE.load(Ordering::Relaxed) {
        return;
    }

    if !path.exists() {
        return;
    }

    #[cfg(windows)]
    {
        match fs::metadata(path) {
            Ok(metadata) => {
                let mut permissions = metadata.permissions();

                if permissions.readonly() {
                    permissions.set_readonly(false);

                    if let Err(error) = fs::set_permissions(path, permissions) {
                        safe_print(format!(
                            "Wanted to add read and write permission to \"{}\", but had an error: {}",
                            path.display(),
                            error
                        ));
                    }
                }
            }
            Err(error) => {
                safe_print(format!(
                    "Wanted to add read and write permission to \"{}\", but had an error: {}",
                    path.display(),
                    error
                ));
            }
        }
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let Ok(metadata) = fs::metadata(path) else {
            return;
        };

        let current_bits = metadata.mode();
        let desired_bits = 0o644_u32 & !*PROCESS_UMASK;

        if current_bits & desired_bits != desired_bits {
            let permissions = fs::Permissions::from_mode(current_bits | desired_bits);

            if let Err(error) = fs::set_permissions(path, permissions) {
                safe_print(format!(
                    "Wanted to add read and write permission to \"{}\", but had an error: {}",
                    path.display(),
                    error
                ));
            }
        }
    }

    #[cfg(not(any(unix, windows)))]
    {
        let _ = path;
    }
}

/// `try_to_make_file_writeable( path, path_stat )`
pub fn try_to_make_file_writeable(path: &Path, path_stat: &Metadata) {
    if DO_NOT_DO_CHMOD_MODE.load(Ordering::Relaxed) {
        return;
    }

    if file_is_writeable(path) {
        return;
    }

    #[cfg(windows)]
    {
        let _ = path_stat;

        match fs::metadata(path) {
            Ok(metadata) => {
                let mut permissions = metadata.permissions();

                if permissions.readonly() {
                    permissions.set_readonly(false);

                    if let Err(error) = fs::set_permissions(path, permissions) {
                        safe_print(format!(
                            "Wanted to add user write permission to \"{}\", but had an error: {}",
                            path.display(),
                            error
                        ));
                    }
                }
            }
            Err(error) => {
                safe_print(format!(
                    "Wanted to add user write permission to \"{}\", but had an error: {}",
                    path.display(),
                    error
                ));
            }
        }
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let current_bits = path_stat.mode();
        let desired_bits = 0o200_u32;

        if current_bits & desired_bits != desired_bits {
            let permissions = fs::Permissions::from_mode(current_bits | desired_bits);

            if let Err(error) = fs::set_permissions(path, permissions) {
                safe_print(format!(
                    "Wanted to add user write permission to \"{}\", but had an error: {}",
                    path.display(),
                    error
                ));
            }
        }
    }

    #[cfg(not(any(unix, windows)))]
    {
        let _ = (path, path_stat);
    }
}

/// `get_device_id( path )`
pub fn get_device_id(path: &Path) -> HydrusResult<u64> {
    let metadata = fs::metadata(path).map_err(|error| {
        io_failure(
            ErrorKind::FileMissing,
            format!("Could not stat \"{}\"", path.display()),
            &error,
        )
    })?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;

        Ok(metadata.dev())
    }

    #[cfg(windows)]
    {
        let _ = metadata;

        windows_volume_serial(path).map(u64::from).ok_or_else(|| {
            HydrusException::new(
                ErrorKind::Veto,
                format!(
                    "Could not get the volume serial number for \"{}\"",
                    path.display()
                ),
            )
        })
    }

    #[cfg(not(any(unix, windows)))]
    {
        let _ = metadata;
        Ok(0)
    }
}

// ============================================================================
// Disk space and partitions
// ============================================================================

/// The subset of `psutil.sdiskpart` that hydrus uses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartitionInfo {
    pub device: String,
    pub mountpoint: PathBuf,
    pub fstype: String,
    pub opts: String,
}

#[cfg(windows)]
fn wide_path(path: &Path) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;

    path.as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

#[cfg(windows)]
fn wide_to_string(buffer: &[u16]) -> String {
    let length = buffer
        .iter()
        .position(|&character| character == 0)
        .unwrap_or(buffer.len());

    String::from_utf16_lossy(&buffer[..length])
}

#[cfg(windows)]
fn volume_root(path: &Path) -> Option<PathBuf> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir().ok()?.join(path)
    };

    absolute.ancestors().last().map(Path::to_path_buf)
}

#[cfg(windows)]
fn windows_volume_serial(path: &Path) -> Option<u32> {
    let root = volume_root(path)?;
    let root_wide = wide_path(&root);

    let mut volume_name = [0_u16; 256];
    let mut file_system_name = [0_u16; 256];
    let mut volume_serial = 0_u32;
    let mut maximum_component_length = 0_u32;
    let mut file_system_flags = 0_u32;

    // SAFETY: the buffers are valid for their declared lengths and the wide
    // path is nul-terminated.
    let result = unsafe {
        GetVolumeInformationW(
            root_wide.as_ptr(),
            volume_name.as_mut_ptr(),
            volume_name.len() as u32,
            &mut volume_serial,
            &mut maximum_component_length,
            &mut file_system_flags,
            file_system_name.as_mut_ptr(),
            file_system_name.len() as u32,
        )
    };

    if result == 0 {
        None
    } else {
        Some(volume_serial)
    }
}

#[cfg(windows)]
fn partition_info_windows(path: &Path) -> Option<PartitionInfo> {
    let root = volume_root(path)?;
    let root_wide = wide_path(&root);

    let mut volume_name = [0_u16; 256];
    let mut file_system_name = [0_u16; 256];
    let mut volume_serial = 0_u32;
    let mut maximum_component_length = 0_u32;
    let mut file_system_flags = 0_u32;

    // SAFETY: the buffers are valid for their declared lengths and the wide
    // path is nul-terminated.
    let result = unsafe {
        GetVolumeInformationW(
            root_wide.as_ptr(),
            volume_name.as_mut_ptr(),
            volume_name.len() as u32,
            &mut volume_serial,
            &mut maximum_component_length,
            &mut file_system_flags,
            file_system_name.as_mut_ptr(),
            file_system_name.len() as u32,
        )
    };

    if result == 0 {
        return None;
    }

    Some(PartitionInfo {
        device: root.to_string_lossy().into_owned(),
        mountpoint: root,
        fstype: wide_to_string(&file_system_name),
        opts: String::new(),
    })
}

#[cfg(unix)]
fn unescape_mount_field(field: &str) -> String {
    let bytes = field.as_bytes();
    let mut output = Vec::with_capacity(bytes.len());
    let mut index = 0;

    while index < bytes.len() {
        if bytes[index] == b'\\' && index + 3 < bytes.len() {
            let octets = &bytes[index + 1..index + 4];

            if octets.iter().all(|byte| (b'0'..=b'7').contains(byte)) {
                let value = u32::from(octets[0] - b'0') * 64
                    + u32::from(octets[1] - b'0') * 8
                    + u32::from(octets[2] - b'0');

                output.push(value as u8);
                index += 4;
                continue;
            }
        }

        output.push(bytes[index]);
        index += 1;
    }

    String::from_utf8_lossy(&output).into_owned()
}

#[cfg(target_os = "linux")]
fn partition_info_linux(path: &Path) -> Option<PartitionInfo> {
    let contents = fs::read_to_string("/proc/mounts")
        .or_else(|_| fs::read_to_string("/proc/self/mounts"))
        .ok()?;

    let path_lower = path.to_string_lossy().to_lowercase();
    let mut best: Option<PartitionInfo> = None;

    for line in contents.lines() {
        let fields: Vec<&str> = line.split_whitespace().collect();

        if fields.len() < 4 {
            continue;
        }

        let mountpoint = unescape_mount_field(fields[1]);

        if !path_lower.starts_with(&mountpoint.to_lowercase()) {
            continue;
        }

        let is_better = best.as_ref().is_none_or(|current| {
            mountpoint.len() > current.mountpoint.as_os_str().len()
        });

        if is_better {
            best = Some(PartitionInfo {
                device: unescape_mount_field(fields[0]),
                mountpoint: PathBuf::from(mountpoint),
                fstype: fields[2].to_string(),
                opts: fields[3].to_string(),
            });
        }
    }

    best
}

#[cfg(all(unix, not(target_os = "linux")))]
fn partition_info_unix_mount(path: &Path) -> Option<PartitionInfo> {
    let output = Command::new("mount").output().ok()?;

    if !output.status.success() {
        return None;
    }

    let text = String::from_utf8_lossy(&output.stdout);
    let path_lower = path.to_string_lossy().to_lowercase();
    let mut best: Option<PartitionInfo> = None;

    for line in text.lines() {
        let Some((device, rest)) = line.split_once(" on ") else {
            continue;
        };
        let Some((mountpoint, type_and_options)) = rest.split_once(" (") else {
            continue;
        };

        let mountpoint = unescape_mount_field(mountpoint);

        if !path_lower.starts_with(&mountpoint.to_lowercase()) {
            continue;
        }

        let fstype = type_and_options
            .split(|character: char| character == ',' || character == ')')
            .next()
            .unwrap_or("")
            .trim()
            .to_string();

        let is_better = best.as_ref().is_none_or(|current| {
            mountpoint.len() > current.mountpoint.as_os_str().len()
        });

        if is_better {
            best = Some(PartitionInfo {
                device: unescape_mount_field(device),
                mountpoint: PathBuf::from(mountpoint),
                fstype,
                opts: type_and_options.to_string(),
            });
        }
    }

    best
}

/// `get_partition_info( path ) -> FakeDiskPart | None`
///
/// The Python version is `lru_cache`d; this is uncached.
pub fn get_partition_info(path: &Path) -> Option<PartitionInfo> {
    #[cfg(windows)]
    {
        return partition_info_windows(path);
    }

    #[cfg(target_os = "linux")]
    {
        return partition_info_linux(path);
    }

    #[cfg(all(unix, not(target_os = "linux")))]
    {
        return partition_info_unix_mount(path);
    }

    #[cfg(not(any(unix, windows)))]
    {
        let _ = path;
        None
    }
}

/// `get_file_system_type( path ) -> str | None`
pub fn get_file_system_type(path: &Path) -> Option<String> {
    get_partition_info(path).map(|partition_info| partition_info.fstype)
}

#[cfg(windows)]
fn disk_usage_windows(path: &Path) -> Option<(u64, u64)> {
    let root = volume_root(path)?;
    let root_wide = wide_path(&root);

    let mut free_bytes_available_to_caller = 0_u64;
    let mut total_number_of_bytes = 0_u64;
    let mut total_number_of_free_bytes = 0_u64;

    // SAFETY: the path is nul-terminated and the out-pointers are valid.
    let result = unsafe {
        GetDiskFreeSpaceExW(
            root_wide.as_ptr(),
            &mut free_bytes_available_to_caller,
            &mut total_number_of_bytes,
            &mut total_number_of_free_bytes,
        )
    };

    if result == 0 {
        return None;
    }

    let _ = total_number_of_free_bytes;

    Some((free_bytes_available_to_caller, total_number_of_bytes))
}

#[cfg(all(target_os = "linux", target_pointer_width = "64"))]
fn disk_usage_statvfs(path: &Path) -> Option<(u64, u64)> {
    use std::os::unix::ffi::OsStrExt;

    let c_path = CString::new(path.as_os_str().as_bytes()).ok()?;
    // SAFETY: this is a plain C integer struct; zeroed is a valid value.
    let mut stat: LinuxStatVfs = unsafe { std::mem::zeroed() };

    // SAFETY: `c_path` is a valid nul-terminated string and `stat` is a valid,
    // correctly sized out-parameter.
    let result = unsafe { statvfs(c_path.as_ptr(), &mut stat) };

    if result != 0 {
        return None;
    }

    let fragment_size = if stat.f_frsize == 0 {
        stat.f_bsize
    } else {
        stat.f_frsize
    };

    if fragment_size == 0 {
        return None;
    }

    Some((
        stat.f_bavail.saturating_mul(fragment_size),
        stat.f_blocks.saturating_mul(fragment_size),
    ))
}

#[cfg(all(unix, not(all(target_os = "linux", target_pointer_width = "64"))))]
fn disk_usage_df(path: &Path) -> Option<(u64, u64)> {
    let output = Command::new("df")
        .arg("-P")
        .arg("-k")
        .arg(path)
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let text = String::from_utf8_lossy(&output.stdout);
    let line = text.lines().nth(1)?;
    let fields: Vec<&str> = line.split_whitespace().collect();

    if fields.len() < 4 {
        return None;
    }

    let total_kib: u64 = fields[1].parse().ok()?;
    let available_kib: u64 = fields[3].parse().ok()?;

    Some((
        available_kib.saturating_mul(1024),
        total_kib.saturating_mul(1024),
    ))
}

fn disk_usage(path: &Path) -> Option<(u64, u64)> {
    #[cfg(windows)]
    {
        return disk_usage_windows(path);
    }

    #[cfg(all(target_os = "linux", target_pointer_width = "64"))]
    {
        return disk_usage_statvfs(path);
    }

    #[cfg(all(unix, not(all(target_os = "linux", target_pointer_width = "64"))))]
    {
        return disk_usage_df(path);
    }

    #[cfg(not(any(unix, windows)))]
    {
        let _ = path;
        None
    }
}

/// `get_free_space( path ) -> int | None`
pub fn get_free_space(path: &Path) -> Option<u64> {
    disk_usage(path).map(|(free, _total)| free)
}

/// `get_total_space( path ) -> int | None`
pub fn get_total_space(path: &Path) -> Option<u64> {
    disk_usage(path).map(|(_free, total)| total)
}

/// `potential_path_device_is_connected( path ) -> bool`
pub fn potential_path_device_is_connected(path: &Path) -> bool {
    match fs::metadata(path) {
        Ok(_) => true,
        Err(error) if error.kind() == io::ErrorKind::NotFound => true,
        Err(error) => !oserror_is_device_access_trouble(&error),
    }
}

// ============================================================================
// Existence, recycling and filename safety
// ============================================================================

/// `path_exists_and_is_dir( path ) -> bool`
pub fn path_exists_and_is_dir(path: &Path) -> bool {
    match fs::metadata(path) {
        Ok(path_stat) => stat_is_dir(&path_stat),
        Err(error) if error.kind() == io::ErrorKind::NotFound => false,
        Err(error) if oserror_is_device_access_trouble(&error) => false,
        Err(_) => false,
    }
}

/// `path_is_free( path ) -> bool`
///
/// On Windows this tries the rename-to-self trick and then a read. On POSIX it
/// opens the file and takes a non-blocking advisory `flock`.
pub fn path_is_free(path: &Path) -> bool {
    if PLATFORM_WINDOWS {
        let stat_result = match fs::metadata(path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return false,
            Err(error) => {
                safe_print(format!(
                    "Could not stat \"{}\": {}",
                    path.display(),
                    error
                ));

                return false;
            }
        };

        if !stat_result.permissions().readonly() {
            match fs::rename(path, path) {
                Ok(()) => return true,
                Err(error) => {
                    safe_print(format!(
                        "Already in use/inaccessible: \"{}\" ({})",
                        path.display(),
                        error
                    ));

                    return false;
                }
            }
        }

        match fs::File::open(path) {
            Ok(_) => true,
            Err(error) => {
                safe_print(format!(
                    "Could not open the file \"{}\": {}",
                    path.display(),
                    error
                ));

                false
            }
        }
    } else {
        let Ok(file) = fs::File::open(path) else {
            safe_print(format!("Could not open the file \"{}\".", path.display()));

            return false;
        };

        if !DO_FLOCK_ALREADY_IN_USE_TEST_IN_POSIX.load(Ordering::Relaxed) {
            return true;
        }

        #[cfg(unix)]
        {
            use std::os::fd::AsRawFd;

            // SAFETY: `file` owns a valid open descriptor for this scope.
            let result = unsafe { flock(file.as_raw_fd(), LOCK_EX | LOCK_NB) };

            if result == 0 {
                return true;
            }

            match io::Error::last_os_error().raw_os_error() {
                // EAGAIN/EWOULDBLOCK: somebody else has an exclusive lock.
                Some(11) | Some(35) => {
                    safe_print(format!(
                        "Already in use/inaccessible due to exclusive lock: \"{}\"",
                        path.display()
                    ));

                    false
                }
                // EACCES/EPERM: locking is not permitted here; treat as free.
                Some(13) | Some(1) => true,
                _ => true,
            }
        }

        #[cfg(not(unix))]
        {
            let _ = &file;

            true
        }
    }
}

/// `recycle_path( path )`
///
/// TODO: platform trash (send2trash/IFileOperation) belongs to a higher layer.
/// Until then this is a retrying *permanent* delete, with the same retry
/// behaviour as the Python `send2trash` loop.
pub fn recycle_path(path: &Path) -> bool {
    if hydrus_globals::file_report_mode() {
        safe_print(format!("Recycling {}", path.display()));
    }

    const MAX_NUM_ATTEMPTS: usize = 3;

    let path_stat_nofollow = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return true,
        Err(error) => {
            safe_print(format!(
                "Trying to recycle \"{}\" failed: {}",
                path.display(),
                error
            ));

            return false;
        }
    };

    #[cfg(windows)]
    try_to_make_file_writeable(path, &path_stat_nofollow);

    for attempt in 0..MAX_NUM_ATTEMPTS {
        if delete_path(path) {
            return true;
        }

        if attempt + 1 < MAX_NUM_ATTEMPTS {
            std::thread::sleep(Duration::from_millis(500));
        }
    }

    safe_print(format!(
        "Could not recycle or delete \"{}\" after {MAX_NUM_ATTEMPTS} attempts.",
        path.display()
    ));

    false
}

/// `sanitize_filename( filename, force_ntfs_rules ) -> str`
pub fn sanitize_filename(filename: &str, force_ntfs_rules: bool) -> String {
    let use_ntfs_rules = PLATFORM_WINDOWS || force_ntfs_rules;

    let mut clean_filename: String = if use_ntfs_rules {
        // \, /, :, *, ?, ", <, >, |
        filename
            .chars()
            .map(|character| {
                if matches!(
                    character,
                    '\\' | '/' | ':' | '*' | '?' | '"' | '<' | '>' | '|'
                ) {
                    '_'
                } else {
                    character
                }
            })
            .collect()
    } else {
        filename
            .chars()
            .map(|character| if character == '/' { '_' } else { character })
            .collect()
    };

    if use_ntfs_rules {
        while clean_filename.ends_with('.') || clean_filename.ends_with(' ') {
            clean_filename.pop();
        }
    }

    clean_filename = clean_filename.trim().to_string();

    if use_ntfs_rules {
        while NTFS_DISALLOWED_NAMES_CASE_INSENSITIVE.contains(&clean_filename.to_lowercase()) {
            clean_filename.pop();
        }
    }

    clean_filename
}

fn path_limits_for_eliding(
    path_character_limit: Option<usize>,
    dirname_character_limit: Option<usize>,
    force_ntfs_rules: bool,
) -> (Option<usize>, Option<usize>, Option<usize>, Option<usize>) {
    let use_ntfs_rules = PLATFORM_WINDOWS || force_ntfs_rules;

    let (max_path_bytes, max_path_characters) = match path_character_limit {
        None => {
            if use_ntfs_rules {
                (None, Some(260))
            } else if PLATFORM_LINUX {
                (Some(4096), None)
            } else if PLATFORM_MACOS {
                (Some(1024), None)
            } else {
                (Some(4096), None)
            }
        }
        Some(limit) => {
            if use_ntfs_rules {
                (None, Some(limit))
            } else {
                (Some(limit), None)
            }
        }
    };

    let (max_dirname_bytes, max_dirname_characters) = match dirname_character_limit {
        None => {
            if use_ntfs_rules {
                (None, Some(128))
            } else {
                (Some(256), None)
            }
        }
        Some(limit) => {
            if use_ntfs_rules {
                (None, Some(limit))
            } else {
                (Some(limit), None)
            }
        }
    };

    (
        max_path_bytes,
        max_path_characters,
        max_dirname_bytes,
        max_dirname_characters,
    )
}

/// `elide_subdirs_safely( destination_directory, subdirs_elidable, ... ) -> str`
pub fn elide_subdirs_safely(
    destination_directory: &str,
    subdirs_elidable: &str,
    path_character_limit: Option<usize>,
    dirname_character_limit: Option<usize>,
    force_ntfs_rules: bool,
) -> HydrusResult<String> {
    if subdirs_elidable.is_empty() {
        return Ok(String::new());
    }

    let use_ntfs_rules = PLATFORM_WINDOWS || force_ntfs_rules;

    let (
        max_path_bytes,
        max_path_characters,
        max_dirname_bytes,
        max_dirname_characters,
    ) = path_limits_for_eliding(
        path_character_limit,
        dirname_character_limit,
        force_ntfs_rules,
    );

    let dirnames: Vec<&str> = subdirs_elidable
        .split(std::path::MAIN_SEPARATOR)
        .collect();
    let dirname_count = dirnames.len().max(1) as isize;

    let mut actual_per_dirname_characters_limit = 64_isize;
    let mut actual_per_dirname_bytes_limit = 256_isize - 10;

    if use_ntfs_rules {
        let max_path_characters = max_path_characters.unwrap_or(260) as isize;
        let typical_filename_characters = max_path_characters / 4;

        let total_available_characters_left = max_path_characters
            - destination_directory.chars().count() as isize
            - typical_filename_characters
            - dirnames.len() as isize
            - 10;

        actual_per_dirname_characters_limit = total_available_characters_left / dirname_count;

        if actual_per_dirname_characters_limit < 4 {
            return Err(HydrusException::new(
                ErrorKind::Veto,
                "Sorry, it looks like the combined export filename or directory would be too long! Try shortening the export directory name!",
            ));
        }

        if let Some(max_dirname_characters) = max_dirname_characters {
            actual_per_dirname_characters_limit =
                actual_per_dirname_characters_limit.min(max_dirname_characters as isize);
        }
    } else {
        let max_path_bytes = max_path_bytes.unwrap_or(4096) as isize;
        let typical_filename_bytes = max_path_bytes / 4;

        let total_available_bytes_left = max_path_bytes
            - destination_directory.len() as isize
            - typical_filename_bytes
            - dirnames.len() as isize
            - 10;

        actual_per_dirname_bytes_limit = total_available_bytes_left / dirname_count;

        if actual_per_dirname_bytes_limit < 4 {
            return Err(HydrusException::new(
                ErrorKind::Veto,
                "Sorry, it looks like the combined export filename or directory would be too long! Try shortening the export directory name!",
            ));
        }

        if let Some(max_dirname_bytes) = max_dirname_bytes {
            actual_per_dirname_bytes_limit =
                actual_per_dirname_bytes_limit.min(max_dirname_bytes as isize);
        }
    }

    let mut dirnames_elided: Vec<String> = Vec::with_capacity(dirnames.len());

    for dirname in dirnames {
        let mut dirname = sanitize_filename(dirname, force_ntfs_rules);

        if dirname.is_empty() {
            dirname = "empty".to_string();
        }

        loop {
            let too_long = if use_ntfs_rules {
                dirname.chars().count() as isize > actual_per_dirname_characters_limit
            } else {
                dirname.len() as isize > actual_per_dirname_bytes_limit
            };

            if !too_long {
                break;
            }

            dirname.pop();
            dirname = sanitize_filename(&dirname, force_ntfs_rules);
        }

        let dirname = dirname.trim().to_string();
        let dirname = if dirname.is_empty() {
            "truncated".to_string()
        } else {
            dirname
        };

        dirnames_elided.push(dirname);
    }

    Ok(dirnames_elided.join(&std::path::MAIN_SEPARATOR.to_string()))
}

/// `elide_filename_safely( destination_directory, subdirs_elidable, base_filename, ... )`
///
/// Returns `( subdirs_elided, base_filename )`.
#[allow(clippy::too_many_arguments)]
pub fn elide_filename_safely(
    destination_directory: &str,
    subdirs_elidable: &str,
    base_filename: &str,
    ext_suffix: &str,
    path_character_limit: Option<usize>,
    dirname_character_limit: Option<usize>,
    filename_character_limit: usize,
    force_ntfs_rules: bool,
) -> HydrusResult<(String, String)> {
    let use_ntfs_rules = PLATFORM_WINDOWS || force_ntfs_rules;

    let base_filename = if base_filename.is_empty() {
        "empty".to_string()
    } else {
        base_filename.to_string()
    };

    let mut destination_directory = destination_directory.to_string();

    let subdirs_elided = if !subdirs_elidable.is_empty() {
        let subdirs_elided = elide_subdirs_safely(
            &destination_directory,
            subdirs_elidable,
            path_character_limit,
            dirname_character_limit,
            force_ntfs_rules,
        )?;

        destination_directory = Path::new(&destination_directory)
            .join(&subdirs_elided)
            .to_string_lossy()
            .into_owned();

        subdirs_elided
    } else {
        subdirs_elidable.to_string()
    };

    let (max_path_bytes, max_path_characters, _max_dirname_bytes, _max_dirname_characters) =
        path_limits_for_eliding(
            path_character_limit,
            dirname_character_limit,
            force_ntfs_rules,
        );

    let mut filename_character_limit = filename_character_limit as isize;

    if let Some(mut max_path_bytes) = max_path_bytes.map(|value| value as isize) {
        max_path_bytes -= 20; // padding

        let max_bytes_with_full_filename =
            destination_directory.len() as isize + 1 + filename_character_limit;

        if max_bytes_with_full_filename > max_path_bytes {
            filename_character_limit -= max_bytes_with_full_filename - max_path_bytes;

            let min_filename_bytes = 18_isize;

            if filename_character_limit <= min_filename_bytes {
                return Err(HydrusException::new(
                    ErrorKind::Veto,
                    "Sorry, it looks like the combined export filename or directory would be too long! Try shortening the export directory name!",
                ));
            }
        }
    }

    if let Some(mut max_path_characters) = max_path_characters.map(|value| value as isize) {
        max_path_characters -= 10; // padding

        let max_characters_with_full_filename =
            destination_directory.chars().count() as isize + 1 + filename_character_limit;

        if max_characters_with_full_filename > max_path_characters {
            filename_character_limit -= max_characters_with_full_filename - max_path_characters;

            let min_filename_chars = 10_isize;

            if filename_character_limit <= min_filename_chars {
                return Err(HydrusException::new(
                    ErrorKind::Veto,
                    "Sorry, it looks like the combined export filename or directory would be too long! Try shortening the export directory name!",
                ));
            }
        }
    }

    if use_ntfs_rules {
        filename_character_limit -= ext_suffix.chars().count() as isize;
    } else {
        filename_character_limit -= ext_suffix.len() as isize;
    }

    if filename_character_limit <= 0 {
        return Err(HydrusException::new(
            ErrorKind::Veto,
            "Sorry, it looks like the export filename would be too long! Try shortening the export phrase or directory!",
        ));
    }

    let mut base_filename = sanitize_filename(&base_filename, force_ntfs_rules);

    loop {
        let too_long = if use_ntfs_rules {
            base_filename.chars().count() as isize > filename_character_limit
        } else {
            base_filename.len() as isize > filename_character_limit
        };

        if !too_long {
            break;
        }

        base_filename.pop();
        base_filename = sanitize_filename(&base_filename, force_ntfs_rules);
    }

    let base_filename = base_filename.trim().to_string();

    if base_filename.is_empty() {
        return Err(HydrusException::new(
            ErrorKind::Veto,
            "Sorry, it looks like the export filename would be too long! Try shortening the export phrase or directory!",
        ));
    }

    Ok((subdirs_elided, base_filename))
}

// ============================================================================
// Boot path resolution
// ============================================================================

/// `figure_out_db_dir( arg_db_dir ) -> str`
pub fn figure_out_db_dir(arg_db_dir: Option<&str>) -> HydrusResult<PathBuf> {
    let mut switching_to_userpath_is_ok = false;

    let db_dir = match arg_db_dir {
        None => {
            if crate::hydrus_constants::running_from_macos_app() {
                match crate::hydrus_constants::userpath_db_dir() {
                    Some(userpath_db_dir) => userpath_db_dir.to_path_buf(),
                    None => {
                        return Err(HydrusException::new(
                            ErrorKind::DirectoryMissing,
                            "The userpath (for macOS App database) could not be determined!",
                        ));
                    }
                }
            } else {
                switching_to_userpath_is_ok = true;

                crate::hydrus_constants::default_db_dir().to_path_buf()
            }
        }
        Some(arg_db_dir) => {
            let expanded = expand_user(arg_db_dir);

            convert_portable_path_to_abs_path(
                &expanded,
                Some(crate::hydrus_constants::base_dir()),
            )
        }
    };

    if directory_is_writeable(&db_dir)? {
        return Ok(db_dir);
    }

    if !switching_to_userpath_is_ok {
        return Err(HydrusException::new(
            ErrorKind::Veto,
            format!(
                "The chosen db dir path \"{}\", which resolved to \"{}\", did not appear to be writeable-to!",
                arg_db_dir.unwrap_or(""),
                db_dir.display()
            ),
        ));
    }

    let Some(userpath_db_dir) = crate::hydrus_constants::userpath_db_dir() else {
        return Err(HydrusException::new(
            ErrorKind::DirectoryMissing,
            format!(
                "The db path \"{}\" was not writeable-to, and the userpath could not be determined!",
                db_dir.display()
            ),
        ));
    };

    if !directory_is_writeable(userpath_db_dir)? {
        return Err(HydrusException::new(
            ErrorKind::Veto,
            format!(
                "Neither the default db path \"{}\", nor the userpath fallback \"{}\", were writeable-to!",
                db_dir.display(),
                userpath_db_dir.display()
            ),
        ));
    }

    safe_print(format!(
        "The given db path \"{}\" is not writeable-to! Falling back to userpath at \"{}\".",
        db_dir.display(),
        userpath_db_dir.display()
    ));

    set_we_switched_to_userpath(true);

    Ok(userpath_db_dir.to_path_buf())
}

/// `figure_out_temp_dir( temp_dir ) -> str`
pub fn figure_out_temp_dir(temp_dir: &str) -> HydrusResult<PathBuf> {
    let original_temp_dir = temp_dir;

    let temp_dir = expand_user(temp_dir);
    let temp_dir =
        convert_portable_path_to_abs_path(&temp_dir, Some(crate::hydrus_constants::base_dir()));

    if directory_is_writeable(&temp_dir)? {
        Ok(temp_dir)
    } else {
        Err(HydrusException::new(
            ErrorKind::Veto,
            format!(
                "The chosen temp dir path \"{}\", which resolved to \"{}\", did not appear to be writeable-to!",
                original_temp_dir,
                temp_dir.display()
            ),
        ))
    }
}

// ============================================================================
// Launching external programs
// ============================================================================

/// `launch_directory( path )`
///
/// The helper process is spawned in a detached thread, exactly like Python.
/// Spawn failures are only printed.
pub fn launch_directory(path: &Path) {
    let path = path.to_path_buf();

    std::thread::spawn(move || {
        let mut command = if PLATFORM_WINDOWS {
            // Python uses os.startfile for directories.
            let mut command = Command::new("explorer");
            command.arg(&path);

            command
        } else if PLATFORM_MACOS || PLATFORM_HAIKU {
            let mut command = Command::new("open");
            command.arg(&path);

            command
        } else if PLATFORM_LINUX {
            let mut command = Command::new("xdg-open");
            command.arg(&path);

            command
        } else {
            safe_print("Unknown platform: cannot launch a directory.");

            return;
        };

        if let Err(error) = command.spawn() {
            safe_print(format!(
                "Could not launch \"{}\": {}",
                path.display(),
                error
            ));
        }
    });
}

/// `open_file_location( path )`
pub fn open_file_location(path: &Path) {
    let path = path.to_path_buf();

    std::thread::spawn(move || {
        let mut command = if PLATFORM_WINDOWS {
            let mut command = Command::new("explorer");
            command.arg("/select,").arg(&path);

            command
        } else if PLATFORM_MACOS {
            let mut command = Command::new("open");
            command.arg("-R").arg(&path);

            command
        } else {
            // Linux and Haiku cannot open file locations in Python either.
            safe_print("This platform cannot open file locations.");

            return;
        };

        if let Err(error) = command.spawn() {
            safe_print(format!(
                "Could not open the file location for \"{}\": {}",
                path.display(),
                error
            ));
        }
    });
}

// ============================================================================
// Filtering and directory listing
// ============================================================================

/// `filter_free_paths( paths ) -> list`
pub fn filter_free_paths(paths: &[PathBuf]) -> Vec<PathBuf> {
    paths
        .iter()
        .filter(|path| path_is_free(path.as_path()))
        .cloned()
        .collect()
}

/// `Filter_older_modified_files( paths, grace_period ) -> list`
pub fn filter_older_modified_files(paths: &[PathBuf], grace_period: i64) -> Vec<PathBuf> {
    let only_older_than = (crate::hydrus_time::get_now() - grace_period) as f64;

    paths
        .iter()
        .filter(|path| {
            fs::metadata(path.as_path())
                .map(|metadata| metadata_mtime_float(&metadata) < only_older_than)
                .unwrap_or(false)
        })
        .cloned()
        .collect()
}

/// `get_all_sub_dirs_non_recursive( path ) -> set`
pub fn get_all_sub_dirs_non_recursive(path: &Path) -> HydrusResult<HashSet<PathBuf>> {
    let entries = fs::read_dir(path).map_err(|error| {
        io_failure(
            ErrorKind::DirectoryMissing,
            format!("Could not scan \"{}\"", path.display()),
            &error,
        )
    })?;

    let mut subdir_paths = HashSet::new();

    for entry in entries.flatten() {
        let entry_path = entry.path();

        // `DirEntry::path().is_dir()` follows symlinks, like
        // `os.DirEntry.is_dir()` does by default.
        if entry_path.is_dir() {
            subdir_paths.insert(entry_path);
        }
    }

    Ok(subdir_paths)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TempDir {
        path: PathBuf,
    }

    impl TempDir {
        fn new(name: &str) -> Self {
            use std::sync::atomic::AtomicU64;

            static COUNTER: AtomicU64 = AtomicU64::new(0);

            let counter = COUNTER.fetch_add(1, Ordering::Relaxed);

            let path = std::env::temp_dir().join(format!(
                "hydrus_paths_test_{}_{}_{}",
                std::process::id(),
                counter,
                name
            ));

            let _ = fs::remove_dir_all(&path);
            fs::create_dir_all(&path).expect("create temp dir");

            Self { path }
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    fn write(path: &Path, bytes: &[u8]) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("create parent dir");
        }

        fs::write(path, bytes).expect("write test file");
    }

    fn set_mtime_secs(path: &Path, seconds: u64) {
        let file = fs::OpenOptions::new()
            .write(true)
            .open(path)
            .expect("open for set_times");

        let time = UNIX_EPOCH + Duration::from_secs(seconds);

        file.set_times(
            fs::FileTimes::new()
                .set_accessed(time)
                .set_modified(time),
        )
        .expect("set file times");
    }

    #[test]
    fn sanitize_filename_rules() {
        assert_eq!(sanitize_filename("a/b:c*?\"<>|d", true), "a_b_c______d");
        assert_eq!(sanitize_filename("foo. . ", true), "foo");
        assert_eq!(sanitize_filename("con", true), "co");
        assert_eq!(sanitize_filename("  normal name  ", true), "normal name");
        assert_eq!(sanitize_filename("a/b", false), "a_b");
    }

    #[test]
    fn append_path_until_no_conflicts_works() {
        let temp = TempDir::new("append");

        let original = temp.path.join("file.txt");

        let first = append_path_until_no_conflicts(&original);

        // Python returns the untouched path when nothing is there yet.
        assert_eq!(first, original);

        write(&original, b"one");

        let second = append_path_until_no_conflicts(&original);

        assert_eq!(second, temp.path.join("file_0.txt"));
    }

    #[test]
    fn portable_paths_round_trip() {
        let temp = TempDir::new("portable");
        let child = temp.path.join("sub").join("file.txt");

        write(&child, b"hello");

        let portable = convert_abs_path_to_portable_path(&child, Some(&temp.path));

        assert_eq!(portable, "sub/file.txt");

        let round_trip =
            convert_portable_path_to_abs_path(Path::new(&portable), Some(&temp.path));

        assert_eq!(round_trip, child);

        let outside = temp
            .path
            .parent()
            .expect("temp parent")
            .join("hydrus_paths_outside");

        let outside_portable = convert_abs_path_to_portable_path(&outside, Some(&temp.path));

        assert!(!outside_portable.starts_with(".."));
        assert_eq!(
            normalize_lexical(Path::new(&outside_portable)),
            normalize_lexical(&outside)
        );

        let collapsed = convert_portable_path_to_abs_path(
            Path::new("sub/./nested/../file.txt"),
            Some(&temp.path),
        );

        assert_eq!(collapsed, temp.path.join("sub").join("file.txt"));
    }

    #[test]
    fn delete_path_removes_files_dirs_and_accepts_missing() {
        let temp = TempDir::new("delete");

        let file_path = temp.path.join("file.txt");
        write(&file_path, b"data");

        assert!(delete_path(&file_path));
        assert!(!file_path.exists());

        let dir_path = temp.path.join("dir").join("nested");
        fs::create_dir_all(&dir_path).expect("create nested dir");
        write(&dir_path.join("a.txt"), b"a");

        assert!(delete_path(&temp.path.join("dir")));
        assert!(!temp.path.join("dir").exists());

        assert!(delete_path(&temp.path.join("never_existed")));
    }

    #[test]
    fn make_sure_directory_exists_creates_and_rejects_files() {
        let temp = TempDir::new("mkdir");
        let nested = temp.path.join("a").join("b").join("c");

        make_sure_directory_exists(&nested).expect("create nested directory");

        assert!(nested.is_dir());

        let file_path = temp.path.join("a_file");
        write(&file_path, b"x");

        assert!(make_sure_directory_exists(&file_path).is_err());
    }

    #[test]
    fn same_size_and_date_helper() {
        let temp = TempDir::new("same");

        let first = temp.path.join("first.txt");
        let second = temp.path.join("second.txt");

        write(&first, b"same");
        write(&second, b"same");

        assert!(copy_times(&first, &second));

        let first_stat = fs::metadata(&first).expect("first metadata");
        let second_stat = fs::metadata(&second).expect("second metadata");

        assert!(dest_stat_has_same_size_and_date_as_source(
            &first_stat,
            &second_stat
        ));

        write(&second, b"different length");

        let second_stat = fs::metadata(&second).expect("second metadata");

        assert!(!dest_stat_has_same_size_and_date_as_source(
            &first_stat,
            &second_stat
        ));
    }

    #[test]
    fn merge_file_moves_and_skips_duplicates() {
        let temp = TempDir::new("merge_file");

        let dest = temp.path.join("dest.txt");
        let source = temp.path.join("source.txt");

        write(&dest, b"hello");
        write(&source, b"hello");
        assert!(copy_times(&source, &dest));

        assert_eq!(merge_file(&source, &dest).expect("merge duplicate"), false);
        assert!(!source.exists());
        assert_eq!(fs::read(&dest).expect("read dest"), b"hello");

        let source = temp.path.join("source2.txt");
        write(&source, b"replacement");

        assert_eq!(merge_file(&source, &dest).expect("merge real move"), true);
        assert!(!source.exists());
        assert_eq!(fs::read(&dest).expect("read dest"), b"replacement");
    }

    #[test]
    fn mirror_file_copies_and_skips_duplicates() {
        let temp = TempDir::new("mirror_file");

        let source = temp.path.join("source.txt");
        let dest = temp.path.join("dest.txt");

        write(&source, b"abc");
        write(&dest, b"abc");
        assert!(copy_times(&source, &dest));

        assert_eq!(mirror_file(&source, &dest).expect("mirror duplicate"), false);

        write(&dest, b"xyz");
        // Move the mtime outside the source's whole-second window; the
        // sameness check is `int( st_mtime )`, as in Python.
        set_mtime_secs(&dest, 1_000_000_000);

        assert_eq!(mirror_file(&source, &dest).expect("mirror copy"), true);
        assert_eq!(fs::read(&dest).expect("read dest"), b"abc");
        assert!(source.exists());
    }

    #[test]
    fn mirror_tree_copies_and_removes_surplus() {
        let temp = TempDir::new("mirror_tree");

        let source = temp.path.join("source");
        let dest = temp.path.join("dest");

        write(&source.join("a.txt"), b"a");
        write(&source.join("sub").join("b.txt"), b"b");

        write(&dest.join("old.txt"), b"old");
        write(&dest.join("old_sub").join("c.txt"), b"c");

        let mut updates = 0_usize;
        let mut text_hook = |_: &str| updates += 1;

        mirror_tree(&source, &dest, Some(&mut text_hook), None).expect("mirror tree");

        assert!(updates > 0);
        assert_eq!(fs::read(dest.join("a.txt")).expect("a"), b"a");
        assert_eq!(
            fs::read(dest.join("sub").join("b.txt")).expect("b"),
            b"b"
        );
        assert!(!dest.join("old.txt").exists());
        assert!(!dest.join("old_sub").exists());

        let cancelled_dest = temp.path.join("cancelled");
        let mut cancel = || true;

        mirror_tree(&source, &cancelled_dest, None, Some(&mut cancel))
            .expect("cancelled mirror");

        assert!(!cancelled_dest.join("a.txt").exists());
    }

    #[test]
    fn merge_tree_merges_into_populated_destination() {
        let temp = TempDir::new("merge_tree");

        let source = temp.path.join("source");
        let dest = temp.path.join("dest");

        write(&source.join("one.txt"), b"one");
        write(&source.join("sub").join("two.txt"), b"two");
        write(&dest.join("keep.txt"), b"keep");

        merge_tree(&source, &dest, None).expect("merge tree");

        assert!(!source.exists());
        assert_eq!(fs::read(dest.join("keep.txt")).expect("keep"), b"keep");
        assert_eq!(fs::read(dest.join("one.txt")).expect("one"), b"one");
        assert_eq!(
            fs::read(dest.join("sub").join("two.txt")).expect("two"),
            b"two"
        );
    }

    #[test]
    fn elide_helpers_shorten_and_reject() {
        let (subdirs, filename) = elide_filename_safely(
            "C:\\export",
            "alpha/bravo",
            "a_really_long_filename_here",
            ".txt",
            Some(80),
            Some(10),
            20,
            true,
        )
        .expect("elide filename");

        assert!(!subdirs.is_empty());
        assert!(!filename.is_empty());
        assert!(filename.chars().count() <= 20 - 4);
        assert!(subdirs
            .split(std::path::MAIN_SEPARATOR)
            .all(|part| part.chars().count() <= 10));

        assert_eq!(
            elide_subdirs_safely("", "", None, None, true).expect("empty subdirs"),
            ""
        );

        assert!(elide_subdirs_safely("C:\\export", "alpha/bravo", Some(10), None, true).is_err());
    }

    #[test]
    fn block_reader_and_file_copy() {
        let temp = TempDir::new("blocks");

        let source = temp.path.join("source.bin");
        let data = vec![7_u8; READ_BLOCK_SIZE * 2 + 17];

        write(&source, &data);

        let blocks: Vec<Vec<u8>> = read_file_like_as_blocks(
            fs::File::open(&source).expect("open source"),
        )
        .collect::<Result<Vec<_>, _>>()
        .expect("read blocks");

        assert!(blocks.len() >= 3);
        assert_eq!(
            blocks.iter().map(|block| block.len()).sum::<usize>(),
            data.len()
        );
        assert!(blocks.iter().all(|block| block.len() <= READ_BLOCK_SIZE));

        let mut copied = Vec::new();

        copy_file_like_to_file_like(
            fs::File::open(&source).expect("open source again"),
            &mut copied,
        )
        .expect("copy blocks");

        assert_eq!(copied, data);
    }

    #[test]
    fn free_path_filtering_and_age_filtering() {
        let temp = TempDir::new("filters");

        let free_file = temp.path.join("free.txt");
        write(&free_file, b"free");

        assert!(path_is_free(&free_file));
        assert_eq!(
            filter_free_paths(&[free_file.clone(), temp.path.join("missing.txt")]),
            vec![free_file]
        );

        let old_file = temp.path.join("old.txt");
        write(&old_file, b"old");
        set_mtime_secs(&old_file, 1_000_000_000);

        let new_file = temp.path.join("new.txt");
        write(&new_file, b"new");

        let older = filter_older_modified_files(&[old_file.clone(), new_file], 60);

        assert_eq!(older, vec![old_file]);
    }

    #[test]
    fn subdir_listing_and_path_checks() {
        let temp = TempDir::new("subdirs");

        let first = temp.path.join("first");
        let second = temp.path.join("second");

        fs::create_dir_all(&first).expect("first dir");
        fs::create_dir_all(&second).expect("second dir");
        write(&temp.path.join("file.txt"), b"x");

        let subdirs = get_all_sub_dirs_non_recursive(&temp.path).expect("list subdirs");

        assert_eq!(subdirs.len(), 2);
        assert!(subdirs.contains(&first));
        assert!(subdirs.contains(&second));
        assert!(path_exists_and_is_dir(&first));
        assert!(!path_exists_and_is_dir(&temp.path.join("file.txt")));
        assert!(!path_exists_and_is_dir(&temp.path.join("missing")));
    }

    #[test]
    fn device_ids_and_disk_space() {
        let temp = TempDir::new("device");

        let file_path = temp.path.join("file.txt");
        write(&file_path, b"x");

        let dir_device = get_device_id(&temp.path).expect("dir device id");
        let file_device = get_device_id(&file_path).expect("file device id");

        assert_eq!(dir_device, file_device);

        if let (Some(free), Some(total)) = (get_free_space(&temp.path), get_total_space(&temp.path))
        {
            assert!(total > 0);
            assert!(free <= total);
        }
    }

    #[test]
    fn recycle_path_deletes_and_accepts_missing() {
        let temp = TempDir::new("recycle");

        let file_path = temp.path.join("file.txt");
        write(&file_path, b"x");

        assert!(recycle_path(&file_path));
        assert!(!file_path.exists());

        let dir_path = temp.path.join("dir");
        write(&dir_path.join("nested.txt"), b"x");

        assert!(recycle_path(&dir_path));
        assert!(!dir_path.exists());

        assert!(recycle_path(&temp.path.join("missing")));
    }

    #[test]
    fn stat_helpers_identify_types_and_samefile() {
        let temp = TempDir::new("stats");

        let file_path = temp.path.join("file.txt");
        let other_path = temp.path.join("other.txt");
        let dir_path = temp.path.join("dir");

        write(&file_path, b"x");
        write(&other_path, b"y");
        fs::create_dir_all(&dir_path).expect("dir");

        let file_stat = fs::metadata(&file_path).expect("file metadata");
        let other_stat = fs::metadata(&other_path).expect("other metadata");
        let dir_stat = fs::metadata(&dir_path).expect("dir metadata");

        assert!(stat_is_file(&file_stat));
        assert!(stat_is_dir(&dir_stat));
        assert!(!stat_is_symlink(&file_stat));
        assert!(stat_is_samefile(&file_stat, &file_stat));
        assert!(!stat_is_samefile(&file_stat, &other_stat));

        assert!(paths_are_same_file(&file_path, &file_path));
        assert!(paths_are_same_file(&dir_path, &dir_path));
        assert!(!paths_are_same_file(&file_path, &other_path));
        assert!(!paths_are_same_file(&file_path, &temp.path.join("missing")));
    }

    #[test]
    fn modified_time_validity_depends_on_platform() {
        assert!(file_modified_time_is_ok(2_000_000_000.0));

        if PLATFORM_WINDOWS {
            assert!(!file_modified_time_is_ok(0.0));
        } else {
            assert!(file_modified_time_is_ok(0.0));
        }
    }
}
