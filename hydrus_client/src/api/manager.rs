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
    pub headers: BTreeMap<String, String>,
    pub body: Vec<u8>,
}

impl ApiRequest {
    pub fn get(path: impl Into<String>) -> Self {
        Self {
            method: "GET".to_string(),
            path: path.into(),
            query: BTreeMap::new(),
            headers: BTreeMap::new(),
            body: Vec::new(),
        }
    }

    pub fn post(path: impl Into<String>, body: Vec<u8>) -> Self {
        Self {
            method: "POST".to_string(),
            body,
            ..Self::get(path)
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
    pub binary: Option<(String, Vec<u8>)>,
}

impl ApiResponse {
    pub fn ok(body: Value) -> Self {
        Self {
            status: 200,
            body,
            binary: None,
        }
    }

    pub fn error(status: u16, message: impl Into<String>) -> Self {
        Self {
            status,
            body: json!({ "error": message.into() }),
            binary: None,
        }
    }

    pub fn file(content_type: impl Into<String>, bytes: Vec<u8>) -> Self {
        Self {
            status: 200,
            body: Value::Null,
            binary: Some((content_type.into(), bytes)),
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

    fn search_files(&self, _tags: &[String]) -> Vec<i64> {
        Vec::new()
    }

    /// Metadata dictionaries for `/get_files/file_metadata`.
    fn file_metadata(&self, _file_ids: &[i64]) -> Vec<Value> {
        Vec::new()
    }

    /// Raw file content with a MIME type for `/get_files/file`.
    fn file_binary(&self, _file_id: i64) -> Option<(String, Vec<u8>)> {
        None
    }

    fn file_ids_for_hashes(&self, _hashes: &[String]) -> Vec<i64> {
        Vec::new()
    }

    /// Adds tags to a local tag service. Returns the number of mappings added.
    fn add_tags(
        &self,
        _service_name: &str,
        _file_ids: &[i64],
        _tags: &[String],
    ) -> Result<usize, String> {
        Err("tag writing is not available".to_string())
    }

    /// Removes tags from a local tag service.
    fn remove_tags(
        &self,
        _service_name: &str,
        _file_ids: &[i64],
        _tags: &[String],
    ) -> Result<usize, String> {
        Err("tag writing is not available".to_string())
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
                .or_else(|| request.query.get("AccessKey"))
                .or_else(|| request.headers.get("hydrus-client-api-access-key"));

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

                let mut body = json!({
                    "file_ids": self.backend.search_files(&tags),
                });

                if request
                    .query
                    .get("return_hashes")
                    .is_some_and(|value| value == "true" || value == "1")
                {
                    let ids: Vec<i64> = body["file_ids"]
                        .as_array()
                        .map(|ids| ids.iter().filter_map(Value::as_i64).collect())
                        .unwrap_or_default();

                    let hashes: Vec<Value> = self
                        .backend
                        .file_metadata(&ids)
                        .into_iter()
                        .filter_map(|metadata| metadata.get("hash").cloned())
                        .collect();

                    body["hashes"] = Value::Array(hashes);
                }

                ApiResponse::ok(body)
            }
            "/get_files/file_metadata" | "/get_files/file_metadata/" => {
                let ids = request_ids(request);
                let ids = if ids.is_empty() {
                    let hashes = request_hashes(request);
                    self.backend.file_ids_for_hashes(&hashes)
                } else {
                    ids
                };

                let metadata = self.backend.file_metadata(&ids);

                if let Some(hash) = request.query.get("hash") {
                    if let Some(found) = metadata.iter().find(|item| {
                        item.get("hash").and_then(Value::as_str) == Some(hash.as_str())
                    }) {
                        return ApiResponse::ok(json!({ "metadata": [found] }));
                    }
                }

                ApiResponse::ok(json!({ "metadata": metadata }))
            }
            "/get_files/file" | "/get_files/file/" => {
                let Some(file_id) = request
                    .query
                    .get("file_id")
                    .and_then(|value| value.parse::<i64>().ok())
                else {
                    return ApiResponse::error(400, "file_id is required");
                };

                match self.backend.file_binary(file_id) {
                    Some((content_type, bytes)) => ApiResponse::file(content_type, bytes),
                    None => ApiResponse::error(404, "file not found"),
                }
            }
            "/add_tags/add_tags" | "/add_tags/delete_tags" => {
                if request.method != "POST" {
                    return ApiResponse::error(405, "POST is required");
                }

                let add = request.path.contains("add_tags");

                let Ok(payload) = serde_json::from_slice::<Value>(&request.body) else {
                    return ApiResponse::error(400, "invalid json body");
                };

                let ids = ids_from_json(&payload);
                let ids = if ids.is_empty() {
                    let hashes: Vec<String> = payload
                        .get("hashes")
                        .and_then(Value::as_array)
                        .map(|values| {
                            values
                                .iter()
                                .filter_map(Value::as_str)
                                .map(str::to_string)
                                .collect()
                        })
                        .unwrap_or_default();

                    self.backend.file_ids_for_hashes(&hashes)
                } else {
                    ids
                };

                let service_tags = payload
                    .get("service_names_to_tags")
                    .and_then(Value::as_object)
                    .cloned()
                    .unwrap_or_default();

                let mut notes = Vec::new();

                for (service, values) in service_tags {
                    let tags: Vec<String> = values
                        .as_array()
                        .map(|values| {
                            values
                                .iter()
                                .filter_map(Value::as_str)
                                .map(str::to_string)
                                .collect()
                        })
                        .unwrap_or_default();

                    let result = if add {
                        self.backend.add_tags(&service, &ids, &tags)
                    } else {
                        self.backend.remove_tags(&service, &ids, &tags)
                    };

                    match result {
                        Ok(count) => notes.push(json!({
                            "service": service,
                            "added": count,
                        })),
                        Err(error) => notes.push(json!({
                            "service": service,
                            "error": error,
                        })),
                    }
                }

                ApiResponse::ok(json!({ "notes": notes }))
            }
            _ => ApiResponse::error(404, "no such endpoint"),
        }
    }
}

fn request_ids(request: &ApiRequest) -> Vec<i64> {
    request
        .query
        .get("file_ids")
        .map(|value| {
            value
                .split(',')
                .filter_map(|id| id.trim().parse::<i64>().ok())
                .collect()
        })
        .unwrap_or_default()
}

fn request_hashes(request: &ApiRequest) -> Vec<String> {
    let mut hashes: Vec<String> = request
        .query
        .get("hashes")
        .map(|value| {
            value
                .split(',')
                .map(str::trim)
                .filter(|hash| !hash.is_empty())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();

    if let Some(hash) = request.query.get("hash") {
        if !hash.is_empty() && !hashes.contains(hash) {
            hashes.push(hash.clone());
        }
    }

    hashes
}

fn ids_from_json(payload: &Value) -> Vec<i64> {
    payload
        .get("file_ids")
        .and_then(Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(|value| value.as_i64().or_else(|| value.as_str()?.parse().ok()))
                .collect()
        })
        .unwrap_or_default()
}

pub fn percent_decode(value: &str) -> String {
    percent_encoding::percent_decode_str(&value.replace('+', " "))
        .decode_utf8_lossy()
        .into_owned()
}

#[cfg(test)]
mod tests;
