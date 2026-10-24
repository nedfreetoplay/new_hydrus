//! Rust port of `hydrus/server/ServerFiles.py`.
//!
//! Server files live in the same two-hex-digit prefix layout as the client's
//! physical storage. The Python module reads the active controller from a
//! global; here [`ServerFiles`] can be constructed with an explicit directory
//! for tests, while the free functions preserve the Python API by resolving the
//! global controller.

use std::collections::HashSet;
use std::ffi::OsStr;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use hydrus_core::files::hydrus_files_physical_storage;
use hydrus_core::hydrus_exceptions::{ErrorKind, HydrusException, HydrusResult};

use crate::server_globals;

/// The Python `file_type` string.
#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ServerFileType {
    File,
    Thumbnail,
}

impl ServerFileType {
    fn includes(self, path: &Path) -> bool {
        let name = path.file_name().and_then(OsStr::to_str).unwrap_or_default();
        let is_thumbnail = name.ends_with(".thumbnail");

        match self {
            Self::File => !is_thumbnail,
            Self::Thumbnail => is_thumbnail,
        }
    }
}

/// A server file store rooted at `server_files`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServerFiles {
    files_dir: PathBuf,
}

impl ServerFiles {
    pub fn new(files_dir: impl Into<PathBuf>) -> Self {
        Self {
            files_dir: files_dir.into(),
        }
    }

    /// Resolve the files directory from `SG.server_controller`.
    pub fn from_global() -> HydrusResult<Self> {
        let controller = server_globals::require_server_controller()?;

        Ok(Self::new(controller.get_files_dir().to_path_buf()))
    }

    pub fn files_dir(&self) -> &Path {
        &self.files_dir
    }

    /// The directory setup from `DB._create_db`: the root plus every two-digit
    /// prefix folder.
    pub fn prepare(&self) -> HydrusResult<()> {
        hydrus_core::hydrus_paths::make_sure_directory_exists(&self.files_dir)?;

        for prefix in hydrus_files_physical_storage::iterate_prefixes("", 2) {
            hydrus_core::hydrus_paths::make_sure_directory_exists(
                &self.files_dir.join(prefix),
            )?;
        }

        Ok(())
    }

    /// `get_expected_file_path( hash )`
    pub fn get_expected_file_path(&self, hash: &[u8]) -> PathBuf {
        let hash_encoded = hex_encode(hash);
        let first_two_chars = &hash_encoded[..hash_encoded.len().min(2)];

        self.files_dir.join(first_two_chars).join(hash_encoded)
    }

    /// `get_expected_thumbnail_path( hash )`
    pub fn get_expected_thumbnail_path(&self, hash: &[u8]) -> PathBuf {
        let hash_encoded = hex_encode(hash);
        let first_two_chars = &hash_encoded[..hash_encoded.len().min(2)];

        self.files_dir
            .join(first_two_chars)
            .join(format!("{hash_encoded}.thumbnail"))
    }

    /// `get_file_path( hash )`
    pub fn get_file_path(&self, hash: &[u8]) -> HydrusResult<PathBuf> {
        let path = self.get_expected_file_path(hash);

        if !path.exists() {
            return Err(HydrusException::new(ErrorKind::NotFound, "File not found!"));
        }

        Ok(path)
    }

    /// `get_thumbnail_path( hash )`
    pub fn get_thumbnail_path(&self, hash: &[u8]) -> HydrusResult<PathBuf> {
        let path = self.get_expected_thumbnail_path(hash);

        if !path.exists() {
            return Err(HydrusException::new(
                ErrorKind::NotFound,
                "Thumbnail not found!",
            ));
        }

        Ok(path)
    }

