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
    let request =
        parse_request(b"GET /api_version?access_key=abc HTTP/1.1\r\nHost: x\r\n\r\n").unwrap();

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
