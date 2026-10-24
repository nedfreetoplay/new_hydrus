//! Popup message queue from `hydrus/client/gui/ClientGUIPopupMessages.py`.

use std::collections::BTreeMap;

use hydrus_core::hydrus_time;

use crate::client_threading::JobStatus;

#[derive(Debug, Clone)]
pub struct PopupMessage {
    pub job_status: JobStatus,
    pub creation_time: f64,
    pub dismissed: bool,
    pub dismissed_time: Option<f64>,
}

impl PopupMessage {
    pub fn new(job_status: JobStatus) -> Self {
        Self {
            job_status,
            creation_time: hydrus_time::get_now_float(),
            dismissed: false,
            dismissed_time: None,
        }
    }

    pub fn message_key(&self) -> i64 {
        self.job_status.get_key()
    }

    pub fn dismiss(&mut self) {
        self.dismissed = true;
        self.dismissed_time = Some(hydrus_time::get_now_float());
    }

    pub fn is_done(&self) -> bool {
        self.job_status.is_done()
    }
}

#[derive(Debug, Default)]
pub struct PopupMessageManager {
    messages: BTreeMap<i64, PopupMessage>,
}

impl PopupMessageManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_message(&mut self, message: PopupMessage) {
        self.messages.insert(message.message_key(), message);
    }

    pub fn add_job(&mut self, job_status: JobStatus) -> i64 {
        let key = job_status.get_key();
        self.add_message(PopupMessage::new(job_status));
        key
    }

    pub fn get_message(&self, key: i64) -> Option<&PopupMessage> {
        self.messages.get(&key)
    }

    pub fn get_message_mut(&mut self, key: i64) -> Option<&mut PopupMessage> {
        self.messages.get_mut(&key)
    }

    pub fn dismiss_message(&mut self, key: i64) -> bool {
        if let Some(message) = self.messages.get_mut(&key) {
            message.dismiss();
            true
        } else {
            false
        }
    }

    pub fn remove_message(&mut self, key: i64) -> Option<PopupMessage> {
        self.messages.remove(&key)
    }

    pub fn clear_dismissed(&mut self) {
        self.messages.retain(|_, message| !message.dismissed);
    }

    pub fn get_dismissed_messages(&self) -> Vec<&PopupMessage> {
        self.messages
            .values()
            .filter(|message| message.dismissed)
            .collect()
    }

    pub fn get_active_messages(&self) -> Vec<&PopupMessage> {
        let mut messages: Vec<&PopupMessage> = self
            .messages
            .values()
            .filter(|message| !message.dismissed)
            .collect();

        messages.sort_by(|a, b| {
            a.creation_time
                .partial_cmp(&b.creation_time)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        messages
    }

    pub fn len(&self) -> usize {
        self.messages.len()
    }

    pub fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn added_jobs_are_queryable_by_key() {
        let mut manager = PopupMessageManager::new();
        let job = JobStatus::new(false, false);
        let key = job.get_key();

        manager.add_job(job);

        assert!(manager.get_message(key).is_some());
        assert_eq!(manager.len(), 1);
    }

    #[test]
    fn dismissing_keeps_the_message_until_cleared() {
        let mut manager = PopupMessageManager::new();
        let key = manager.add_job(JobStatus::new(false, false));

        assert!(manager.dismiss_message(key));
        assert_eq!(manager.get_dismissed_messages().len(), 1);

        manager.clear_dismissed();

        assert!(manager.is_empty());
    }

    #[test]
    fn active_messages_are_creation_ordered() {
        let mut manager = PopupMessageManager::new();

        let first = JobStatus::new(false, false);
        let second = JobStatus::new(false, false);
        let mut first_message = PopupMessage::new(first);
        let mut second_message = PopupMessage::new(second);
        first_message.creation_time = 2.0;
        second_message.creation_time = 1.0;
        manager.add_message(first_message);
        manager.add_message(second_message);

        let active = manager.get_active_messages();

        assert!(active[0].creation_time <= active[1].creation_time);
    }

    #[test]
    fn removing_returns_the_message() {
        let mut manager = PopupMessageManager::new();
        let key = manager.add_job(JobStatus::new(false, false));

        assert_eq!(manager.remove_message(key).unwrap().message_key(), key);
        assert!(manager.is_empty());
    }
}
