use super::*;

fn hash() -> Vec<u8> {
    vec![0xab, 0xcd]
}

#[test]
fn statuses_match_python_actions() {
    let add = ContentUpdate::new(
        ContentAction::Add,
        b"local tags".to_vec(),
        ContentType::Mappings,
    );
    let petition = ContentUpdate::new(
        ContentAction::Petition,
        b"local tags".to_vec(),
        ContentType::Mappings,
    );
    let rescind = ContentUpdate::new(
        ContentAction::Rescind,
        b"local tags".to_vec(),
        ContentType::Mappings,
    );

    assert_eq!(add.status_for_action(), Some(ContentStatus::Current));
    assert_eq!(
        petition.status_for_action(),
        Some(ContentStatus::Petitioned)
    );
    assert_eq!(rescind.status_for_action(), None);
}

#[test]
fn packages_filter_updates_by_service_and_type() {
    let mut package = ContentUpdatePackage::new();
    package.add_update(
        ContentUpdate::new(
            ContentAction::Add,
            b"local tags".to_vec(),
            ContentType::Mappings,
        )
        .with_tags(["blue eyes"])
        .with_hashes([hash()]),
    );
    package.add_update(ContentUpdate::new(
        ContentAction::Add,
        b"my files".to_vec(),
        ContentType::Files,
    ));

    assert_eq!(package.get_mappings(b"local tags").len(), 1);
    assert_eq!(package.get_files(b"my files").len(), 1);
    assert_eq!(package.get_files(b"local tags").len(), 0);
}

#[test]
fn api_dict_is_json_friendly() {
    let update = ContentUpdate::new(
        ContentAction::Add,
        b"local tags".to_vec(),
        ContentType::Mappings,
    )
    .with_tags(["blue eyes"])
    .with_hashes([hash()]);

    let dict = update.to_dict_for_api();

    assert_eq!(dict["action"], json!("add"));
    assert_eq!(dict["service_key"], json!("6c6f63616c2074616773"));
    assert_eq!(dict["tags"][0], json!("blue eyes"));
}
