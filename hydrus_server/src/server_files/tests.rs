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
