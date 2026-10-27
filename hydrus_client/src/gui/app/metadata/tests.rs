use super::*;
use crate::networking::client_networking_url_class::{
    PathComponent, URL_TYPE_FILE, URL_TYPE_GALLERY,
};

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
