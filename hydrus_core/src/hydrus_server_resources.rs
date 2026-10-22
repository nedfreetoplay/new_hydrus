//! A port of `hydrus.core.networking.HydrusServerResources`.
//!
//! Twisted supplies the `Resource` base class, the reactor, deferreds and the
//! static-file producers. Those pieces belong to the HTTP integration layer,
//! so this module ports the parts that are actually Hydrus logic:
//!
//! * the welcome/robots response bodies,
//! * local-only domain checking,
//! * user-agent and network-version checking,
//! * `ResponseContext` and its automatic cache rules,
//! * HTTP range parsing,
//! * exception-to-status-code mapping and error bodies,
//! * CORS/OPTIONS headers,
//! * assembling the response headers and body/file plan.
//!
//! The original `generate_eris` embeds several hundred lines of hand-coloured
//! ASCII art. That art is presentation, not protocol, so this port keeps the
//! same summary information and returns a small, valid HTML welcome page
//! instead of copying the bitmap. Everything else follows the Python code.

use std::path::PathBuf;
use std::sync::Arc;

use chrono::{Duration, Utc};
use serde_json::{json, Value};

use crate::hydrus_constants::{
    CLIENT_API_VERSION, Mime, NETWORK_VERSION, SOFTWARE_VERSION, ServiceType,
};
use crate::hydrus_data::hex_decode;
use crate::hydrus_exceptions::{ErrorKind, HydrusException, HydrusResult};
use crate::hydrus_serialisable::{SerialisableObject, python_json_dumps};
use crate::hydrus_server_request::{HydrusRequest, RequestHeaders};

/// The service information the resource layer needs.
///
/// Python's resources call methods on a full service object. This trait keeps
/// the resource logic independent of the repository/client-API classes, which
/// have their own ports elsewhere.
pub trait HydrusResourceService {
    fn get_name(&self) -> String;
    fn get_service_type(&self) -> ServiceType;
    fn get_service_key(&self) -> Vec<u8> {
        Vec::new()
    }
    fn allows_non_local_connections(&self) -> bool;
    fn supports_cors(&self) -> bool {
        false
    }
    fn use_normie_eris(&self) -> bool {
        false
    }
    fn logs_requests(&self) -> bool {
        false
    }
    fn report_data_used(&self, num_bytes: i64) {
        let _ = num_bytes;
    }
    fn report_request_used(&self) {}
}

/// `get_server_summary_texts`, preserving the Python tuple's order.
pub fn get_server_summary_texts(service: &dyn HydrusResourceService) -> [String; 5] {
    let name = service.get_name();
    let service_type = service.get_service_type();

    let welcome_text_1 = format!("This is <b>{name}</b>,");
    let welcome_text_2 = format!("a {}.", service_type.long_str());
    let welcome_text_3 = format!("Software version {SOFTWARE_VERSION}");

    let welcome_text_4 = if service_type == ServiceType::ClientApiService {
        format!("API version {CLIENT_API_VERSION}")
    } else {
        format!("Network version {NETWORK_VERSION}")
    };

    let welcome_text_5 = if service.allows_non_local_connections() {
        "It responds to requests from any host."
    } else {
        "It only responds to requests from localhost."
    };

    [
        welcome_text_1,
        welcome_text_2,
        welcome_text_3,
        welcome_text_4,
        welcome_text_5.to_string(),
    ]
}

/// `generate_eris`, using a plain HTML greeting instead of the ASCII art.
pub fn generate_eris(service: &dyn HydrusResourceService) -> String {
    if service.use_normie_eris() {
        return generate_normie_eris(service);
    }

    let name = service.get_name();
    let texts = get_server_summary_texts(service);

    format!(
        "<html><head><title>{name}</title></head><body><pre>\n{}\n{}\n{}\n{}\n{}\n</pre></body></html>",
        texts[0], texts[1], texts[2], texts[3], texts[4]
    )
}

/// `generate_normie_eris`.
pub fn generate_normie_eris(service: &dyn HydrusResourceService) -> String {
    let name = service.get_name();
    let texts = get_server_summary_texts(service);

    format!(
        "<html><head><title>{name}</title></head><body><div id=\"ascgen-image\"><pre>\n{}\n{}\n{}\n{}\n{}\n</pre></div></body></html>",
        texts[0], texts[1], texts[2], texts[3], texts[4]
    )
}

/// A Python `HydrusDomain`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HydrusDomain {
    local_only: bool,
}

impl HydrusDomain {
    pub const fn new(local_only: bool) -> Self {
        Self { local_only }
    }

    /// `check_valid`.
    pub fn check_valid(&self, client_ip: &str) -> HydrusResult<()> {
        if self.local_only
            && !matches!(client_ip, "127.0.0.1" | "::1" | "::ffff:127.0.0.1")
        {
            return Err(HydrusException::new(
                ErrorKind::InsufficientCredentials,
                "Only local access allowed!",
            ));
        }

        Ok(())
    }

    pub fn is_local(&self) -> bool {
        self.local_only
    }
}

/// A response cookie: `( name, value, kwargs )` in the Python code.
#[derive(Clone, Debug, Default)]
pub struct ResponseCookie {
    pub name: String,
    pub value: String,
    /// Extra keyword arguments such as `path`, `domain`, `secure`, or
    /// `max_age`.
    pub options: indexmap::IndexMap<String, String>,
}

impl ResponseCookie {
    pub fn new(name: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
            options: indexmap::IndexMap::new(),
        }
    }

    pub fn with_option(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.options.insert(key.into(), value.into());
        self
    }
}

/// The acceptable body types for a response.
pub enum ResponseBody {
    Serialisable(Arc<dyn SerialisableObject>),
    Text(String),
    Bytes(Vec<u8>),
}

impl From<&str> for ResponseBody {
    fn from(value: &str) -> Self {
        ResponseBody::Text(value.to_string())
    }
}