    /// `iterate_all_paths( file_type )`, collected eagerly like the other
    /// generator ports in `hydrus_core`.
    ///
    /// Missing prefix directories are an error, matching `os.listdir` in the
    /// Python source. The server creates every prefix during database setup.
    pub fn iterate_all_paths(&self, file_type: ServerFileType) -> HydrusResult<Vec<PathBuf>> {
        let mut paths = Vec::new();

        for prefix in hydrus_files_physical_storage::iterate_prefixes("", 2) {
            let directory = self.files_dir.join(prefix);

            let entries = fs::read_dir(&directory).map_err(|error| {
                io_error(
                    ErrorKind::DirectoryMissing,
                    format!("Could not list the server storage directory \"{}\"", directory.display()),
                    error,
                )
            })?;

            for entry in entries {
                let entry = entry.map_err(|error| {
                    io_error(
                        ErrorKind::DirectoryMissing,
                        format!("Could not read an entry in \"{}\"", directory.display()),
                        error,
                    )
                })?;

                let path = entry.path();

                if file_type.includes(&path) {
                    paths.push(path);
                }
            }
        }

        paths.sort();

        Ok(paths)
    }

    /// `get_all_hashes( file_type )`
    ///
    /// Python's thumbnail branch passes the `.thumbnail` suffix to
    /// `bytes.fromhex`; this port strips the suffix first, which is what the
    /// operation is actually meant to return.
    pub fn get_all_hashes(
        &self,
        file_type: ServerFileType,
    ) -> HydrusResult<HashSet<Vec<u8>>> {
        let mut hashes = HashSet::new();

        for path in self.iterate_all_paths(file_type)? {
            let name = path.file_name().and_then(OsStr::to_str).ok_or_else(|| {
                HydrusException::new(
                    ErrorKind::DamagedOrUnusualFile,
                    format!("The server storage path \"{}\" has no usable filename.", path.display()),
                )
            })?;

            let hash_text = match file_type {
                ServerFileType::File => name,
                ServerFileType::Thumbnail => name
                    .strip_suffix(".thumbnail")
                    .unwrap_or(name),
            };

            let hash = hex_decode(hash_text).map_err(|()| {
                HydrusException::new(
                    ErrorKind::DamagedOrUnusualFile,
                    format!("The server storage filename \"{name}\" is not a valid hash."),
                )
            })?;

            hashes.insert(hash);
        }

        Ok(hashes)
    }
}

// The free functions below preserve the Python module API. The HTTP resource
// layer will call them once it is wired up; keeping them now means callers do
// not have to change shape later.

/// `get_all_hashes( file_type )`
#[allow(dead_code)]
pub fn get_all_hashes(file_type: ServerFileType) -> HydrusResult<HashSet<Vec<u8>>> {
    ServerFiles::from_global()?.get_all_hashes(file_type)
}

/// `get_expected_file_path( hash )`
#[allow(dead_code)]
pub fn get_expected_file_path(hash: &[u8]) -> HydrusResult<PathBuf> {
    Ok(ServerFiles::from_global()?.get_expected_file_path(hash))
}

/// `get_expected_thumbnail_path( hash )`
#[allow(dead_code)]
pub fn get_expected_thumbnail_path(hash: &[u8]) -> HydrusResult<PathBuf> {
    Ok(ServerFiles::from_global()?.get_expected_thumbnail_path(hash))
}

/// `get_file_path( hash )`
#[allow(dead_code)]
pub fn get_file_path(hash: &[u8]) -> HydrusResult<PathBuf> {
    ServerFiles::from_global()?.get_file_path(hash)
}

/// `get_thumbnail_path( hash )`
#[allow(dead_code)]
pub fn get_thumbnail_path(hash: &[u8]) -> HydrusResult<PathBuf> {
    ServerFiles::from_global()?.get_thumbnail_path(hash)
}

/// `iterate_all_paths( file_type )`
#[allow(dead_code)]
pub fn iterate_all_paths(file_type: ServerFileType) -> HydrusResult<Vec<PathBuf>> {
    ServerFiles::from_global()?.iterate_all_paths(file_type)
}

fn io_error(kind: ErrorKind, context: impl std::fmt::Display, error: io::Error) -> HydrusException {
    HydrusException::new(kind, format!("{context}: {error}"))
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";

    let mut text = String::with_capacity(bytes.len() * 2);

    for byte in bytes {
        text.push(char::from(HEX[usize::from(byte >> 4)]));
        text.push(char::from(HEX[usize::from(byte & 0x0F)]));
    }

    text
}

