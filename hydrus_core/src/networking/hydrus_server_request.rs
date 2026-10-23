//! A port of `hydrus.core.networking.HydrusServerRequest`.
//!
//! Python subclasses Twisted's `Request`; Rust has no Twisted stack here, so
//! [`HydrusRequest`] is a plain request record. It holds the same state the
//! Hydrus code adds to the Twisted object: parsed arguments, response context,
//! account, disconnect callbacks, profiling result, and so on.
//!
//! The HTTP machinery that fills these fields lives in the server integration
//! layer; keeping the state separate means the resource logic can be tested
//! without a running reactor.

use std::any::Any;
use std::fs::File;
use std::ops::{Deref, DerefMut};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use indexmap::IndexMap;

use crate::hydrus_constants::Mime;
use crate::hydrus_data::safe_print;
use crate::networking::hydrus_network::Account;
use crate::networking::hydrus_network_variable_handling::ParsedRequestArguments;
use crate::networking::hydrus_server_resources::ResponseContext;
use crate::hydrus_temp::clean_up_temp_path;
use crate::hydrus_time::{get_now_precise, time_delta_to_pretty_timedelta};

/// Case-insensitive request headers.
///
/// Twisted exposes headers through `requestHeaders`; this is the small subset
/// of that API the Hydrus server uses.
#[derive(Clone, Debug, Default)]
pub struct RequestHeaders {
    values: IndexMap<String, Vec<String>>,
}

impl RequestHeaders {
    pub fn new() -> Self {
        Self::default()
    }

    fn key(name: &str) -> String {
        name.to_ascii_lowercase()
    }

    pub fn set(&mut self, name: &str, value: impl Into<String>) {
        self.values
            .insert(Self::key(name), vec![value.into()]);
    }

    pub fn add(&mut self, name: &str, value: impl Into<String>) {
        self.values
            .entry(Self::key(name))
            .or_default()
            .push(value.into());
    }

    pub fn remove(&mut self, name: &str) -> Option<Vec<String>> {
        self.values.shift_remove(&Self::key(name))
    }

    pub fn has_header(&self, name: &str) -> bool {
        self.values.contains_key(&Self::key(name))
    }

    /// `requestHeaders.getRawHeaders`
    pub fn get_raw_headers(&self, name: &str) -> Option<&[String]> {
        self.values.get(&Self::key(name)).map(Vec::as_slice)
    }

    /// The first value for a header, which is what the Hydrus code reads.
    pub fn get_first(&self, name: &str) -> Option<&str> {
        self.get_raw_headers(name)?
            .first()
            .map(String::as_str)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, &[String])> {
        self.values
            .iter()
            .map(|(name, values)| (name.as_str(), values.as_slice()))
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
}

/// The file handle and path produced by [`crate::hydrus_temp::get_temp_path`].
///
/// Python stores these together as `request.temp_file_info`.
pub struct TempFileInfo {
    pub file_handle: File,
    pub temp_path: PathBuf,
}

impl TempFileInfo {
    pub fn new(file_handle: File, temp_path: PathBuf) -> Self {
        Self {
            file_handle,
            temp_path,
        }
    }
}

/// A Python `HydrusRequest`.
pub struct HydrusRequest {
    pub method: String,
    pub path: String,
    pub client_ip: String,
    pub request_headers: RequestHeaders,

    pub start_time: Instant,
    pub parsed_request_args: ParsedRequestArguments,
    pub hydrus_response_context: Option<ResponseContext>,
    pub hydrus_account: Option<Arc<Account>>,
    pub client_api_permissions: Option<Arc<dyn Any + Send + Sync>>,
    pub disconnect_callables: Vec<Arc<dyn Fn() + Send + Sync>>,
    pub preferred_mime: Mime,
    pub disconnected: bool,
    pub profile_result: Option<Arc<dyn Any + Send + Sync>>,

    /// Set by `_check_user_agent`.
    pub is_hydrus_user_agent: bool,
    /// Set by resource code that writes a request body to disk.
    pub temp_file_info: Option<TempFileInfo>,
}

impl HydrusRequest {
    pub fn new(method: impl Into<String>, path: impl Into<String>) -> Self {
        Self {
            method: method.into(),
            path: path.into(),
            client_ip: String::new(),
            request_headers: RequestHeaders::new(),
            start_time: get_now_precise(),
            parsed_request_args: ParsedRequestArguments::new(),
            hydrus_response_context: None,
            hydrus_account: None,
            client_api_permissions: None,
            disconnect_callables: Vec::new(),
            preferred_mime: Mime::Json,
            disconnected: false,
            profile_result: None,
            is_hydrus_user_agent: false,
            temp_file_info: None,
        }
    }

    pub fn with_client_ip(mut self, client_ip: impl Into<String>) -> Self {
        self.client_ip = client_ip.into();
        self
    }

