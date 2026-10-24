
use super::*;

struct MockTransport {
    status: u16,
    body: Vec<u8>,
    fail: bool,
}

impl HttpTransport for MockTransport {
    fn execute(&self, _request: &NetworkRequest) -> HydrusResult<NetworkResponse> {
        if self.fail {
            return Err(HydrusException::new(ErrorKind::Unknown, "mock failure"));
        }

        Ok(NetworkResponse {
            status: self.status,
            headers: BTreeMap::new(),
            body: self.body.clone(),
        })
    }
}

fn engine(status: u16, body: &[u8], fail: bool) -> NetworkEngine<MockTransport> {
    NetworkEngine::new(MockTransport {
        status,
        body: body.to_vec(),
        fail,
    })
}

#[test]
fn successful_job_records_body() {
    let mut engine = engine(200, b"hello", false);
    engine.add_job(NetworkJob::get("https://example.com"));

    let job = engine.run_next().unwrap();

    assert!(job.is_done());
    assert_eq!(job.get_response(), Some(b"hello".as_slice()));
    assert!(engine.queue().is_empty());
}

#[test]
fn http_errors_are_recorded_as_failures() {
    let mut engine = engine(500, b"", false);
    engine.add_job(NetworkJob::get("https://example.com"));

    let job = engine.run_next().unwrap();

    assert!(job.is_errored());
    assert_eq!(job.error.as_deref(), Some("HTTP 500"));
}

#[test]
fn transport_errors_are_recorded() {
    let mut engine = engine(200, b"", true);
    engine.add_job(NetworkJob::get("https://example.com"));

    let job = engine.run_next().unwrap();

    assert!(job.is_errored());
    assert!(job.error.as_deref().unwrap().contains("mock failure"));
}

#[test]
fn empty_queue_returns_none() {
    let mut engine = engine(200, b"", false);

    assert!(engine.run_next().is_none());
}

#[test]
fn request_carries_method_headers_and_body() {
    let mut job = NetworkJob::post("https://example.com", b"body".to_vec());
    job.set_header("X-Test", "1");

    let request = NetworkRequest::from(&job);

    assert_eq!(request.method, "POST");
    assert_eq!(request.headers["X-Test"], "1");
    assert_eq!(request.body.as_deref(), Some(b"body".as_slice()));
}
