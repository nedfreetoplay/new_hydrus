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
