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
