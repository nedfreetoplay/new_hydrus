
use super::*;

struct MockBackend;

impl ApiBackend for MockBackend {
    fn get_services(&self) -> Vec<ApiServiceInfo> {
        vec![ApiServiceInfo {
            name: "my files".to_string(),
            service_key: "abcd".to_string(),
            service_type: "local file domain".to_string(),
        }]
    }

    fn search_files(&self, tags: &[String]) -> Vec<String> {
        tags.iter().map(|tag| format!("hash:{tag}")).collect()
    }
}

#[test]
fn api_version_reports_client_api_version() {
    let manager = ApiManager::new(MockBackend);

    let response = manager.handle(&ApiRequest::get("/api_version"));

    assert_eq!(response.status, 200);
    assert_eq!(response.body["version"], json!(CLIENT_API_VERSION));
}

#[test]
fn services_are_serialised() {
    let manager = ApiManager::new(MockBackend);

    let response = manager.handle(&ApiRequest::get("/get_services"));

    assert_eq!(response.body["services"][0]["name"], json!("my files"));
}

#[test]
fn search_files_parses_tag_list() {
    let manager = ApiManager::new(MockBackend);
    let request =
        ApiRequest::get("/get_files/search_files").with_query("tags", "blue eyes, blonde hair");

    let response = manager.handle(&request);

    assert_eq!(response.body["file_ids"][0], json!("hash:blue eyes"));
    assert_eq!(response.body["file_ids"][1], json!("hash:blonde hair"));
}

#[test]
fn unknown_routes_are_404() {
    let manager = ApiManager::new(MockBackend);

    assert_eq!(manager.handle(&ApiRequest::get("/nope")).status, 404);
}

#[test]
fn access_key_is_checked_when_configured() {
    let manager = ApiManager::with_access_key(MockBackend, "secret");

    assert_eq!(manager.handle(&ApiRequest::get("/api_version")).status, 401);

    let allowed =
        manager.handle(&ApiRequest::get("/api_version").with_query("access_key", "secret"));

    assert_eq!(allowed.status, 200);
}
