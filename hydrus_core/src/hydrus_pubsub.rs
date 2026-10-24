//! Rust port of `hydrus/core/HydrusPubSub.py`.
//!
//! Python's pub/sub keys subscribers by `(topic, method_name)` using weak
//! references, then calls `getattr( obj, method_name )`. Rust cannot reflect
//! over method names, so a subscriber implements [`Subscriber`] and dispatches
//! the name itself. Everything else -- the weak registry, the queued vs.
//! immediate publish, the idle event and the shutdown escape -- is preserved.
//!
//! Message payloads are `serde_json::Value` argument lists plus a key/value
//! object, so one queue can carry every topic while subscribers decode what
//! they expect.

use std::any::Any;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Condvar, Mutex, Weak};

use serde_json::{Map, Value};

use crate::hydrus_exceptions::{ErrorKind, HydrusResult};
use crate::hydrus_globals;
use crate::hydrus_profiling;

/// One published call: a topic plus positional and keyword arguments.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PubSubMessage {
    pub topic: String,
    pub args: Vec<Value>,
    pub kwargs: Map<String, Value>,
}

impl PubSubMessage {
    pub fn new(topic: impl Into<String>) -> Self {
        Self {
            topic: topic.into(),
            ..Self::default()
        }
    }

    pub fn with_args(topic: impl Into<String>, args: Vec<Value>) -> Self {
        Self {
            topic: topic.into(),
            args,
            ..Self::default()
        }
    }

    pub fn arg(&self, index: usize) -> Option<&Value> {
        self.args.get(index)
    }

    pub fn kwarg(&self, name: &str) -> Option<&Value> {
        self.kwargs.get(name)
    }
}

/// The Rust replacement for duck-typed `getattr( obj, method_name )`.
///
/// `has_method` mirrors `hasattr`; `call` performs the dispatch. Returning a
/// `Shutdown` error stops the current `process` batch, exactly like Python's
/// `except ShutdownException: return False`.
///
/// The `Any` supertrait lets callers (for example a `valid_callable` that
/// accepts only the controller) downcast the trait object to a concrete type.
pub trait Subscriber: Any + Send + Sync {
    fn has_method(&self, method_name: &str) -> bool;

    fn call(&self, method_name: &str, message: &PubSubMessage) -> HydrusResult<()>;
}

type ValidCallable = Box<dyn Fn(&dyn Subscriber) -> bool + Send + Sync>;

#[derive(Default)]
struct TopicSubscribers {
    objects: Vec<Weak<dyn Subscriber>>,
    method_names: HashSet<String>,
}

#[derive(Default)]
struct PubSubState {
    queue: Vec<PubSubMessage>,
    topics: HashMap<String, TopicSubscribers>,
    shutdown: bool,
    idle: bool,
}

/// `HydrusPubSub`.
pub struct PubSub {
    state: Mutex<PubSubState>,
    received_job_event: Condvar,
    idle_event: Condvar,
    valid_callable: ValidCallable,
}

impl Default for PubSub {
    fn default() -> Self {
        Self::new()
    }
}

impl PubSub {
    /// `HydrusPubSub( valid_callable = lambda o: True )`
    pub fn new() -> Self {
        Self::with_valid_callable(|_| true)
    }

