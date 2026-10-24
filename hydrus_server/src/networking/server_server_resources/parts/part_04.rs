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
fn nullification_period_response(
    runtime: &ServerResourceRuntime,
    request: &HydrusRequest,
) -> HydrusResult<ResponseContext> {
    let period = request
        .parsed_request_args
        .get_i64("nullification_period")?;

    runtime
        .db
        .set_service_nullification_period(runtime.service.get_service_key(), period)?;

    Ok(ResponseContext::with_status_and_body(200, ""))
}
fn update_period_response(
    runtime: &ServerResourceRuntime,
    request: &HydrusRequest,
) -> HydrusResult<ResponseContext> {
    let period = request.parsed_request_args.get_i64("update_period")?;

    runtime
        .db
        .set_service_update_period(runtime.service.get_service_key(), period)?;

    Ok(ResponseContext::with_status_and_body(200, ""))
}
fn subject_account_key(request: &HydrusRequest) -> HydrusResult<Vec<u8>> {
    request
        .parsed_request_args
        .get_bytes("subject_account_key")
}
fn other_account_response(
    runtime: &ServerResourceRuntime,
    request: &HydrusRequest,
) -> HydrusResult<ResponseContext> {
    let subject_key = subject_account_key(request)?;
    let account = runtime
        .db
        .get_account_from_account_key(runtime.service.get_service_key(), &subject_key)?;

    network_response(("account", ArgValue::other_object(Arc::new(account))))
}
fn account_info_response(
    runtime: &ServerResourceRuntime,
    request: &HydrusRequest,
) -> HydrusResult<ResponseContext> {
    let subject_key = subject_account_key(request)?;
    let subject_id = runtime.db.get_account_id(&subject_key)?;
    let info = runtime
        .db
        .get_account_info(runtime.service.get_service_key(), subject_id)?;

    let mut values = IndexMap::new();

    for (key, value) in info {
        values.insert(key, ArgValue::from(value));
    }

    network_response(("account_info", ArgValue::Dict(values)))
}
fn account_key_from_content_response(
    runtime: &ServerResourceRuntime,
    request: &HydrusRequest,
) -> HydrusResult<ResponseContext> {
    let hash = request.parsed_request_args.get_bytes("subject_hash")?;
    let content = match request
        .parsed_request_args
        .get_string_or_none("subject_tag")?
    {
        Some(tag) => Content::mapping(tag, hash),
        None => Content::files(vec![hash]),
    };

    let account_key = runtime
        .db
        .get_account_key_from_content(runtime.service.get_service_key(), &content)?;

    network_response(("subject_account_key", ArgValue::from(account_key)))
}
fn modify_account_type_response(
    runtime: &ServerResourceRuntime,
    request: &HydrusRequest,
) -> HydrusResult<ResponseContext> {
    let subject_key = subject_account_key(request)?;
    let account_type_key = request
        .parsed_request_args
        .get_bytes("account_type_key")?;

    runtime.db.modify_account_account_type(
        runtime.service.get_service_key(),
        &subject_key,
        &account_type_key,
    )?;

    refresh_accounts(runtime)?;

    Ok(ResponseContext::with_status_and_body(200, ""))
}
