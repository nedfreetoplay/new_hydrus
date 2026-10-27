use super::*;
use crate::networking::client_networking_url_class::{
    PathComponent, URL_TYPE_FILE, URL_TYPE_GALLERY,
};

#[test]
fn manage_tags_counts_tags_across_the_selection() {
    use crate::controller::Controller;
    use crate::db::{master, master_tags};

    let directory = std::env::temp_dir().join(format!(
        "hydrus_manage_tags_{}_{}",
        std::process::id(),
        rand::random::<u64>()
    ));
    let controller = Controller::new(&directory).unwrap();
    controller.init_model().unwrap();
    let db = controller.db().db();

    let service = controller
        .services()
        .get_service(b"local tags")
        .unwrap()
        .service_id;
    let first = master::get_hash_id(db, &[0x31; 32]).unwrap();
    let second = master::get_hash_id(db, &[0x32; 32]).unwrap();
    let shared = master_tags::get_tag_id(db, "shared tag").unwrap();
    let only_first = master_tags::get_tag_id(db, "only first").unwrap();

    mappings::add_mappings(
        db,
        service,
        ContentStatus::Current,
        first,
        &[shared, only_first],
    )
    .unwrap();
    mappings::add_mappings(db, service, ContentStatus::Current, second, &[shared]).unwrap();

    let json = manage_tags_json(&controller, &[vec![0x31; 32], vec![0x32; 32]]);
    let tags = json["services"][0]["tags"].as_array().unwrap();
    let counts: BTreeMap<&str, i64> = tags
        .iter()
        .map(|value| {
            (
                value["tag"].as_str().unwrap(),
                value["count"].as_i64().unwrap(),
            )
        })
        .collect();

    assert_eq!(counts.get("shared tag"), Some(&2));
    assert_eq!(counts.get("only first"), Some(&1));

    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn url_class_json_round_trips_editor_fields() {
    let value = json!({
        "name": "gallery",
        "netloc": "www.example.com",
        "url_type": URL_TYPE_GALLERY,
        "preferred_scheme": "https",
        "path": "gallery/{gallery_id}",
        "parameters": "page=1\nsort=date",
        "match_subdomains": true,
        "keep_matched_subdomains": true,
        "example_url": "https://example.com/gallery/123",
    });

    let class = url_class_from_json(&value, None).unwrap();

    assert_eq!(class.name, "gallery");
    assert_eq!(class.netloc, "example.com");
    assert_eq!(class.url_type, URL_TYPE_GALLERY);
    assert!(class.match_subdomains);
    assert!(class.keep_matched_subdomains);
    assert_eq!(class.parameters["page"], Some("1".to_string()));
    assert_eq!(
        class.path_components[1],
        PathComponent::Variable {
            name: "gallery_id".to_string(),
            default: String::new()
        }
    );
    assert!(class.matches("https://cdn.example.com/gallery/123?page=1&sort=date"));
}

#[test]
fn url_class_editor_can_edit_an_existing_class() {
    let mut class = UrlClass::new("old", "example.com").with_fixed_path(["old"]);
    class.url_type = URL_TYPE_FILE;

    let value = json!({
        "name": "new",
        "netloc": "example.com",
        "url_type": URL_TYPE_GALLERY,
        "path": "new/{id}",
    });

    let updated = url_class_from_json(&value, Some(&class)).unwrap();

    assert_eq!(updated.name, "new");
    assert_eq!(updated.url_type, URL_TYPE_GALLERY);
    assert!(updated.matches("https://example.com/new/7"));
    assert!(!updated.matches("https://example.com/old/7"));
}

#[test]
fn apply_tags_to_hashes_adds_and_removes_for_every_file() {
    use crate::db::master;

    let directory = std::env::temp_dir().join(format!(
        "hydrus_apply_tags_{}_{}",
        std::process::id(),
        rand::random::<u64>()
    ));
    let controller = Controller::new(&directory).unwrap();
    controller.init_model().unwrap();
    let db = controller.db().db();

    let service = controller
        .services()
        .get_service(b"local tags")
        .unwrap()
        .service_id;
    let first_hash = vec![0x41; 32];
    let second_hash = vec![0x42; 32];
    let first = master::get_hash_id(db, &first_hash).unwrap();
    let second = master::get_hash_id(db, &second_hash).unwrap();
    let shared = master_tags::get_tag_id(db, "shared tag").unwrap();

    mappings::add_mappings(db, service, ContentStatus::Current, first, &[shared]).unwrap();
    mappings::add_mappings(db, service, ContentStatus::Current, second, &[shared]).unwrap();

    let (added, removed) = apply_tags_to_hashes(
        &controller,
        &[first_hash.clone(), second_hash.clone()],
        service,
        "new tag\n- shared tag\n, another tag",
    )
    .unwrap();

    assert_eq!((added, removed), (4, 2));

    for (hash_id, expected) in [(first, "new tag"), (second, "another tag")] {
        let tag_ids = mappings::get_tag_ids(db, service, ContentStatus::Current, hash_id).unwrap();
        let tags = master_tags::get_tags(db, &tag_ids).unwrap();

        assert!(tags.iter().any(|tag| tag == expected));
        assert!(!tags.iter().any(|tag| tag == "shared tag"));
    }

    let (added, removed) =
        apply_tags_to_hashes(&controller, &[first_hash], service, "- missing tag").unwrap();

    assert_eq!((added, removed), (0, 0));

    let _ = std::fs::remove_dir_all(directory);
}
