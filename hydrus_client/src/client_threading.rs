//! Job status primitives from `hydrus/client/ClientThreading.py`.

use std::collections::HashMap;
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

use hydrus_core::hydrus_time;

#[derive(Debug, Clone)]
pub struct JobStatus {
    inner: Arc<Inner>,
}

#[derive(Debug)]
struct Inner {
    key: i64,
    created: Instant,
    created_timestamp: f64,
    finished: Mutex<Option<Instant>>,
    texts: Mutex<Vec<String>>,
    titles: Mutex<Vec<String>>,
    gauges: Mutex<Vec<(i64, i64)>>,
    urls: Mutex<Vec<String>>,
    files: Mutex<Vec<Vec<u8>>>,
    variables: Mutex<HashMap<String, String>>,
    cancelled: Mutex<bool>,
    paused: Mutex<bool>,
    pausable: bool,
    cancellable: bool,
    error: Mutex<Option<String>>,
    done: Mutex<bool>,
    done_signal: Condvar,
}

impl JobStatus {
    pub fn new(pausable: bool, cancellable: bool) -> Self {
        Self {
            inner: Arc::new(Inner {
                key: rand::random(),
                created: Instant::now(),
                created_timestamp: hydrus_time::get_now_float(),
                finished: Mutex::new(None),
                texts: Mutex::new(Vec::new()),
                titles: Mutex::new(Vec::new()),
                gauges: Mutex::new(Vec::new()),
                urls: Mutex::new(Vec::new()),
                files: Mutex::new(Vec::new()),
                variables: Mutex::new(HashMap::new()),
                cancelled: Mutex::new(false),
                paused: Mutex::new(false),
                pausable,
                cancellable,
                error: Mutex::new(None),
                done: Mutex::new(false),
                done_signal: Condvar::new(),
            }),
        }
    }

    pub fn get_key(&self) -> i64 {
        self.inner.key
    }

    pub fn get_creation_time(&self) -> f64 {
        self.inner.created_timestamp
    }

    pub fn time_running(&self) -> f64 {
        self.inner.created.elapsed().as_secs_f64()
    }

    pub fn is_cancellable(&self) -> bool {
        self.inner.cancellable
    }

    pub fn is_pausable(&self) -> bool {
        self.inner.pausable
    }

    pub fn is_cancelled(&self) -> bool {
        *self.inner.cancelled.lock().unwrap()
    }

    pub fn cancel(&self) {
        *self.inner.cancelled.lock().unwrap() = true;
        self.finish();
    }

    pub fn is_paused(&self) -> bool {
        *self.inner.paused.lock().unwrap()
    }

    pub fn pause_play(&self) {
        if self.inner.pausable {
            let mut paused = self.inner.paused.lock().unwrap();
            *paused = !*paused;
        }
    }
}

pub type ThreadCall = Box<dyn FnOnce() + Send>;

mod status;

#[cfg(test)]
mod tests;
