use super::*;

#[test]
fn notifications_keep_order_and_drop_empty_messages() {
    let mut log = NotificationLog::default();

    log.push("info", "first");
    log.push("error", "  ");
    log.push("error", "second");

    assert_eq!(log.len(), 2);
    assert_eq!(log.entries()[0].text, "first");
    assert_eq!(log.entries()[0].level, "info");
    assert_eq!(log.entries()[1].text, "second");
    assert!(log.entries()[0].timestamp > 0);
}

#[test]
fn notifications_can_be_trimmed_to_the_configured_limit() {
    let mut log = NotificationLog::default();

    for index in 0..10 {
        log.push("info", format!("message {index}"));
    }

    log.trim_to(3);

    assert_eq!(log.len(), 3);
    assert_eq!(log.entries()[0].text, "message 7");
    assert_eq!(log.entries()[2].text, "message 9");
}

#[test]
fn notifications_are_capped_and_can_be_cleared() {
    let mut log = NotificationLog::default();

    for index in 0..(LIMIT + 10) {
        log.push("info", format!("message {index}"));
    }

    assert_eq!(log.len(), LIMIT);
    assert_eq!(log.entries()[0].text, "message 10");

    log.clear();

    assert!(log.is_empty());
}
