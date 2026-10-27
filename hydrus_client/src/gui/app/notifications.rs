//! Recent popup messages, mirroring Hydrus' popup message manager.
//!
//! Notifications are transient in the Qt status bar; this keeps a bounded
//! history so the user can review what happened after the message disappeared.

use hydrus_core::hydrus_time;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Notification {
    pub timestamp: i64,
    pub level: String,
    pub text: String,
}

pub const LIMIT: usize = 200;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NotificationLog {
    entries: Vec<Notification>,
}

impl NotificationLog {
    pub fn push(&mut self, level: impl Into<String>, text: impl Into<String>) {
        let text = text.into();

        if text.trim().is_empty() {
            return;
        }

        self.entries.push(Notification {
            timestamp: hydrus_time::get_now(),
            level: level.into(),
            text,
        });

        if self.entries.len() > LIMIT {
            let excess = self.entries.len() - LIMIT;
            self.entries.drain(0..excess);
        }
    }

    pub fn entries(&self) -> &[Notification] {
        &self.entries
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

#[cfg(test)]
mod tests;
