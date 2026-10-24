
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
