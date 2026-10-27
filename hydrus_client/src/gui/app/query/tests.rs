use super::*;
use crate::controller::Controller;
use crate::db::{ClientDB, files_metadata, files_storage, mappings, master, master_tags};
use hydrus_core::files::hydrus_file_handling::FileInfo;
use hydrus_core::hydrus_constants::Mime;

fn database() -> (ClientDB, std::path::PathBuf) {
    let directory = std::env::temp_dir().join(format!(
        "hydrus_client_query_{}_{}",
        std::process::id(),
        rand::random::<u64>()
    ));

    (ClientDB::new(&directory).unwrap(), directory)
}

#[test]
fn multi_word_tags_stay_one_predicate() {
    let (db, directory) = database();

    master_tags::get_tag_id(db.db(), "blue eyes").unwrap();
    master_tags::get_tag_id(db.db(), "sky").unwrap();

    let plan = search::plan(
        db.db(),
        &crate::services::ServicesManager::default(),
        "blue eyes -sky system:inbox",
    )
    .unwrap();
    let terms: Vec<&str> = plan.tags.iter().map(|term| term.text.as_str()).collect();

    assert_eq!(terms, vec!["blue eyes", "sky"]);
    assert!(!plan.tags[1].inclusive);
    assert_eq!(plan.predicates.len(), 1);

    drop(db);
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn unknown_phrases_fall_back_to_separate_words() {
    let (db, directory) = database();

    master_tags::get_tag_id(db.db(), "blue eyes").unwrap();

    let plan = search::plan(
        db.db(),
        &crate::services::ServicesManager::default(),
        "blue eyes mystery",
    )
    .unwrap();
    let terms: Vec<&str> = plan.tags.iter().map(|term| term.text.as_str()).collect();

    assert_eq!(terms, vec!["blue eyes", "mystery"]);

    drop(db);
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn or_groups_match_any_member() {
    let directory = std::env::temp_dir().join(format!(
        "hydrus_client_query_or_{}_{}",
        std::process::id(),
        rand::random::<u64>()
    ));
    let controller = Controller::new(&directory).unwrap();
    controller.init_model().unwrap();
    let db = controller.db().db();

    let file_service = controller
        .services()
        .get_service(b"my files")
        .unwrap()
        .service_id;
    let tag_service = controller
        .services()
        .get_service(b"local tags")
        .unwrap()
        .service_id;

    let alpha = master::get_hash_id(db, &[0x51; 32]).unwrap();
    let beta = master::get_hash_id(db, &[0x52; 32]).unwrap();
    let other = master::get_hash_id(db, &[0x53; 32]).unwrap();

    for hash_id in [alpha, beta, other] {
        files_storage::add_files(
            db,
            file_service,
            ContentStatus::Current,
            &[hash_id],
            Some(hydrus_core::hydrus_time::get_now()),
        )
        .unwrap();
    }

    let alpha_tag = master_tags::get_tag_id(db, "alpha").unwrap();
    let beta_tag = master_tags::get_tag_id(db, "beta").unwrap();
    mappings::add_mappings(db, tag_service, ContentStatus::Current, alpha, &[alpha_tag]).unwrap();
    mappings::add_mappings(db, tag_service, ContentStatus::Current, beta, &[beta_tag]).unwrap();

    let mut page = QueryPage::new("p1", "files", "my files");
    page.query = "{alpha | beta}".to_string();
    run_query(&controller, &mut page).unwrap();

    assert_eq!(page.hashes.len(), 2);
    assert!(page.hashes.contains(&vec![0x51; 32]));
    assert!(page.hashes.contains(&vec![0x52; 32]));

    page.query = "{alp* | beta}".to_string();
    run_query(&controller, &mut page).unwrap();

    assert_eq!(page.hashes.len(), 2);

    page.query = "-{alpha | beta}".to_string();
    run_query(&controller, &mut page).unwrap();

    assert_eq!(page.hashes, vec![vec![0x53; 32]]);

    page.query = "{alpha | system:width > 100}".to_string();
    files_metadata::set_file_info(
        db,
        beta,
        &FileInfo {
            size: 1024,
            mime: Mime::Png,
            width: Some(400),
            height: Some(200),
            duration_ms: None,
            num_frames: None,
            has_audio: false,
            num_words: None,
        },
    )
    .unwrap();
    run_query(&controller, &mut page).unwrap();

    assert_eq!(page.hashes.len(), 2);

    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn append_hashes_dedupes_and_updates_fixed_pages() {
    let mut page = QueryPage::new("p1", "files", "my files");
    page.hashes = vec![vec![1], vec![2]];

    append_hashes(&mut page, vec![vec![2], vec![3]]);

    assert_eq!(page.hashes, vec![vec![1], vec![2], vec![3]]);
    assert_eq!(page.status, "3 files");

    page.fixed_hashes = Some(vec![vec![9]]);
    append_hashes(&mut page, vec![vec![3], vec![9]]);

    assert_eq!(page.fixed_hashes, Some(vec![vec![9], vec![3]]));
    assert_eq!(page.hashes, vec![vec![1], vec![2], vec![3], vec![9]]);
}

#[test]
fn hex_round_trips_hashes() {
    let hash = vec![0x00, 0xab, 0xff, 0x10];

    assert_eq!(unhex(&hex(&hash)), Some(hash));
    assert_eq!(unhex("xyz"), None);
}

#[test]
fn system_predicates_filter_real_files() {
    let directory = std::env::temp_dir().join(format!(
        "hydrus_client_query_system_{}_{}",
        std::process::id(),
        rand::random::<u64>()
    ));
    let controller = Controller::new(&directory).unwrap();
    controller.init_model().unwrap();
    let db = controller.db().db();

    let file_service = controller
        .services()
        .get_service(b"my files")
        .unwrap()
        .service_id;
    let tag_service = controller
        .services()
        .get_service(b"local tags")
        .unwrap()
        .service_id;

    let wide = master::get_hash_id(db, &[0x11; 32]).unwrap();
    let narrow = master::get_hash_id(db, &[0x22; 32]).unwrap();

    for hash_id in [wide, narrow] {
        files_storage::add_files(
            db,
            file_service,
            ContentStatus::Current,
            &[hash_id],
            Some(hydrus_core::hydrus_time::get_now()),
        )
        .unwrap();
    }

    files_metadata::set_file_info(
        db,
        wide,
        &FileInfo {
            size: 2048,
            mime: Mime::Png,
            width: Some(200),
            height: Some(100),
            duration_ms: None,
            num_frames: None,
            has_audio: false,
            num_words: None,
        },
    )
    .unwrap();

    files_metadata::set_file_info(
        db,
        narrow,
        &FileInfo {
            size: 1024,
            mime: Mime::Png,
            width: Some(20),
            height: Some(10),
            duration_ms: None,
            num_frames: None,
            has_audio: false,
            num_words: None,
        },
    )
    .unwrap();

    let tag = master_tags::get_tag_id(db, "blue eyes").unwrap();
    mappings::add_mappings(db, tag_service, ContentStatus::Current, wide, &[tag]).unwrap();

    let mut page = QueryPage::new("p1", "files", "my files");
    page.query = "blue eyes system:width > 100".to_string();
    run_query(&controller, &mut page).unwrap();

    assert_eq!(page.hashes.len(), 1);
    assert_eq!(page.hashes[0], vec![0x11; 32]);

    page.query = "system:filetype = png".to_string();
    page.sort = "size".to_string();
    page.sort_ascending = false;
    run_query(&controller, &mut page).unwrap();

    assert_eq!(page.hashes, vec![vec![0x11; 32], vec![0x22; 32]]);

    page.sort_ascending = true;
    run_query(&controller, &mut page).unwrap();

    assert_eq!(page.hashes, vec![vec![0x22; 32], vec![0x11; 32]]);

    let _ = std::fs::remove_dir_all(directory);
}
