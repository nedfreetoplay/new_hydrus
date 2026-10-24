
use super::*;

#[test]
fn defaults_to_all_known_tags() {
    let context = TagContext::default();

    assert!(context.is_all_known_tags());
    assert!(context.include_current_tags);
    assert!(context.include_pending_tags);
    assert_eq!(context.display_service_key, context.service_key);
}

#[test]
fn missing_services_fall_back_to_combined() {
    let mut context = TagContext::new(b"gone".to_vec(), true, true, None);
    context.fix_missing_services(|_| false);

    assert!(context.is_all_known_tags());
    assert_eq!(
        context.display_service_key,
        client_constants::COMBINED_TAG_SERVICE_KEY
    );
}

#[test]
fn serialisation_round_trips() {
    let context = TagContext::new(b"local tags".to_vec(), true, false, None);
    let info = context.get_serialisable_info();
    let mut loaded = TagContext::default();

    loaded.initialise_from_serialisable_info(&info).unwrap();

    assert_eq!(loaded, context);
}

#[test]
fn api_dict_contains_expected_keys() {
    let dict = TagContext::default().to_dict_for_api();

    assert_eq!(dict["service_key"], json!("616c6c206b6e6f776e2074616773"));
    assert_eq!(dict["include_current_tags"], json!(true));
}
