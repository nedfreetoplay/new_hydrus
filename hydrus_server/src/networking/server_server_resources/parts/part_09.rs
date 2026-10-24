fn services_post_response(
    runtime: &ServerResourceRuntime,
    request: &HydrusRequest,
) -> HydrusResult<ResponseContext> {
    let values = request.parsed_request_args.get_list("services")?;
    let mut services = Vec::with_capacity(values.len());

    for value in values {
        let service = value.downcast_object::<ServerService>().ok_or_else(|| {
            HydrusException::new(ErrorKind::BadRequest, "Did not get valid services!")
        })?;

        services.push(service.duplicate()?);
    }

    let unique_ports: std::collections::HashSet<u16> =
        services.iter().map(ServerService::get_port).collect();

    if unique_ports.len() < services.len() {
        return Err(HydrusException::new(
            ErrorKind::BadRequest,
            "It looks like some of those services share ports! Please give them unique ports!",
        ));
    }

    let access_keys = runtime.db.modify_services(&services)?;

    network_response((
        "service_keys_to_access_keys",
        ArgValue::BytesDict(access_keys.into_iter().collect()),
    ))
}

fn petition_response(
    runtime: &ServerResourceRuntime,
    request: &HydrusRequest,
) -> HydrusResult<ResponseContext> {
    let content_type = u16::try_from(request.parsed_request_args.get_i64("content_type")?)
        .ok()
        .and_then(ContentType::try_from_int)
        .ok_or_else(|| {
            HydrusException::new(ErrorKind::BadRequest, "Unknown petition content type!")
        })?;
    let status = u8::try_from(request.parsed_request_args.get_i64("status")?)
        .ok()
        .and_then(ContentStatus::try_from_int)
        .ok_or_else(|| {
            HydrusException::new(ErrorKind::BadRequest, "Unknown petition status!")
        })?;

    let mut subject_account_key =
        request.parsed_request_args.get_bytes_or_none("subject_account_key")?;
    let mut reason = request.parsed_request_args.get_string_or_none("reason")?;

    if subject_account_key.is_none() || reason.is_none() {
        let service_id = runtime
            .db
            .get_service_id(runtime.service.get_service_key())?;
        let summaries = petition_summary_for(
            runtime,
            service_id,
            content_type,
            status,
            1,
            subject_account_key.as_deref(),
            reason.as_deref(),
        )?;

        let Some(header) = summaries.into_iter().next() else {
            return Err(HydrusException::new(
                ErrorKind::NotFound,
                "Sorry, no petitions were found!",
            ));
        };

        subject_account_key = Some(header.account_key);
        reason = Some(header.reason);
    }

    let subject_account_key = subject_account_key.ok_or_else(|| {
        HydrusException::new(ErrorKind::BadRequest, "Missing subject account key!")
    })?;
    let reason = reason.ok_or_else(|| {
        HydrusException::new(ErrorKind::BadRequest, "Missing petition reason!")
    })?;

    let service_id = runtime
        .db
        .get_service_id(runtime.service.get_service_key())?;
    let subject_account_id = runtime.db.get_account_id(&subject_account_key)?;
    let reason_id = runtime.db.get_reason_id(&reason)?;

    let petition = match (content_type, status) {
        (ContentType::Files, _) => {
            runtime
                .db
                .repository_get_file_petition(service_id, subject_account_id, reason_id)?
        }
        (ContentType::Mappings, _) => runtime
            .db
            .repository_get_mapping_petition(service_id, subject_account_id, reason_id)?,
        (ContentType::TagParents, ContentStatus::Pending) => runtime
            .db
            .repository_get_tag_parent_pend(service_id, subject_account_id, reason_id)?,
        (ContentType::TagParents, _) => runtime
            .db
            .repository_get_tag_parent_petition(service_id, subject_account_id, reason_id)?,
        (ContentType::TagSiblings, ContentStatus::Pending) => runtime
            .db
            .repository_get_tag_sibling_pend(service_id, subject_account_id, reason_id)?,
        (ContentType::TagSiblings, _) => runtime
            .db
            .repository_get_tag_sibling_petition(service_id, subject_account_id, reason_id)?,
        _ => {
            return Err(HydrusException::new(
                ErrorKind::BadRequest,
                "Unknown content type!",
            ));
        }
    };

    network_response(("petition", ArgValue::object(Arc::new(petition))))
}
