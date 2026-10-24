//! Pub/sub bridge to the Qt thread (`ClientController.App`/`PubSubEventCatcher`).

use std::collections::VecDeque;
use std::sync::{Mutex, OnceLock};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PubSubEvent {
    pub topic: String,
    pub message: String,
}

#[derive(Debug, Default)]
pub struct PubSubEventCatcher {
    queue: Mutex<VecDeque<PubSubEvent>>,
}

impl PubSubEventCatcher {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn post(&self, topic: impl Into<String>, message: impl Into<String>) {
        self.queue.lock().unwrap().push_back(PubSubEvent {
            topic: topic.into(),
            message: message.into(),
        });
    }

    pub fn has_events(&self) -> bool {
        !self.queue.lock().unwrap().is_empty()
    }

    pub fn len(&self) -> usize {
        self.queue.lock().unwrap().len()
    }

    pub fn drain(&self) -> Vec<PubSubEvent> {
        self.queue.lock().unwrap().drain(..).collect()
    }

    pub fn drain_topic(&self, topic: &str) -> Vec<PubSubEvent> {
        let mut queue = self.queue.lock().unwrap();
        let mut matching = Vec::new();
        let mut remaining = VecDeque::new();

        for event in queue.drain(..) {
            if event.topic == topic {
                matching.push(event);
            } else {
                remaining.push_back(event);
            }
        }

        *queue = remaining;

        matching
    }

    pub fn clear(&self) {
        self.queue.lock().unwrap().clear();
    }
}

static GLOBAL_CATCHER: OnceLock<PubSubEventCatcher> = OnceLock::new();

pub fn global() -> &'static PubSubEventCatcher {
    GLOBAL_CATCHER.get_or_init(PubSubEventCatcher::new)
}

pub fn post(topic: impl Into<String>, message: impl Into<String>) {
    global().post(topic, message);
}

/// Called by the GUI/event-loop integration when the Qt side is ready.
pub fn drain() -> Vec<PubSubEvent> {
    global().drain()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn events_are_fifo() {
        let catcher = PubSubEventCatcher::new();
        catcher.post("message", "first");
        catcher.post("message", "second");

        let events = catcher.drain();

        assert_eq!(events.len(), 2);
        assert_eq!(events[0].message, "first");
        assert_eq!(events[1].message, "second");
        assert!(!catcher.has_events());
    }

    #[test]
    fn drain_topic_keeps_other_events() {
        let catcher = PubSubEventCatcher::new();
        catcher.post("a", "1");
        catcher.post("b", "2");
        catcher.post("a", "3");

        let events = catcher.drain_topic("a");

        assert_eq!(events.len(), 2);
        assert_eq!(catcher.len(), 1);
        assert_eq!(catcher.drain()[0].topic, "b");
    }

    #[test]
    fn clear_removes_everything() {
        let catcher = PubSubEventCatcher::new();
        catcher.post("x", "y");
        catcher.clear();

        assert!(!catcher.has_events());
    }
}
