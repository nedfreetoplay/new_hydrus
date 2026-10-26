//! Small local HTTP transport for the Client API.

use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{self, JoinHandle};

use hydrus_core::hydrus_exceptions::{ErrorKind, HydrusException, HydrusResult};

use crate::api::manager::{ApiBackend, ApiManager, ApiRequest, percent_decode};

const MAX_REQUEST_BYTES: usize = 32 * 1024 * 1024;

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

    pub fn start(self) -> HydrusResult<ApiServerHandle> {
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
    stream.set_read_timeout(Some(std::time::Duration::from_secs(10)))?;

    let request_bytes = read_request(&mut stream)?;

    let response = match parse_request(&request_bytes) {
        Some(request) => manager.handle(&request),
        None => crate::api::manager::ApiResponse::error(400, "malformed request"),
    };

    let (content_type, body): (String, Vec<u8>) = match response.binary {
        Some((content_type, bytes)) => (content_type, bytes),
        None => (
            "application/json".to_string(),
            response.body.to_string().into_bytes(),
        ),
    };

    let status_text = match response.status {
        200 => "OK",
        400 => "Bad Request",
        401 => "Unauthorized",
        404 => "Not Found",
        405 => "Method Not Allowed",
        _ => "Error",
    };

    let header = format!(
        "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        response.status,
        status_text,
        content_type,
        body.len()
    );

    stream.write_all(header.as_bytes())?;
    stream.write_all(&body)?;
    stream.flush()
}

fn read_request(stream: &mut TcpStream) -> std::io::Result<Vec<u8>> {
    let mut data = Vec::new();
    let mut buffer = [0u8; 8192];
    let mut content_length = 0usize;
    let mut header_end = None;

    loop {
        let read = stream.read(&mut buffer)?;

        if read == 0 {
            break;
        }

        data.extend_from_slice(&buffer[..read]);

        if data.len() > MAX_REQUEST_BYTES {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "request is too large",
            ));
        }

        if header_end.is_none() {
            if let Some(position) = find_bytes(&data, b"\r\n\r\n") {
                header_end = Some(position + 4);
                content_length = content_length_from_headers(&data[..position]);
            }
        }

        if let Some(header_end) = header_end {
            if data.len() >= header_end + content_length {
                break;
            }
        }
    }

    Ok(data)
}

fn content_length_from_headers(headers: &[u8]) -> usize {
    let text = String::from_utf8_lossy(headers);

    for line in text.lines().skip(1) {
        if let Some((name, value)) = line.split_once(':') {
            if name.trim().eq_ignore_ascii_case("content-length") {
                return value.trim().parse().unwrap_or(0);
            }
        }
    }

    0
}

fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

fn parse_request(data: &[u8]) -> Option<ApiRequest> {
    let header_end = find_bytes(data, b"\r\n\r\n")?;
    let head = String::from_utf8_lossy(&data[..header_end]);
    let body = data[header_end + 4..].to_vec();

    let mut lines = head.lines();
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
            query.insert(percent_decode(name), percent_decode(value));
        }
    }

    let mut headers = BTreeMap::new();

    for line in lines {
        if let Some((name, value)) = line.split_once(':') {
            headers.insert(name.trim().to_ascii_lowercase(), value.trim().to_string());
        }
    }

    Some(ApiRequest {
        method,
        path: percent_decode(path),
        query,
        headers,
        body,
    })
}

#[cfg(test)]
mod tests;