    pub fn is_get(&self) -> bool {
        self.method == "GET"
    }

    pub fn is_post(&self) -> bool {
        self.method == "POST"
    }

    /// `_clean_up_temp_file`
    pub fn clean_up_temp_file(&mut self) {
        if let Some(temp_file_info) = self.temp_file_info.take() {
            clean_up_temp_path(temp_file_info.file_handle, &temp_file_info.temp_path);
        }
    }

    /// Call every registered disconnect callback, swallowing exceptions the
    /// same way Python does.
    pub fn run_disconnect_callables(&mut self) {
        for callback in &self.disconnect_callables {
            let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| callback()));
        }
    }
}

/// A Python `HydrusRequestLogging`.
///
/// Twisted calls `finish` when a response is complete. This wrapper logs the
/// same one-line summary after the response has been written.
pub struct HydrusRequestLogging {
    request: HydrusRequest,
    pub code: Option<u16>,
    pub finished: bool,
}

impl HydrusRequestLogging {
    pub fn new(request: HydrusRequest) -> Self {
        Self {
            request,
            code: None,
            finished: false,
        }
    }

    /// `finish`, returning the logged message.
    ///
    /// Python gets the port from `request.getHost()`. The HTTP layer supplies
    /// it here because this request record does not own a socket.
    pub fn finish(&mut self, host_port: u16) -> String {
        let status_code = match &self.request.hydrus_response_context {
            Some(response_context) => response_context.get_status_code(),
            None => self.code.unwrap_or(200),
        };

        let elapsed = self
            .request
            .start_time
            .elapsed()
            .as_secs_f64();

        let message = format!(
            "{} {} {} {} in {}",
            host_port,
            self.request.method,
            self.request.path,
            status_code,
            time_delta_to_pretty_timedelta(Some(elapsed), true, false)
        );

        safe_print(&message);

        self.finished = true;

        message
    }
}

impl Deref for HydrusRequestLogging {
    type Target = HydrusRequest;

    fn deref(&self) -> &Self::Target {
        &self.request
    }
}

impl DerefMut for HydrusRequestLogging {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.request
    }
}

impl From<HydrusRequest> for HydrusRequestLogging {
    fn from(request: HydrusRequest) -> Self {
        Self::new(request)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_defaults_match_python() {
        let request = HydrusRequest::new("GET", "/");

        assert!(request.is_get());
        assert!(!request.is_post());
        assert_eq!(request.preferred_mime, Mime::Json);
        assert!(!request.disconnected);
        assert!(request.parsed_request_args.is_empty());
        assert!(request.hydrus_response_context.is_none());
        assert!(request.hydrus_account.is_none());
        assert!(request.client_api_permissions.is_none());
        assert!(request.disconnect_callables.is_empty());
        assert!(request.profile_result.is_none());
    }

    #[test]
    fn method_helpers_are_exact() {
        assert!(HydrusRequest::new("GET", "/").is_get());
        assert!(HydrusRequest::new("POST", "/").is_post());
        assert!(!HydrusRequest::new("get", "/").is_get());
        assert!(!HydrusRequest::new("OPTIONS", "/").is_get());
    }

    #[test]
    fn headers_are_case_insensitive_and_keep_all_values() {
        let mut headers = RequestHeaders::new();

        headers.add("User-Agent", "hydrus/20");
        headers.add("user-agent", "other/1");

        assert!(headers.has_header("USER-AGENT"));
        assert_eq!(headers.get_first("User-Agent"), Some("hydrus/20"));
        assert_eq!(
            headers.get_raw_headers("user-agent"),
            Some(["hydrus/20".to_string(), "other/1".to_string()].as_slice())
        );

        headers.set("User-Agent", "replaced/1");

        assert_eq!(headers.get_first("user-agent"), Some("replaced/1"));
        assert!(headers.remove("USER-AGENT").is_some());
        assert!(!headers.has_header("user-agent"));
    }

    #[test]
    fn logging_finish_reports_response_status() {
        let mut request = HydrusRequest::new("GET", "/welcome");
        request.hydrus_response_context =
            Some(ResponseContext::with_status_and_body(404, "not found"));

        let mut logging = HydrusRequestLogging::new(request);

        let message = logging.finish(45871);

        assert!(logging.finished);
        assert!(message.starts_with("45871 GET /welcome 404 in "));
    }

    #[test]
    fn logging_finish_falls_back_to_code_then_200() {
        let mut logging = HydrusRequestLogging::new(HydrusRequest::new("POST", "/x"));
        logging.code = Some(503);

        assert!(logging.finish(1).starts_with("1 POST /x 503 in "));

        let mut logging = HydrusRequestLogging::new(HydrusRequest::new("POST", "/x"));

        assert!(logging.finish(1).starts_with("1 POST /x 200 in "));
    }
}