    /// `HydrusPubSub( valid_callable )`
    pub fn with_valid_callable<F>(valid_callable: F) -> Self
    where
        F: Fn(&dyn Subscriber) -> bool + Send + Sync + 'static,
    {
        Self {
            state: Mutex::new(PubSubState {
                idle: true,
                ..PubSubState::default()
            }),
            received_job_event: Condvar::new(),
            idle_event: Condvar::new(),
            valid_callable: Box::new(valid_callable),
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, PubSubState> {
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// The `(object, method_name)` pairs for a topic, with expired weak refs
    /// and invalid objects dropped. Called without the state lock held so the
    /// valid callable may itself use the pub/sub.
    fn callable_tuples(&self, topic: &str) -> Vec<(Arc<dyn Subscriber>, String)> {
        let (objects, method_names) = {
            let state = self.lock();

            match state.topics.get(topic) {
                Some(topic_subscribers) => (
                    topic_subscribers.objects.clone(),
                    topic_subscribers
                        .method_names
                        .iter()
                        .cloned()
                        .collect::<Vec<_>>(),
                ),
                None => return Vec::new(),
            }
        };

        let mut callable_tuples = Vec::new();

        for weak in objects {
            let Some(object) = weak.upgrade() else {
                continue;
            };

            if !(self.valid_callable)(object.as_ref()) {
                continue;
            }

            for method_name in &method_names {
                if object.has_method(method_name) {
                    callable_tuples.push((object.clone(), method_name.clone()));
                }
            }
        }

        callable_tuples
    }

    fn call_one(
        &self,
        object: &Arc<dyn Subscriber>,
        method_name: &str,
        message: &PubSubMessage,
    ) -> HydrusResult<()> {
        if hydrus_profiling::is_profile_mode("ui") && message.topic != "message" {
            let summary = format!("Profiling pubsub: {}", message.topic);
            let result: Mutex<Option<HydrusResult<()>>> = Mutex::new(None);

            hydrus_profiling::profile(
                &summary,
                || {
                    *result.lock().unwrap() = Some(object.call(method_name, message));
                },
                hydrus_globals::pubsub_profile_min_job_time_ms(),
                false,
            );

            result.into_inner().unwrap().unwrap_or(Ok(()))
        } else {
            object.call(method_name, message)
        }
    }

    /// `currently_idle`
    pub fn currently_idle(&self) -> bool {
        self.lock().idle
    }

    /// `process`. Returns `false` if a subscriber raised a shutdown exception.
    pub fn process(&self) -> bool {
        let messages = {
            let mut state = self.lock();

            if state.queue.is_empty() {
                return true;
            }

            // Only take one batch, like Python: callables registered by this
            // batch's handlers are not used until the next call.
            std::mem::take(&mut state.queue)
        };

        let mut okay = true;

        for message in messages {
            if !okay {
                break;
            }

            let not_a_report = message.topic != "message";

            if hydrus_globals::pubsub_report_mode() && not_a_report {
                crate::hydrus_data::safe_print(format!(
                    "pubsub: {:?} {:?} {:?}",
                    message.topic, message.args, message.kwargs
                ));
            }

            for (object, method_name) in self.callable_tuples(&message.topic) {
                match self.call_one(&object, &method_name, &message) {
                    Ok(()) => {}
                    Err(e) if e.is_a(ErrorKind::Shutdown) => {
                        okay = false;

                        break;
                    }
                    Err(e) => {
                        crate::hydrus_data::safe_print(crate::hydrus_data::print_exception(
                            &e, false,
                        ));
                    }
                }
            }
        }

        {
            let mut state = self.lock();

            state.idle = true;
        }

        self.idle_event.notify_all();

        okay
    }

    /// `pub`. Named `publish` because `pub` is a Rust keyword.
    pub fn publish(&self, message: PubSubMessage) {
        {
            let mut state = self.lock();

            if state.shutdown {
                drop(state);

                self.publish_immediate(message);

                return;
            }

            state.queue.push(message);
            state.idle = false;
        }

        self.received_job_event.notify_all();
    }

    /// `pubimmediate`
    pub fn publish_immediate(&self, message: PubSubMessage) {
        for (object, method_name) in self.callable_tuples(&message.topic) {
            // Python lets exceptions from an immediate publish propagate.
            let _ = self.call_one(&object, &method_name, &message);
        }
    }

    /// `shutdown`
    pub fn shutdown(&self) {
        {
            let mut state = self.lock();

            state.shutdown = true;
            state.idle = true;
        }

        self.received_job_event.notify_all();
        self.idle_event.notify_all();
    }

    /// `sub`
    pub fn sub<T>(&self, object: &Arc<T>, method_name: impl Into<String>, topic: impl Into<String>)
    where
        T: Subscriber + 'static,
    {
        let object = Arc::clone(object);
        let object: Arc<dyn Subscriber> = object;

        self.sub_dyn(&object, method_name, topic);
    }

    /// `sub` for an already type-erased subscriber.
    ///
    /// Python accepts any object; the generics on [`PubSub::sub`] need a sized
    /// type, so callers that only have a `dyn Subscriber` (the controller's
    /// `ThreadingController::subscribe` hook) use this instead.
    pub fn sub_dyn(
        &self,
        object: &Arc<dyn Subscriber>,
        method_name: impl Into<String>,
        topic: impl Into<String>,
    ) {
        let topic = topic.into();
        let method_name = method_name.into();

        let mut state = self.lock();

        let topic_subscribers = state.topics.entry(topic).or_default();

        let already_subscribed = topic_subscribers.objects.iter().any(|weak| {
            std::ptr::eq(
                Weak::as_ptr(weak).cast::<()>(),
                Arc::as_ptr(object).cast::<()>(),
            )
        });

        if !already_subscribed {
            topic_subscribers.objects.push(Arc::downgrade(object));
        }

        topic_subscribers.method_names.insert(method_name);
    }

    /// `wait_on_pub`
    pub fn wait_on_pub(&self) {
        let mut state = self.lock();

        while state.queue.is_empty() && !state.shutdown {
            state = self
                .received_job_event
                .wait(state)
                .unwrap_or_else(|e| e.into_inner());
        }
    }

    /// `wait_until_free`
    pub fn wait_until_free(&self) {
        let mut state = self.lock();

        while !state.idle {
            state = self
                .idle_event
                .wait(state)
                .unwrap_or_else(|e| e.into_inner());
        }
    }

    /// `work_to_do`
    pub fn work_to_do(&self) -> bool {
        !self.lock().queue.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex as StdMutex;

    struct RecordingSubscriber {
        name: &'static str,
        calls: StdMutex<Vec<(String, String, PubSubMessage)>>,
        allowed_methods: Vec<&'static str>,
    }

    impl RecordingSubscriber {
        fn new(name: &'static str, allowed_methods: &[&'static str]) -> Arc<Self> {
            Arc::new(Self {
                name,
                calls: StdMutex::new(Vec::new()),
                allowed_methods: allowed_methods.to_vec(),
            })
        }

        fn calls(&self) -> Vec<(String, String, PubSubMessage)> {
            self.calls.lock().unwrap().clone()
        }
    }

    impl Subscriber for RecordingSubscriber {
        fn has_method(&self, method_name: &str) -> bool {
            self.allowed_methods.contains(&method_name)
        }

        fn call(&self, method_name: &str, message: &PubSubMessage) -> HydrusResult<()> {
            self.calls.lock().unwrap().push((
                self.name.to_string(),
                method_name.to_string(),
                message.clone(),
            ));

            Ok(())
        }
    }

    #[test]
    fn queued_publish_reaches_subscribers() {
        let pubsub = PubSub::new();

        let first = RecordingSubscriber::new("first", &["handle"]);
        let second = RecordingSubscriber::new("second", &["handle", "other"]);

        pubsub.sub(&first, "handle", "topic");
        pubsub.sub(&second, "handle", "topic");
        pubsub.sub(&second, "other", "topic");
        pubsub.sub(&second, "handle", "different");

        assert!(pubsub.currently_idle());
        assert!(!pubsub.work_to_do());

        pubsub.publish(PubSubMessage::with_args(
            "topic",
            vec![Value::from(1), Value::from("two")],
        ));

        assert!(!pubsub.currently_idle());
        assert!(pubsub.work_to_do());

        assert!(pubsub.process());

        assert!(pubsub.currently_idle());
        assert!(!pubsub.work_to_do());

        // Two methods on the second object, matching Python's topic-level
        // method sets, plus the first object's one call.
        assert_eq!(first.calls().len(), 1);
        assert_eq!(second.calls().len(), 2);

        let message = &second.calls()[0].2;

        assert_eq!(message.arg(0), Some(&Value::from(1)));
    }

    #[test]
    fn missing_methods_are_skipped() {
        let pubsub = PubSub::new();
        let object = RecordingSubscriber::new("object", &["handle"]);

        pubsub.sub(&object, "handle", "topic");
        pubsub.sub(&object, "not_implemented", "topic");
        pubsub.publish(PubSubMessage::new("topic"));

        assert!(pubsub.process());
        assert_eq!(object.calls().len(), 1);
    }

    #[test]
    fn dropped_subscribers_are_forgotten() {
        let pubsub = PubSub::new();

        {
            let object = RecordingSubscriber::new("object", &["handle"]);

            pubsub.sub(&object, "handle", "topic");
        }

        pubsub.publish(PubSubMessage::new("topic"));

        assert!(pubsub.process());
        assert!(!pubsub.work_to_do());
    }

    #[test]
    fn invalid_callable_skips_objects() {
        let pubsub = PubSub::with_valid_callable(|object| {
            let object: &dyn std::any::Any = object;

            object
                .downcast_ref::<RecordingSubscriber>()
                .map(|object| object.name != "invalid")
                .unwrap_or(false)
        });

        let valid = RecordingSubscriber::new("valid", &["handle"]);
        let invalid = RecordingSubscriber::new("invalid", &["handle"]);

        pubsub.sub(&valid, "handle", "topic");
        pubsub.sub(&invalid, "handle", "topic");
        pubsub.publish(PubSubMessage::new("topic"));

        assert!(pubsub.process());
        assert_eq!(valid.calls().len(), 1);
        assert!(invalid.calls().is_empty());
    }

    #[test]
    fn immediate_publish_after_shutdown() {
        let pubsub = PubSub::new();
        let object = RecordingSubscriber::new("object", &["handle"]);

        pubsub.sub(&object, "handle", "topic");
        pubsub.shutdown();

        pubsub.publish(PubSubMessage::new("topic"));

        assert!(pubsub.currently_idle());
        assert!(!pubsub.work_to_do());
        assert_eq!(object.calls().len(), 1);
    }
}
