async fn handle_request(
    request: Request<Incoming>,
    context: Arc<ServiceContext>,
    client_ip: String,
) -> Result<Response<Full<Bytes>>, Infallible> {
    let mut hydrus_request = match build_hydrus_request(request, client_ip).await {
        Ok(request) => request,
        Err(error) => return Ok(plain_error_response(&error)),
    };

    let path = hydrus_request
        .path
        .trim_matches('/')
        .to_string();

    let response_context = if matches!(path.as_str(), "" | "favicon.ico" | "robots.txt") {
        let adapter = Arc::new(ServiceAdapter {
            service: Arc::clone(context.service()),
        });
        let service = HydrusService::new(adapter);

        match service.render_resource(&path) {
            Ok(Some(response)) => response,
            Ok(None) => not_found_response(&path),
            Err(error) => error_response(&context.service_name(), &error)
                .unwrap_or_else(|_| not_found_response(&path)),
        }
    } else {
        let routes =
            match ServerResourceTree::for_service_type(context.service().get_service_type()) {
                Ok(routes) => routes,
                Err(error) => return Ok(plain_error_response(&error)),
            };

        let Some(route) = routes.get_child(&path).copied() else {
            return Ok(plain_error_response(&HydrusException::new(
                ErrorKind::NotFound,
                format!("No server resource is registered at \"{path}\"."),
            )));
        };

        match render_resource(route.kind, route, &context.runtime, &mut hydrus_request) {
            Ok(response) => response,
            Err(error) => match error_response(&context.service_name(), &error) {
                Ok(response) => response,
                Err(error) => return Ok(plain_error_response(&error)),
            },
        }
    };

    let status_code = response_context.get_status_code();

    let result = match response_from_context(&hydrus_request, &response_context, context.service()).await {
        Ok(response) => Ok(response),
        Err(error) => Ok(plain_error_response(&error)),
    };

    let elapsed = hydrus_request.start_time.elapsed().as_secs_f64();

    println!(
        "hydrus_server: {} {} {} {} in {:.3}s",
        context.service_name(),
        hydrus_request.method,
        hydrus_request.path,
        status_code,
        elapsed
    );

    hydrus_request.clean_up_temp_file();

    result
}

async fn build_hydrus_request(
    request: Request<Incoming>,
    client_ip: String,
) -> HydrusResult<HydrusRequest> {
    let method = request.method().as_str().to_string();
    let uri = request.uri().clone();
    let mut headers = RequestHeaders::new();

    for (name, value) in request.headers() {
        if let Ok(value) = value.to_str() {
            headers.add(name.as_str(), value);
        }
    }

    let body = Limited::new(request.into_body(), FAT_HTTP_CHANNEL_MAX_LENGTH)
        .collect()
        .await
        .map_err(|error| {
            HydrusException::new(
                ErrorKind::BadRequest,
                format!("Could not read the request body: {error}"),
            )
        })?
        .to_bytes()
        .to_vec();

    let mut hydrus_request =
        HydrusRequest::new(method.clone(), uri.path().to_string()).with_client_ip(client_ip);
    hydrus_request.request_headers = headers;
    hydrus_request.body = body;

    if method == "GET" {
        hydrus_request.parsed_request_args = parse_query_args(uri.query())?;
    } else if method == "POST" {
        let content_type = hydrus_request
            .request_headers
            .get_first("Content-Type")
            .map(|value| value.split(';').next().unwrap_or_default().trim().to_lowercase());

        if content_type.as_deref() == Some("application/json") {
            hydrus_request.parsed_request_args =
                parse_network_bytes_to_parsed_hydrus_args(&hydrus_request.body)?;
        } else if hydrus_request.body.is_empty() {
            hydrus_request.parsed_request_args = ParsedRequestArguments::new();
        } else {
            let (mut file_handle, temp_path) =
                hydrus_core::hydrus_temp::get_temp_path("file_upload", "", None)?;

            std::io::Write::write_all(&mut file_handle, &hydrus_request.body).map_err(
                |error| {
                    HydrusException::new(
                        ErrorKind::BadRequest,
                        format!("Could not write the upload to disk: {error}"),
                    )
                },
            )?;
            std::io::Write::flush(&mut file_handle).map_err(|error| {
                HydrusException::new(
                    ErrorKind::BadRequest,
                    format!("Could not flush the upload to disk: {error}"),
                )
            })?;

            let parsed_args =
                hydrus_core::networking::hydrus_network_variable_handling::parse_file_arguments(
                    &temp_path,
                    false,
                );

            match parsed_args {
                Ok(parsed_args) => {
                    hydrus_request.parsed_request_args = parsed_args;
                    hydrus_request.temp_file_info = Some(
                        hydrus_core::networking::hydrus_server_request::TempFileInfo::new(
                            file_handle,
                            temp_path,
                        ),
                    );
                }
                Err(error) => {
                    hydrus_core::hydrus_temp::clean_up_temp_path(file_handle, &temp_path);
                    return Err(error);
                }
            }
        }
    }

    Ok(hydrus_request)
}
