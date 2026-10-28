//! Local Client API routing (`hydrus/client/ClientAPI.py` and
//! `networking/api/ClientLocalServer.py`).

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use hydrus_core::hydrus_constants::{Filter, CLIENT_API_VERSION};
use hydrus_core::hydrus_tags::TagFilter;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// Client API permissions, from `ClientAPI.CLIENT_API_PERMISSION_*`.
pub const PERMISSION_ADD_URLS: u16 = 0;
pub const PERMISSION_ADD_FILES: u16 = 1;
pub const PERMISSION_ADD_TAGS: u16 = 2;
pub const PERMISSION_SEARCH_FILES: u16 = 3;
pub const PERMISSION_MANAGE_PAGES: u16 = 4;
pub const PERMISSION_MANAGE_HEADERS: u16 = 5;
pub const PERMISSION_MANAGE_DATABASE: u16 = 6;
pub const PERMISSION_ADD_NOTES: u16 = 7;
pub const PERMISSION_MANAGE_FILE_RELATIONSHIPS: u16 = 8;
pub const PERMISSION_EDIT_RATINGS: u16 = 9;
pub const PERMISSION_MANAGE_POPUPS: u16 = 10;
pub const PERMISSION_EDIT_TIMES: u16 = 11;
pub const PERMISSION_COMMIT_PENDING: u16 = 12;
pub const PERMISSION_SEE_LOCAL_PATHS: u16 = 13;

/// `ALLOWED_PERMISSIONS` with the labels Hydrus shows in the permissions
/// dialog.
pub const ALLOWED_PERMISSIONS: &[(u16, &str)] = &[
    (PERMISSION_ADD_FILES, "import and delete files"),
    (PERMISSION_ADD_TAGS, "edit file tags"),
    (PERMISSION_ADD_URLS, "import and edit urls"),
    (PERMISSION_SEARCH_FILES, "search for and fetch files"),
    (PERMISSION_MANAGE_PAGES, "manage pages"),
    (PERMISSION_MANAGE_HEADERS, "manage cookies and headers"),
    (PERMISSION_MANAGE_DATABASE, "manage database"),
    (PERMISSION_ADD_NOTES, "edit file notes"),
    (
        PERMISSION_MANAGE_FILE_RELATIONSHIPS,
        "edit file relationships",
    ),
    (PERMISSION_EDIT_RATINGS, "edit file ratings"),
    (PERMISSION_MANAGE_POPUPS, "manage popups"),
    (PERMISSION_EDIT_TIMES, "edit file times"),
    (PERMISSION_COMMIT_PENDING, "commit pending"),
    (PERMISSION_SEE_LOCAL_PATHS, "see local file paths"),
];

/// One access key and its permissions. Persisted in `api permissions.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiPermissions {
    pub name: String,
    pub access_key: String,
    #[serde(default)]
    pub permits_everything: bool,
    #[serde(default)]
    pub basic_permissions: Vec<u16>,
    #[serde(default)]
    pub search_tag_filter: Vec<(String, String)>,
}

impl ApiPermissions {
    pub fn new(name: impl Into<String>, access_key: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            access_key: access_key.into(),
            permits_everything: true,
            basic_permissions: Vec::new(),
            search_tag_filter: Vec::new(),
        }
    }

    pub fn has_permission(&self, permission: u16) -> bool {
        self.permits_everything || self.basic_permissions.contains(&permission)
    }

    pub fn tag_filter(&self) -> TagFilter {
        let mut filter = TagFilter::new();

        for (slice, rule) in &self.search_tag_filter {
            filter.set_rule(
                slice,
                if rule == "whitelist" {
                    Filter::Whitelist
                } else {
                    Filter::Blacklist
                },
            );
        }

        filter.clean_rules();
        filter
    }

    pub fn search_is_unrestricted(&self) -> bool {
        self.permits_everything || self.tag_filter().allows_everything()
    }
}

pub fn load_permissions(path: &Path) -> Vec<ApiPermissions> {
    let Ok(text) = std::fs::read_to_string(path) else {
        return Vec::new();
    };

    serde_json::from_str(&text).unwrap_or_default()
}

pub fn save_permissions(path: &Path, permissions: &[ApiPermissions]) -> std::io::Result<()> {
    let dump = serde_json::to_string_pretty(permissions).map_err(std::io::Error::other)?;
    std::fs::write(path, dump)
}

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
    permissions: Vec<ApiPermissions>,
}

impl<B: ApiBackend> ApiManager<B> {
    pub fn new(backend: B) -> Self {
        Self {
            backend,
            permissions: Vec::new(),
        }
    }

    /// A single unrestricted key. Kept for callers that do not use the
    /// permissions file.
    pub fn with_access_key(backend: B, access_key: impl Into<String>) -> Self {
        Self {
            backend,
            permissions: vec![ApiPermissions::new("default", access_key)],
        }
    }

    pub fn with_permissions(backend: B, permissions: Vec<ApiPermissions>) -> Self {
        Self {
            backend,
            permissions,
        }
    }

    pub fn backend(&self) -> &B {
        &self.backend
    }

    pub fn permissions(&self) -> &[ApiPermissions] {
        &self.permissions
    }

