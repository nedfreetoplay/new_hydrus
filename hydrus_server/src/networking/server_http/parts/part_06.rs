fn parse_query_args(query: Option<&str>) -> HydrusResult<ParsedRequestArguments> {
    let mut request_args: IndexMap<Vec<u8>, Vec<Vec<u8>>> = IndexMap::new();

    if let Some(query) = query {
        for pair in query.split('&') {
            if pair.is_empty() {
                continue;
            }

            let (key, value) = pair.split_once('=').unwrap_or((pair, ""));

            request_args
                .entry(percent_decode(key).into_bytes())
                .or_default()
                .push(percent_decode(value).into_bytes());
        }
    }

    parse_hydrus_network_get_args(&request_args)
}

async fn response_from_context(
    request: &HydrusRequest,
    response_context: &ResponseContext,
    service: &Arc<ServerService>,
) -> HydrusResult<Response<Full<Bytes>>> {
    let adapter = ServiceAdapter {
        service: Arc::clone(service),
    };
    let plan = build_render_plan(request, response_context, &adapter)?;

    let mut builder = Response::builder().status(
        StatusCode::from_u16(plan.status_code).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
    );

    builder = builder.header(
        "Server",
        format!(
            "{} v{}",
            service.get_service_type().long_str(),
            SOFTWARE_VERSION
        ),
    );

    for (name, values) in plan.headers.iter() {
        for value in values {
            builder = builder.header(name, value);
        }
    }

    for cookie in response_context.get_cookies() {
        builder = builder.header("Set-Cookie", format_cookie(cookie));
    }

    let body = render_plan_body(&plan).await?;

    builder.body(body).map_err(|error| {
        HydrusException::new(
            ErrorKind::Unknown,
            format!("Could not build the HTTP response: {error}"),
        )
    })
}

async fn render_plan_body(plan: &RenderPlan) -> HydrusResult<Full<Bytes>> {
    if let Some(path) = &plan.file_path {
        let data = tokio::fs::read(path).await.map_err(|error| {
            HydrusException::new(
                ErrorKind::FileMissing,
                format!("Could not read response file \"{}\": {error}", path.display()),
            )
        })?;

        let data = if let Some((offset, length)) = plan.file_range {
            let start = offset.max(0) as usize;
            let end = start.saturating_add(length.max(0) as usize);

            data.get(start..end).unwrap_or(&data).to_vec()
        } else {
            data
        };

        return Ok(Full::new(Bytes::from(data)));
    }

    Ok(Full::new(Bytes::from(
        plan.body.clone().unwrap_or_default(),
    )))
}

fn format_cookie(cookie: &ResponseCookie) -> String {
    let mut value = format!("{}={}", cookie.name, cookie.value);

    for (key, option_value) in &cookie.options {
        value.push_str("; ");
        value.push_str(key);

        if !option_value.is_empty() {
            value.push('=');
            value.push_str(option_value);
        }
    }

    value
}

fn not_found_response(path: &str) -> ResponseContext {
    ResponseContext::with_status_and_body(404, format!("No resource at \"{path}\"."))
}

fn plain_error_response(error: &HydrusException) -> Response<Full<Bytes>> {
    let status = StatusCode::from_u16(
        hydrus_core::networking::hydrus_server_resources::error_status_code(error.kind()),
    )
    .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);

    Response::builder()
        .status(status)
        .header("Content-Type", "text/plain; charset=utf-8")
        .body(Full::new(Bytes::from(error.to_string())))
        .expect("the fallback error response is valid")
}

fn build_tls_acceptor(settings: &TlsContextSettings) -> HydrusResult<TlsAcceptor> {
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();

    let certificates: Vec<CertificateDer<'static>> = settings
        .certificates
        .iter()
        .map(|certificate| CertificateDer::from(certificate.der.clone()))
        .collect();

    let mut key_reader = Cursor::new(settings.private_key_pem.clone());
    let private_key = rustls_pemfile::private_key(&mut key_reader)
        .map_err(|error| {
            HydrusException::new(
                ErrorKind::Unknown,
                format!("Could not parse the TLS private key: {error}"),
            )
        })?
        .ok_or_else(|| {
            HydrusException::new(ErrorKind::FileMissing, "No TLS private key was found.")
        })?;

    let config = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certificates, private_key)
        .map_err(|error| {
            HydrusException::new(
                ErrorKind::Unknown,
                format!("Could not build the TLS server configuration: {error}"),
            )
        })?;

    Ok(TlsAcceptor::from(Arc::new(config)))
}
