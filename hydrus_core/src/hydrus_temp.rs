//! Rust port of `hydrus/core/HydrusTemp.py`.
//!
//! Python asks `HG.controller` for the hydrus temp dir. The controller lives
//! above this crate, so the port stores the path in a process-global and
//! exposes [`set_hydrus_temp_dir`] for boot code to call. Unit-test builds use
//! a thread-local override instead so tests can run in parallel.
//!
//! `tempfile.mkstemp` returns `(fd, path)`. The Rust equivalent returns an open
//! [`std::fs::File`] alongside the path; dropping the file closes it.

use std::collections::HashSet;
use std::fs::{File, OpenOptions};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(test)]
use std::cell::RefCell;

use crate::hydrus_constants::PLATFORM_WINDOWS;
use crate::hydrus_exceptions::{ErrorKind, HydrusException, HydrusResult};
use crate::hydrus_time::{get_now, time_has_passed};

static HYDROS_TEMP_DIR: OnceLock<Mutex<Option<PathBuf>>> = OnceLock::new();

// Unit tests run concurrently in one process, so they must not share the
// process-global temp dir: a test that deletes its temp dir would otherwise
// pull the rug out from under every other test. In test builds
// `set_hydrus_temp_dir` writes a thread-local override instead, which keeps
// each test's temp dir private. See `set_hydrus_temp_dir`.
#[cfg(test)]
thread_local! {
    static TEST_HYDROS_TEMP_DIR: RefCell<Option<PathBuf>> = const { RefCell::new(None) };
}

static TEMP_PATH_LOCK: Mutex<()> = Mutex::new(());

static IN_USE_TEMP_PATHS: OnceLock<Mutex<HashSet<(i64, PathBuf)>>> = OnceLock::new();

fn in_use_paths() -> &'static Mutex<HashSet<(i64, PathBuf)>> {
    IN_USE_TEMP_PATHS.get_or_init(|| Mutex::new(HashSet::new()))
}

fn unique_suffix() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.subsec_nanos())
        .unwrap_or(0);

    format!("{:x}{:x}", std::process::id(), nanos)
}

fn temp_error(message: impl Into<String>) -> HydrusException {
    HydrusException::new(ErrorKind::DirectoryMissing, message)
}

/// Set the hydrus working temp dir, normally called during boot.
///
/// In unit-test builds this writes a thread-local override rather than the
/// process-global, so parallel tests each get their own temp dir.
pub fn set_hydrus_temp_dir(path: impl Into<PathBuf>) {
    let path = path.into();

    #[cfg(test)]
    TEST_HYDROS_TEMP_DIR.with(|temp_dir| *temp_dir.borrow_mut() = Some(path));

    #[cfg(not(test))]
    {
        *HYDROS_TEMP_DIR
            .get_or_init(|| Mutex::new(None))
            .lock()
            .unwrap() = Some(path);
    }
}

/// The hydrus working temp dir, or the OS temp dir if boot has not set one.
pub fn get_hydrus_temp_dir() -> PathBuf {
    #[cfg(test)]
    if let Some(path) = TEST_HYDROS_TEMP_DIR.with(|temp_dir| temp_dir.borrow().clone()) {
        return path;
    }

    HYDROS_TEMP_DIR
        .get_or_init(|| Mutex::new(None))
        .lock()
        .unwrap()
        .clone()
        .unwrap_or_else(get_current_temp_dir)
}

/// `get_current_temp_dir()`
pub fn get_current_temp_dir() -> PathBuf {
    std::env::temp_dir()
}

/// `get_current_sqlite_temp_dir()`
pub fn get_current_sqlite_temp_dir() -> PathBuf {
    std::env::var_os("SQLITE_TMPDIR")
        .map(PathBuf::from)
        .unwrap_or_else(get_current_temp_dir)
}

/// `initialise_hydrus_temp_dir()`
pub fn initialise_hydrus_temp_dir() -> HydrusResult<PathBuf> {
    create_unique_directory(&get_current_temp_dir(), "hydrus")
}

/// `set_env_temp_dir( path )`
///
/// As in Python, this also updates `TMPDIR`/`TEMP`/`TMP` and the process temp
/// dir. Note that `std::env::set_var` is unsafe in edition 2024 because other
/// threads may be reading the environment; hydrus calls this during early boot.
pub fn set_env_temp_dir(path: &Path) -> HydrusResult<()> {
    crate::hydrus_paths::make_sure_directory_exists(path).map_err(|_| {
        temp_error(format!(
            "Could not create the temp dir \"{}\"!",
            path.display()
        ))
    })?;

    if !crate::hydrus_paths::directory_is_writeable(path).unwrap_or(false) {
        return Err(temp_error(format!(
            "The given temp directory, \"{}\", does not seem to be writeable-to!",
            path.display()
        )));
    }

    // SAFETY: called during early boot, before worker threads that read the
    // environment are spawned.
    unsafe {
        for name in ["TMPDIR", "TEMP", "TMP"] {
            std::env::set_var(name, path);
        }
    }

    set_hydrus_temp_dir(path);

    Ok(())
}

