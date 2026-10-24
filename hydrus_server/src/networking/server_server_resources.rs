//! Rust port of the first resource layer from
//! `hydrus/server/networking/ServerServerResources.py`.
//!
//! Twisted supplies the request transport and deferreds. This module ports the
//! resource policy and the simple network endpoints (busy, access keys,
//! sessions, shutdown, account/options/account-types/service-info). Repository
//! content resources are intentionally reported as not implemented until their
//! DB layer is ported.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use indexmap::IndexMap;

use hydrus_core::hydrus_constants::{ContentType, Mime, PermissionAction, REPOSITORIES};
use hydrus_core::hydrus_data::hex_to_none_or_bytes;
use hydrus_core::hydrus_exceptions::{ErrorKind, HydrusException, HydrusResult};
use hydrus_core::hydrus_sessions::{Account as SessionAccountTrait, HydrusSessionManagerServer};
use hydrus_core::hydrus_time::get_now;
use hydrus_core::networking::hydrus_network::ServerService;
use hydrus_core::networking::hydrus_network_variable_handling::{
    ArgValue, ParsedRequestArguments, dump_hydrus_args_to_network_bytes,
};
use hydrus_core::networking::hydrus_server_request::HydrusRequest;
use hydrus_core::networking::hydrus_server_resources::{
    ResponseBody, ResponseContext, ResponseCookie, apply_user_agent_check,
    parse_hydrus_network_access_key,
};

use crate::networking::server_server::{ServerResourceKind, ServerRoute};
use crate::server_db::ServerDB;

/// Everything a resource needs from the running server.
pub struct ServerResourceRuntime {
    pub db: Arc<ServerDB>,
    pub session_manager: Arc<HydrusSessionManagerServer>,
    pub service: Arc<ServerService>,
    pub busy: Arc<AtomicBool>,
    pub shutdown_requested: Arc<AtomicBool>,
}

/// Dispatch one route to its resource implementation.
pub fn render_resource(
    kind: ServerResourceKind,
    route: ServerRoute,
    runtime: &ServerResourceRuntime,
    request: &mut HydrusRequest,
) -> HydrusResult<ResponseContext> {
    let allowed_methods = allowed_methods(kind);

    if request.method == "OPTIONS" {
        return Ok(ResponseContext::with_status_and_body(200, ""));
    }

    if !allowed_methods.contains(&request.method.as_str()) {
        return Ok(ResponseContext::with_status_and_body(
            405,
            "Method Not Allowed",
        ));
    }

    route.domain.check_valid(&request.client_ip)?;
    apply_user_agent_check(request)?;

    if kind != ServerResourceKind::BusyCheck && runtime.busy.load(Ordering::Relaxed) {
        return Err(HydrusException::new(
            ErrorKind::ServerBusy,
            "This server is busy, please try again later.",
        ));
    }

    if is_restricted(kind) {
        establish_account(runtime, request)?;
        check_account_rules(kind, request)?;
    }

    let response = match kind {
        ServerResourceKind::BusyCheck => busy_response(runtime),
        ServerResourceKind::AccessKey => access_key_response(runtime, request)?,
        ServerResourceKind::AccessKeyVerification => {
            access_key_verification_response(runtime, request)?
        }
        ServerResourceKind::AutoCreateAccountTypes => {
            auto_create_account_types_response(runtime)?
        }
        ServerResourceKind::RestrictedAutoCreateRegistrationKey => {
            auto_create_registration_key_response(runtime, request)?
        }
        ServerResourceKind::SessionKey => session_key_response(runtime, request)?,
        ServerResourceKind::Shutdown => shutdown_response(runtime),
        ServerResourceKind::RestrictedAccount => account_response(request)?,
        ServerResourceKind::RestrictedOptions => options_response(runtime)?,
        ServerResourceKind::RestrictedAllAccounts => all_accounts_response(runtime, request)?,
        ServerResourceKind::RestrictedAccountTypes if request.method == "POST" => {
            ResponseContext::with_status_and_body(
                501,
                "This server resource is not ported yet.",
            )
        }
        ServerResourceKind::RestrictedAccountTypes => account_types_response(runtime)?,
        ServerResourceKind::RestrictedRegistrationKeys => {
            registration_keys_response(runtime, request)?
        }
        ServerResourceKind::RestrictedServiceInfo => service_info_response(runtime)?,
        ServerResourceKind::RestrictedServices => services_response(runtime)?,
        _ => ResponseContext::with_status_and_body(
            501,
            "This server resource is not ported yet.",
        ),
    };

    Ok(response)
}

