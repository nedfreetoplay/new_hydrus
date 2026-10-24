//! Watchers and subscriptions from `ClientImportWatchers.py` and
//! `ClientImportSubscriptions.py`.

use std::collections::BTreeMap;

use hydrus_core::hydrus_exceptions::{HydrusResult, unknown};

use crate::client_defaults::CheckerOptions;
use crate::importing::file_seeds::FileSeed;
use crate::networking::client_networking::{HttpTransport, NetworkEngine};
use crate::networking::client_networking_jobs::NetworkJob;
use crate::parsing::formula::{ParseFormula, ParsingContext};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WatcherImport {
    pub url: String,
    pub subject: String,
    pub checker_options: CheckerOptions,
    pub paused: bool,
    pub last_check_time: i64,
    pub creation_time: i64,
    pub file_seeds: Vec<FileSeed>,
    pub highlighted: bool,
}

impl WatcherImport {
    pub fn new(url: impl Into<String>, subject: impl Into<String>, creation_time: i64) -> Self {
        Self {
            url: url.into(),
            subject: subject.into(),
            checker_options: CheckerOptions {
                intended_files_per_check: 4,
                never_faster_than: 300,
                never_slower_than: 86_400,
                death_file_velocity: (1, 3 * 86_400),
            },
            paused: false,
            last_check_time: 0,
            creation_time,
            file_seeds: Vec::new(),
            highlighted: false,
        }
    }

    pub fn get_url(&self) -> &str {
        &self.url
    }

    pub fn get_subject(&self) -> &str {
        &self.subject
    }

    pub fn get_creation_time(&self) -> i64 {
        self.creation_time
    }

    pub fn get_next_check_time(&self) -> i64 {
        self.last_check_time + self.checker_options.never_faster_than
    }

    pub fn is_due(&self, now: i64) -> bool {
        !self.paused && now >= self.get_next_check_time()
    }

    pub fn pause_play_checking(&mut self) {
        self.paused = !self.paused;
    }

    pub fn mark_checked(&mut self, now: i64) {
        self.last_check_time = now;
    }

    pub fn add_file_seed(&mut self, seed: FileSeed) {
        if !self
            .file_seeds
            .iter()
            .any(|existing| existing.file_seed_data == seed.file_seed_data)
        {
            self.file_seeds.push(seed);
        }
    }

    pub fn get_num_seeds(&self) -> usize {
        self.file_seeds.len()
    }

    pub fn set_highlighted(&mut self, highlighted: bool) {
        self.highlighted = highlighted;
    }

    /// Fetches the watched URL and queues any URLs found by the formula.
    pub fn run_check<T: HttpTransport>(
        &mut self,
        engine: &mut NetworkEngine<T>,
        formula: &dyn ParseFormula,
        now: i64,
    ) -> HydrusResult<usize> {
        let page = fetch_page(engine, &self.url)?;
        let before = self.get_num_seeds();

        for url in formula.parse(&ParsingContext::default(), &page, true) {
            self.add_file_seed(FileSeed::url(url, now));
        }

        self.mark_checked(now);

        Ok(self.get_num_seeds() - before)
    }
}