impl From<String> for ResponseBody {
    fn from(value: String) -> Self {
        ResponseBody::Text(value)
    }
}

impl From<Vec<u8>> for ResponseBody {
    fn from(value: Vec<u8>) -> Self {
        ResponseBody::Bytes(value)
    }
}

impl From<Arc<dyn SerialisableObject>> for ResponseBody {
    fn from(value: Arc<dyn SerialisableObject>) -> Self {
        ResponseBody::Serialisable(value)
    }
}

/// A Python `ResponseContext`.
#[derive(Debug)]
pub struct ResponseContext {
    status_code: u16,
    mime: Mime,
    body_bytes: Option<Vec<u8>>,
    path: Option<PathBuf>,
    cookies: Vec<ResponseCookie>,
    is_attachment: bool,
    max_age: Option<u64>,
}

impl ResponseContext {
    /// The Python constructor (including the automatic `max_age` rules).
    ///
    /// `body` is `None` when no body is supplied; `path` is `None` when the
    /// response is not a file download.
    pub fn new(
        status_code: u16,
        mime: Mime,
        body: Option<ResponseBody>,
        path: Option<PathBuf>,
        cookies: Vec<ResponseCookie>,
        is_attachment: bool,
        max_age: Option<u64>,
    ) -> HydrusResult<Self> {
        let body_bytes = match body {
            None => None,
            Some(ResponseBody::Serialisable(serialisable)) => {
                Some(serialisable.dump_to_network_bytes()?)
            }
            Some(ResponseBody::Text(text)) => Some(text.into_bytes()),
            Some(ResponseBody::Bytes(bytes)) => Some(bytes),
        };

        let max_age = max_age.or_else(|| {
            if body_bytes.is_some() {
                Some(4)
            } else if path.is_some() {
                Some(86400 * 365)
            } else {
                None
            }
        });

        Ok(Self {
            status_code,
            mime,
            body_bytes,
            path,
            cookies,
            is_attachment,
            max_age,
        })
    }

    /// A compact constructor for text responses, used by tests and simple
    /// resources.
    pub fn with_status_and_body(status_code: u16, body: impl Into<String>) -> Self {
        Self::new(
            status_code,
            Mime::Json,
            Some(ResponseBody::Text(body.into())),
            None,
            Vec::new(),
            false,
            None,
        )
        .expect("a text response body cannot fail to serialise")
    }

    pub fn from_body(
        status_code: u16,
        mime: Mime,
        body: impl Into<ResponseBody>,
    ) -> HydrusResult<Self> {
        Self::new(
            status_code,
            mime,
            Some(body.into()),
            None,
            Vec::new(),
            false,
            None,
        )
    }

    pub fn from_path(status_code: u16, mime: Mime, path: impl Into<PathBuf>) -> HydrusResult<Self> {
        Self::new(
            status_code,
            mime,
            None,
            Some(path.into()),
            Vec::new(),
            false,
            None,
        )
    }

    pub fn get_body_bytes(&self) -> Option<&[u8]> {
        self.body_bytes.as_deref()
    }

    pub fn get_cookies(&self) -> &[ResponseCookie] {
        &self.cookies
    }

    pub fn get_mime(&self) -> Mime {
        self.mime
    }

    pub fn get_path(&self) -> Option<&PathBuf> {
        self.path.as_ref()
    }

    pub fn get_status_code(&self) -> u16 {
        self.status_code
    }

    pub fn get_max_age(&self) -> Option<u64> {
        self.max_age
    }

    pub fn set_max_age(&mut self, max_age: Option<u64>) {
        self.max_age = max_age;
    }

    pub fn has_body(&self) -> bool {
        self.body_bytes.is_some()
    }

    pub fn has_path(&self) -> bool {
        self.path.is_some()
    }

    pub fn is_attachment_download(&self) -> bool {
        self.is_attachment
    }
}

/// One entry from `_parse_range_header`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RangeRequest {
    pub range_start: Option<i64>,
    pub range_end: Option<i64>,
    pub offset: i64,
    pub block_size: i64,
}

/// `_parse_range_header`.
///
/// Python uses unbounded `split('-')`, so a pair with more than one hyphen is
/// a malformed request. The arithmetic (including the suffix-range offset and
/// the start-past-end empty result) follows the Python code exactly.
pub fn parse_range_header(
    range_header: Option<&str>,
    filesize: i64,
) -> HydrusResult<Vec<RangeRequest>> {
    let Some(range_header) = range_header else {
        return Ok(Vec::new());
    };

    let Some((unit, range_pairs_string)) = range_header.split_once('=') else {
        return Err(HydrusException::new(
            ErrorKind::BadRequest,
            "Did not understand range header!",
        ));
    };

    if unit != "bytes" {
        return Err(HydrusException::new(
            ErrorKind::RangeNotSatisfiable,
            "Do not support anything other than bytes in Range header!",
        ));
    }

    let range_pair_strings: Vec<&str> = range_pairs_string.split(',').collect();

    if range_pair_strings
        .iter()
        .any(|range_pair_string| !range_pair_string.contains('-'))
    {
        return Err(HydrusException::new(
            ErrorKind::RangeNotSatisfiable,
            "Did not understand the Range header's range pair(s)!",
        ));
    }

    let mut offset_and_block_size_pairs = Vec::new();

    for range_pair_string in range_pair_strings {
        let mut parts = range_pair_string.trim().split('-');

        let (Some(range_start), Some(range_end)) = (parts.next(), parts.next()) else {
            return Err(HydrusException::new(
                ErrorKind::RangeNotSatisfiable,
                "Did not understand the Range header's range pair(s)!",
            ));
        };

        if parts.next().is_some() {
            return Err(HydrusException::new(
                ErrorKind::RangeNotSatisfiable,
                "Did not understand the Range header's range pair(s)!",
            ));
        }

        let range_start = if range_start.is_empty() {
            None
        } else {
            Some(parse_range_integer(range_start)?)
        };

        let range_end = if range_end.is_empty() {
            None
        } else {
            Some(parse_range_integer(range_end)?)
        };

        if range_start.is_none() && range_end.is_none() {
            return Err(HydrusException::new(
                ErrorKind::RangeNotSatisfiable,
                "Undefined Range header pair given!",
            ));
        }

        if let (Some(range_start), Some(range_end)) = (range_start, range_end)
            && range_start > range_end
        {
            return Err(HydrusException::new(
                ErrorKind::RangeNotSatisfiable,
                "The Range header had an invalid pair!",
            ));
        }

        let (offset, block_size, range_end) = match (range_start, range_end) {
            (None, Some(range_end)) => (filesize - range_end, range_end, Some(range_end)),
            (Some(range_start), None) => (range_start, filesize - range_start, None),
            (Some(range_start), Some(mut range_end)) => {
                if range_start > filesize {
                    // Python clears the whole list and stops processing.
                    offset_and_block_size_pairs.clear();
                    return Ok(offset_and_block_size_pairs);
                }

                if range_end > filesize {
                    range_end = filesize - 1;
                }

                (range_start, (range_end + 1) - range_start, Some(range_end))
            }
            (None, None) => unreachable!("checked above"),
        };

        offset_and_block_size_pairs.push(RangeRequest {
            range_start,
            range_end,
            offset,
            block_size,
        });
    }

    Ok(offset_and_block_size_pairs)
}