fn allowed_methods(kind: ServerResourceKind) -> &'static [&'static str] {
    match kind {
        ServerResourceKind::Shutdown => &["POST"],
        ServerResourceKind::RestrictedAccountTypes => &["GET", "POST"],
        ServerResourceKind::RestrictedOptionsModifyNullificationPeriod
        | ServerResourceKind::RestrictedOptionsModifyUpdatePeriod
        | ServerResourceKind::RestrictedAccountModifyAccountType
        | ServerResourceKind::RestrictedAccountModifyBan
        | ServerResourceKind::RestrictedAccountModifyDeleteAllContent
        | ServerResourceKind::RestrictedAccountModifyExpires
        | ServerResourceKind::RestrictedAccountModifySetMessage
        | ServerResourceKind::RestrictedAccountModifyUnban
        | ServerResourceKind::RestrictedBackup
        | ServerResourceKind::RestrictedLockOn
        | ServerResourceKind::RestrictedLockOff
        | ServerResourceKind::RestrictedMaintenanceRegenServiceInfo
        | ServerResourceKind::RestrictedRestartServices
        | ServerResourceKind::RestrictedVacuum => &["POST"],
        ServerResourceKind::RestrictedRepositoryFile => &["GET", "POST"],
        ServerResourceKind::RestrictedTagFilter => &["GET", "POST"],
        _ => &["GET"],
    }
}

fn is_restricted(kind: ServerResourceKind) -> bool {
    matches!(
        kind,
        ServerResourceKind::RestrictedAccount
            | ServerResourceKind::RestrictedOptions
            | ServerResourceKind::RestrictedAllAccounts
            | ServerResourceKind::RestrictedAccountOtherAccount
            | ServerResourceKind::RestrictedAccountModifyAccountType
            | ServerResourceKind::RestrictedAccountModifyBan
            | ServerResourceKind::RestrictedAccountModifyDeleteAllContent
            | ServerResourceKind::RestrictedAccountModifyExpires
            | ServerResourceKind::RestrictedAccountModifySetMessage
            | ServerResourceKind::RestrictedAccountModifyUnban
            | ServerResourceKind::RestrictedAccountInfo
            | ServerResourceKind::RestrictedAccountKeyFromContent
            | ServerResourceKind::RestrictedAccountTypes
            | ServerResourceKind::RestrictedRegistrationKeys
            | ServerResourceKind::RestrictedServiceInfo
            | ServerResourceKind::RestrictedMaintenanceRegenServiceInfo
            | ServerResourceKind::RestrictedBackup
            | ServerResourceKind::RestrictedLockOn
            | ServerResourceKind::RestrictedLockOff
            | ServerResourceKind::RestrictedRestartServices
            | ServerResourceKind::RestrictedServices
            | ServerResourceKind::RestrictedVacuum
            | ServerResourceKind::RestrictedNumPetitions
            | ServerResourceKind::RestrictedPetition
            | ServerResourceKind::RestrictedPetitionsSummary
            | ServerResourceKind::RestrictedUpdate
            | ServerResourceKind::RestrictedMetadataUpdate
            | ServerResourceKind::RestrictedRepositoryFile
            | ServerResourceKind::RestrictedIp
            | ServerResourceKind::RestrictedRepositoryThumbnail
            | ServerResourceKind::RestrictedTagFilter
    )
}

fn busy_response(runtime: &ServerResourceRuntime) -> ResponseContext {
    ResponseContext::with_status_and_body(
        200,
        if runtime.busy.load(Ordering::Relaxed) {
            "1"
        } else {
            "0"
        },
    )
}

fn access_key_response(
    runtime: &ServerResourceRuntime,
    request: &HydrusRequest,
) -> HydrusResult<ResponseContext> {
    let registration_key = request.parsed_request_args.get_bytes("registration_key")?;
    let access_key = runtime
        .db
        .get_access_key(runtime.service.get_service_key(), &registration_key)?;

    network_response(("access_key", ArgValue::from(access_key)))
}

fn access_key_verification_response(
    runtime: &ServerResourceRuntime,
    request: &HydrusRequest,
) -> HydrusResult<ResponseContext> {
    let access_key = parse_hydrus_network_access_key(&request.request_headers, true)?
        .expect("required access key was returned");

    let verified = runtime
        .db
        .verify_access_key(runtime.service.get_service_key(), &access_key)?;

    network_response(("verified", ArgValue::from(verified)))
}

