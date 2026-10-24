//! Local Client API routing (`hydrus/client/ClientAPI.py` and
//! `networking/api/ClientLocalServer.py`).

use std::collections::BTreeMap;

use hydrus_core::hydrus_constants::CLIENT_API_VERSION;
use serde_json::{Value, json};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApiRequest {
    pub method: String,
    pub path: String,
    pub query: BTreeMap<String, String>,
    pub body: Vec<u8>,
}

impl ApiRequest {
    pub fn get(path: impl Into<String>) -> Self {
        Self {
            method: "GET".to_string(),
            path: path.into(),
            query: BTreeMap::new(),
            body: Vec::new(),
        }
    }

    pub fn with_query(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.query.insert(key.into(), value.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ApiResponse {
    pub status: u16,
    pub body: Value,
}

impl ApiResponse {
    pub fn ok(body: Value) -> Self {
        Self { status: 200, body }
    }

    pub fn error(status: u16, message: impl Into<String>) -> Self {
        Self {
            status,
            body: json!({ "error": message.into() }),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApiServiceInfo {
    pub name: String,
    pub service_key: String,
    pub service_type: String,
}

pub trait ApiBackend: Send + Sync {
    fn api_version(&self) -> i64 {
        CLIENT_API_VERSION as i64
    }

    fn get_services(&self) -> Vec<ApiServiceInfo>;

    fn search_files(&self, _tags: &[String]) -> Vec<String> {
        Vec::new()
    }
}

pub struct ApiManager<B: ApiBackend> {
    backend: B,
    access_key: Option<String>,
}

impl<B: ApiBackend> ApiManager<B> {
    pub fn new(backend: B) -> Self {
        Self {
            backend,
            access_key: None,
        }
    }

    pub fn with_access_key(backend: B, access_key: impl Into<String>) -> Self {
        Self {
            backend,
            access_key: Some(access_key.into()),
        }
    }

    pub fn backend(&self) -> &B {
        &self.backend
    }

    pub fn handle(&self, request: &ApiRequest) -> ApiResponse {
        if let Some(access_key) = &self.access_key {
            let supplied = request
                .query
                .get("access_key")
                .or_else(|| request.query.get("AccessKey"));

            if supplied != Some(access_key) {
                return ApiResponse::error(401, "access key required");
            }
        }

        match request.path.as_str() {
            "/api_version" | "/api_version/" => {
                ApiResponse::ok(json!({ "version": self.backend.api_version() }))
            }
            "/get_services" | "/get_services/" => ApiResponse::ok(json!({
                "services": self.backend.get_services().iter().map(|service| json!({
                    "name": service.name,
                    "service_key": service.service_key,
                    "type": service.service_type,
                })).collect::<Vec<_>>()
            })),
            "/get_files/search_files" | "/get_files/search_files/" => {
                let tags: Vec<String> = request
                    .query
                    .get("tags")
                    .map(|tags| {
                        tags.split(',')
                            .map(str::trim)
                            .filter(|tag| !tag.is_empty())
                            .map(str::to_string)
                            .collect()
                    })
                    .unwrap_or_default();

                ApiResponse::ok(json!({
                    "file_ids": self.backend.search_files(&tags),
                }))
            }
            _ => ApiResponse::error(404, "no such endpoint"),
        }
    }
}

#[cfg(test)]
mod tests {
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
}