fn parse_range_integer(text: &str) -> HydrusResult<i64> {
    text.trim()
        .parse::<i64>()
        .map(i64::abs)
        .map_err(|_| HydrusException::new(ErrorKind::BadRequest, "Could not parse a Range header number!"))
}

/// `_parse_hydrus_network_access_key`.
pub fn parse_hydrus_network_access_key(
    headers: &RequestHeaders,
    key_required: bool,
) -> HydrusResult<Option<Vec<u8>>> {
    let Some(hex_key) = headers.get_first("Hydrus-Key") else {
        if key_required {
            return Err(HydrusException::new(
                ErrorKind::MissingCredentials,
                "No hydrus key header found!",
            ));
        }

        return Ok(None);
    };

    let access_key = hex_decode(hex_key).map_err(|_| {
        HydrusException::new(ErrorKind::BadRequest, "Could not parse the hydrus key!")
    })?;

    Ok(Some(access_key))
}

/// Check a `User-Agent` header and return whether it identifies a Hydrus
/// client.
///
/// `_check_user_agent` sets `request.is_hydrus_user_agent` first and then
/// raises on a version mismatch; [`apply_user_agent_check`] preserves that
/// ordering for request state.
pub fn check_user_agent(user_agent: Option<&str>) -> HydrusResult<bool> {
    let Some(user_agent) = user_agent else {
        return Ok(false);
    };

    for user_agent_token in user_agent.split(' ') {
        if let Some((client, network_version)) = user_agent_token.split_once('/') {
            if client != "hydrus" {
                continue;
            }

            let network_version = network_version
                .split_once(' ')
                .map(|(version, _)| version)
                .unwrap_or(network_version);

            let network_version = network_version.parse::<i64>().map_err(|_| {
                HydrusException::new(
                    ErrorKind::BadRequest,
                    "Could not parse the hydrus network version!",
                )
            })?;

            if network_version == i64::from(NETWORK_VERSION) {
                return Ok(true);
            }

            let message = if network_version < i64::from(NETWORK_VERSION) {
                "Your client is out of date; please download the latest release."
            } else {
                "This server is out of date; please ask its admin to update to the latest release."
            };

            return Err(HydrusException::new(
                ErrorKind::NetworkVersion,
                format!(
                    "Network version mismatch! This server's network version is {NETWORK_VERSION}, whereas your client's is {network_version}! {message}"
                ),
            ));
        }
    }

    Ok(false)
}

/// Set `request.is_hydrus_user_agent`, then apply the version check.
pub fn apply_user_agent_check(request: &mut HydrusRequest) -> HydrusResult<()> {
    request.is_hydrus_user_agent = false;
    request.is_hydrus_user_agent = check_user_agent(request.request_headers.get_first("User-Agent"))?;

    Ok(())
}

/// Map an exception to the HTTP status code used by
/// `_errback_handle_processing_error`.
pub fn error_status_code(kind: ErrorKind) -> u16 {
    if kind.is_a(ErrorKind::BadRequest) {
        400
    } else if kind.is_a(ErrorKind::MissingCredentials) || kind.is_a(ErrorKind::DoesNotSupportCors) {
        401
    } else if kind.is_a(ErrorKind::InsufficientCredentials) {
        403
    } else if kind.is_a(ErrorKind::NotFound)
        || kind.is_a(ErrorKind::DataMissing)
        || kind.is_a(ErrorKind::FileMissing)
    {
        404
    } else if kind.is_a(ErrorKind::NotAcceptable) {
        406
    } else if kind.is_a(ErrorKind::Conflict) {
        409
    } else if kind.is_a(ErrorKind::RangeNotSatisfiable) {
        416
    } else if kind.is_a(ErrorKind::Session) {
        419
    } else if kind.is_a(ErrorKind::UnprocessableEntity) {
        422
    } else if kind.is_a(ErrorKind::NetworkVersion) {
        426
    } else if kind.is_a(ErrorKind::ServerBusy) || kind.is_a(ErrorKind::Shutdown) {
        503
    } else if kind.is_a(ErrorKind::Bandwidth) {
        509
    } else if kind.is_a(ErrorKind::Server) {
        500
    } else {
        500
    }
}