fn fetch_page<T: HttpTransport>(engine: &mut NetworkEngine<T>, url: &str) -> HydrusResult<String> {
    engine.add_job(NetworkJob::get(url.to_string()));

    let job = engine
        .run_next()
        .ok_or_else(|| unknown("network engine did not run the job"))?;

    job.get_response()
        .map(|body| String::from_utf8_lossy(body).into_owned())
        .ok_or_else(|| unknown(job.error.unwrap_or_else(|| "page fetch failed".to_string())))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MultipleWatcherImport {
    pub url: String,
    pub watchers: Vec<WatcherImport>,
    pub highlighted_watcher: Option<String>,
}

impl MultipleWatcherImport {
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            watchers: Vec::new(),
            highlighted_watcher: None,
        }
    }

    pub fn add_watcher(&mut self, watcher: WatcherImport) {
        self.watchers.push(watcher);
    }

    pub fn get_watchers(&self) -> &[WatcherImport] {
        &self.watchers
    }

    pub fn get_num_watchers(&self) -> usize {
        self.watchers.len()
    }

    pub fn get_num_seeds(&self) -> usize {
        self.watchers.iter().map(WatcherImport::get_num_seeds).sum()
    }

    pub fn set_highlighted_watcher(&mut self, subject: Option<String>) {
        self.highlighted_watcher = subject;
    }

    pub fn get_highlighted_watcher(&self) -> Option<&str> {
        self.highlighted_watcher.as_deref()
    }

    /// Runs every due watcher and returns the number of newly queued seeds.
    pub fn run_due_checks<T: HttpTransport>(
        &mut self,
        engine: &mut NetworkEngine<T>,
        formula: &dyn ParseFormula,
        now: i64,
    ) -> HydrusResult<usize> {
        let mut added = 0;

        for watcher in &mut self.watchers {
            if watcher.is_due(now) {
                added += watcher.run_check(engine, formula, now)?;
            }
        }

        Ok(added)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Subscription {
    pub name: String,
    pub query: String,
    pub checker_options: CheckerOptions,
    pub paused: bool,
    pub last_check_time: i64,
    pub no_work_until: i64,
    pub initial_file_limit: Option<i64>,
    pub periodic_file_limit: Option<i64>,
    pub file_seeds: Vec<FileSeed>,
}

impl Subscription {
    pub fn new(name: impl Into<String>, query: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            query: query.into(),
            checker_options: CheckerOptions {
                intended_files_per_check: 4,
                never_faster_than: 86_400,
                never_slower_than: 90 * 86_400,
                death_file_velocity: (1, 180 * 86_400),
            },
            paused: false,
            last_check_time: 0,
            no_work_until: 0,
            initial_file_limit: None,
            periodic_file_limit: None,
            file_seeds: Vec::new(),
        }
    }

    pub fn get_name(&self) -> &str {
        &self.name
    }

    pub fn get_query(&self) -> &str {
        &self.query
    }

    pub fn get_best_earliest_next_work_time(&self) -> i64 {
        self.no_work_until
            .max(self.last_check_time + self.checker_options.never_faster_than)
    }

    pub fn is_due(&self, now: i64) -> bool {
        !self.paused && now >= self.get_best_earliest_next_work_time()
    }

    pub fn pause_resume(&mut self) {
        self.paused = !self.paused;
    }

    pub fn set_paused(&mut self, paused: bool) {
        self.paused = paused;
    }

    pub fn set_no_work_until(&mut self, timestamp: i64) {
        self.no_work_until = timestamp;
    }

    pub fn set_file_limits(&mut self, initial: Option<i64>, periodic: Option<i64>) {
        self.initial_file_limit = initial;
        self.periodic_file_limit = periodic;
    }

    pub fn add_file_seed(&mut self, seed: FileSeed) {
        self.file_seeds.push(seed);
    }

    pub fn get_num_seeds(&self) -> usize {
        self.file_seeds.len()
    }

    /// Runs the subscription's next query against an explicit gallery URL.
    pub fn run_query<T: HttpTransport>(
        &mut self,
        engine: &mut NetworkEngine<T>,
        url: &str,
        formula: &dyn ParseFormula,
        now: i64,
    ) -> HydrusResult<usize> {
        let page = fetch_page(engine, url)?;
        let urls = formula.parse(&ParsingContext::default(), &page, true);

        for url in &urls {
            self.add_file_seed(FileSeed::url(url.clone(), now));
        }

        self.last_check_time = now;

        Ok(urls.len())
    }
}

#[derive(Debug, Default)]
pub struct SubscriptionsManager {
    subscriptions: BTreeMap<String, Subscription>,
}

impl SubscriptionsManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_subscription(&mut self, subscription: Subscription) {
        self.subscriptions
            .insert(subscription.name.clone(), subscription);
    }

    pub fn remove_subscription(&mut self, name: &str) -> bool {
        self.subscriptions.remove(name).is_some()
    }

    pub fn get_subscription(&self, name: &str) -> Option<&Subscription> {
        self.subscriptions.get(name)
    }

    pub fn get_subscription_mut(&mut self, name: &str) -> Option<&mut Subscription> {
        self.subscriptions.get_mut(name)
    }

    pub fn get_subscriptions(&self) -> Vec<&Subscription> {
        self.subscriptions.values().collect()
    }

    pub fn set_subscriptions(&mut self, subscriptions: Vec<Subscription>) {
        self.subscriptions = subscriptions
            .into_iter()
            .map(|subscription| (subscription.name.clone(), subscription))
            .collect();
    }

    pub fn get_due_subscription(&self, now: i64) -> Option<&Subscription> {
        self.subscriptions
            .values()
            .filter(|subscription| subscription.is_due(now))
            .min_by_key(|subscription| subscription.get_best_earliest_next_work_time())
    }

    pub fn pause_all_for_editing(&mut self) {
        for subscription in self.subscriptions.values_mut() {
            subscription.set_paused(true);
        }
    }

    pub fn len(&self) -> usize {
        self.subscriptions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.subscriptions.is_empty()
    }
}

#[cfg(test)]
mod tests {
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
}
