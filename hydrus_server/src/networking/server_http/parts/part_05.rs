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

    match response_from_context(&hydrus_request, &response_context, context.service()).await {
        Ok(response) => Ok(response),
        Err(error) => Ok(plain_error_response(&error)),
    }
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
            return Err(HydrusException::new(
                ErrorKind::BadRequest,
                "Did not recognise Content-Type header for this upload.",
            ));
        }
    }

    Ok(hydrus_request)
}
