//! Network engine abstraction and reqwest transport
//! (`hydrus/client/networking/ClientNetworking.py`).

use std::collections::BTreeMap;

use hydrus_core::hydrus_exceptions::{ErrorKind, HydrusException, HydrusResult};

use crate::networking::client_networking_jobs::{NetworkJob, NetworkJobQueue};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetworkRequest {
    pub method: String,
    pub url: String,
    pub headers: BTreeMap<String, String>,
    pub body: Option<Vec<u8>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetworkResponse {
    pub status: u16,
    pub headers: BTreeMap<String, String>,
    pub body: Vec<u8>,
}

pub trait HttpTransport: Send + Sync {
    fn execute(&self, request: &NetworkRequest) -> HydrusResult<NetworkResponse>;
}

impl From<&NetworkJob> for NetworkRequest {
    fn from(job: &NetworkJob) -> Self {
        Self {
            method: job.method.clone(),
            url: job.url.clone(),
            headers: job.headers.clone(),
            body: job.body.clone(),
        }
    }
}

pub struct ReqwestTransport {
    client: reqwest::blocking::Client,
}

impl ReqwestTransport {
    pub fn new(timeout_seconds: u64) -> HydrusResult<Self> {
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(timeout_seconds))
            .user_agent(crate::client_defaults::DEFAULT_USER_AGENT)
            .build()
            .map_err(|error| HydrusException::new(ErrorKind::Unknown, error.to_string()))?;

        Ok(Self { client })
    }
}

impl HttpTransport for ReqwestTransport {
    fn execute(&self, request: &NetworkRequest) -> HydrusResult<NetworkResponse> {
        let method = reqwest::Method::from_bytes(request.method.as_bytes())
            .map_err(|error| HydrusException::new(ErrorKind::Unknown, error.to_string()))?;

        let mut builder = self.client.request(method, &request.url);

        for (name, value) in &request.headers {
            builder = builder.header(name, value);
        }

        if let Some(body) = &request.body {
            builder = builder.body(body.clone());
        }

        let response = builder
            .send()
            .map_err(|error| HydrusException::new(ErrorKind::Unknown, error.to_string()))?;

        let status = response.status().as_u16();
        let headers = response
            .headers()
            .iter()
            .map(|(name, value)| {
                (
                    name.as_str().to_string(),
                    value.to_str().unwrap_or_default().to_string(),
                )
            })
            .collect();

        let body = response
            .bytes()
            .map_err(|error| HydrusException::new(ErrorKind::Unknown, error.to_string()))?
            .to_vec();

        Ok(NetworkResponse {
            status,
            headers,
            body,
        })
    }
}

pub struct NetworkEngine<T: HttpTransport> {
    transport: T,
    queue: NetworkJobQueue,
}

impl<T: HttpTransport> NetworkEngine<T> {
    pub fn new(transport: T) -> Self {
        Self {
            transport,
            queue: NetworkJobQueue::new(),
        }
    }

    pub fn add_job(&mut self, job: NetworkJob) {
        self.queue.add_job(job);
    }

    /// Runs the next ready job. Errors are recorded on the job so callers can
    /// retry through the queue.
    pub fn run_next(&mut self) -> Option<NetworkJob> {
        let mut job = self.queue.pop_ready()?;
        job.start();

        let request = NetworkRequest::from(&job);

        match self.transport.execute(&request) {
            Ok(response) if (200..300).contains(&response.status) => {
                job.succeed(response.body);
            }
            Ok(response) => {
                job.fail(format!("HTTP {}", response.status));
            }
            Err(error) => {
                job.fail(error.to_string());
            }
        }

        Some(job)
    }

    pub fn queue(&self) -> &NetworkJobQueue {
        &self.queue
    }

    pub fn queue_mut(&mut self) -> &mut NetworkJobQueue {
        &mut self.queue
    }

    pub fn transport(&self) -> &T {
        &self.transport
    }
}

#[cfg(test)]
mod tests {
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
}
