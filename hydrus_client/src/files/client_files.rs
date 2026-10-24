//! File read/write helpers from `hydrus/client/files/ClientFiles.py`.

use std::path::Path;
use std::sync::RwLock;

use hydrus_core::hydrus_exceptions::{ErrorKind, HydrusException, HydrusResult};
use hydrus_core::hydrus_paths;

pub fn read_file(path: &Path) -> HydrusResult<Vec<u8>> {
    std::fs::read(path).map_err(|error| {
        HydrusException::new(
            ErrorKind::FileMissing,
            format!("Could not read {}: {error}", path.display()),
        )
    })
}

pub fn write_file(path: &Path, data: &[u8]) -> HydrusResult<()> {
    if let Some(parent) = path.parent() {
        hydrus_paths::make_sure_directory_exists(parent)?;
    }

    std::fs::write(path, data).map_err(|error| {
        HydrusException::new(
            ErrorKind::FileMissing,
            format!("Could not write {}: {error}", path.display()),
        )
    })
}

pub fn file_exists(path: &Path) -> bool {
    path.is_file()
}

pub fn get_size(path: &Path) -> HydrusResult<u64> {
    std::fs::metadata(path)
        .map(|metadata| metadata.len())
        .map_err(|error| {
            HydrusException::new(
                ErrorKind::FileMissing,
                format!("Could not stat {}: {error}", path.display()),
            )
        })
}

pub fn delete_file(path: &Path) -> HydrusResult<()> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(HydrusException::new(
            ErrorKind::FileMissing,
            format!("Could not delete {}: {error}", path.display()),
        )),
    }
}

/// Cooperative read/write lock used while a file is being read or rewritten.
#[derive(Debug, Default)]
pub struct FileRWLock {
    lock: RwLock<()>,
}

impl FileRWLock {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_read<T>(&self, call: impl FnOnce() -> T) -> T {
        let _guard = self.lock.read().unwrap();
        call()
    }

    pub fn with_write<T>(&self, call: impl FnOnce() -> T) -> T {
        let _guard = self.lock.write().unwrap();
        call()
    }

    pub fn is_locked(&self) -> bool {
        self.lock.try_write().is_err()
    }

    pub fn readers_are_working(&self) -> bool {
        self.is_locked()
    }

    pub fn writers_are_waiting_or_working(&self) -> bool {
        self.is_locked()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir() -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "hydrus_client_files_{}_{}",
            std::process::id(),
            rand::random::<u64>()
        ))
    }

    #[test]
    fn write_read_delete_round_trip() {
        let directory = temp_dir();
        let path = directory.join("sub").join("file.bin");

        write_file(&path, b"hello").unwrap();

        assert!(file_exists(&path));
        assert_eq!(get_size(&path).unwrap(), 5);
        assert_eq!(read_file(&path).unwrap(), b"hello");

        delete_file(&path).unwrap();

        assert!(!file_exists(&path));

        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn deleting_a_missing_file_is_ok() {
        let path = temp_dir().join("missing.bin");

        assert!(delete_file(&path).is_ok());
    }

    #[test]
    fn read_lock_allows_shared_readers() {
        let lock = FileRWLock::new();

        let value = lock.with_read(|| lock.with_read(|| 42));

        assert_eq!(value, 42);
        assert!(!lock.is_locked());
    }

    #[test]
    fn write_lock_is_exclusive() {
        let lock = FileRWLock::new();
        let locked = lock.with_write(|| lock.is_locked());

        assert!(locked);
    }
}
