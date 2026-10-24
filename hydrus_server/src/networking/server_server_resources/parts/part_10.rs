fn petition_summary_for(
    runtime: &ServerResourceRuntime,
    service_id: i64,
    content_type: ContentType,
    status: ContentStatus,
    limit: usize,
    subject_account_key: Option<&[u8]>,
    reason: Option<&str>,
) -> HydrusResult<Vec<hydrus_core::networking::hydrus_network::PetitionHeader>> {
    let account_id = match subject_account_key {
        Some(key) => Some(runtime.db.get_account_id(key)?),
        None => None,
    };
    let reason_id = match reason {
        Some(reason) => Some(runtime.db.get_reason_id(reason)?),
        None => None,
    };

    match (content_type, status) {
        (ContentType::Files, _) => runtime.db.repository_get_file_petitions_summary(
            service_id, limit, account_id, reason_id,
        ),
        (ContentType::Mappings, _) => runtime
            .db
            .repository_get_mapping_petitions_summary(service_id, limit, account_id, reason_id),
        (ContentType::TagParents, ContentStatus::Pending) => runtime
            .db
            .repository_get_tag_parent_pends_summary(service_id, limit, account_id, reason_id),
        (ContentType::TagParents, _) => runtime
            .db
            .repository_get_tag_parent_petitions_summary(service_id, limit, account_id, reason_id),
        (ContentType::TagSiblings, ContentStatus::Pending) => runtime
            .db
            .repository_get_tag_sibling_pends_summary(service_id, limit, account_id, reason_id),
        (ContentType::TagSiblings, _) => runtime
            .db
            .repository_get_tag_sibling_petitions_summary(service_id, limit, account_id, reason_id),
        _ => Err(HydrusException::new(
            ErrorKind::BadRequest,
            "Unknown content type!",
        )),
    }
}

fn account_types_post_response(
    runtime: &ServerResourceRuntime,
    request: &HydrusRequest,
) -> HydrusResult<ResponseContext> {
    let values = request.parsed_request_args.get_list("account_types")?;
    let mut account_types = Vec::with_capacity(values.len());

    for value in values {
        let account_type = value.downcast_object::<AccountType>().ok_or_else(|| {
            HydrusException::new(ErrorKind::BadRequest, "Did not get valid account types!")
        })?;

        account_types.push(account_type.clone());
    }

    let replacements = request
        .parsed_request_args
        .get_required("deletee_account_type_keys_to_new_account_type_keys")?
        .as_bytes_dict()
        .ok_or_else(|| {
            HydrusException::new(
                ErrorKind::BadRequest,
                "Did not get valid account type replacements!",
            )
        })?
        .iter()
        .map(|(deletee, replacement)| (deletee.clone(), replacement.clone()))
        .collect();

    runtime.db.modify_account_types(
        runtime.service.get_service_key(),
        &account_types,
        &replacements,
    )?;

    runtime
        .session_manager
        .refresh_accounts(runtime.service.get_service_key(), None)?;

    Ok(ResponseContext::with_status_and_body(200, ""))
}
