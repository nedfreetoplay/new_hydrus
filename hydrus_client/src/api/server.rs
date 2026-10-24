//! Small local HTTP transport for the Client API.

use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{self, JoinHandle};

use hydrus_core::hydrus_exceptions::{ErrorKind, HydrusException, HydrusResult};

use crate::api::manager::{ApiBackend, ApiManager, ApiRequest};

pub struct ApiServer<B: ApiBackend + 'static> {
    listener: TcpListener,
    manager: Arc<ApiManager<B>>,
    shutdown: Arc<AtomicBool>,
}

pub struct ApiServerHandle {
    pub address: SocketAddr,
    shutdown: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl ApiServerHandle {
    pub fn stop(mut self) {
        self.shutdown.store(true, Ordering::Relaxed);
        let _ = TcpStream::connect(self.address);

        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

impl<B: ApiBackend + 'static> ApiServer<B> {
    pub fn bind(
        address: impl std::net::ToSocketAddrs,
        manager: Arc<ApiManager<B>>,
    ) -> HydrusResult<Self> {
        let listener = TcpListener::bind(address).map_err(|error| {
            HydrusException::new(ErrorKind::Unknown, format!("Could not bind API: {error}"))
        })?;

        Ok(Self {
            listener,
            manager,
            shutdown: Arc::new(AtomicBool::new(false)),
        })
    }

    pub fn local_addr(&self) -> HydrusResult<SocketAddr> {
        self.listener
            .local_addr()
            .map_err(|error| HydrusException::new(ErrorKind::Unknown, error.to_string()))
    }

    pub fn start(mut self) -> HydrusResult<ApiServerHandle> {
        let address = self.local_addr()?;
        let shutdown = self.shutdown.clone();
        let thread_shutdown = shutdown.clone();
        let manager = self.manager.clone();

        self.listener
            .set_nonblocking(true)
            .map_err(|error| HydrusException::new(ErrorKind::Unknown, error.to_string()))?;

        let thread = thread::spawn(move || {
            while !thread_shutdown.load(Ordering::Relaxed) {
                match self.listener.accept() {
                    Ok((stream, _)) => {
                        let _ = handle_connection(stream, &manager);
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(std::time::Duration::from_millis(10));
                    }
                    Err(_) => break,
                }
            }
        });

        Ok(ApiServerHandle {
            address,
            shutdown,
            thread: Some(thread),
        })
    }
}

fn handle_connection<B: ApiBackend>(
    mut stream: TcpStream,
    manager: &ApiManager<B>,
) -> std::io::Result<()> {
    stream.set_read_timeout(Some(std::time::Duration::from_secs(5)))?;

    let mut buffer = [0u8; 8192];
    let read = stream.read(&mut buffer)?;
    let request_text = String::from_utf8_lossy(&buffer[..read]);

    let response = match parse_request(&request_text) {
        Some(request) => manager.handle(&request),
        None => crate::api::manager::ApiResponse::error(400, "malformed request"),
    };

    let body = response.body.to_string();
    let status_text = match response.status {
        200 => "OK",
        400 => "Bad Request",
        401 => "Unauthorized",
        404 => "Not Found",
        _ => "Error",
    };

    let response_text = format!(
        "HTTP/1.1 {} {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        response.status,
        status_text,
        body.len(),
        body
    );

    stream.write_all(response_text.as_bytes())?;
    stream.flush()
}

fn parse_request(text: &str) -> Option<ApiRequest> {
    let mut lines = text.lines();
    let request_line = lines.next()?;
    let mut parts = request_line.split_whitespace();
    let method = parts.next()?.to_string();
    let target = parts.next()?;

    let (path, query_text) = match target.split_once('?') {
        Some((path, query)) => (path, Some(query)),
        None => (target, None),
    };

    let mut query = BTreeMap::new();

    if let Some(query_text) = query_text {
        for pair in query_text.split('&') {
            if pair.is_empty() {
                continue;
            }

            let (name, value) = pair.split_once('=').unwrap_or((pair, ""));
            query.insert(name.to_string(), value.to_string());
        }
    }

    Some(ApiRequest {
        method,
        path: path.to_string(),
        query,
        body: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::manager::{ApiResponse, ApiServiceInfo};
    use serde_json::json;
    use std::io::Write;

    struct MockBackend;

    impl ApiBackend for MockBackend {
        fn get_services(&self) -> Vec<ApiServiceInfo> {
            Vec::new()
        }
    }

    fn manager() -> Arc<ApiManager<MockBackend>> {
        Arc::new(ApiManager::new(MockBackend))
    }

    #[test]
    fn parses_get_request_target() {
        let request = parse_request("GET /api_version?access_key=abc HTTP/1.1\r\nHost: x\r\n\r\n").unwrap();

        assert_eq!(request.method, "GET");
        assert_eq!(request.path, "/api_version");
        assert_eq!(request.query["access_key"], "abc");
    }

    #[test]
    fn server_answers_api_version() {
        let server = ApiServer::bind("127.0.0.1:0", manager()).unwrap();
        let handle = server.start().unwrap();
        let address = handle.address;

        let mut stream = TcpStream::connect(address).unwrap();
        stream
            .write_all(b"GET /api_version HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
            .unwrap();

        let mut response = String::new();
        stream.read_to_string(&mut response).unwrap();

        assert!(response.starts_with("HTTP/1.1 200 OK"));
        assert!(response.contains("\"version\""));

        handle.stop();
    }

    #[test]
    fn unknown_endpoint_returns_404() {
        let server = ApiServer::bind("127.0.0.1:0", manager()).unwrap();
        let handle = server.start().unwrap();

        let mut stream = TcpStream::connect(handle.address).unwrap();
        stream
            .write_all(b"GET /nope HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
            .unwrap();

        let mut response = String::new();
        stream.read_to_string(&mut response).unwrap();

        assert!(response.starts_with("HTTP/1.1 404 Not Found"));

        handle.stop();
    }

    #[test]
    fn manager_response_is_json_encoded() {
        let response = ApiResponse::ok(json!({ "x": 1 }));
        assert_eq!(response.body.to_string(), "{\"x\":1}");
    }
}