fn auto_create_account_types_response(
    runtime: &ServerResourceRuntime,
) -> HydrusResult<ResponseContext> {
    let account_types = runtime
        .db
        .get_account_types(runtime.service.get_service_key())?;

    let account_types: Vec<ArgValue> = account_types
        .into_iter()
        .filter(|account_type| account_type.supports_auto_create_account())
        .map(|account_type| ArgValue::object(account_type))
        .collect();

    network_response(("account_types", ArgValue::List(account_types)))
}

fn auto_create_registration_key_response(
    runtime: &ServerResourceRuntime,
    request: &HydrusRequest,
) -> HydrusResult<ResponseContext> {
    let account_type_key = request.parsed_request_args.get_bytes("account_type_key")?;
    let service_key = runtime.service.get_service_key();
    let service_id = runtime.db.get_service_id(service_key)?;
    let account_type_id = runtime
        .db
        .get_account_type_id(service_id, &account_type_key)?;
    let account_type = runtime.db.get_account_type(service_id, account_type_id)?;

    if !account_type.supports_auto_create_account() {
        return Err(HydrusException::new(
            ErrorKind::BadRequest,
            format!(
                "\"{}\" accounts do not support auto-creation!",
                account_type.get_title()
            ),
        ));
    }

    if !account_type.can_auto_create_account_now() {
        return Err(HydrusException::new(
            ErrorKind::BadRequest,
            format!(
                "Please wait a bit--there are no new \"{}\" accounts available for now!",
                account_type.get_title()
            ),
        ));
    }

    let keys = runtime.db.generate_registration_keys(
        service_id,
        1,
        account_type_id,
        None,
        None,
    )?;
    let key = keys.into_iter().next().ok_or_else(|| {
        HydrusException::new(ErrorKind::Unknown, "No registration key was generated.")
    })?;

    network_response(("registration_key", ArgValue::from(key)))
}

fn session_key_response(
    runtime: &ServerResourceRuntime,
    request: &HydrusRequest,
) -> HydrusResult<ResponseContext> {
    let access_key = parse_hydrus_network_access_key(&request.request_headers, true)?
        .expect("required access key was returned");

    let (session_key, expires) = runtime
        .session_manager
        .add_session(runtime.service.get_service_key(), &access_key)?;

    let max_age = expires - get_now();
    let cookie = ResponseCookie::new("session_key", hex_encode(&session_key))
        .with_option("max_age", max_age.to_string())
        .with_option("path", "/");

    ResponseContext::new(
        200,
        Mime::Json,
        None,
        None,
        vec![cookie],
        false,
        None,
    )
}

fn shutdown_response(runtime: &ServerResourceRuntime) -> ResponseContext {
    runtime.shutdown_requested.store(true, Ordering::Relaxed);

    ResponseContext::with_status_and_body(200, "")
}

fn account_response(request: &HydrusRequest) -> HydrusResult<ResponseContext> {
    let account = request.hydrus_account.as_ref().ok_or_else(|| {
        HydrusException::new(ErrorKind::MissingCredentials, "No account on request!")
    })?;

    network_response(("account", ArgValue::other_object(Arc::clone(account))))
}

fn options_response(runtime: &ServerResourceRuntime) -> HydrusResult<ResponseContext> {
    let service_options = if REPOSITORIES.contains(&runtime.service.get_service_type()) {
        let mut options = IndexMap::new();
        options.insert(
            "update_period".to_string(),
            ArgValue::from(runtime.service.get_update_period()),
        );
        options.insert(
            "nullification_period".to_string(),
            ArgValue::from(runtime.service.get_nullification_period()),
        );
        ArgValue::Dict(options)
    } else {
        ArgValue::Dict(IndexMap::new())
    };

    network_response(("service_options", service_options))
}

fn all_accounts_response(
    runtime: &ServerResourceRuntime,
    request: &HydrusRequest,
) -> HydrusResult<ResponseContext> {
    check_permission(request, ContentType::Accounts, PermissionAction::Moderate)?;

    let accounts = runtime
        .db
        .get_all_accounts(runtime.service.get_service_key())?;

    let accounts: Vec<ArgValue> = accounts
        .into_iter()
        .map(|account| ArgValue::other_object(Arc::new(account)))
        .collect();

    network_response(("accounts", ArgValue::List(accounts)))
}

