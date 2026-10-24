use super::*;

pub(super) use hydrus_core::hydrus_constants::DEFAULT_SERVICE_PORT;
pub(super) 

use std::sync::atomic::{AtomicU64, Ordering};
use hydrus_core::hydrus_db::NullDBController;

static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

pub(super) struct TestDir {
    pub(super) path: PathBuf,
}

impl TestDir {
    pub(super) fn new() -> Self {
        let suffix = format!(
            "{}_{}",
            std::process::id(),
            TEST_COUNTER.fetch_add(1, Ordering::Relaxed)
        );

        let path = std::env::temp_dir()
            .join("hydrus-server-db-tests")
            .join(suffix);

        std::fs::create_dir_all(&path).unwrap();

        Self { path }
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

pub(super) fn make_db(dir: &TestDir) -> Arc<ServerDB> {
    hydrus_core::initialise();

    ServerDB::new(Arc::new(NullDBController), &dir.path, "server").unwrap()
}

pub(super) fn add_file_to_new_repository(
    db: &ServerDB,
    dir: &TestDir,
    hash: Vec<u8>,
    label: &str,
) -> (i64, Vec<u8>, i64) {
    let source_path = dir.path.join(format!("{label}.bin"));
    std::fs::write(&source_path, b"repository file").unwrap();

    let file = ServerFileInput {
        hash: hash.clone(),
        size: 15,
        mime: Mime::OctetStream,
        width: None,
        height: None,
        duration_ms: None,
        num_frames: None,
        num_words: None,
        path: source_path,
        thumbnail: None,
    };

    let master_hash_id = db.add_file(&file).unwrap();
    let service = generate_service(
        vec![0x22; 32],
        ServiceType::FileRepository,
        format!("{label} repository"),
        DEFAULT_SERVICE_PORT,
        None,
    )
    .unwrap();
    let service_id = db.add_service(&service).unwrap();
    let service_key = service.get_service_key().to_vec();
    let (hash_map, _) = generate_repository_master_map_table_names(service_id);
    let (current_files, _, _, _, _) =
        generate_repository_files_table_names(service_id);
    let db_base = db.db().db_base().unwrap();

    db_base
        .execute(
            &format!(
                "INSERT INTO {hash_map} ( master_hash_id, hash_id_timestamp ) VALUES ( ?, ? );"
            ),
            &[&master_hash_id, &hydrus_time::get_now()],
        )
        .unwrap();
    let service_hash_id = db_base.get_last_row_id();
    db_base
        .execute(
            &format!(
                "INSERT INTO {current_files} ( service_hash_id, account_id, file_timestamp ) VALUES ( ?, ?, ? );"
            ),
            &[&service_hash_id, &1_i64, &hydrus_time::get_now()],
        )
        .unwrap();

    (service_id, service_key, master_hash_id)
}

pub(super) fn create_file_repository(db: &ServerDB, label: &str) -> (i64, Vec<u8>) {
    let service = generate_service(
        vec![0x33; 32],
        ServiceType::FileRepository,
        format!("{label} repository"),
        DEFAULT_SERVICE_PORT,
        None,
    )
    .unwrap();
    let service_id = db.add_service(&service).unwrap();

    (service_id, service.get_service_key().to_vec())
}

pub(super) fn create_file_repository_with_access(
    db: &ServerDB,
    label: &str,
) -> (i64, Vec<u8>, Vec<u8>) {
    let service = generate_service(
        vec![0x34; 32],
        ServiceType::FileRepository,
        format!("{label} repository"),
        DEFAULT_SERVICE_PORT,
        None,
    )
    .unwrap();
    let (service_id, access_key) =
        db.add_service_with_admin_access_key(&service).unwrap();

    (
        service_id,
        service.get_service_key().to_vec(),
        access_key,
    )
}

pub(super) fn create_tag_repository(db: &ServerDB, label: &str) -> (i64, Vec<u8>) {
    let service = generate_service(
        vec![0x44; 32],
        ServiceType::TagRepository,
        format!("{label} repository"),
        DEFAULT_SERVICE_PORT,
        None,
    )
    .unwrap();
    let service_id = db.add_service(&service).unwrap();

    (service_id, service.get_service_key().to_vec())
}