    fn requested_access_key(request: &ApiRequest) -> Option<&str> {
        request
            .query
            .get("access_key")
            .or_else(|| request.query.get("AccessKey"))
            .or_else(|| request.headers.get("hydrus-client-api-access-key"))
            .map(String::as_str)
    }

    fn permissions_for(
        &self,
        request: &ApiRequest,
    ) -> Result<Option<&ApiPermissions>, ApiResponse> {
        if self.permissions.is_empty() {
            return Ok(None);
        }

        let supplied = Self::requested_access_key(request);

        match supplied.and_then(|key| {
            self.permissions
                .iter()
                .find(|permissions| permissions.access_key == key)
        }) {
            Some(permissions) => Ok(Some(permissions)),
            None => Err(ApiResponse::error(401, "access key required")),
        }
    }

    fn check_permission(
        permissions: Option<&ApiPermissions>,
        permission: u16,
    ) -> Option<ApiResponse> {
        let permissions = permissions?;

        if permissions.has_permission(permission) {
            None
        } else {
            let label = ALLOWED_PERMISSIONS
                .iter()
                .find(|(value, _)| *value == permission)
                .map(|(_, label)| *label)
                .unwrap_or("this action");

            Some(ApiResponse::error(
                403,
                format!("you do not have permission to {label}"),
            ))
        }
    }

    fn filter_file_ids(&self, permissions: Option<&ApiPermissions>, ids: Vec<i64>) -> Vec<i64> {
        let Some(permissions) = permissions else {
            return ids;
        };

        if permissions.search_is_unrestricted() {
            return ids;
        }

        let filter = permissions.tag_filter();
        let metadata = self.backend.file_metadata(&ids);
        let mut allowed = BTreeSet::new();

        for item in metadata {
            let Some(file_id) = item.get("file_id").and_then(Value::as_i64) else {
                continue;
            };
            let Some(tags) = item.get("tags").and_then(Value::as_object) else {
                continue;
            };

            let passes = tags.values().any(|values| {
                values.as_array().is_some_and(|values| {
                    values
                        .iter()
                        .filter_map(Value::as_str)
                        .any(|tag| filter.tag_ok(tag, false))
                })
            });

            if passes {
                allowed.insert(file_id);
            }
        }

        ids.into_iter()
            .filter(|file_id| allowed.contains(file_id))
            .collect()
    }

    fn check_files_visible(
        &self,
        permissions: Option<&ApiPermissions>,
        ids: &[i64],
    ) -> Option<ApiResponse> {
        let Some(permissions) = permissions else {
            return None;
        };

        if permissions.search_is_unrestricted() {
            return None;
        }

        let visible = self.filter_file_ids(Some(permissions), ids.to_vec());

        if visible.len() == ids.len() {
            None
        } else {
            Some(ApiResponse::error(
                403,
                "you do not have access to all those files",
            ))
        }
    }

    fn check_search_tags(
        permissions: Option<&ApiPermissions>,
        tags: &[String],
    ) -> Option<ApiResponse> {
        let permissions = permissions?;

        if permissions.search_is_unrestricted() {
            return None;
        }

        let filter = permissions.tag_filter();

        if tags.is_empty() || tags.iter().any(|tag| !filter.tag_ok(tag, false)) {
            return Some(ApiResponse::error(
                403,
                format!(
                    "you do not have permission to do this search; your tag search permissions are: {}",
                    filter.to_permitted_string()
                ),
            ));
        }

        None
    }

    pub fn handle(&self, request: &ApiRequest) -> ApiResponse {
        let permissions = match self.permissions_for(request) {
            Ok(permissions) => permissions,
            Err(response) => return response,
        };

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
                if let Some(response) = Self::check_permission(permissions, PERMISSION_SEARCH_FILES)
                {
                    return response;
                }

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

                if let Some(response) = Self::check_search_tags(permissions, &tags) {
                    return response;
                }

                let ids = self.filter_file_ids(permissions, self.backend.search_files(&tags));
                let mut body = json!({ "file_ids": ids });

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
                if let Some(response) = Self::check_permission(permissions, PERMISSION_SEARCH_FILES)
                {
                    return response;
                }

                let ids = request_ids(request);
                let ids = if ids.is_empty() {
                    let hashes = request_hashes(request);
                    self.backend.file_ids_for_hashes(&hashes)
                } else {
                    ids
                };

                if let Some(response) = self.check_files_visible(permissions, &ids) {
                    return response;
                }

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
                if let Some(response) = Self::check_permission(permissions, PERMISSION_SEARCH_FILES)
                {
                    return response;
                }

                let Some(file_id) = request
                    .query
                    .get("file_id")
                    .and_then(|value| value.parse::<i64>().ok())
                else {
                    return ApiResponse::error(400, "file_id is required");
                };

                if let Some(response) = self.check_files_visible(permissions, &[file_id]) {
                    return response;
                }

                match self.backend.file_binary(file_id) {
                    Some((content_type, bytes)) => ApiResponse::file(content_type, bytes),
                    None => ApiResponse::error(404, "file not found"),
                }
            }
            "/add_tags/add_tags" | "/add_tags/delete_tags" => {
                if let Some(response) = Self::check_permission(permissions, PERMISSION_ADD_TAGS) {
                    return response;
                }

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