fn account_types_response(runtime: &ServerResourceRuntime) -> HydrusResult<ResponseContext> {
    let account_types = runtime
        .db
        .get_account_types(runtime.service.get_service_key())?;

    let account_types: Vec<ArgValue> = account_types
        .into_iter()
        .map(ArgValue::object)
        .collect();

    network_response(("account_types", ArgValue::List(account_types)))
}

fn registration_keys_response(
    runtime: &ServerResourceRuntime,
    request: &HydrusRequest,
) -> HydrusResult<ResponseContext> {
    check_permission(request, ContentType::Accounts, PermissionAction::Create)?;

    let num = request.parsed_request_args.get_i64("num")?;
    if num <= 0 {
        return Err(HydrusException::new(
            ErrorKind::BadRequest,
            "The requested number of registration keys must be positive.",
        ));
    }

    let account_type_key = request.parsed_request_args.get_bytes("account_type_key")?;
    let expires = request.parsed_request_args.get_i64_or_none("expires")?;
    let service_id = runtime
        .db
        .get_service_id(runtime.service.get_service_key())?;
    let account_type_id = runtime
        .db
        .get_account_type_id(service_id, &account_type_key)?;

    let keys = runtime.db.generate_registration_keys(
        service_id,
        num as usize,
        account_type_id,
        expires,
        None,
    )?;

    let keys: Vec<ArgValue> = keys.into_iter().map(ArgValue::from).collect();

    network_response(("registration_keys", ArgValue::List(keys)))
}

fn service_info_response(runtime: &ServerResourceRuntime) -> HydrusResult<ResponseContext> {
    let service_info = runtime
        .db
        .get_service_info(runtime.service.get_service_key())?;

    let info: Vec<ArgValue> = service_info
        .into_iter()
        .map(|(info_type, value)| {
            ArgValue::List(vec![ArgValue::from(info_type), ArgValue::from(value)])
        })
        .collect();

    network_response(("service_info", ArgValue::List(info)))
}

fn services_response(runtime: &ServerResourceRuntime) -> HydrusResult<ResponseContext> {
    let services = runtime.db.get_services(None)?;
    let services: Vec<ArgValue> = services
        .into_iter()
        .map(|service| ArgValue::other_object(Arc::new(service)))
        .collect();

    network_response(("services", ArgValue::List(services)))
}

fn network_response(
    (key, value): (&str, ArgValue),
) -> HydrusResult<ResponseContext> {
    let mut args = ParsedRequestArguments::new();

    args.insert(key, value);

    let body = dump_hydrus_args_to_network_bytes(&args)?;

    ResponseContext::new(
        200,
        Mime::Json,
        Some(ResponseBody::Bytes(body)),
        None,
        Vec::new(),
        false,
        None,
    )
}

fn establish_account(
    runtime: &ServerResourceRuntime,
    request: &mut HydrusRequest,
) -> HydrusResult<()> {
    let service_key = runtime.service.get_service_key();

    let session_key = session_key_from_headers(request)?;

    let session_account: Arc<dyn SessionAccountTrait> = match session_key {
        Some(session_key) => runtime
            .session_manager
            .get_account(service_key, &session_key)?,
        None => {
            let access_key = parse_hydrus_network_access_key(&request.request_headers, false)?;

            let Some(access_key) = access_key else {
                return Err(HydrusException::new(
                    ErrorKind::MissingCredentials,
                    "No credentials found in request!",
                ));
            };

            runtime
                .session_manager
                .get_account_from_access_key(service_key, &access_key)?
        }
    };

    let account_key = session_account.get_account_key();
    let account = runtime
        .db
        .get_account_from_account_key(service_key, &account_key)?;

    request.hydrus_account = Some(Arc::new(account));

    Ok(())
}

fn session_key_from_headers(request: &HydrusRequest) -> HydrusResult<Option<Vec<u8>>> {
    let Some(cookie_header) = request.request_headers.get_first("Cookie") else {
        return Ok(None);
    };

    for cookie in cookie_header.split(';') {
        let Some((name, value)) = cookie.trim().split_once('=') else {
            continue;
        };

        if name.trim() != "session_key" {
            continue;
        }

        return hex_to_none_or_bytes(Some(value.trim()))
            .map(Some)
            .ok_or_else(|| {
                HydrusException::new(
                    ErrorKind::BadRequest,
                    "Problem parsing cookies for Session Cookie!",
                )
            });
    }

    Ok(None)
}

