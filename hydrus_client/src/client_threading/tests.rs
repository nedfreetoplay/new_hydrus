
use super::*;

#[test]
fn status_texts_are_a_stack() {
    let job = JobStatus::new(false, false);
    job.set_status_text("first");
    job.set_status_text("second");

    assert_eq!(job.get_status_text().as_deref(), Some("second"));

    job.delete_status_text();

    assert_eq!(job.get_status_text().as_deref(), Some("first"));
}

#[test]
fn cancel_finishes_the_job() {
    let job = JobStatus::new(false, true);
    assert!(!job.is_done());

    job.cancel();

    assert!(job.is_cancelled());
    assert!(job.is_done());
}

#[test]
fn to_string_includes_known_fields() {
    let job = JobStatus::new(false, false);
    job.set_status_title("Working");
    job.set_gauge(Some(2), Some(5));
    job.add_url("https://example.com");

    let text = job.to_string();

    assert!(text.contains("Working"));
    assert!(text.contains("2/5"));
    assert!(text.contains("https://example.com"));
}
