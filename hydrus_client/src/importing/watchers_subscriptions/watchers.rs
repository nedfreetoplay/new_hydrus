//! Watcher imports.

use super::*;

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

pub(super) fn fetch_page<T: HttpTransport>(
    engine: &mut NetworkEngine<T>,
    url: &str,
) -> HydrusResult<String> {
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
