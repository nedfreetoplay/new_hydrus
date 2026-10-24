
use super::*;

#[test]
fn watcher_becomes_due_after_checker_delay() {
    let mut watcher = WatcherImport::new("https://example.com", "page", 0);
    watcher.mark_checked(100);

    assert!(!watcher.is_due(399));
    assert!(watcher.is_due(400));
}

#[test]
fn paused_watchers_are_not_due() {
    let mut watcher = WatcherImport::new("https://example.com", "page", 0);
    watcher.mark_checked(0);
    watcher.pause_play_checking();

    assert!(!watcher.is_due(100_000));
}

#[test]
fn multiple_watchers_aggregate_seeds() {
    let mut multi = MultipleWatcherImport::new("https://example.com");
    let mut first = WatcherImport::new("https://example.com/a", "a", 0);
    first.add_file_seed(FileSeed::url("https://example.com/1", 0));
    let second = WatcherImport::new("https://example.com/b", "b", 0);
    multi.add_watcher(first);
    multi.add_watcher(second);

    assert_eq!(multi.get_num_watchers(), 2);
    assert_eq!(multi.get_num_seeds(), 1);
}

#[test]
fn subscription_respects_no_work_until() {
    let mut subscription = Subscription::new("sub", "query");
    subscription.set_no_work_until(10_000);

    assert!(!subscription.is_due(86_399));
    assert!(subscription.is_due(86_400));
}

#[test]
fn subscriptions_manager_finds_earliest_due() {
    let mut manager = SubscriptionsManager::new();
    let soon = Subscription::new("soon", "a");
    let earlier = Subscription::new("earlier", "b");
    manager.add_subscription(soon);
    manager.add_subscription(earlier);

    assert!(manager.get_due_subscription(i64::MAX).is_some());
    manager.pause_all_for_editing();

    assert!(manager.get_due_subscription(i64::MAX).is_none());
}

struct MockTransport {
    body: Vec<u8>,
}

impl crate::networking::client_networking::HttpTransport for MockTransport {
    fn execute(
        &self,
        _request: &crate::networking::client_networking::NetworkRequest,
    ) -> HydrusResult<crate::networking::client_networking::NetworkResponse> {
        Ok(crate::networking::client_networking::NetworkResponse {
            status: 200,
            headers: BTreeMap::new(),
            body: self.body.clone(),
        })
    }
}

struct DummyFormula(Vec<String>);

impl ParseFormula for DummyFormula {
    fn parse(
        &self,
        _context: &ParsingContext,
        _text: &str,
        _collapse_newlines: bool,
    ) -> Vec<String> {
        self.0.clone()
    }

    fn name(&self) -> &str {
        "dummy"
    }
}

fn engine() -> NetworkEngine<MockTransport> {
    NetworkEngine::new(MockTransport {
        body: b"page".to_vec(),
    })
}

#[test]
fn watcher_checks_fetch_and_queue_seeds() {
    let mut watcher = WatcherImport::new("https://example.com", "page", 0);
    let formula = DummyFormula(vec!["https://example.com/a.jpg".to_string()]);

    let mut engine = engine();
    assert_eq!(watcher.run_check(&mut engine, &formula, 1000).unwrap(), 1);
    assert_eq!(watcher.get_num_seeds(), 1);
    assert!(!watcher.is_due(1000));
}

#[test]
fn multiple_watchers_run_due_checks_only() {
    let mut multi = MultipleWatcherImport::new("https://example.com");

    let mut due = WatcherImport::new("https://example.com/a", "a", 0);
    due.mark_checked(0);

    let mut paused = WatcherImport::new("https://example.com/b", "b", 0);
    paused.pause_play_checking();

    multi.add_watcher(due);
    multi.add_watcher(paused);

    let formula = DummyFormula(vec!["https://example.com/file.jpg".to_string()]);
    let mut engine = engine();

    assert_eq!(
        multi.run_due_checks(&mut engine, &formula, 10_000).unwrap(),
        1
    );
    assert_eq!(multi.get_num_seeds(), 1);
}

#[test]
fn subscription_queries_update_last_check_time() {
    let mut subscription = Subscription::new("sub", "query");
    let formula = DummyFormula(vec![
        "https://example.com/a.jpg".to_string(),
        "https://example.com/b.jpg".to_string(),
    ]);
    let mut engine = engine();

    assert_eq!(
        subscription
            .run_query(&mut engine, "https://example.com/search", &formula, 5000)
            .unwrap(),
        2
    );
    assert_eq!(subscription.get_num_seeds(), 2);
    assert_eq!(subscription.last_check_time, 5000);
}
