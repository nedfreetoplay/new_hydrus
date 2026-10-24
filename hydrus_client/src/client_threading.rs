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

    pub fn set_status_title(&self, title: impl Into<String>) {
        self.inner.titles.lock().unwrap().push(title.into());
    }

    pub fn get_status_title(&self) -> Option<String> {
        self.inner.titles.lock().unwrap().last().cloned()
    }

    pub fn delete_status_title(&self) {
        self.inner.titles.lock().unwrap().pop();
    }

    pub fn set_status_text(&self, text: impl Into<String>) {
        self.inner.texts.lock().unwrap().push(text.into());
    }

    pub fn get_status_text(&self) -> Option<String> {
        self.inner.texts.lock().unwrap().last().cloned()
    }

    pub fn delete_status_text(&self) {
        self.inner.texts.lock().unwrap().pop();
    }

    pub fn set_gauge(&self, num_done: Option<i64>, num_to_do: Option<i64>) {
        self.inner
            .gauges
            .lock()
            .unwrap()
            .push((num_done.unwrap_or(-1), num_to_do.unwrap_or(-1)));
    }

    pub fn get_gauge(&self) -> Option<(Option<i64>, Option<i64>)> {
        self.inner
            .gauges
            .lock()
            .unwrap()
            .last()
            .map(|(done, total)| {
                (
                    (*done >= 0).then_some(*done),
                    (*total >= 0).then_some(*total),
                )
            })
    }

    pub fn add_url(&self, url: impl Into<String>) {
        self.inner.urls.lock().unwrap().push(url.into());
    }

    pub fn get_urls(&self) -> Vec<String> {
        self.inner.urls.lock().unwrap().clone()
    }

    pub fn set_files(&self, hashes: Vec<Vec<u8>>) {
        *self.inner.files.lock().unwrap() = hashes;
    }

    pub fn get_files(&self) -> Vec<Vec<u8>> {
        self.inner.files.lock().unwrap().clone()
    }

    pub fn set_variable(&self, name: impl Into<String>, value: impl Into<String>) {
        self.inner
            .variables
            .lock()
            .unwrap()
            .insert(name.into(), value.into());
    }

    pub fn get_if_has_variable(&self, name: &str) -> Option<String> {
        self.inner.variables.lock().unwrap().get(name).cloned()
    }

    pub fn set_error(&self, error: impl Into<String>) {
        *self.inner.error.lock().unwrap() = Some(error.into());
        self.finish();
    }

    pub fn had_error(&self) -> bool {
        self.inner.error.lock().unwrap().is_some()
    }

    pub fn get_error(&self) -> Option<String> {
        self.inner.error.lock().unwrap().clone()
    }

    pub fn finish(&self) {
        let mut finished = self.inner.finished.lock().unwrap();
        if finished.is_none() {
            *finished = Some(Instant::now());
        }
        drop(finished);

        let mut done = self.inner.done.lock().unwrap();
        *done = true;
        self.inner.done_signal.notify_all();
    }

    pub fn finish_and_dismiss(&self) {
        self.finish();
    }

    pub fn is_done(&self) -> bool {
        *self.inner.done.lock().unwrap()
    }

    pub fn wait_if_needed(&self) {
        while self.is_paused() && !self.is_cancelled() {
            std::thread::sleep(Duration::from_millis(50));
        }
    }

    pub fn wait_until_done(&self) {
        let mut done = self.inner.done.lock().unwrap();
        while !*done {
            done = self.inner.done_signal.wait(done).unwrap();
        }
    }

    /// `ToString()`
    pub fn to_string(&self) -> String {
        let mut lines = Vec::new();

        if let Some(title) = self.get_status_title() {
            lines.push(title);
        }

        if let Some(text) = self.get_status_text() {
            lines.push(text);
        }

        if let Some((done, total)) = self.get_gauge() {
            match (done, total) {
                (Some(done), Some(total)) => lines.push(format!("{done}/{total}")),
                (Some(done), None) => lines.push(done.to_string()),
                _ => {}
            }
        }

        for url in self.get_urls() {
            lines.push(url);
        }

        if let Some(error) = self.get_error() {
            lines.push(format!("ERROR: {error}"));
        }

        lines.join("\n")
    }
}

pub type ThreadCall = Box<dyn FnOnce() + Send>;

#[cfg(test)]
mod tests;
