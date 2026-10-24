//! Gallery and simple-URL import lifecycle from `ClientImportGallery.py` and
//! `ClientImportSimpleURLs.py`.

use std::collections::BTreeMap;

use hydrus_core::hydrus_exceptions::{HydrusResult, unknown};

use crate::importing::file_seeds::FileSeed;
use crate::networking::client_networking::{HttpTransport, NetworkEngine};
use crate::networking::client_networking_jobs::NetworkJob;
use crate::parsing::formula::{ParseFormula, ParsingContext};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImportStatus {
    Ready,
    Working,
    Paused,
    Finished,
    Error(String),
}

impl ImportStatus {
    pub fn is_finished(&self) -> bool {
        matches!(self, Self::Finished)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportQueue {
    pub status: ImportStatus,
    pub num_done: i64,
    pub num_to_do: i64,
}

impl Default for ImportQueue {
    fn default() -> Self {
        Self {
            status: ImportStatus::Ready,
            num_done: 0,
            num_to_do: 0,
        }
    }
}

impl ImportQueue {
    pub fn pause_play(&mut self) {
        self.status = match self.status {
            ImportStatus::Paused => ImportStatus::Ready,
            ImportStatus::Ready | ImportStatus::Working => ImportStatus::Paused,
            _ => return,
        };
    }

    pub fn mark_finished(&mut self) {
        self.status = ImportStatus::Finished;
    }

    pub fn set_progress(&mut self, num_done: i64, num_to_do: i64) {
        self.num_done = num_done;
        self.num_to_do = num_to_do;
    }

    pub fn get_value_range(&self) -> (i64, i64) {
        (self.num_done, self.num_to_do)
    }

    pub fn is_paused(&self) -> bool {
        self.status == ImportStatus::Paused
    }

    pub fn is_finished(&self) -> bool {
        self.status.is_finished()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GalleryImport {
    pub query: String,
    pub source_name: String,
    pub initial_search_urls: Vec<String>,
    pub file_queue: ImportQueue,
    pub gallery_queue: ImportQueue,
    pub file_seeds: Vec<FileSeed>,
    pub gallery_seeds: BTreeMap<String, String>,
    pub creation_time: i64,
}

impl GalleryImport {
    pub fn new(
        query: impl Into<String>,
        source_name: impl Into<String>,
        creation_time: i64,
    ) -> Self {
        Self {
            query: query.into(),
            source_name: source_name.into(),
            initial_search_urls: Vec::new(),
            file_queue: ImportQueue::default(),
            gallery_queue: ImportQueue::default(),
            file_seeds: Vec::new(),
            gallery_seeds: BTreeMap::new(),
            creation_time,
        }
    }

    pub fn get_query_text(&self) -> &str {
        &self.query
    }

    pub fn get_source_name(&self) -> &str {
        &self.source_name
    }

    pub fn get_creation_time(&self) -> i64 {
        self.creation_time
    }

    pub fn pause_play_files(&mut self) {
        self.file_queue.pause_play();
    }

    pub fn pause_play_gallery(&mut self) {
        self.gallery_queue.pause_play();
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

    /// Adds a gallery URL to the queue, keyed by URL.
    pub fn add_gallery_seed(&mut self, url: impl Into<String>) {
        let url = url.into();
        self.gallery_seeds.entry(url.clone()).or_insert(url);
    }

    pub fn get_num_seeds(&self) -> usize {
        self.file_seeds.len()
    }

    pub fn get_num_gallery_seeds(&self) -> usize {
        self.gallery_seeds.len()
    }

    pub fn files_finished(&self) -> bool {
        self.file_queue.is_finished()
    }

    pub fn gallery_finished(&self) -> bool {
        self.gallery_queue.is_finished()
    }

    pub fn get_status(&self) -> &ImportStatus {
        if self.file_queue.is_finished() && self.gallery_queue.is_finished() {
            &ImportStatus::Finished
        } else {
            &self.file_queue.status
        }
    }

    /// Fetches the next queued gallery page. Returns the page URL and body.
    pub fn run_next_gallery_page<T: HttpTransport>(
        &mut self,
        engine: &mut NetworkEngine<T>,
    ) -> Option<(String, HydrusResult<String>)> {
        let url = self
            .gallery_seeds
            .keys()
            .next()
            .cloned()
            .or_else(|| self.initial_search_urls.first().cloned())?;

        self.gallery_queue.status = ImportStatus::Working;
        engine.add_job(NetworkJob::get(url.clone()));

        let job = engine.run_next()?;

        let result = match job.get_response() {
            Some(body) => Ok(String::from_utf8_lossy(body).into_owned()),
            None => Err(unknown(
                job.error
                    .unwrap_or_else(|| "gallery request failed".to_string()),
            )),
        };

        self.gallery_seeds.remove(&url);
        self.gallery_queue.mark_finished();

        Some((url, result))
    }

    /// Parses a gallery page and queues every returned URL as a file seed.
    pub fn apply_gallery_formula(
        &mut self,
        formula: &dyn ParseFormula,
        text: &str,
        created: i64,
    ) -> usize {
        let mut added = 0;

        for url in formula.parse(&ParsingContext::default(), text, true) {
            let before = self.get_num_seeds();
            self.add_file_seed(FileSeed::url(url, created));

            if self.get_num_seeds() > before {
                added += 1;
            }
        }

        added
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SimpleDownloaderJob {
    pub url: String,
    pub status: ImportStatus,
}

impl SimpleDownloaderJob {
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            status: ImportStatus::Ready,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SimpleDownloaderImport {
    pub formula_name: String,
    pub jobs: Vec<SimpleDownloaderJob>,
    pub current_job: Option<usize>,
    pub file_seeds: Vec<FileSeed>,
}

impl SimpleDownloaderImport {
    pub fn new(formula_name: impl Into<String>) -> Self {
        Self {
            formula_name: formula_name.into(),
            jobs: Vec::new(),
            current_job: None,
            file_seeds: Vec::new(),
        }
    }

    pub fn get_formula_name(&self) -> &str {
        &self.formula_name
    }

    pub fn set_formula_name(&mut self, name: impl Into<String>) {
        self.formula_name = name.into();
    }

    pub fn set_pending_jobs(&mut self, urls: impl IntoIterator<Item = String>) {
        self.jobs = urls.into_iter().map(SimpleDownloaderJob::new).collect();
        self.current_job = None;
    }

    pub fn pend_job(&mut self, url: impl Into<String>) {
        self.jobs.push(SimpleDownloaderJob::new(url));
    }

    pub fn advance_job(&mut self) -> Option<&SimpleDownloaderJob> {
        let index = self
            .jobs
            .iter()
            .position(|job| job.status == ImportStatus::Ready)?;
        self.jobs[index].status = ImportStatus::Working;
        self.current_job = Some(index);
        self.jobs.get(index)
    }

    pub fn delay_job(&mut self) {
        if let Some(index) = self.current_job {
            self.jobs[index].status = ImportStatus::Ready;
            self.current_job = None;
        }
    }

    pub fn delete_job(&mut self) {
        if let Some(index) = self.current_job.take() {
            self.jobs.remove(index);
        }
    }

    pub fn add_file_seed(&mut self, seed: FileSeed) {
        self.file_seeds.push(seed);
    }

    pub fn get_num_seeds(&self) -> usize {
        self.file_seeds.len()
    }

    pub fn get_num_pending_jobs(&self) -> usize {
        self.jobs
            .iter()
            .filter(|job| job.status == ImportStatus::Ready)
            .count()
    }

    pub fn get_status(&self) -> ImportStatus {
        if self.jobs.is_empty() || self.jobs.iter().all(|job| job.status.is_finished()) {
            ImportStatus::Finished
        } else if self
            .jobs
            .iter()
            .any(|job| job.status == ImportStatus::Working)
        {
            ImportStatus::Working
        } else {
            ImportStatus::Ready
        }
    }

    /// Fetches the next job's page. The job is marked finished on success.
    pub fn run_next_job<T: HttpTransport>(
        &mut self,
        engine: &mut NetworkEngine<T>,
    ) -> Option<(String, HydrusResult<String>)> {
        let index = self
            .jobs
            .iter()
            .position(|job| job.status == ImportStatus::Ready)?;

        let url = self.jobs[index].url.clone();
        self.jobs[index].status = ImportStatus::Working;
        self.current_job = Some(index);
        engine.add_job(NetworkJob::get(url.clone()));

        let job = engine.run_next()?;

        let result = match job.get_response() {
            Some(body) => {
                self.jobs[index].status = ImportStatus::Finished;
                Ok(String::from_utf8_lossy(body).into_owned())
            }
            None => {
                self.jobs[index].status = ImportStatus::Error(
                    job.error
                        .clone()
                        .unwrap_or_else(|| "simple download failed".to_string()),
                );
                Err(unknown(
                    job.error
                        .unwrap_or_else(|| "simple download failed".to_string()),
                ))
            }
        };

        self.current_job = None;

        Some((url, result))
    }

    /// Parses a fetched page and queues the returned file URLs.
    pub fn apply_downloader_formula(
        &mut self,
        formula: &dyn ParseFormula,
        text: &str,
        created: i64,
    ) -> usize {
        let urls = formula.parse(&ParsingContext::default(), text, true);

        for url in &urls {
            self.add_file_seed(FileSeed::url(url.clone(), created));
        }

        urls.len()
    }
}

#[cfg(test)]
mod tests;
