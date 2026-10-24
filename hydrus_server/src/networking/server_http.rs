//! HTTP/TLS integration for the server.
//!
//! The core crate deliberately stops at request/response policy. This module
//! supplies the actual listener: it accepts TLS connections, turns hyper
//! requests into [`HydrusRequest`] values, dispatches through the server route
//! tree, and writes the resulting [`RenderPlan`] back to the network.

use std::convert::Infallible;
use std::io::Cursor;
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Duration;

use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::service::service_fn;
use hyper::{Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use indexmap::IndexMap;
use rustls::pki_types::CertificateDer;
use tokio::net::TcpListener;
use tokio_rustls::TlsAcceptor;

use hydrus_core::hydrus_constants::ServiceType;
use hydrus_core::hydrus_exceptions::{ErrorKind, HydrusException, HydrusResult};
use hydrus_core::networking::hydrus_network::ServerService;
use hydrus_core::networking::hydrus_network_variable_handling::{
    ParsedRequestArguments, parse_hydrus_network_get_args,
    parse_network_bytes_to_parsed_hydrus_args, percent_decode,
};
use hydrus_core::networking::hydrus_server::HydrusService;
use hydrus_core::networking::hydrus_server_context_factory::{
    TlsContextSettings, generate_ssl_context_settings,
};
use hydrus_core::networking::hydrus_server_request::{HydrusRequest, RequestHeaders};
use hydrus_core::networking::hydrus_server_resources::{
    HydrusResourceService, RenderPlan, ResponseContext, ResponseCookie, build_render_plan,
    error_response,
};

use crate::networking::server_server::ServerResourceTree;
use crate::networking::server_server_resources::{ServerResourceRuntime, render_resource};
use crate::server_controller::Controller;

/// One service listener and the runtime state shared by its requests.
struct ServiceContext {
    runtime: Arc<ServerResourceRuntime>,
}

impl ServiceContext {
    fn service(&self) -> &Arc<ServerService> {
        &self.runtime.service
    }

    fn service_name(&self) -> String {
        self.service().get_name()
    }
}

/// Adapter from the server's `ServerService` to the core resource contract.
struct ServiceAdapter {
    service: Arc<ServerService>,
}

impl HydrusResourceService for ServiceAdapter {
    fn get_name(&self) -> String {
        self.service.get_name()
    }

    fn get_service_type(&self) -> ServiceType {
        self.service.get_service_type()
    }

    fn get_service_key(&self) -> Vec<u8> {
        self.service.get_service_key().to_vec()
    }

    fn allows_non_local_connections(&self) -> bool {
        self.service.allows_non_local_connections()
    }

    fn supports_cors(&self) -> bool {
        self.service.supports_cors()
    }

    fn use_normie_eris(&self) -> bool {
        self.service.use_normie_eris()
    }

    fn logs_requests(&self) -> bool {
        self.service.logs_requests()
    }

    fn report_data_used(&self, num_bytes: i64) {
        self.service.report_data_used(num_bytes);
    }

    fn report_request_used(&self) {
        self.service.report_request_used();
    }
}

/// Start listeners for every hydrus service in the database.
pub async fn serve(controller: Arc<Controller>) -> HydrusResult<()> {
    let db = controller.open_database()?;
    let (cert_path, key_path) = db.db().get_ssl_paths()?;
    let tls_settings = generate_ssl_context_settings(&cert_path, &key_path)?;
    let acceptor = Arc::new(build_tls_acceptor(&tls_settings)?);

    let services = db.get_services(None)?;

    let mut handles = Vec::new();

    for service in services {
        if !matches!(
            service.get_service_type(),
            ServiceType::ServerAdmin | ServiceType::FileRepository | ServiceType::TagRepository
        ) {
            continue;
        }

        let port = service.get_port();
        let runtime = Arc::new(controller.resource_runtime(service.get_service_key())?);
        let context = Arc::new(ServiceContext { runtime });
        let acceptor = Arc::clone(&acceptor);

        handles.push(tokio::spawn(async move {
            serve_service(port, context, acceptor).await
        }));
    }

    if handles.is_empty() {
        return Err(HydrusException::new(
            ErrorKind::Unknown,
            "There are no server services to listen on.",
        ));
    }

    let shutdown = controller.shutdown_requested();

    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                break;
            }
            _ = tokio::time::sleep(Duration::from_millis(200)) => {
                if shutdown.load(Ordering::Relaxed) {
                    break;
                }
            }
        }
    }

    for handle in handles {
        handle.abort();
    }

    Ok(())
}

async fn serve_service(
    port: u16,
    context: Arc<ServiceContext>,
    acceptor: Arc<TlsAcceptor>,
) -> HydrusResult<()> {
    let address = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = TcpListener::bind(address).await.map_err(|error| {
        HydrusException::new(
            ErrorKind::Network,
            format!("Could not bind server service port {port}: {error}"),
        )
    })?;

    println!(
        "hydrus_server: listening for \"{}\" on https://0.0.0.0:{port}",
        context.service_name()
    );

    loop {
        let (stream, peer_address) = match listener.accept().await {
            Ok(connection) => connection,
            Err(error) => {
                eprintln!("hydrus_server: accept failed on port {port}: {error}");

                continue;
            }
        };

        let context = Arc::clone(&context);
        let acceptor = Arc::clone(&acceptor);

        tokio::spawn(async move {
            let tls_stream = match acceptor.accept(stream).await {
                Ok(stream) => stream,
                Err(error) => {
                    eprintln!(
                        "hydrus_server: TLS handshake from {} failed: {error}",
                        peer_address
                    );

                    return;
                }
            };

            let io = TokioIo::new(tls_stream);
            let client_ip = peer_address.ip().to_string();
            let service = service_fn(move |request| {
                let context = Arc::clone(&context);
                let client_ip = client_ip.clone();

                async move { handle_request(request, context, client_ip).await }
            });

            if let Err(error) = hyper::server::conn::http1::Builder::new()
                .serve_connection(io, service)
                .await
            {
                eprintln!("hydrus_server: HTTP connection failed: {error}");
            }
        });
    }
}

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

    let body = request
        .into_body()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn query_parser_handles_repeated_and_percent_encoded_values() {
        let args = parse_query_args(Some("hash=01ab&hash=02cd&reason=blue%20sky")).unwrap();

        assert_eq!(args.get_bytes("hash").unwrap(), vec![0x01, 0xAB]);
        assert_eq!(args.get_string("reason").unwrap(), "blue sky");
    }

    #[test]
    fn cookie_formatter_matches_the_http_cookie_shape() {
        let cookie = ResponseCookie::new("session_key", "abcd")
            .with_option("max_age", "100")
            .with_option("path", "/");

        assert_eq!(
            format_cookie(&cookie),
            "session_key=abcd; max_age=100; path=/"
        );
    }

}
