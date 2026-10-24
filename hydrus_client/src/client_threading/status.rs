//! Job status mutation and presentation.

use super::*;

impl JobStatus {
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
