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
        ServerResourceKind::RestrictedOptionsModifyNullificationPeriod => {
            nullification_period_response(runtime, request)?
        }
        ServerResourceKind::RestrictedOptionsModifyUpdatePeriod => {
            update_period_response(runtime, request)?
        }
        ServerResourceKind::RestrictedAccountOtherAccount => {
            other_account_response(runtime, request)?
        }
        ServerResourceKind::RestrictedAccountInfo => account_info_response(runtime, request)?,
        ServerResourceKind::RestrictedAccountKeyFromContent => {
            account_key_from_content_response(runtime, request)?
        }
        ServerResourceKind::RestrictedAccountModifyAccountType => {
            modify_account_type_response(runtime, request)?
        }
        ServerResourceKind::RestrictedAccountModifyBan => {
            modify_account_ban_response(runtime, request)?
        }
        ServerResourceKind::RestrictedAccountModifyDeleteAllContent => {
            modify_account_delete_all_content_response(runtime, request)?
        }
        ServerResourceKind::RestrictedAccountModifyExpires => {
            modify_account_expires_response(runtime, request)?
        }
        ServerResourceKind::RestrictedAccountModifySetMessage => {
            modify_account_set_message_response(runtime, request)?
        }
        ServerResourceKind::RestrictedAccountModifyUnban => {
            modify_account_unban_response(runtime, request)?
        }
        ServerResourceKind::RestrictedMaintenanceRegenServiceInfo => {
            regenerate_service_info_response(runtime)?
        }
        ServerResourceKind::RestrictedNumPetitions => num_petitions_response(runtime)?,
        ServerResourceKind::RestrictedPetitionsSummary => {
            petitions_summary_response(runtime, request)?
        }
        ServerResourceKind::RestrictedMetadataUpdate => metadata_response(runtime)?,
        ServerResourceKind::RestrictedRepositoryFile if request.method == "POST" => {
            ResponseContext::with_status_and_body(
                501,
                "File upload resource is not ported yet.",
            )
        }
        ServerResourceKind::RestrictedRepositoryFile => repository_file_response(runtime, request)?,
        ServerResourceKind::RestrictedIp => ip_response(runtime, request)?,
        ServerResourceKind::RestrictedRepositoryThumbnail => {
            repository_thumbnail_response(runtime, request)?
        }
        ServerResourceKind::RestrictedTagFilter if request.method == "POST" => {
            ResponseContext::with_status_and_body(
                501,
                "Tag filter writing is not ported yet.",
            )
        }
        ServerResourceKind::RestrictedTagFilter => tag_filter_response(runtime)?,
        ServerResourceKind::RestrictedLockOn => {
            runtime.busy.store(true, Ordering::Relaxed);
            ResponseContext::with_status_and_body(200, "")
        }
        ServerResourceKind::RestrictedLockOff => {
            runtime.busy.store(false, Ordering::Relaxed);
            ResponseContext::with_status_and_body(200, "")
        }
        ServerResourceKind::RestrictedBackup => {
            runtime.db.backup()?;
            ResponseContext::with_status_and_body(200, "")
        }
        ServerResourceKind::RestrictedVacuum => {
            runtime.db.vacuum()?;
            ResponseContext::with_status_and_body(200, "")
        }
        ServerResourceKind::RestrictedRestartServices => {
            ResponseContext::with_status_and_body(200, "")
        }
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