/// Whether the Python code has an explicit `isinstance` branch for this
/// exception. Anything else gets the "encountered an error it could not
/// handle" traceback text.
pub fn is_mapped_error(kind: ErrorKind) -> bool {
    kind.is_a(ErrorKind::BadRequest)
        || kind.is_a(ErrorKind::MissingCredentials)
        || kind.is_a(ErrorKind::DoesNotSupportCors)
        || kind.is_a(ErrorKind::InsufficientCredentials)
        || kind.is_a(ErrorKind::NotFound)
        || kind.is_a(ErrorKind::DataMissing)
        || kind.is_a(ErrorKind::FileMissing)
        || kind.is_a(ErrorKind::NotAcceptable)
        || kind.is_a(ErrorKind::Conflict)
        || kind.is_a(ErrorKind::RangeNotSatisfiable)
        || kind.is_a(ErrorKind::Session)
        || kind.is_a(ErrorKind::UnprocessableEntity)
        || kind.is_a(ErrorKind::NetworkVersion)
        || kind.is_a(ErrorKind::ServerBusy)
        || kind.is_a(ErrorKind::Shutdown)
        || kind.is_a(ErrorKind::Bandwidth)
        || kind.is_a(ErrorKind::Server)
}

/// Build the JSON error `ResponseContext`.
///
/// Python has a real traceback for the unexpected-error branch; Rust's
/// [`HydrusException`] keeps the exception message instead, which is the part
/// the client displays.
pub fn error_response(
    service_name: &str,
    error: &HydrusException,
) -> HydrusResult<ResponseContext> {
    let status_code = error_status_code(error.kind());

    let error_summary = if is_mapped_error(error.kind()) {
        error.to_string()
    } else {
        format!(
            "The \"{service_name}\" encountered an error it could not handle!\n\n{}",
            error
        )
    };

    // Keep the key order of the Python dict; serde_json is built with
    // `preserve_order` in this crate.
    let body_dict: Value = json!({
        "error": error_summary,
        "exception_type": error.kind().name(),
        "status_code": status_code,
        "version": CLIENT_API_VERSION,
        "hydrus_version": SOFTWARE_VERSION,
    });

    let body = python_json_dumps(&body_dict);

    ResponseContext::new(
        status_code,
        Mime::Json,
        Some(ResponseBody::Text(body)),
        None,
        Vec::new(),
        false,
        None,
    )
}

/// Build the headers and response context for an OPTIONS request.
///
/// `allowed_methods` should already reflect which of GET/POST the concrete
/// resource implements.
pub fn build_options_response(
    allowed_methods: &[&str],
    origin_present: bool,
    supports_cors: bool,
) -> HydrusResult<(RequestHeaders, ResponseContext)> {
    let mut headers = RequestHeaders::new();
    let allowed_methods_string = allowed_methods.join(", ");

    if origin_present {
        if supports_cors {
            headers.set("Access-Control-Allow-Headers", "*");
            headers.set("Access-Control-Allow-Origin", "*");
            headers.set("Access-Control-Allow-Methods", allowed_methods_string);
            headers.set("Access-Control-Max-Age", "86400");
        } else {
            return Err(HydrusException::new(
                ErrorKind::DoesNotSupportCors,
                "This service does not support CORS.",
            ));
        }
    } else {
        headers.set("Allow", allowed_methods_string);
    }

    let response_context = ResponseContext::new(
        200,
        Mime::PlainText,
        None,
        None,
        Vec::new(),
        false,
        None,
    )?;

    Ok((headers, response_context))
}

/// The response headers plus the payload the HTTP layer should write.
///
/// A `file_path` with `file_range = None` means "stream the whole file";
/// a range streams `(offset, block_size)` from the same file.
#[derive(Debug)]
pub struct RenderPlan {
    pub status_code: u16,
    pub headers: RequestHeaders,
    pub body: Option<Vec<u8>>,
    pub file_path: Option<PathBuf>,
    pub file_range: Option<(i64, i64)>,
    pub do_finish: bool,
}

/// `_callback_render_response_context` without Twisted's producers.
pub fn build_render_plan(
    request: &HydrusRequest,
    response_context: &ResponseContext,
    service: &dyn HydrusResourceService,
) -> HydrusResult<RenderPlan> {
    let mut headers = RequestHeaders::new();

    if request.request_headers.has_header("Origin") && service.supports_cors() {
        headers.set("Access-Control-Allow-Origin", "*");
    }

    let filesize = if let Some(path) = response_context.get_path() {
        if !path.exists() {
            return Err(HydrusException::new(
                ErrorKind::NotFound,
                "File not found. This was discovered later than expected, so hydev might like to know about this.",
            ));
        }

        Some(
            std::fs::metadata(path)
                .map_err(|_| {
                    HydrusException::new(ErrorKind::NotFound, "Could not read the response file!")
                })?
                .len() as i64,
        )
    } else {
        None
    };

    let offset_and_block_size_pairs = match filesize {
        Some(filesize) => parse_range_header(request.request_headers.get_first("Range"), filesize)?,
        None => Vec::new(),
    };

    let mut status_code = response_context.get_status_code();

    if status_code == 200
        && response_context.has_path()
        && !offset_and_block_size_pairs.is_empty()
    {
        status_code = 206;
    }

    if let Some(max_age) = response_context.get_max_age() {
        let expires = Utc::now() + Duration::seconds(max_age as i64);
        headers.set(
            "Expires",
            expires.format("%a, %d %b %Y %H:%M:%S GMT").to_string(),
        );
        headers.set("Cache-Control", format!("max-age={max_age}"));
    }

    let content_disposition_type = if response_context.is_attachment_download() {
        "attachment"
    } else {
        "inline"
    };

    let mut body = None;
    let mut file_path = None;
    let mut file_range = None;
    let mut do_finish = true;
    let mut content_length = 0;

    if let (Some(path), Some(filesize)) = (response_context.get_path(), filesize) {
        let mime = response_context.get_mime();
        let content_type = mime.mimetype_str();
        let filename = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();

        headers.set(
            "Content-Disposition",
            format!("{content_disposition_type}; filename=\"{filename}\""),
        );

        if offset_and_block_size_pairs.len() <= 1 {
            headers.set("Content-Type", content_type);

            if offset_and_block_size_pairs.is_empty() {
                content_length = filesize;
                headers.set("Content-Length", content_length.to_string());
                file_path = Some(path.clone());
            } else {
                let range_request = offset_and_block_size_pairs[0];
                let header_range_end = range_request.range_end.unwrap_or(filesize - 1);

                content_length = range_request.block_size;

                headers.set("Accept-Ranges", "bytes");
                headers.set(
                    "Content-Range",
                    format!(
                        "bytes {}-{}/{}",
                        range_request.offset, header_range_end, filesize
                    ),
                );
                headers.set("Content-Length", content_length.to_string());

                file_path = Some(path.clone());
                file_range = Some((range_request.offset, range_request.block_size));
            }
        } else {
            return Err(HydrusException::new(
                ErrorKind::RangeNotSatisfiable,
                "Can only support Single Range requests at the moment!",
            ));
        }

        do_finish = false;
    } else if let Some(body_bytes) = response_context.get_body_bytes() {
        let mime = response_context.get_mime();
        let mut content_type = mime.mimetype_str().to_string();

        if mime == Mime::Html {
            content_type.push_str("; charset=UTF-8");
        }

        content_length = body_bytes.len() as i64;

        headers.set("Content-Type", content_type);
        headers.set("Content-Length", content_length.to_string());
        headers.set("Content-Disposition", content_disposition_type);

        body = Some(body_bytes.to_vec());
    } else if status_code != 204 {
        // 204 is No Content, and Twisted does not need a Content-Length for it.
        headers.set("Content-Length", "0");
    }

    service.report_data_used(content_length);
    service.report_request_used();

    Ok(RenderPlan {
        status_code,
        headers,
        body,
        file_path,
        file_range,
        do_finish,
    })
}

