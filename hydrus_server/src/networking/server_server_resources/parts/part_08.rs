fn update_response(
    runtime: &ServerResourceRuntime,
    request: &HydrusRequest,
) -> HydrusResult<ResponseContext> {
    let update_hash = request.parsed_request_args.get_bytes("update_hash")?;

    if !runtime.service.has_update_hash(&update_hash)? {
        return Err(HydrusException::new(
            ErrorKind::NotFound,
            "This update hash does not exist on this service!",
        ));
    }

    let path = ServerFiles::new(runtime.db.get_files_dir()).get_file_path(&update_hash)?;

    ResponseContext::new(
        200,
        Mime::OctetStream,
        None,
        Some(path),
        Vec::new(),
        false,
        None,
    )
}

fn update_post_response(
    runtime: &ServerResourceRuntime,
    request: &HydrusRequest,
) -> HydrusResult<ResponseContext> {
    let value = request
        .parsed_request_args
        .get_required("client_to_server_update")?;
    let mut update = value
        .downcast_object::<ClientToServerUpdate>()
        .ok_or_else(|| {
            HydrusException::new(
                ErrorKind::BadRequest,
                "Did not get a valid client to server update!",
            )
        })?
        .clone();

    let account = request.hydrus_account.as_ref().ok_or_else(|| {
        HydrusException::new(ErrorKind::MissingCredentials, "No account on request!")
    })?;

    if runtime.service.get_service_type() == ServiceType::TagRepository {
        update.apply_tag_filter_to_pending_mappings(&runtime.service.get_tag_filter());
    }

    let timestamp = runtime.service.get_metadata()?.get_next_update_begin() + 1;

    runtime.db.repository_process_client_to_server_update(
        runtime.service.get_service_key(),
        account,
        &update,
        timestamp,
    )?;

    Ok(ResponseContext::with_status_and_body(200, ""))
}

fn metadata_response(
    runtime: &ServerResourceRuntime,
    request: &HydrusRequest,
) -> HydrusResult<ResponseContext> {
    let since = request.parsed_request_args.get_i64_default("since", 0)?;
    let metadata_slice = runtime.service.get_metadata_slice(since)?;

    network_response((
        "metadata_slice",
        ArgValue::object(Arc::new(metadata_slice)),
    ))
}

fn tag_filter_post_response(
    runtime: &ServerResourceRuntime,
    request: &HydrusRequest,
) -> HydrusResult<ResponseContext> {
    let value = request.parsed_request_args.get_required("tag_filter")?;
    let tag_filter = value
        .downcast_object::<TagFilter>()
        .ok_or_else(|| {
            HydrusException::new(ErrorKind::BadRequest, "Did not get a valid tag filter!")
        })?
        .clone();

    runtime.service.set_tag_filter(tag_filter);
    runtime.db.save_service(&runtime.service)?;

    Ok(ResponseContext::with_status_and_body(200, ""))
}

fn repository_file_post_response(
    runtime: &ServerResourceRuntime,
    request: &HydrusRequest,
) -> HydrusResult<ResponseContext> {
    let account = request.hydrus_account.as_ref().ok_or_else(|| {
        HydrusException::new(ErrorKind::MissingCredentials, "No account on request!")
    })?;
    let account_id = runtime.db.get_account_id(&account.get_account_key())?;
    let service_id = runtime
        .db
        .get_service_id(runtime.service.get_service_key())?;

    let hash = request.parsed_request_args.get_bytes("hash")?;
    let path = PathBuf::from(request.parsed_request_args.get_string("path")?);
    let size = u64::try_from(request.parsed_request_args.get_i64("size")?).map_err(|_| {
        HydrusException::new(ErrorKind::BadRequest, "File size was negative!")
    })?;
    let mime_int = u16::try_from(request.parsed_request_args.get_i64("mime")?)
        .ok()
        .and_then(Mime::try_from_int)
        .ok_or_else(|| {
            HydrusException::new(ErrorKind::BadRequest, "Unknown file mime!")
        })?;

    let width = request
        .parsed_request_args
        .get_i64_or_none("width")?
        .and_then(|value| u32::try_from(value).ok());
    let height = request
        .parsed_request_args
        .get_i64_or_none("height")?
        .and_then(|value| u32::try_from(value).ok());
    let duration_ms = request.parsed_request_args.get_i64_or_none("duration")?;
    let num_frames = request
        .parsed_request_args
        .get_i64_or_none("num_frames")?
        .and_then(|value| u32::try_from(value).ok());
    let num_words = request.parsed_request_args.get_i64_or_none("num_words")?;

    let ip = runtime
        .service
        .log_uploader_ips()
        .then(|| request.client_ip.clone());

    let timestamp = runtime.service.get_metadata()?.get_next_update_begin() + 1;

    let file = ServerFileInput {
        hash,
        size,
        mime: mime_int,
        width,
        height,
        duration_ms,
        num_frames,
        num_words,
        path,
        thumbnail: None,
    };

    runtime.db.repository_add_file(
        service_id,
        account_id,
        &file,
        false,
        timestamp,
        ip.as_deref(),
    )?;

    Ok(ResponseContext::with_status_and_body(200, ""))
}
