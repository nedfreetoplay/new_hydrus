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
        ServerResourceKind::RestrictedOptionsModifyNullificationPeriod
        | ServerResourceKind::RestrictedOptionsModifyUpdatePeriod
        | ServerResourceKind::RestrictedMaintenanceRegenServiceInfo => {
            check_permission(request, ContentType::Options, PermissionAction::Moderate)?;
        }
        ServerResourceKind::RestrictedAccountOtherAccount
        | ServerResourceKind::RestrictedAccountInfo
        | ServerResourceKind::RestrictedAccountKeyFromContent
        | ServerResourceKind::RestrictedAccountModifyAccountType
        | ServerResourceKind::RestrictedAccountModifyBan
        | ServerResourceKind::RestrictedAccountModifyDeleteAllContent
        | ServerResourceKind::RestrictedAccountModifyExpires
        | ServerResourceKind::RestrictedAccountModifySetMessage
        | ServerResourceKind::RestrictedAccountModifyUnban => {
            check_permission(request, ContentType::Accounts, PermissionAction::Moderate)?;
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
