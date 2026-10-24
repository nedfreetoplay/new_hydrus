//! Network job state machine from `hydrus/client/networking/ClientNetworkingJobs.py`.
//!
//! The transport is intentionally separate so the same job metadata can be used
//! by reqwest, the Client API or tests.

use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkJobStatus {
    Ready,
    Working,
    Done,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetworkJob {
    pub url: String,
    pub method: String,
    pub headers: BTreeMap<String, String>,
    pub body: Option<Vec<u8>>,
    pub attempt: i64,
    pub max_attempts: i64,
    pub status: NetworkJobStatus,
    pub error: Option<String>,
    pub response: Option<Vec<u8>>,
}

impl NetworkJob {
    pub fn get(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            method: "GET".to_string(),
            headers: BTreeMap::new(),
            body: None,
            attempt: 0,
            max_attempts: 5,
            status: NetworkJobStatus::Ready,
            error: None,
            response: None,
        }
    }

    pub fn post(url: impl Into<String>, body: Vec<u8>) -> Self {
        Self {
            method: "POST".to_string(),
            body: Some(body),
            ..Self::get(url)
        }
    }

    pub fn set_header(&mut self, name: impl Into<String>, value: impl Into<String>) {
        self.headers.insert(name.into(), value.into());
    }

    pub fn start(&mut self) {
        self.attempt += 1;
        self.status = NetworkJobStatus::Working;
        self.error = None;
    }

    pub fn succeed(&mut self, response: Vec<u8>) {
        self.status = NetworkJobStatus::Done;
        self.response = Some(response);
    }

    pub fn fail(&mut self, error: impl Into<String>) {
        self.status = NetworkJobStatus::Error;
        self.error = Some(error.into());
    }

    pub fn can_retry(&self) -> bool {
        self.status == NetworkJobStatus::Error && self.attempt < self.max_attempts
    }

    pub fn retry(&mut self) -> bool {
        if !self.can_retry() {
            return false;
        }

        self.status = NetworkJobStatus::Ready;
        self.error = None;
        true
    }

    pub fn is_done(&self) -> bool {
        self.status == NetworkJobStatus::Done
    }

    pub fn is_errored(&self) -> bool {
        self.status == NetworkJobStatus::Error
    }

    pub fn get_response(&self) -> Option<&[u8]> {
        self.response.as_deref()
    }
}

#[derive(Debug, Default)]
pub struct NetworkJobQueue {
    jobs: Vec<NetworkJob>,
}

impl NetworkJobQueue {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_job(&mut self, job: NetworkJob) {
        self.jobs.push(job);
    }

    pub fn pop_ready(&mut self) -> Option<NetworkJob> {
        let index = self
            .jobs
            .iter()
            .position(|job| job.status == NetworkJobStatus::Ready)?;

        Some(self.jobs.remove(index))
    }

    pub fn requeue_failed(&mut self) -> usize {
        let mut count = 0;

        for job in &mut self.jobs {
            if job.retry() {
                count += 1;
            }
        }

        count
    }

    pub fn get_num_ready(&self) -> usize {
        self.jobs
            .iter()
            .filter(|job| job.status == NetworkJobStatus::Ready)
            .count()
    }

    pub fn get_num_working(&self) -> usize {
        self.jobs
            .iter()
            .filter(|job| job.status == NetworkJobStatus::Working)
            .count()
    }

    pub fn get_num_done(&self) -> usize {
        self.jobs.iter().filter(|job| job.is_done()).count()
    }

    pub fn len(&self) -> usize {
        self.jobs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.jobs.is_empty()
    }

    pub fn jobs(&self) -> &[NetworkJob] {
        &self.jobs
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn successful_job_records_response() {
        let mut job = NetworkJob::get("https://example.com");
        job.start();
        job.succeed(b"hello".to_vec());

        assert!(job.is_done());
        assert_eq!(job.get_response(), Some(b"hello".as_slice()));
        assert_eq!(job.attempt, 1);
    }

    #[test]
    fn failed_jobs_retry_until_max_attempts() {
        let mut job = NetworkJob::get("https://example.com");
        job.max_attempts = 2;

        job.start();
        job.fail("timeout");
        assert!(job.can_retry());
        assert!(job.retry());

        job.start();
        job.fail("timeout");
        assert!(!job.can_retry());
        assert!(!job.retry());
    }

    #[test]
    fn queue_pops_ready_jobs_in_order() {
        let mut queue = NetworkJobQueue::new();
        queue.add_job(NetworkJob::get("https://example.com/a"));
        queue.add_job(NetworkJob::get("https://example.com/b"));

        let first = queue.pop_ready().unwrap();

        assert_eq!(first.url, "https://example.com/a");
        assert_eq!(queue.get_num_ready(), 1);
    }

    #[test]
    fn requeue_failed_jobs_respects_attempts() {
        let mut queue = NetworkJobQueue::new();
        let mut job = NetworkJob::get("https://example.com");
        job.max_attempts = 1;
        job.start();
        job.fail("nope");
        queue.add_job(job);

        assert_eq!(queue.requeue_failed(), 0);
    }
}
