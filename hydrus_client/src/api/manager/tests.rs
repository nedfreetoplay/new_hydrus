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

    fn search_files(&self, _tags: &[String]) -> Vec<i64> {
        vec![1, 2]
    }

    fn file_metadata(&self, file_ids: &[i64]) -> Vec<Value> {
        file_ids
            .iter()
            .map(|file_id| json!({"file_id": file_id, "hash": "ab"}))
            .collect()
    }

    fn file_binary(&self, file_id: i64) -> Option<(String, Vec<u8>)> {
        (file_id == 1).then(|| ("image/png".to_string(), vec![1, 2, 3]))
    }

    fn file_ids_for_hashes(&self, hashes: &[String]) -> Vec<i64> {
        hashes.iter().map(|_| 1).collect()
    }

    fn add_tags(
        &self,
        _service_name: &str,
        file_ids: &[i64],
        tags: &[String],
    ) -> Result<usize, String> {
        Ok(file_ids.len() * tags.len())
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

    assert_eq!(response.body["file_ids"][0], json!(1));
    assert_eq!(response.body["file_ids"][1], json!(2));
}

#[test]
fn unknown_routes_are_404() {
    let manager = ApiManager::new(MockBackend);

    assert_eq!(manager.handle(&ApiRequest::get("/nope")).status, 404);
}

#[test]
fn metadata_and_binary_file_endpoints_work() {
    let manager = ApiManager::new(MockBackend);

    let metadata =
        manager.handle(&ApiRequest::get("/get_files/file_metadata").with_query("file_ids", "1,2"));

    assert_eq!(metadata.body["metadata"].as_array().unwrap().len(), 2);

    let file = manager.handle(&ApiRequest::get("/get_files/file").with_query("file_id", "1"));

    assert_eq!(file.status, 200);
    assert_eq!(file.binary, Some(("image/png".to_string(), vec![1, 2, 3])));

    let missing = manager.handle(&ApiRequest::get("/get_files/file").with_query("file_id", "99"));

    assert_eq!(missing.status, 404);
}

#[test]
fn add_tags_accepts_hashes_and_counts_mappings() {
    let manager = ApiManager::new(MockBackend);
    let payload = json!({
        "hashes": ["aa"],
        "service_names_to_tags": {"local tags": ["blue eyes", "sky"]}
    })
    .to_string();

    let response = manager.handle(&ApiRequest::post(
        "/add_tags/add_tags",
        payload.into_bytes(),
    ));

    assert_eq!(response.status, 200);
    assert_eq!(response.body["notes"][0]["added"], json!(2));
}

#[test]
fn access_key_is_checked_when_configured() {
    let manager = ApiManager::with_access_key(MockBackend, "secret");

    assert_eq!(manager.handle(&ApiRequest::get("/api_version")).status, 401);

    let allowed =
        manager.handle(&ApiRequest::get("/api_version").with_query("access_key", "secret"));

    assert_eq!(allowed.status, 200);
}