fn check_account_rules(
    kind: ServerResourceKind,
    request: &HydrusRequest,
) -> HydrusResult<()> {
    if !matches!(
        kind,
        ServerResourceKind::RestrictedAccount | ServerResourceKind::RestrictedOptions
    ) {
        let account = request.hydrus_account.as_ref().ok_or_else(|| {
            HydrusException::new(ErrorKind::MissingCredentials, "No account on request!")
        })?;

        account.check_functional()?;
    }

    match kind {
        ServerResourceKind::RestrictedAllAccounts => {
            check_permission(request, ContentType::Accounts, PermissionAction::Moderate)?;
        }
        ServerResourceKind::RestrictedAccountTypes => {
            check_permission(request, ContentType::Accounts, PermissionAction::Create)?;
        }
        ServerResourceKind::RestrictedRegistrationKeys => {
            check_permission(request, ContentType::Accounts, PermissionAction::Create)?;
        }
        ServerResourceKind::RestrictedServices => {
            check_permission(request, ContentType::Services, PermissionAction::Moderate)?;
        }
        _ => {}
    }

    Ok(())
}

fn check_permission(
    request: &HydrusRequest,
    content_type: ContentType,
    action: PermissionAction,
) -> HydrusResult<()> {
    let account = request.hydrus_account.as_ref().ok_or_else(|| {
        HydrusException::new(ErrorKind::MissingCredentials, "No account on request!")
    })?;

    account.check_permission(content_type, action)
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";

    let mut text = String::with_capacity(bytes.len() * 2);

    for byte in bytes {
        text.push(char::from(HEX[usize::from(byte >> 4)]));
        text.push(char::from(HEX[usize::from(byte & 0x0F)]));
    }

    text
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::path::PathBuf;
    use std::sync::atomic::AtomicU64;

    use hydrus_core::hydrus_db::NullDBController;
    use hydrus_core::hydrus_sessions::HydrusSessionManagerServer;
    use hydrus_core::networking::hydrus_network_variable_handling::parse_network_bytes_to_parsed_hydrus_args;
    use hydrus_core::networking::hydrus_server::{LOCAL_DOMAIN, REMOTE_DOMAIN};

    use crate::server_db::ServerSessionController;

    static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

    struct TestDir {
        path: PathBuf,
    }

    impl TestDir {
        fn new() -> Self {
            let suffix = format!(
                "{}_{}",
                std::process::id(),
                TEST_COUNTER.fetch_add(1, Ordering::Relaxed)
            );

            let path = std::env::temp_dir()
                .join("hydrus-server-resource-tests")
                .join(suffix);

            std::fs::create_dir_all(&path).unwrap();

            Self { path }
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }

    struct Fixture {
        _dir: TestDir,
        db: Arc<ServerDB>,
        runtime: Arc<ServerResourceRuntime>,
        admin_key: Vec<u8>,
    }

    impl Fixture {
        fn new() -> Self {
            hydrus_core::initialise();

            let dir = TestDir::new();
            let db = ServerDB::new(Arc::new(NullDBController), &dir.path, "server").unwrap();
            let session_manager = HydrusSessionManagerServer::new(Arc::new(
                ServerSessionController::new(Arc::clone(&db)),
            ))
            .unwrap();

            let service = db.get_services(None).unwrap().remove(0);
            let admin_key = service.get_service_key().to_vec();

            let runtime = Arc::new(ServerResourceRuntime {
                db: Arc::clone(&db),
                session_manager,
                service: Arc::new(service),
                busy: Arc::new(AtomicBool::new(false)),
                shutdown_requested: Arc::new(AtomicBool::new(false)),
            });

            Self {
                _dir: dir,
                db,
                runtime,
                admin_key,
            }
        }

        fn render(
            &self,
            kind: ServerResourceKind,
            request: &mut HydrusRequest,
        ) -> HydrusResult<ResponseContext> {
            render_resource(
                kind,
                ServerRoute {
                    kind,
                    domain: REMOTE_DOMAIN,
                },
                &self.runtime,
                request,
            )
        }

        fn request(&self, method: &str) -> HydrusRequest {
            HydrusRequest::new(method, "/").with_client_ip("127.0.0.1")
        }
    }

    fn body_args(response: &ResponseContext) -> ParsedRequestArguments {
        parse_network_bytes_to_parsed_hydrus_args(
            response.get_body_bytes().expect("the response has a body"),
        )
        .expect("the response is a hydrus network dictionary")
    }

    #[test]
    fn busy_check_reports_the_runtime_flag() {
        let fixture = Fixture::new();
        let mut request = fixture.request("GET");

        let response = fixture
            .render(ServerResourceKind::BusyCheck, &mut request)
            .unwrap();

        assert_eq!(response.get_status_code(), 200);
        assert_eq!(response.get_body_bytes(), Some(&b"0"[..]));
    }

    #[test]
    fn access_key_endpoint_issues_a_key_for_the_init_token() {
        let fixture = Fixture::new();
        let mut request = fixture.request("GET");

        request
            .parsed_request_args
            .insert("registration_key", ArgValue::from(b"init".as_slice()));

        let response = fixture
            .render(ServerResourceKind::AccessKey, &mut request)
            .unwrap();

        let args = body_args(&response);
        let access_key = args.get_bytes("access_key").unwrap();

        assert_eq!(access_key.len(), 32);

        let mut verification = fixture.request("GET");
        verification
            .request_headers
            .set("Hydrus-Key", hex_encode(&access_key));

        let response = fixture
            .render(
                ServerResourceKind::AccessKeyVerification,
                &mut verification,
            )
            .unwrap();

        assert!(body_args(&response).get_bool("verified").unwrap());
    }

    #[test]
    fn session_key_endpoint_sets_a_cookie() {
        let fixture = Fixture::new();
        let access_key = fixture.db.get_access_key(&fixture.admin_key, b"init").unwrap();

        let mut request = fixture.request("GET");
        request
            .request_headers
            .set("Hydrus-Key", hex_encode(&access_key));

        let response = fixture
            .render(ServerResourceKind::SessionKey, &mut request)
            .unwrap();

        assert_eq!(response.get_status_code(), 200);
        assert_eq!(response.get_cookies().len(), 1);
        assert_eq!(response.get_cookies()[0].name, "session_key");
    }

    #[test]
    fn account_endpoint_returns_the_established_account() {
        let fixture = Fixture::new();
        let access_key = fixture.db.get_access_key(&fixture.admin_key, b"init").unwrap();

        let mut request = fixture.request("GET");
        request
            .request_headers
            .set("Hydrus-Key", hex_encode(&access_key));

        let response = fixture
            .render(ServerResourceKind::RestrictedAccount, &mut request)
            .unwrap();

        assert_eq!(response.get_status_code(), 200);
        assert!(response.get_body_bytes().is_some());
        assert!(request.hydrus_account.is_some());
    }

    #[test]
    fn account_types_endpoint_returns_both_default_types() {
        let fixture = Fixture::new();
        let access_key = fixture.db.get_access_key(&fixture.admin_key, b"init").unwrap();

        let mut request = fixture.request("GET");
        request
            .request_headers
            .set("Hydrus-Key", hex_encode(&access_key));

        let response = fixture
            .render(ServerResourceKind::RestrictedAccountTypes, &mut request)
            .unwrap();

        let args = body_args(&response);

        assert_eq!(args.get_list("account_types").unwrap().len(), 2);
    }

    #[test]
    fn shutdown_endpoint_sets_the_shutdown_flag() {
        let fixture = Fixture::new();
        let mut request = fixture.request("POST");

        let response = render_resource(
            ServerResourceKind::Shutdown,
            ServerRoute {
                kind: ServerResourceKind::Shutdown,
                domain: LOCAL_DOMAIN,
            },
            &fixture.runtime,
            &mut request,
        )
        .unwrap();

        assert_eq!(response.get_status_code(), 200);
        assert!(fixture.runtime.shutdown_requested.load(Ordering::Relaxed));
    }

    #[test]
    fn repository_resources_are_explicitly_not_ported_yet() {
        let fixture = Fixture::new();
        let access_key = fixture.db.get_access_key(&fixture.admin_key, b"init").unwrap();

        let mut request = fixture.request("GET");
        request
            .request_headers
            .set("Hydrus-Key", hex_encode(&access_key));

        let response = fixture
            .render(ServerResourceKind::RestrictedRepositoryFile, &mut request)
            .unwrap();

        assert_eq!(response.get_status_code(), 501);
    }
}
