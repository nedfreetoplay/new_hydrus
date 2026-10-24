//! Subscription imports.

use super::watchers::fetch_page;
use super::*;

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
