//! Rust port of `hydrus/server/ServerController.py`.
//!
//! The controller is being built up in layers. This first layer owns the
//! server directory layout and installs the process-global controller that
//! `ServerFiles` reads. The database, service life cycle and HTTP listeners
//! will be added on top of this bootstrap without changing that contract.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use hydrus_core::hydrus_exceptions::HydrusResult;

use crate::server_files::ServerFiles;
use crate::server_globals::{self, ServerController};

/// The server's `Controller`.
pub struct Controller {
    db_dir: PathBuf,
    files_dir: PathBuf,
}

impl Controller {
    /// `Controller( db_dir, logger )` for the filesystem layer.
    pub fn new(db_dir: impl Into<PathBuf>) -> Self {
        let db_dir = db_dir.into();
        let files_dir = db_dir.join("server_files");

        Self { db_dir, files_dir }
    }

    pub fn db_dir(&self) -> &Path {
        &self.db_dir
    }

    pub fn files(&self) -> ServerFiles {
        ServerFiles::new(&self.files_dir)
    }

    /// Create the `server_files` root and its prefix directories.
    pub fn prepare_files_dir(&self) -> HydrusResult<()> {
        self.files().prepare()
    }

    /// Install this controller in the `ServerGlobals` slot.
    pub fn install(self: &Arc<Self>) {
        let controller: Arc<dyn ServerController> = self.clone();

        server_globals::set_server_controller(&controller);
    }
}

impl ServerController for Controller {
    fn get_files_dir(&self) -> &Path {
        &self.files_dir
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::fs;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

    struct TestDir {
        path: PathBuf,
    }

    impl TestDir {
        fn new() -> Self {
            let suffix = format!(
                "{}_{}",
                std::process::id(),
                TEST_COUNTER.fetch_add(1, Ordering::Relaxed)
            );

            let path = std::env::temp_dir()
                .join("hydrus-server-controller-tests")
                .join(suffix);

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
    fn controller_derives_the_server_files_directory() {
        let dir = TestDir::new();
        let controller = Controller::new(&dir.path);

        assert_eq!(controller.db_dir(), dir.path);
        assert_eq!(
            controller.get_files_dir(),
            dir.path.join("server_files")
        );
    }

    #[test]
    fn preparing_creates_the_storage_tree() {
        let dir = TestDir::new();
        let controller = Controller::new(&dir.path);

        controller.prepare_files_dir().unwrap();

        assert!(controller.get_files_dir().is_dir());
        assert!(controller.get_files_dir().join("00").is_dir());
        assert!(controller.get_files_dir().join("ff").is_dir());
    }
}
