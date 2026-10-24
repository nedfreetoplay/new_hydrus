//! Split out from the parent module.

use super::*;

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
