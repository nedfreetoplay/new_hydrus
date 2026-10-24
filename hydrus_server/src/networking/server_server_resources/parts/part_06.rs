fn repository_file_response(
    runtime: &ServerResourceRuntime,
    request: &HydrusRequest,
) -> HydrusResult<ResponseContext> {
    let hash = request.parsed_request_args.get_bytes("hash")?;
    let service_key = runtime.service.get_service_key();
    let (valid, mime) = runtime.db.repository_has_file(service_key, &hash)?;

    if !valid {
        return Err(HydrusException::new(
            ErrorKind::NotFound,
            "File not found on this service!",
        ));
    }

    let path = ServerFiles::new(runtime.db.get_files_dir()).get_file_path(&hash)?;

    ResponseContext::new(
        200,
        mime.unwrap_or(Mime::OctetStream),
        None,
        Some(path),
        Vec::new(),
        false,
        None,
    )
}
fn repository_thumbnail_response(
    runtime: &ServerResourceRuntime,
    request: &HydrusRequest,
) -> HydrusResult<ResponseContext> {
    let hash = request.parsed_request_args.get_bytes("hash")?;
    let path = ServerFiles::new(runtime.db.get_files_dir()).get_thumbnail_path(&hash)?;

    ResponseContext::new(
        200,
        Mime::Png,
        None,
        Some(path),
        Vec::new(),
        false,
        None,
    )
}
fn ip_response(
    runtime: &ServerResourceRuntime,
    request: &HydrusRequest,
) -> HydrusResult<ResponseContext> {
    let hash = request.parsed_request_args.get_bytes("hash")?;
    let (ip, timestamp) = runtime
        .db
        .get_ip_timestamp(runtime.service.get_service_key(), &hash)?;

    let mut values = IndexMap::new();
    values.insert("ip".to_string(), ArgValue::from(ip));
    values.insert("timestamp".to_string(), ArgValue::from(timestamp));

    network_response(("ip_info", ArgValue::Dict(values)))
}
fn tag_filter_response(runtime: &ServerResourceRuntime) -> HydrusResult<ResponseContext> {
    network_response((
        "tag_filter",
        ArgValue::object(Arc::new(runtime.service.get_tag_filter())),
    ))
}
fn num_petitions_response(runtime: &ServerResourceRuntime) -> HydrusResult<ResponseContext> {
    let service_id = runtime
        .db
        .get_service_id(runtime.service.get_service_key())?;
    let mut total = 0;

    total += runtime
        .db
        .repository_get_file_petitions_summary(service_id, usize::MAX, None, None)?
        .len();
    total += runtime
        .db
        .repository_get_mapping_petitions_summary(service_id, usize::MAX, None, None)?
        .len();
    total += runtime
        .db
        .repository_get_tag_parent_petitions_summary(service_id, usize::MAX, None, None)?
        .len();
    total += runtime
        .db
        .repository_get_tag_sibling_petitions_summary(service_id, usize::MAX, None, None)?
        .len();

    network_response(("num_petitions", ArgValue::from(total as i64)))
}
fn petitions_summary_response(
    runtime: &ServerResourceRuntime,
    request: &HydrusRequest,
) -> HydrusResult<ResponseContext> {
    let service_id = runtime
        .db
        .get_service_id(runtime.service.get_service_key())?;
    let limit = request
        .parsed_request_args
        .get_i64_default("limit", 100)?
        .max(1) as usize;
    let content_type = request
        .parsed_request_args
        .get_i64_default("content_type", i64::from(ContentType::Files.as_int()))?;

    let headers = match u16::try_from(content_type)
        .ok()
        .and_then(ContentType::try_from_int)
    {
        Some(ContentType::Files) => {
            runtime
                .db
                .repository_get_file_petitions_summary(service_id, limit, None, None)?
        }
        Some(ContentType::Mappings) => {
            runtime
                .db
                .repository_get_mapping_petitions_summary(service_id, limit, None, None)?
        }
        Some(ContentType::TagParents) => {
            runtime
                .db
                .repository_get_tag_parent_petitions_summary(service_id, limit, None, None)?
        }
        Some(ContentType::TagSiblings) => {
            runtime
                .db
                .repository_get_tag_sibling_petitions_summary(service_id, limit, None, None)?
        }
        _ => {
            return Err(HydrusException::new(
                ErrorKind::BadRequest,
                "Unknown petition content type.",
            ));
        }
    };

    let headers: Vec<ArgValue> = headers
        .into_iter()
        .map(|header| ArgValue::object(Arc::new(header)))
        .collect();

    network_response(("petitions_summary", ArgValue::List(headers)))
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