/// The small part of Python's `HydrusResource` base class that is not
/// Twisted-specific.
pub struct HydrusResource {
    service: Arc<dyn HydrusResourceService>,
    service_key: Vec<u8>,
    domain: HydrusDomain,
}

impl HydrusResource {
    pub fn new(service: Arc<dyn HydrusResourceService>, domain: HydrusDomain) -> Self {
        let service_key = service.get_service_key();

        Self {
            service,
            service_key,
            domain,
        }
    }

    pub fn get_service(&self) -> &Arc<dyn HydrusResourceService> {
        &self.service
    }

    pub fn get_service_key(&self) -> &[u8] {
        &self.service_key
    }

    pub fn get_domain(&self) -> HydrusDomain {
        self.domain
    }

    /// `_callback_check_service_restrictions` without the deferred plumbing.
    pub fn check_service_restrictions(&self, request: &mut HydrusRequest) -> HydrusResult<()> {
        self.domain.check_valid(&request.client_ip)?;
        apply_user_agent_check(request)?;

        Ok(())
    }

    pub fn parse_hydrus_network_access_key(
        &self,
        request: &HydrusRequest,
        key_required: bool,
    ) -> HydrusResult<Option<Vec<u8>>> {
        parse_hydrus_network_access_key(&request.request_headers, key_required)
    }

    pub fn parse_range_header(
        &self,
        request: &HydrusRequest,
        filesize: i64,
    ) -> HydrusResult<Vec<RangeRequest>> {
        parse_range_header(request.request_headers.get_first("Range"), filesize)
    }

    pub fn build_options_response(
        &self,
        request: &HydrusRequest,
        allowed_methods: &[&str],
    ) -> HydrusResult<(RequestHeaders, ResponseContext)> {
        build_options_response(
            allowed_methods,
            request.request_headers.has_header("Origin"),
            self.service.supports_cors(),
        )
    }

    pub fn build_render_plan(
        &self,
        request: &HydrusRequest,
        response_context: &ResponseContext,
    ) -> HydrusResult<RenderPlan> {
        build_render_plan(request, response_context, self.service.as_ref())
    }

    pub fn report_data_used(&self, num_bytes: i64) {
        self.service.report_data_used(num_bytes);
    }

    pub fn report_request_used(&self) {
        self.service.report_request_used();
    }
}

/// `HydrusResourceRobotsTXT._thread_do_get_job`.
pub const ROBOTS_TXT: &str = "User-agent: *\nDisallow: /";

pub fn robots_txt_response() -> ResponseContext {
    ResponseContext::new(
        200,
        Mime::PlainText,
        Some(ResponseBody::Text(ROBOTS_TXT.to_string())),
        None,
        Vec::new(),
        false,
        None,
    )
    .expect("the robots body cannot fail to serialise")
}

