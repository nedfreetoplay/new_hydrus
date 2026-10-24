fn modify_account_ban_response(
    runtime: &ServerResourceRuntime,
    request: &HydrusRequest,
) -> HydrusResult<ResponseContext> {
    let subject_key = subject_account_key(request)?;
    let reason = request.parsed_request_args.get_string("reason")?;
    let expires = request.parsed_request_args.get_i64_or_none("expires")?;

    runtime.db.modify_account_ban(
        runtime.service.get_service_key(),
        &subject_key,
        &reason,
        expires,
    )?;

    refresh_accounts(runtime)?;

    Ok(ResponseContext::with_status_and_body(200, ""))
}
fn modify_account_delete_all_content_response(
    runtime: &ServerResourceRuntime,
    request: &HydrusRequest,
) -> HydrusResult<ResponseContext> {
    let subject_key = subject_account_key(request)?;
    let admin = request.hydrus_account.as_ref().ok_or_else(|| {
        HydrusException::new(ErrorKind::MissingCredentials, "No admin account on request!")
    })?;

    let deleted = runtime.db.modify_account_delete_all_content(
        runtime.service.get_service_key(),
        &admin.get_account_key(),
        &subject_key,
    )?;

    network_response(("everything_was_deleted", ArgValue::from(deleted)))
}
fn modify_account_expires_response(
    runtime: &ServerResourceRuntime,
    request: &HydrusRequest,
) -> HydrusResult<ResponseContext> {
    let subject_key = subject_account_key(request)?;
    let expires = request.parsed_request_args.get_i64_or_none("expires")?;

    runtime.db.modify_account_expires(
        runtime.service.get_service_key(),
        &subject_key,
        expires,
    )?;

    refresh_accounts(runtime)?;

    Ok(ResponseContext::with_status_and_body(200, ""))
}
fn modify_account_set_message_response(
    runtime: &ServerResourceRuntime,
    request: &HydrusRequest,
) -> HydrusResult<ResponseContext> {
    let subject_key = subject_account_key(request)?;
    let message = request.parsed_request_args.get_string("message")?;

    runtime.db.modify_account_set_message(
        runtime.service.get_service_key(),
        &subject_key,
        &message,
    )?;

    refresh_accounts(runtime)?;

    Ok(ResponseContext::with_status_and_body(200, ""))
}
fn modify_account_unban_response(
    runtime: &ServerResourceRuntime,
    request: &HydrusRequest,
) -> HydrusResult<ResponseContext> {
    let subject_key = subject_account_key(request)?;

    runtime
        .db
        .modify_account_unban(runtime.service.get_service_key(), &subject_key)?;

    refresh_accounts(runtime)?;

    Ok(ResponseContext::with_status_and_body(200, ""))
}
fn regenerate_service_info_response(
    runtime: &ServerResourceRuntime,
) -> HydrusResult<ResponseContext> {
    let service_id = runtime
        .db
        .get_service_id(runtime.service.get_service_key())?;

    runtime.db.regenerate_service_info(Some(service_id), None)?;

    Ok(ResponseContext::with_status_and_body(200, ""))
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


fn refresh_accounts(runtime: &ServerResourceRuntime) -> HydrusResult<()> {
    runtime
        .session_manager
        .refresh_accounts(runtime.service.get_service_key(), None)
}
