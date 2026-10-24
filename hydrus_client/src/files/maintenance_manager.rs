//! In-memory maintenance queue from `ClientFilesMaintenanceManager.py`.

use std::collections::BTreeMap;
use std::sync::Mutex;

use hydrus_core::hydrus_time;

use crate::files::maintenance::normalised_weight;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaintenanceJob {
    pub hash: Vec<u8>,
    pub job_type: i32,
    pub time_can_start: i64,
}

#[derive(Debug, Default)]
pub struct FilesMaintenanceManager {
    jobs: Mutex<BTreeMap<Vec<u8>, BTreeMap<i32, i64>>>,
}

impl FilesMaintenanceManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn schedule_job(&self, hash: Vec<u8>, job_type: i32, time_can_start: i64) {
        self.jobs
            .lock()
            .unwrap()
            .entry(hash)
            .or_default()
            .insert(job_type, time_can_start);
    }

    pub fn schedule_job_now(&self, hash: Vec<u8>, job_type: i32) {
        self.schedule_job(hash, job_type, hydrus_time::get_now());
    }

    pub fn cancel_jobs(&self, job_type: i32) {
        let mut jobs = self.jobs.lock().unwrap();

        for hash_jobs in jobs.values_mut() {
            hash_jobs.remove(&job_type);
        }

        jobs.retain(|_, hash_jobs| !hash_jobs.is_empty());
    }

    pub fn clear_jobs(&self, hashes: &[Vec<u8>], job_type: i32) {
        let mut jobs = self.jobs.lock().unwrap();

        for hash in hashes {
            if let Some(hash_jobs) = jobs.get_mut(hash) {
                hash_jobs.remove(&job_type);
            }
        }

        jobs.retain(|_, hash_jobs| !hash_jobs.is_empty());
    }

    pub fn clear_all(&self) {
        self.jobs.lock().unwrap().clear();
    }

    pub fn get_job_counts(&self) -> BTreeMap<i32, i64> {
        let mut counts = BTreeMap::new();

        for hash_jobs in self.jobs.lock().unwrap().values() {
            for job_type in hash_jobs.keys() {
                *counts.entry(*job_type).or_insert(0) += 1;
            }
        }

        counts
    }

    pub fn get_jobs(&self, mandated_job_types: Option<&[i32]>) -> Vec<MaintenanceJob> {
        let now = hydrus_time::get_now();
        let jobs = self.jobs.lock().unwrap();
        let mut due = Vec::new();

        for (hash, hash_jobs) in jobs.iter() {
            for (job_type, time_can_start) in hash_jobs {
                if *time_can_start > now {
                    continue;
                }

                if mandated_job_types.is_some_and(|types| !types.contains(job_type)) {
                    continue;
                }

                due.push(MaintenanceJob {
                    hash: hash.clone(),
                    job_type: *job_type,
                    time_can_start: *time_can_start,
                });
            }
        }

        due.sort_by_key(|job| (job.time_can_start, job.job_type));
        due
    }

    /// Runs the oldest due job and removes it from the queue.
    pub fn run_next<F>(&self, mut run: F) -> Option<MaintenanceJob>
    where
        F: FnMut(&MaintenanceJob) -> bool,
    {
        let job = self.get_jobs(None).into_iter().next()?;

        let succeeded = run(&job);

        if succeeded {
            self.clear_jobs(&[job.hash.clone()], job.job_type);
        }

        Some(job)
    }

    pub fn get_normalised_weight(&self) -> i64 {
        self.get_job_counts()
            .into_iter()
            .map(|(job_type, count)| normalised_weight(job_type) * count)
            .sum()
    }

    pub fn is_empty(&self) -> bool {
        self.jobs.lock().unwrap().is_empty()
    }
}

#[cfg(test)]
mod tests;