/// `HydrusResourceWelcome._thread_do_get_job`.
pub fn welcome_response(service: &dyn HydrusResourceService) -> ResponseContext {
    ResponseContext::new(
        200,
        Mime::Html,
        Some(ResponseBody::Text(generate_eris(service))),
        None,
        Vec::new(),
        false,
        None,
    )
    .expect("the welcome body cannot fail to serialise")
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::hydrus_serialisable::SerialisableDictionary;

    struct TestService {
        name: String,
        service_type: ServiceType,
        allows_non_local: bool,
        supports_cors: bool,
        normie_eris: bool,
    }

    impl TestService {
        fn new(name: &str, service_type: ServiceType) -> Self {
            Self {
                name: name.to_string(),
                service_type,
                allows_non_local: false,
                supports_cors: false,
                normie_eris: false,
            }
        }
    }

    impl HydrusResourceService for TestService {
        fn get_name(&self) -> String {
            self.name.clone()
        }

        fn get_service_type(&self) -> ServiceType {
            self.service_type
        }

        fn get_service_key(&self) -> Vec<u8> {
            vec![1, 2, 3]
        }

        fn allows_non_local_connections(&self) -> bool {
            self.allows_non_local
        }

        fn supports_cors(&self) -> bool {
            self.supports_cors
        }

        fn use_normie_eris(&self) -> bool {
            self.normie_eris
        }
    }

    fn text_response(body: &str) -> ResponseContext {
        ResponseContext::from_body(200, Mime::Json, body).unwrap()
    }

    #[test]
    fn summary_texts_match_client_api_and_repository_shapes() {
        let mut service = TestService::new("my api", ServiceType::ClientApiService);
        service.allows_non_local = true;

        let texts = get_server_summary_texts(&service);

        assert_eq!(texts[0], "This is <b>my api</b>,");
        assert_eq!(texts[1], "a client api.");
        assert_eq!(texts[2], format!("Software version {SOFTWARE_VERSION}"));
        assert_eq!(texts[3], format!("API version {CLIENT_API_VERSION}"));
        assert_eq!(texts[4], "It responds to requests from any host.");

        let service = TestService::new("repo", ServiceType::FileRepository);
        let texts = get_server_summary_texts(&service);

        assert_eq!(texts[1], "a hydrus file repository.");
        assert_eq!(texts[3], format!("Network version {NETWORK_VERSION}"));
        assert_eq!(texts[4], "It only responds to requests from localhost.");
    }

    #[test]
    fn eris_pages_include_the_summary() {
        let service = TestService::new("my api", ServiceType::ClientApiService);

        let eris = generate_eris(&service);

        assert!(eris.contains("my api"));
        assert!(eris.contains("This is <b>my api</b>,"));
        assert!(eris.contains(&format!("API version {CLIENT_API_VERSION}")));

        let mut service = service;
        service.normie_eris = true;

        assert!(generate_normie_eris(&service).contains("ascgen-image"));
    }

    #[test]
    fn local_domain_only_accepts_loopback_forms() {
        let local = HydrusDomain::new(true);

        for ip in ["127.0.0.1", "::1", "::ffff:127.0.0.1"] {
            assert!(local.check_valid(ip).is_ok(), "{ip}");
        }

        let error = local.check_valid("8.8.8.8").unwrap_err();

        assert_eq!(error.kind(), ErrorKind::InsufficientCredentials);
        assert_eq!(error.to_string(), "Only local access allowed!");
        assert!(local.is_local());

        assert!(HydrusDomain::new(false).check_valid("8.8.8.8").is_ok());
    }

    #[test]
    fn response_context_auto_max_age_rules() {
        let body = ResponseContext::from_body(200, Mime::Json, "hi").unwrap();

        assert_eq!(body.get_max_age(), Some(4));
        assert!(body.has_body());
        assert!(!body.has_path());
        assert_eq!(body.get_body_bytes(), Some(b"hi".as_slice()));

        let path = ResponseContext::from_path(200, Mime::Jpeg, "file.jpg").unwrap();

        assert_eq!(path.get_max_age(), Some(86400 * 365));
        assert!(!path.has_body());
        assert!(path.has_path());

        let empty =
            ResponseContext::new(204, Mime::Json, None, None, Vec::new(), false, None).unwrap();

        assert_eq!(empty.get_max_age(), None);

        let explicit =
            ResponseContext::new(200, Mime::Json, Some("x".into()), None, Vec::new(), false, Some(0))
                .unwrap();

        assert_eq!(explicit.get_max_age(), Some(0));

        let mut mutable = text_response("x");
        mutable.set_max_age(Some(99));
        assert_eq!(mutable.get_max_age(), Some(99));
    }

    #[test]
    fn response_context_serialises_serialisable_bodies() {
        let dictionary = Arc::new(SerialisableDictionary::new());

        let context =
            ResponseContext::from_body(200, Mime::Json, Arc::clone(&dictionary) as Arc<dyn SerialisableObject>)
                .unwrap();

        assert_eq!(
            context.get_body_bytes(),
            Some(dictionary.dump_to_network_bytes().unwrap().as_slice())
        );
    }

    #[test]
    fn response_context_keeps_cookies_and_attachment_flag() {
        let cookie = ResponseCookie::new("session", "abc").with_option("path", "/");

        let context = ResponseContext::new(
            200,
            Mime::Json,
            None,
            None,
            vec![cookie],
            true,
            None,
        )
        .unwrap();

        assert_eq!(context.get_cookies().len(), 1);
        assert_eq!(context.get_cookies()[0].name, "session");
        assert_eq!(context.get_cookies()[0].options["path"], "/");
        assert!(context.is_attachment_download());
    }

    #[test]
    fn range_header_parsing_matches_python() {
        assert!(parse_range_header(None, 1000).unwrap().is_empty());

        assert_eq!(
            parse_range_header(Some("bytes=0-99"), 1000).unwrap(),
            vec![RangeRequest {
                range_start: Some(0),
                range_end: Some(99),
                offset: 0,
                block_size: 100,
            }]
        );

        assert_eq!(
            parse_range_header(Some("bytes=100-"), 1000).unwrap(),
            vec![RangeRequest {
                range_start: Some(100),
                range_end: None,
                offset: 100,
                block_size: 900,
            }]
        );

        assert_eq!(
            parse_range_header(Some("bytes=-50"), 1000).unwrap(),
            vec![RangeRequest {
                range_start: None,
                range_end: Some(50),
                offset: 950,
                block_size: 50,
            }]
        );

        assert_eq!(
            parse_range_header(Some("bytes=0-9, 20-29"), 1000)
                .unwrap()
                .len(),
            2
        );

        assert_eq!(
            parse_range_header(Some("bytes=0-2000"), 1000).unwrap(),
            vec![RangeRequest {
                range_start: Some(0),
                range_end: Some(999),
                offset: 0,
                block_size: 1000,
            }]
        );

        assert!(
            parse_range_header(Some("bytes=2000-3000"), 1000)
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn range_header_rejects_bad_requests() {
        for (header, kind) in [
            ("0-1", ErrorKind::BadRequest),
            ("items=0-1", ErrorKind::RangeNotSatisfiable),
            ("bytes=0", ErrorKind::RangeNotSatisfiable),
            ("bytes=-", ErrorKind::RangeNotSatisfiable),
            ("bytes=10-5", ErrorKind::RangeNotSatisfiable),
            ("bytes=1-2-3", ErrorKind::RangeNotSatisfiable),
        ] {
            let error = parse_range_header(Some(header), 1000).unwrap_err();

            assert_eq!(error.kind(), kind, "{header}");
        }
    }

    #[test]
    fn exception_status_mapping_matches_the_server() {
        for (kind, status) in [
            (ErrorKind::BadRequest, 400),
            (ErrorKind::MissingCredentials, 401),
            (ErrorKind::DoesNotSupportCors, 401),
            (ErrorKind::InsufficientCredentials, 403),
            (ErrorKind::NotFound, 404),
            (ErrorKind::DataMissing, 404),
            (ErrorKind::FileMissing, 404),
            (ErrorKind::NotAcceptable, 406),
            (ErrorKind::Conflict, 409),
            (ErrorKind::RangeNotSatisfiable, 416),
            (ErrorKind::Session, 419),
            (ErrorKind::UnprocessableEntity, 422),
            (ErrorKind::NetworkVersion, 426),
            (ErrorKind::ServerBusy, 503),
            (ErrorKind::Shutdown, 503),
            (ErrorKind::Bandwidth, 509),
            (ErrorKind::Server, 500),
            (ErrorKind::Hydrus, 500),
        ] {
            assert_eq!(error_status_code(kind), status, "{kind}");
        }
    }

    #[test]
    fn error_response_has_the_expected_json_fields() {
        let service = TestService::new("my api", ServiceType::ClientApiService);
        let error = HydrusException::new(ErrorKind::NotFound, "no such thing");

        let context = error_response(&service.get_name(), &error).unwrap();

        assert_eq!(context.get_status_code(), 404);
        assert_eq!(context.get_mime(), Mime::Json);
        assert_eq!(context.get_max_age(), Some(4));

        let body = serde_json::from_slice::<Value>(context.get_body_bytes().unwrap()).unwrap();

        assert_eq!(body["error"], "no such thing");
        assert_eq!(body["exception_type"], "NotFoundException");
        assert_eq!(body["status_code"], 404);
        assert_eq!(body["version"], CLIENT_API_VERSION);
        assert_eq!(body["hydrus_version"], SOFTWARE_VERSION);
    }

    #[test]
    fn unexpected_errors_get_the_service_wrapped_summary() {
        let error = HydrusException::new(ErrorKind::Unknown, "boom");

        let context = error_response("my api", &error).unwrap();
        let body = serde_json::from_slice::<Value>(context.get_body_bytes().unwrap()).unwrap();

        assert_eq!(context.get_status_code(), 500);
        assert_eq!(
            body["error"],
            "The \"my api\" encountered an error it could not handle!\n\nboom"
        );
        assert_eq!(body["exception_type"], "UnknownException");
    }

    #[test]
    fn user_agent_check_recognises_hydrus_and_versions() {
        assert_eq!(check_user_agent(None).unwrap(), false);
        assert_eq!(check_user_agent(Some("Mozilla/5.0")).unwrap(), false);
        assert_eq!(check_user_agent(Some("hydrus/20")).unwrap(), true);
        assert_eq!(check_user_agent(Some("Mozilla/5.0 hydrus/20")).unwrap(), true);

        let error = check_user_agent(Some("hydrus/19")).unwrap_err();

        assert_eq!(error.kind(), ErrorKind::NetworkVersion);
        assert_eq!(
            error.to_string(),
            "Network version mismatch! This server's network version is 20, whereas your client's is 19! Your client is out of date; please download the latest release."
        );

        let error = check_user_agent(Some("hydrus/21")).unwrap_err();

        assert_eq!(
            error.to_string(),
            "Network version mismatch! This server's network version is 20, whereas your client's is 21! This server is out of date; please ask its admin to update to the latest release."
        );

        assert_eq!(check_user_agent(Some("hydrus/abc")).unwrap_err().kind(), ErrorKind::BadRequest);
    }

    #[test]
    fn apply_user_agent_check_updates_request_state() {
        let mut request = HydrusRequest::new("GET", "/");
        request.request_headers.set("User-Agent", "hydrus/20");

        apply_user_agent_check(&mut request).unwrap();

        assert!(request.is_hydrus_user_agent);

        let mut request = HydrusRequest::new("GET", "/");
        request.request_headers.set("User-Agent", "Mozilla/5.0");

        apply_user_agent_check(&mut request).unwrap();

        assert!(!request.is_hydrus_user_agent);
    }

    #[test]
    fn hydrus_key_parsing_handles_required_and_optional() {
        let mut headers = RequestHeaders::new();

        assert_eq!(
            parse_hydrus_network_access_key(&headers, true)
                .unwrap_err()
                .kind(),
            ErrorKind::MissingCredentials
        );
        assert_eq!(
            parse_hydrus_network_access_key(&headers, false).unwrap(),
            None
        );

        headers.set("Hydrus-Key", "00ff10");

        assert_eq!(
            parse_hydrus_network_access_key(&headers, true).unwrap(),
            Some(vec![0x00, 0xFF, 0x10])
        );

        headers.set("Hydrus-Key", "xyz");

        assert_eq!(
            parse_hydrus_network_access_key(&headers, true)
                .unwrap_err()
                .kind(),
            ErrorKind::BadRequest
        );
    }

    #[test]
    fn options_headers_honour_cors_support() {
        let (headers, context) = build_options_response(&["GET", "POST"], false, false).unwrap();

        assert_eq!(headers.get_first("Allow"), Some("GET, POST"));
        assert_eq!(context.get_status_code(), 200);
        assert_eq!(context.get_mime(), Mime::PlainText);
        assert!(!context.has_body());

        let (headers, _) = build_options_response(&["GET"], true, true).unwrap();

        assert_eq!(headers.get_first("Access-Control-Allow-Origin"), Some("*"));
        assert_eq!(headers.get_first("Access-Control-Allow-Methods"), Some("GET"));
        assert_eq!(headers.get_first("Access-Control-Max-Age"), Some("86400"));

        let error = build_options_response(&["GET"], true, false).unwrap_err();

        assert_eq!(error.kind(), ErrorKind::DoesNotSupportCors);
    }

    #[test]
    fn render_plan_writes_text_headers() {
        let service = Arc::new(TestService::new("my api", ServiceType::ClientApiService));
        let resource = HydrusResource::new(service, HydrusDomain::new(true));
        let request = HydrusRequest::new("GET", "/");
        let context = ResponseContext::from_body(200, Mime::Html, "<html></html>").unwrap();

        let plan = resource.build_render_plan(&request, &context).unwrap();

        assert_eq!(plan.status_code, 200);
        assert_eq!(
            plan.headers.get_first("Content-Type"),
            Some("text/html; charset=UTF-8")
        );
        assert_eq!(plan.headers.get_first("Content-Length"), Some("13"));
        assert_eq!(plan.headers.get_first("Content-Disposition"), Some("inline"));
        assert_eq!(plan.headers.get_first("Cache-Control"), Some("max-age=4"));
        assert!(plan.headers.has_header("Expires"));
        assert_eq!(plan.body, Some(b"<html></html>".to_vec()));
        assert!(plan.file_path.is_none());
        assert!(plan.do_finish);
    }

    #[test]
    fn render_plan_handles_204_and_attachment() {
        let service = Arc::new(TestService::new("my api", ServiceType::ClientApiService));
        let resource = HydrusResource::new(service, HydrusDomain::new(true));
        let request = HydrusRequest::new("GET", "/");
        let context =
            ResponseContext::new(204, Mime::Json, None, None, Vec::new(), false, None).unwrap();

        let plan = resource.build_render_plan(&request, &context).unwrap();

        assert_eq!(plan.status_code, 204);
        assert!(!plan.headers.has_header("Content-Length"));
        assert!(plan.body.is_none());

        let mut attachment =
            ResponseContext::from_body(200, Mime::PlainText, "body").unwrap();
        attachment.set_max_age(None);
        let plan = resource.build_render_plan(&request, &attachment).unwrap();

        assert_eq!(
            plan.headers.get_first("Content-Disposition"),
            Some("inline")
        );

        let attachment = ResponseContext::new(
            200,
            Mime::PlainText,
            Some("body".into()),
            None,
            Vec::new(),
            true,
            None,
        )
        .unwrap();
        let plan = resource.build_render_plan(&request, &attachment).unwrap();

        assert_eq!(
            plan.headers.get_first("Content-Disposition"),
            Some("attachment")
        );
    }

    #[test]
    fn render_plan_streams_files_and_ranges() {
        let service = Arc::new(TestService::new("my api", ServiceType::ClientApiService));
        let resource = HydrusResource::new(service, HydrusDomain::new(true));

        let path = std::env::temp_dir().join(format!(
            "hydrus_core_server_range_{}_{}.bin",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));

        std::fs::write(&path, b"0123456789").unwrap();

        let context = ResponseContext::from_path(200, Mime::OctetStream, &path).unwrap();

        let request = HydrusRequest::new("GET", "/file");
        let plan = resource.build_render_plan(&request, &context).unwrap();

        assert_eq!(plan.status_code, 200);
        assert_eq!(plan.headers.get_first("Content-Length"), Some("10"));
        let content_disposition = plan
            .headers
            .get_first("Content-Disposition")
            .expect("a file response has a disposition");

        assert!(content_disposition.starts_with("inline; filename=\"hydrus_core_server_range_"));
        assert!(content_disposition.ends_with(".bin\""));
        assert_eq!(plan.file_path.as_deref(), Some(path.as_path()));
        assert_eq!(plan.file_range, None);
        assert!(!plan.do_finish);

        let mut request = HydrusRequest::new("GET", "/file");
        request.request_headers.set("Range", "bytes=2-5");

        let plan = resource.build_render_plan(&request, &context).unwrap();

        assert_eq!(plan.status_code, 206);
        assert_eq!(plan.headers.get_first("Content-Range"), Some("bytes 2-5/10"));
        assert_eq!(plan.headers.get_first("Content-Length"), Some("4"));
        assert_eq!(plan.headers.get_first("Accept-Ranges"), Some("bytes"));
        assert_eq!(plan.file_range, Some((2, 4)));

        let mut request = HydrusRequest::new("GET", "/file");
        request.request_headers.set("Range", "bytes=0-1,3-4");

        let error = resource.build_render_plan(&request, &context).unwrap_err();

        assert_eq!(error.kind(), ErrorKind::RangeNotSatisfiable);

        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn render_plan_writes_body_and_length() {
        let service = Arc::new(TestService::new("my api", ServiceType::ClientApiService));
        let resource = HydrusResource::new(service, HydrusDomain::new(true));
        let request = HydrusRequest::new("GET", "/");
        let context = text_response("12345");

        let plan = resource.build_render_plan(&request, &context).unwrap();

        assert_eq!(plan.body.as_deref(), Some(b"12345".as_slice()));
    }

    #[test]
    fn robots_and_welcome_resources_have_expected_bodies() {
        let context = robots_txt_response();

        assert_eq!(context.get_mime(), Mime::PlainText);
        assert_eq!(context.get_body_bytes(), Some(ROBOTS_TXT.as_bytes()));

        let service = TestService::new("my api", ServiceType::ClientApiService);
        let context = welcome_response(&service);

        assert_eq!(context.get_mime(), Mime::Html);
        assert!(
            std::str::from_utf8(context.get_body_bytes().unwrap())
                .unwrap()
                .contains("my api")
        );
    }
}
