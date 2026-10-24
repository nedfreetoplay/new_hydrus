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
mod tests;