/// `get_sub_temp_dir( prefix = '' )`
pub fn get_sub_temp_dir(prefix: &str) -> HydrusResult<PathBuf> {
    let parent = get_hydrus_temp_dir();

    crate::hydrus_paths::make_sure_directory_exists(&parent)?;

    create_unique_directory(&parent, prefix)
}

/// `get_temp_path( prefix, suffix = '', dir = None )`
pub fn get_temp_path(
    prefix: &str,
    suffix: &str,
    dir: Option<&Path>,
) -> HydrusResult<(File, PathBuf)> {
    let dir = dir
        .map(Path::to_path_buf)
        .unwrap_or_else(get_hydrus_temp_dir);

    crate::hydrus_paths::make_sure_directory_exists(&dir)?;

    let stem = format!("{prefix}_");

    for _ in 0..100 {
        let path = dir.join(format!("{stem}{}{suffix}", unique_suffix()));

        match OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(file) => return Ok((file, path)),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => {
                return Err(temp_error(format!(
                    "Could not create a temp file in \"{}\": {error}",
                    dir.display()
                )));
            }
        }
    }

    Err(temp_error(format!(
        "Could not create a unique temp file in \"{}\"!",
        dir.display()
    )))
}

fn create_unique_directory(parent: &Path, prefix: &str) -> HydrusResult<PathBuf> {
    for _ in 0..100 {
        let path = parent.join(format!("{prefix}{}", unique_suffix()));

        match std::fs::create_dir(&path) {
            Ok(()) => return Ok(path),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => {
                return Err(temp_error(format!(
                    "Could not create a temp directory in \"{}\": {error}",
                    parent.display()
                )));
            }
        }
    }

    Err(temp_error(format!(
        "Could not create a unique temp directory in \"{}\"!",
        parent.display()
    )))
}

/// `clean_up_temp_path( os_file_handle, temp_path )`
///
/// The file handle is consumed, which closes it, just like `os.close`.
pub fn clean_up_temp_path(file_handle: File, temp_path: &Path) {
    drop(file_handle);

    if PLATFORM_WINDOWS {
        try_to_make_file_writeable(temp_path);
    }

    if let Err(_error) = std::fs::remove_file(temp_path) {
        let _guard = TEMP_PATH_LOCK.lock().unwrap();

        in_use_paths()
            .lock()
            .unwrap()
            .insert((get_now(), temp_path.to_path_buf()));
    }
}

fn try_to_make_file_writeable(path: &Path) {
    if let Ok(metadata) = std::fs::metadata(path) {
        let mut permissions = metadata.permissions();

        if permissions.readonly() {
            permissions.set_readonly(false);

            let _ = std::fs::set_permissions(path, permissions);
        }
    }
}

/// `clean_up_old_temp_paths()`
pub fn clean_up_old_temp_paths() {
    let _guard = TEMP_PATH_LOCK.lock().unwrap();

    let mut paths = in_use_paths().lock().unwrap();

    let rows: Vec<(i64, PathBuf)> = paths.iter().cloned().collect();

    for (time_failed, temp_path) in rows {
        if time_has_passed(Some(time_failed + 60)) {
            match std::fs::remove_file(&temp_path) {
                Ok(()) => {
                    paths.remove(&(time_failed, temp_path));
                }
                Err(_) => {
                    if time_has_passed(Some(time_failed + 1200)) {
                        paths.remove(&(time_failed, temp_path));
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn temp_dirs_are_unique() {
        let first = get_sub_temp_dir("hydrus-test").unwrap();
        let second = get_sub_temp_dir("hydrus-test").unwrap();

        assert_ne!(first, second);
        assert!(first.exists());
        assert!(second.exists());

        let _ = std::fs::remove_dir_all(first);
        let _ = std::fs::remove_dir_all(second);
    }

    #[test]
    fn temp_files_are_created_and_removable() {
        let (file, path) = get_temp_path("hydrus-test", ".txt", None).unwrap();

        assert!(path.exists());
        assert!(
            path.file_name()
                .unwrap()
                .to_string_lossy()
                .contains("hydrus-test_")
        );

        clean_up_temp_path(file, &path);

        assert!(!path.exists());
    }

    #[test]
    fn sqlite_temp_dir_prefers_the_env_var() {
        // Reading is safe; setting the variable would disturb other tests.
        if std::env::var_os("SQLITE_TMPDIR").is_some() {
            assert_eq!(
                get_current_sqlite_temp_dir(),
                PathBuf::from(std::env::var_os("SQLITE_TMPDIR").unwrap())
            );
        } else {
            assert_eq!(get_current_sqlite_temp_dir(), get_current_temp_dir());
        }
    }

    #[test]
    fn hydrus_temp_dir_round_trip() {
        let original = get_hydrus_temp_dir();
        let path = get_sub_temp_dir("hydrus-tempdir-test").unwrap();

        set_hydrus_temp_dir(&path);

        assert_eq!(get_hydrus_temp_dir(), path);

        let (file, temp_path) = get_temp_path("inside", "", None).unwrap();

        assert_eq!(temp_path.parent().unwrap(), path.as_path());

        clean_up_temp_path(file, &temp_path);

        let _ = std::fs::remove_dir_all(path);

        set_hydrus_temp_dir(original);
    }
}