fn hex_decode(text: &str) -> Result<Vec<u8>, ()> {
    if !text.len().is_multiple_of(2) {
        return Err(());
    }

    let mut bytes = Vec::with_capacity(text.len() / 2);

    for pair in text.as_bytes().chunks_exact(2) {
        let high = hex_digit(pair[0])?;
        let low = hex_digit(pair[1])?;

        bytes.push((high << 4) | low);
    }

    Ok(bytes)
}

fn hex_digit(byte: u8) -> Result<u8, ()> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        b'A'..=b'F' => Ok(byte - b'A' + 10),
        _ => Err(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

    struct TestDir {
        path: PathBuf,
    }

    impl TestDir {
        fn new(label: &str) -> Self {
            let suffix = format!(
                "{}_{}_{}",
                std::process::id(),
                TEST_COUNTER.fetch_add(1, Ordering::Relaxed),
                label
            );

            let path = std::env::temp_dir().join("hydrus-server-files-tests").join(suffix);

            fs::create_dir_all(&path).unwrap();

            Self { path }
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    #[test]
    fn prepare_creates_every_prefix() {
        let dir = TestDir::new("prepare");
        let files = ServerFiles::new(&dir.path);

        files.prepare().unwrap();

        assert!(files.files_dir().is_dir());

        for prefix in hydrus_files_physical_storage::iterate_prefixes("", 2) {
            assert!(dir.path.join(&prefix).is_dir(), "missing prefix {prefix}");
        }
    }

    #[test]
    fn expected_paths_use_the_hash_prefix() {
        let files = ServerFiles::new("server_files");
        let hash = [0x12, 0xAB, 0xCD];

        assert_eq!(
            files.get_expected_file_path(&hash),
            PathBuf::from("server_files").join("12").join("12abcd")
        );
        assert_eq!(
            files.get_expected_thumbnail_path(&hash),
            PathBuf::from("server_files").join("12").join("12abcd.thumbnail")
        );
    }

    #[test]
    fn hashes_and_paths_are_separated_by_file_type() {
        let dir = TestDir::new("hashes");
        let files = ServerFiles::new(&dir.path);

        files.prepare().unwrap();

        let hash = [0xAB, 0xCD];
        let file_path = files.get_expected_file_path(&hash);
        let thumbnail_path = files.get_expected_thumbnail_path(&hash);

        fs::write(&file_path, b"file").unwrap();
        fs::write(&thumbnail_path, b"thumbnail").unwrap();

        assert_eq!(
            files.get_all_hashes(ServerFileType::File).unwrap(),
            HashSet::from([hash.to_vec()])
        );
        assert_eq!(
            files.get_all_hashes(ServerFileType::Thumbnail).unwrap(),
            HashSet::from([hash.to_vec()])
        );

        let file_paths = files.iterate_all_paths(ServerFileType::File).unwrap();
        let thumbnail_paths = files.iterate_all_paths(ServerFileType::Thumbnail).unwrap();

        assert_eq!(file_paths, vec![file_path.clone()]);
        assert_eq!(thumbnail_paths, vec![thumbnail_path.clone()]);
        assert_eq!(files.get_file_path(&hash).unwrap(), file_path);
        assert_eq!(files.get_thumbnail_path(&hash).unwrap(), thumbnail_path);
    }

    #[test]
    fn missing_paths_are_not_found() {
        let dir = TestDir::new("missing");
        let files = ServerFiles::new(&dir.path);

        files.prepare().unwrap();

        let error = files.get_file_path(&[0x01]).unwrap_err();

        assert!(error.is_a(ErrorKind::NotFound));
        assert_eq!(error.to_string(), "File not found!");

        let error = files.get_thumbnail_path(&[0x01]).unwrap_err();

        assert!(error.is_a(ErrorKind::NotFound));
        assert_eq!(error.to_string(), "Thumbnail not found!");
    }

    #[test]
    fn invalid_storage_filename_is_rejected() {
        let dir = TestDir::new("invalid");
        let files = ServerFiles::new(&dir.path);

        files.prepare().unwrap();

        fs::write(dir.path.join("ab").join("not-a-hash"), b"x").unwrap();

        let error = files.get_all_hashes(ServerFileType::File).unwrap_err();

        assert!(error.is_a(ErrorKind::DamagedOrUnusualFile));
        assert!(error.to_string().contains("not-a-hash"));
    }
}
