//! Rust port of `hydrus/core/processes/HydrusThreading.py`.
//!
//! Python subclasses `threading.Thread`, stores per-thread dictionaries, and
//! reaches the controller through `HG.controller`. The Rust port:
//!
//! * tracks worker state by [`std::thread::ThreadId`] in a process-global
//!   registry (there is no public `Thread::is_alive`);
//! * replaces the controller reference with the object-safe
//!   [`ThreadingController`] trait;
//! * uses closures for `HydrusData.Call` work items;
//! * uses a small [`Event`] (`Mutex<bool>` + `Condvar`) where Python uses
//!   `threading.Event`.
//!
//! The scheduling rules are otherwise unchanged: a fast and a slow
//! [`JobScheduler`], `SingleJob`/`RepeatingJob`, the ten-jobs-per-tick cap, and
//! the pool's "make another thread unless we are at 250" logic.

use std::collections::{HashSet, VecDeque};
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, Weak};
use std::thread::ThreadId;
use std::time::Duration;

use rand::Rng;

use crate::hydrus_data::{debug_print, print_exception, safe_print};
use crate::hydrus_exceptions::{ErrorKind, HydrusException, HydrusResult, shutdown};
use crate::hydrus_globals;
use crate::hydrus_numbers::to_human_int;
use crate::hydrus_profiling;
use crate::hydrus_pubsub::{PubSubMessage, Subscriber};
use crate::hydrus_time::{
    get_now, get_now_float, get_timedelta_until_time_float, time_delta_to_pretty_timedelta,
    time_has_passed, time_has_passed_float,
};

/// A `threading.Event` equivalent.
///
/// `wait` returns `true` if the event was (or became) set, which mirrors
/// Python's return value.
#[derive(Default)]
pub struct Event {
    set: Mutex<bool>,
    condvar: Condvar,
}

impl Event {
    pub fn new() -> Self {
        Self::default()
    }

    fn lock(&self) -> MutexGuard<'_, bool> {
        self.set.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn set(&self) {
        *self.lock() = true;
        self.condvar.notify_all();
    }

    pub fn clear(&self) {
        *self.lock() = false;
    }

    pub fn is_set(&self) -> bool {
        *self.lock()
    }

    /// `event.wait( timeout )`
    pub fn wait(&self, timeout: Duration) -> bool {
        let guard = self.lock();
        let (guard, _) = self
            .condvar
            .wait_timeout_while(guard, timeout, |set| !*set)
            .unwrap_or_else(|e| e.into_inner());

        *guard
    }
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|e| e.into_inner())
}

// ---------------------------------------------------------------------------
// Thread bookkeeping
// ---------------------------------------------------------------------------

static NEXT_THREAD_CLEAROUT: AtomicI64 = AtomicI64::new(0);

static THREAD_REGISTRY: Mutex<Option<ThreadRegistry>> = Mutex::new(None);

#[derive(Default)]
struct ThreadRegistry {
    known: HashSet<ThreadId>,
    shutting_down: HashSet<ThreadId>,
    daemons: HashSet<ThreadId>,
    dead: HashSet<ThreadId>,
}

fn registry() -> MutexGuard<'static, Option<ThreadRegistry>> {
    let mut guard = lock(&THREAD_REGISTRY);

    if guard.is_none() {
        *guard = Some(ThreadRegistry::default());
    }

    guard
}

fn with_registry<T>(func: impl FnOnce(&mut ThreadRegistry) -> T) -> T {
    let mut guard = registry();
    let registry = guard.as_mut().expect("registry was just created");

    func(registry)
}

/// `clear_out_dead_threads()`
pub fn clear_out_dead_threads() {
    with_registry(|registry| {
        let dead: Vec<ThreadId> = registry.dead.drain().collect();

        for thread_id in dead {
            registry.known.remove(&thread_id);
            registry.shutting_down.remove(&thread_id);
            registry.daemons.remove(&thread_id);
        }
    });
}

/// `get_thread_info()`
///
/// The Python function returns a mutable dictionary. The Rust port only needs
/// the side effect of registering the thread and the periodic cleanup, so it
/// returns whether the thread is already marked as shutting down.
fn touch_current_thread() -> bool {
    if time_has_passed(Some(NEXT_THREAD_CLEAROUT.load(Ordering::Relaxed))) {
        clear_out_dead_threads();

        NEXT_THREAD_CLEAROUT.store(get_now() + 600, Ordering::Relaxed);
    }

    let me = std::thread::current().id();

    with_registry(|registry| {
        registry.known.insert(me);

        registry.shutting_down.contains(&me)
    })
}

/// Register a managed thread that is not a `DAEMON` (for example the job
/// scheduler). It follows the `HG.model_shutdown` rule rather than the
/// daemon-only `HG.started_shutdown` rule.
pub fn register_thread(thread_id: ThreadId) {
    with_registry(|registry| {
        registry.known.insert(thread_id);
    });
}

/// Mark a managed thread as a `DAEMON`.
pub fn register_daemon_thread(thread_id: ThreadId) {
    with_registry(|registry| {
        registry.known.insert(thread_id);
        registry.daemons.insert(thread_id);
    });
}

/// Mark a managed thread as finished, so the periodic cleanup can forget it.
pub fn mark_thread_dead(thread_id: ThreadId) {
    with_registry(|registry| {
        registry.dead.insert(thread_id);
    });
}

/// `shutdown_thread( thread )`
pub fn shutdown_thread(thread_id: ThreadId) {
    if hydrus_globals::shutdown_report_mode() {
        debug_print(format!(
            "Thread {thread_id:?} is getting an external shutdown call."
        ));
    }

    with_registry(|registry| {
        registry.known.insert(thread_id);
        registry.shutting_down.insert(thread_id);
    });
}

/// `is_thread_shutting_down()`
pub fn is_thread_shutting_down() -> bool {
    if hydrus_globals::doing_fast_exit() {
        return true;
    }

    let me = std::thread::current().id();

    let is_daemon = with_registry(|registry| registry.daemons.contains(&me));

    if is_daemon {
        if hydrus_globals::started_shutdown() {
            return true;
        }
    } else if hydrus_globals::model_shutdown() {
        return true;
    }

    if touch_current_thread() {
        return true;
    }

    let marked = with_registry(|registry| registry.shutting_down.contains(&me));

    marked
}

/// `check_if_thread_shutting_down()`
pub fn check_if_thread_shutting_down() -> HydrusResult<()> {
    if is_thread_shutting_down() {
        Err(shutdown("Thread is shutting down!"))
    } else {
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Small helpers
// ---------------------------------------------------------------------------

/// `RegularJobChecker`
pub struct RegularJobChecker {
    period: f64,
    next_check: Mutex<f64>,
}

impl RegularJobChecker {
    pub fn new(period: f64) -> Self {
        Self {
            period,
            next_check: Mutex::new(get_now_float()),
        }
    }

    pub fn due(&self) -> bool {
        let mut next_check = lock(&self.next_check);

        if time_has_passed_float(*next_check) {
            *next_check = get_now_float() + self.period;

            true
        } else {
            false
        }
    }
}

/// `BigJobPauser`
pub struct BigJobPauser {
    period: f64,
    wait_time: f64,
    next_pause: Mutex<f64>,
}

impl BigJobPauser {
    pub fn new(period: f64, wait_time: f64) -> Self {
        Self {
            period,
            wait_time,
            next_pause: Mutex::new(get_now_float() + period),
        }
    }

    pub fn pause(&self) {
        let mut next_pause = lock(&self.next_pause);

        if time_has_passed_float(*next_pause) {
            std::thread::sleep(Duration::from_secs_f64(self.wait_time));

            *next_pause = get_now_float() + self.period;
        }
    }
}

// ---------------------------------------------------------------------------
// Controller interface
// ---------------------------------------------------------------------------

/// A scheduled or threaded unit of work. It returns a [`HydrusResult`] so a
/// `ShutdownException` can stop the worker, as in Python.
#[derive(Clone)]
pub struct ThreadCall {
    label: String,
    func: Arc<dyn Fn() -> HydrusResult<()> + Send + Sync>,
}

impl ThreadCall {
    pub fn new(
        label: impl Into<String>,
        func: impl Fn() -> HydrusResult<()> + Send + Sync + 'static,
    ) -> Self {
        Self {
            label: label.into(),
            func: Arc::new(func),
        }
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    pub fn call(&self) -> HydrusResult<()> {
        (self.func)()
    }
}

/// Convenience wrapper for a plain closure that cannot fail.
pub fn thread_call<F>(label: impl Into<String>, func: F) -> ThreadCall
where
    F: Fn() + Send + Sync + 'static,
{
    ThreadCall::new(label, move || {
        func();

        Ok(())
    })
}

/// The controller surface used by the threading subsystem.
pub trait ThreadingController: Send + Sync {
    fn doing_fast_exit(&self) -> bool;

    fn subscribe(&self, subscriber: Arc<dyn Subscriber>, method_name: &str, topic: &str);

    fn call_to_thread(&self, call: ThreadCall) -> HydrusResult<()>;

    fn acquire_thread_slot(&self, thread_type: &str) -> bool;

    fn release_thread_slot(&self, thread_type: &str);

    fn thread_slots_are_available(&self, thread_type: &str) -> bool;

    fn just_woke_from_sleep(&self) -> bool;

    fn good_time_to_start_background_work(&self) -> bool;

    fn good_time_to_start_foreground_work(&self) -> bool;
}

/// A printable snapshot of a worker, used by the client/server UI.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ThreadSummary {
    pub name: String,
    pub current_job: String,
}

// ---------------------------------------------------------------------------
// Daemons
// ---------------------------------------------------------------------------

/// `class DAEMON`
///
/// This is the shared state of a daemon thread. Concrete work is supplied by a
/// [`DaemonTask`]; the daemon itself handles wake/shutdown topics.
pub struct Daemon {
    controller: Arc<dyn ThreadingController>,
    name: String,
    event: Event,
    thread_id: Mutex<Option<ThreadId>>,
    shutdown_requested: AtomicBool,
    task: Mutex<Option<Arc<dyn DaemonTask>>>,
}

/// The body of a daemon thread.
pub trait DaemonTask: Send + Sync {
    fn run(&self, daemon: &Daemon);

    fn get_current_job_summary(&self) -> String;
}

impl Daemon {
    pub fn new(controller: Arc<dyn ThreadingController>, name: impl Into<String>) -> Arc<Self> {
        let name = name.into();

        let daemon = Arc::new(Self {
            controller,
            name,
            event: Event::new(),
            thread_id: Mutex::new(None),
            shutdown_requested: AtomicBool::new(false),
            task: Mutex::new(None),
        });

        daemon.controller.subscribe(
            Arc::clone(&daemon) as Arc<dyn Subscriber>,
            "wake",
            "wake_daemons",
        );

        daemon.controller.subscribe(
            Arc::clone(&daemon) as Arc<dyn Subscriber>,
            "shutdown",
            "shutdown",
        );

        daemon
    }

    pub fn controller(&self) -> &dyn ThreadingController {
        self.controller.as_ref()
    }

    pub fn get_name(&self) -> &str {
        &self.name
    }

    pub fn get_current_job_summary(&self) -> String {
        lock(&self.task)
            .as_ref()
            .map(|task| task.get_current_job_summary())
            .unwrap_or_else(|| "unknown job".to_string())
    }

    fn do_pre_call(&self) {
        if hydrus_globals::daemon_report_mode() {
            safe_print(format!("{} doing a job.", self.name));
        }
    }

    fn do_await(&self, wait_time: f64, event_can_wake: bool) -> HydrusResult<()> {
        let time_to_start = get_now_float() + wait_time;

        while !time_has_passed_float(time_to_start) {
            if event_can_wake {
                if self.event.wait(Duration::from_secs_f64(1.0)) {
                    self.event.clear();

                    return Ok(());
                }
            } else {
                std::thread::sleep(Duration::from_secs(1));
            }

            check_if_thread_shutting_down()?;
        }

        Ok(())
    }

    /// `wake()`
    pub fn wake(&self) {
        self.event.set();
    }

    /// `shutdown()`
    pub fn shutdown(&self) {
        self.shutdown_requested.store(true, Ordering::Relaxed);

        if let Some(thread_id) = *lock(&self.thread_id) {
            shutdown_thread(thread_id);
        }

        self.wake();
    }

    fn spawn(self: &Arc<Self>, task: Arc<dyn DaemonTask>) -> HydrusResult<()> {
        *lock(&self.task) = Some(Arc::clone(&task));

        let daemon = Arc::clone(self);
        let name = self.name.clone();

        std::thread::Builder::new()
            .name(name)
            .spawn(move || {
                let thread_id = std::thread::current().id();

                register_daemon_thread(thread_id);
                *lock(&daemon.thread_id) = Some(thread_id);

                if daemon.shutdown_requested.load(Ordering::Relaxed) {
                    shutdown_thread(thread_id);
                }

                task.run(&daemon);

                mark_thread_dead(thread_id);
            })
            .map_err(|e| {
                HydrusException::new(ErrorKind::Unknown, format!("Could not start daemon: {e}"))
            })?;

        Ok(())
    }
}

impl Subscriber for Daemon {
    fn has_method(&self, method_name: &str) -> bool {
        matches!(method_name, "wake" | "shutdown" | "set")
    }

    fn call(&self, method_name: &str, _message: &PubSubMessage) -> HydrusResult<()> {
        match method_name {
            "shutdown" => self.shutdown(),
            // `set` is DAEMONWorker's pub/sub wake method.
            _ => self.wake(),
        }

        Ok(())
    }
}

pub type DaemonCallable = Arc<dyn Fn(&dyn ThreadingController) -> HydrusResult<()> + Send + Sync>;

type CanStart = Arc<dyn Fn(&dyn ThreadingController) -> bool + Send + Sync>;

/// `class DAEMONWorker`
pub struct DaemonWorker {
    callable: DaemonCallable,
    label: String,
    period: f64,
    init_wait: f64,
    pre_call_wait: f64,
    can_start: CanStart,
}

impl DaemonWorker {
    /// `DAEMONWorker( controller, name, callable, topics = None, period = 3600,
    /// init_wait = 3, pre_call_wait = 0 )`
    #[allow(clippy::too_many_arguments)]
    pub fn spawn(
        controller: Arc<dyn ThreadingController>,
        name: impl Into<String>,
        callable: DaemonCallable,
        label: impl Into<String>,
        topics: &[&str],
        period: f64,
        init_wait: f64,
        pre_call_wait: f64,
    ) -> HydrusResult<Arc<Daemon>> {
        Self::spawn_with_can_start(
            controller,
            name,
            callable,
            label,
            topics,
            period,
            init_wait,
            pre_call_wait,
            Arc::new(|_| true),
        )
    }

    /// `class DAEMONBackgroundWorker`
    #[allow(clippy::too_many_arguments)]
    pub fn spawn_background(
        controller: Arc<dyn ThreadingController>,
        name: impl Into<String>,
        callable: DaemonCallable,
        label: impl Into<String>,
        topics: &[&str],
        period: f64,
        init_wait: f64,
        pre_call_wait: f64,
    ) -> HydrusResult<Arc<Daemon>> {
        Self::spawn_with_can_start(
            controller,
            name,
            callable,
            label,
            topics,
            period,
            init_wait,
            pre_call_wait,
            Arc::new(|controller| controller.good_time_to_start_background_work()),
        )
    }

    /// `class DAEMONForegroundWorker`
    #[allow(clippy::too_many_arguments)]
    pub fn spawn_foreground(
        controller: Arc<dyn ThreadingController>,
        name: impl Into<String>,
        callable: DaemonCallable,
        label: impl Into<String>,
        topics: &[&str],
        period: f64,
        init_wait: f64,
        pre_call_wait: f64,
    ) -> HydrusResult<Arc<Daemon>> {
        Self::spawn_with_can_start(
            controller,
            name,
            callable,
            label,
            topics,
            period,
            init_wait,
            pre_call_wait,
            Arc::new(|controller| controller.good_time_to_start_foreground_work()),
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn spawn_with_can_start(
        controller: Arc<dyn ThreadingController>,
        name: impl Into<String>,
        callable: DaemonCallable,
        label: impl Into<String>,
        topics: &[&str],
        period: f64,
        init_wait: f64,
        pre_call_wait: f64,
        can_start: CanStart,
    ) -> HydrusResult<Arc<Daemon>> {
        let daemon = Daemon::new(Arc::clone(&controller), name);

        for topic in topics {
            controller.subscribe(Arc::clone(&daemon) as Arc<dyn Subscriber>, "set", topic);
        }

        let worker = Arc::new(Self {
            callable,
            label: label.into(),
            period,
            init_wait,
            pre_call_wait,
            can_start,
        });

        daemon.spawn(worker)?;

        Ok(daemon)
    }
}

impl DaemonTask for DaemonWorker {
    fn run(&self, daemon: &Daemon) {
        let result = self.run_inner(daemon);

        match result {
            Ok(()) => {}
            Err(e) if e.is_a(ErrorKind::Shutdown) => {
                if hydrus_globals::shutdown_report_mode() {
                    debug_print(format!(
                        "Daemon worker \"{}\" encountered a Shutdown exception outside of the main callable.",
                        daemon.get_name()
                    ));
                }
            }
            Err(e) => {
                safe_print(format!(
                    "Daemon {} encountered an exception:",
                    daemon.get_name()
                ));
                safe_print(print_exception(&e, false));
            }
        }

        if hydrus_globals::shutdown_report_mode() {
            debug_print(format!(
                "Daemon CallToWorker \"{}\" is shut down!",
                daemon.get_name()
            ));
        }
    }

    fn get_current_job_summary(&self) -> String {
        self.label.clone()
    }
}

impl DaemonWorker {
    fn run_inner(&self, daemon: &Daemon) -> HydrusResult<()> {
        daemon.do_await(self.init_wait, true)?;

        loop {
            check_if_thread_shutting_down()?;

            daemon.do_await(self.pre_call_wait, false)?;

            check_if_thread_shutting_down()?;

            while !(self.can_start)(daemon.controller()) {
                std::thread::sleep(Duration::from_secs(1));

                check_if_thread_shutting_down()?;
            }

            check_if_thread_shutting_down()?;

            daemon.do_pre_call();

            match (self.callable)(daemon.controller()) {
                Ok(()) => {}
                Err(e) if e.is_a(ErrorKind::Shutdown) => return Err(e),
                Err(e) => {
                    safe_print(format!(
                        "Daemon {} encountered an exception:",
                        daemon.get_name()
                    ));
                    safe_print(print_exception(&e, false));
                }
            }

            daemon.do_await(self.period, true)?;
        }
    }
}

// ---------------------------------------------------------------------------
// CallToThread
// ---------------------------------------------------------------------------

struct CallQueue {
    items: VecDeque<Option<ThreadCall>>,
}

/// `class THREADCallToThread`
pub struct ThreadCallToThread {
    name: String,
    queue: Mutex<CallQueue>,
    queue_ready: Condvar,
    event: Event,
    currently_working: AtomicBool,
    current_job: Mutex<Option<String>>,
    thread_id: Mutex<Option<ThreadId>>,
}

impl ThreadCallToThread {
    pub fn new(
        controller: Arc<dyn ThreadingController>,
        name: impl Into<String>,
    ) -> HydrusResult<Arc<Self>> {
        let name = name.into();

        let call_to_thread = Arc::new(Self {
            name: name.clone(),
            queue: Mutex::new(CallQueue {
                items: VecDeque::new(),
            }),
            queue_ready: Condvar::new(),
            event: Event::new(),
            // Start true so two quick successive calls do not pick this
            // thread before it has actually taken its first job.
            currently_working: AtomicBool::new(true),
            current_job: Mutex::new(None),
            thread_id: Mutex::new(None),
        });

        controller.subscribe(
            Arc::clone(&call_to_thread) as Arc<dyn Subscriber>,
            "wake",
            "wake_daemons",
        );

        controller.subscribe(
            Arc::clone(&call_to_thread) as Arc<dyn Subscriber>,
            "shutdown",
            "shutdown",
        );

        let runner = Arc::clone(&call_to_thread);

        let handle = std::thread::Builder::new()
            .name(name)
            .spawn(move || {
                let thread_id = std::thread::current().id();

                register_daemon_thread(thread_id);
                *lock(&runner.thread_id) = Some(thread_id);

                runner.run();

                mark_thread_dead(thread_id);
            })
            .map_err(|e| {
                HydrusException::new(
                    ErrorKind::Unknown,
                    format!("Could not start a call-to-thread worker: {e}"),
                )
            })?;

        *lock(&call_to_thread.thread_id) = Some(handle.thread().id());

        Ok(call_to_thread)
    }

    pub fn get_name(&self) -> &str {
        &self.name
    }

    pub fn get_current_job_summary(&self) -> String {
        lock(&self.current_job)
            .clone()
            .unwrap_or_else(|| "no job".to_string())
    }

    pub fn currently_working(&self) -> bool {
        self.currently_working.load(Ordering::Relaxed)
    }

    /// `put( callable, *args, **kwargs )`
    pub fn put(&self, call: ThreadCall) {
        self.currently_working.store(true, Ordering::Relaxed);

        {
            let mut queue = lock(&self.queue);

            queue.items.push_back(Some(call));
        }

        self.queue_ready.notify_one();
        self.event.set();
    }

    /// `shutdown()`
    pub fn shutdown(&self) {
        if let Some(thread_id) = *lock(&self.thread_id) {
            shutdown_thread(thread_id);
        }

        {
            let mut queue = lock(&self.queue);

            queue.items.push_back(None);
        }

        self.queue_ready.notify_all();
        self.event.set();
    }

    fn do_pre_call(&self) {
        if hydrus_globals::daemon_report_mode() {
            safe_print(format!("{} doing a job.", self.name));
        }
    }

    fn next_call(&self) -> Option<ThreadCall> {
        let mut queue = lock(&self.queue);

        while queue.items.is_empty() {
            queue = self
                .queue_ready
                .wait(queue)
                .unwrap_or_else(|e| e.into_inner());
        }

        queue.items.pop_front().flatten()
    }

    fn run(&self) {
        loop {
            if check_if_thread_shutting_down().is_err() {
                return;
            }

            let Some(call) = self.next_call() else {
                self.currently_working.store(false, Ordering::Relaxed);

                if hydrus_globals::shutdown_report_mode() {
                    debug_print(format!(
                        "Daemon CallToWorker \"{}\" encountered a Shutdown exception while waiting on its job queue.",
                        self.name
                    ));
                }

                return;
            };

            self.do_pre_call();

            *lock(&self.current_job) = Some(call.label().to_string());

            let result = if hydrus_profiling::is_profile_mode("threads") {
                let summary = format!("Profiling CallTo Job: {}", call.label());
                let result: Mutex<Option<HydrusResult<()>>> = Mutex::new(None);

                hydrus_profiling::profile(
                    &summary,
                    || {
                        *lock(&result) = Some(call.call());
                    },
                    hydrus_globals::callto_profile_min_job_time_ms(),
                    false,
                );

                result
                    .into_inner()
                    .unwrap_or_else(|e| e.into_inner())
                    .unwrap_or(Ok(()))
            } else {
                call.call()
            };

            *lock(&self.current_job) = None;
            self.currently_working.store(false, Ordering::Relaxed);

            match result {
                Ok(()) => {}
                Err(e) if e.is_a(ErrorKind::Shutdown) => {
                    if hydrus_globals::shutdown_report_mode() {
                        debug_print(format!(
                            "Daemon CallToWorker \"{}\" encountered a Shutdown exception while processing a job.",
                            self.name
                        ));
                    }

                    return;
                }
                Err(e) => {
                    safe_print(format!(
                        "Daemon CallToWorker \"{}\" encountered an exception:",
                        self.name
                    ));
                    safe_print(print_exception(&e, false));
                }
            }

            std::thread::sleep(Duration::from_micros(10));
        }
    }
}

impl Subscriber for ThreadCallToThread {
    fn has_method(&self, method_name: &str) -> bool {
        matches!(method_name, "wake" | "shutdown")
    }

    fn call(&self, method_name: &str, _message: &PubSubMessage) -> HydrusResult<()> {
        if method_name == "shutdown" {
            self.shutdown();
        } else {
            self.event.set();
        }

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Thread worker pool
// ---------------------------------------------------------------------------

struct PoolState {
    call_to_threads: Vec<Arc<ThreadCallToThread>>,
    long_running_call_to_threads: Vec<Arc<ThreadCallToThread>>,
    busy_status_text: String,
    busy_status_tooltip: String,
    busy_status_last_check: f64,
}

/// `class ThreadWorkerPool`
pub struct ThreadWorkerPool {
    // Weak avoids a controller <-> pool reference cycle.
    controller: Weak<dyn ThreadingController>,
    state: Mutex<PoolState>,
}

impl ThreadWorkerPool {
    pub fn new(controller: Arc<dyn ThreadingController>) -> Self {
        Self {
            controller: Arc::downgrade(&controller),
            state: Mutex::new(PoolState {
                call_to_threads: Vec::new(),
                long_running_call_to_threads: Vec::new(),
                busy_status_text: String::new(),
                busy_status_tooltip: String::new(),
                busy_status_last_check: 0.0,
            }),
        }
    }

    fn controller(&self) -> HydrusResult<Arc<dyn ThreadingController>> {
        self.controller.upgrade().ok_or_else(|| {
            HydrusException::new(ErrorKind::Shutdown, "Controller is no longer available!")
        })
    }

    /// `get_call_to_thread()`
    pub fn get_call_to_thread(&self) -> HydrusResult<Arc<ThreadCallToThread>> {
        let controller = self.controller()?;

        let mut state = lock(&self.state);

        for call_to_thread in &state.call_to_threads {
            if !call_to_thread.currently_working() {
                return Ok(Arc::clone(call_to_thread));
            }
        }

        let mut ok_to_make_one = state.call_to_threads.len() < 250;

        if !ok_to_make_one {
            let me = std::thread::current().id();

            let called_from_pool = state
                .call_to_threads
                .iter()
                .any(|thread| *lock(&thread.thread_id) == Some(me))
                || state
                    .long_running_call_to_threads
                    .iter()
                    .any(|thread| *lock(&thread.thread_id) == Some(me));

            // We are calling from inside the pool. Make one anyway to avoid a
            // deadlock.
            ok_to_make_one = called_from_pool;
        }

        if ok_to_make_one {
            let call_to_thread = ThreadCallToThread::new(controller, "CallToThread")?;

            state.call_to_threads.push(Arc::clone(&call_to_thread));

            Ok(call_to_thread)
        } else {
            let index = rand::thread_rng().gen_range(0..state.call_to_threads.len());

            Ok(Arc::clone(&state.call_to_threads[index]))
        }
    }

    /// `get_call_to_thread_long_running()`
    pub fn get_call_to_thread_long_running(&self) -> HydrusResult<Arc<ThreadCallToThread>> {
        let controller = self.controller()?;

        let mut state = lock(&self.state);

        for call_to_thread in &state.long_running_call_to_threads {
            if !call_to_thread.currently_working() {
                return Ok(Arc::clone(call_to_thread));
            }
        }

        let call_to_thread = ThreadCallToThread::new(controller, "call_to_thread_long_running")?;

        state
            .long_running_call_to_threads
            .push(Arc::clone(&call_to_thread));

        Ok(call_to_thread)
    }

    /// `get_thread_pool_busy_status()`
    pub fn get_thread_pool_busy_status(&self) -> (String, String) {
        let mut state = lock(&self.state);

        if time_has_passed(Some((state.busy_status_last_check + 10.0) as i64)) {
            let num_threads = state
                .call_to_threads
                .iter()
                .filter(|thread| thread.currently_working())
                .count();

            state.busy_status_text = if num_threads <= 3 {
                String::new()
            } else if num_threads <= 8 {
                "working".to_string()
            } else {
                "busy".to_string()
            };

            state.busy_status_tooltip = format!(
                "There were {} threads doing jobs at last check.",
                to_human_int(num_threads as i128)
            );

            state.busy_status_last_check = get_now() as f64;
        }

        (
            state.busy_status_text.clone(),
            state.busy_status_tooltip.clone(),
        )
    }

    /// `get_threads_snapshot()`
    pub fn get_threads_snapshot(
        &self,
    ) -> (Vec<Arc<ThreadCallToThread>>, Vec<Arc<ThreadCallToThread>>) {
        let state = lock(&self.state);

        (
            state.call_to_threads.clone(),
            state.long_running_call_to_threads.clone(),
        )
    }

    /// `maintain_call_to_threads()`
    pub fn maintain_call_to_threads(&self) {
        let mut state = lock(&self.state);

        let mut keep = Vec::new();

        for thread in state.call_to_threads.drain(..) {
            if thread.currently_working() {
                keep.push(thread);
            } else {
                thread.shutdown();
            }
        }

        state.call_to_threads = keep;

        let mut keep = Vec::new();

        for thread in state.long_running_call_to_threads.drain(..) {
            if thread.currently_working() {
                keep.push(thread);
            } else {
                thread.shutdown();
            }
        }

        state.long_running_call_to_threads = keep;
    }

    /// `shutdown()`
    pub fn shutdown(&self) {
        let state = lock(&self.state);

        for thread in &state.call_to_threads {
            thread.shutdown();
        }

        for thread in &state.long_running_call_to_threads {
            thread.shutdown();
        }
    }
}

// ---------------------------------------------------------------------------
// Job scheduler
// ---------------------------------------------------------------------------

/// `class JobScheduler`
pub struct JobScheduler {
    name: String,
    waiting: Mutex<Vec<Arc<SchedulableJob>>>,
    new_job_arrived: Event,
    cancel_filter_needed: Event,
    sort_needed: Event,
    thread_id: Mutex<Option<ThreadId>>,
    shutdown_requested: AtomicBool,
}

impl JobScheduler {
    pub fn new(controller: Arc<dyn ThreadingController>) -> Arc<Self> {
        let scheduler = Arc::new(Self {
            name: "Job Scheduler".to_string(),
            waiting: Mutex::new(Vec::new()),
            new_job_arrived: Event::new(),
            cancel_filter_needed: Event::new(),
            sort_needed: Event::new(),
            thread_id: Mutex::new(None),
            shutdown_requested: AtomicBool::new(false),
        });

        controller.subscribe(
            Arc::clone(&scheduler) as Arc<dyn Subscriber>,
            "shutdown",
            "shutdown",
        );

        scheduler
    }

    /// Start the scheduler thread. Python starts it in `init_model`.
    pub fn start(self: &Arc<Self>) -> HydrusResult<()> {
        let weak = Arc::downgrade(self);

        let handle = std::thread::Builder::new()
            .name(self.name.clone())
            .spawn(move || {
                let thread_id = std::thread::current().id();

                register_thread(thread_id);

                if let Some(scheduler) = weak.upgrade() {
                    *lock(&scheduler.thread_id) = Some(thread_id);
                }

                loop {
                    let Some(scheduler) = weak.upgrade() else {
                        return;
                    };

                    if !scheduler.loop_once() {
                        mark_thread_dead(thread_id);

                        return;
                    }
                }
            })
            .map_err(|e| {
                HydrusException::new(
                    ErrorKind::Unknown,
                    format!("Could not start the job scheduler: {e}"),
                )
            })?;

        *lock(&self.thread_id) = Some(handle.thread().id());

        Ok(())
    }

    pub fn get_name(&self) -> &str {
        &self.name
    }

    pub fn get_current_job_summary(&self) -> String {
        format!("{} jobs", to_human_int(lock(&self.waiting).len() as i128))
    }

    fn filter_cancelled(&self) {
        let mut waiting = lock(&self.waiting);

        waiting.retain(|job| !job.is_cancelled());
    }

    fn get_loop_wait_time(&self) -> f64 {
        let waiting = lock(&self.waiting);

        match waiting.first() {
            None => 0.2,
            Some(next_job) => next_job.get_time_delta_until_due().min(1.0),
        }
    }

    fn no_work_to_start(&self) -> bool {
        let waiting = lock(&self.waiting);

        match waiting.first() {
            None => true,
            Some(next_job) => !next_job.is_due(),
        }
    }

    fn sort_waiting(&self) {
        let mut waiting = lock(&self.waiting);

        waiting.sort_by(|a, b| {
            a.get_next_work_time()
                .partial_cmp(&b.get_next_work_time())
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    }

    fn start_work(&self) {
        let mut jobs_started = 0;

        loop {
            let next_job = {
                let mut waiting = lock(&self.waiting);

                if waiting.is_empty() || jobs_started >= 10 {
                    break;
                }

                if !waiting[0].is_due() {
                    // The front is not due, so nor is the rest of the list.
                    break;
                }

                waiting.remove(0)
            };

            if next_job.is_cancelled() {
                continue;
            }

            if next_job.slot_ok() {
                // Important: this happens outside the waiting lock.
                if next_job.start_work().is_ok() {
                    jobs_started += 1;
                }
            } else {
                // The delay was automatically set by slot_ok.
                self.insert_job(next_job);
            }
        }
    }

    fn insert_job(&self, job: Arc<SchedulableJob>) {
        let mut waiting = lock(&self.waiting);

        let index = waiting
            .partition_point(|existing| existing.get_next_work_time() <= job.get_next_work_time());

        waiting.insert(index, job);
    }

    /// `add_job( job )`
    pub fn add_job(&self, job: Arc<SchedulableJob>) {
        self.insert_job(job);

        self.new_job_arrived.set();
    }

    /// `clear_out_dead()`
    pub fn clear_out_dead(&self) {
        let mut waiting = lock(&self.waiting);

        waiting.retain(|job| !job.is_dead());
    }

    /// `get_jobs()`
    pub fn get_jobs(&self) -> Vec<Arc<SchedulableJob>> {
        lock(&self.waiting).clone()
    }

    /// `get_pretty_job_summary()`
    pub fn get_pretty_job_summary(&self) -> String {
        let waiting = lock(&self.waiting);

        let mut lines = vec![format!("{} jobs:", to_human_int(waiting.len() as i128))];

        lines.extend(waiting.iter().map(|job| job.to_string()));

        lines.join("\n")
    }

    /// `job_cancelled()`
    pub fn job_cancelled(&self) {
        self.cancel_filter_needed.set();
    }

    /// `work_times_have_changed()`
    pub fn work_times_have_changed(&self) {
        self.sort_needed.set();
    }

    /// `shutdown()`
    pub fn shutdown(&self) {
        self.shutdown_requested.store(true, Ordering::Relaxed);

        if let Some(thread_id) = *lock(&self.thread_id) {
            shutdown_thread(thread_id);
        }

        self.new_job_arrived.set();
    }

    fn loop_once(&self) -> bool {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            while self.no_work_to_start() {
                if is_thread_shutting_down() {
                    return false;
                }

                if self.cancel_filter_needed.is_set() {
                    self.filter_cancelled();
                    self.cancel_filter_needed.clear();
                }

                if self.sort_needed.is_set() {
                    self.sort_waiting();
                    self.sort_needed.clear();

                    // If some work is now due, do it.
                    continue;
                }

                let wait_time = self.get_loop_wait_time();

                self.new_job_arrived
                    .wait(Duration::from_secs_f64(wait_time));
                self.new_job_arrived.clear();
            }

            self.start_work();

            true
        }));

        match result {
            Ok(keep_going) => keep_going,
            Err(_) => {
                safe_print("Job Scheduler encountered an exception in its mainloop.");

                true
            }
        }
    }
}

impl Subscriber for JobScheduler {
    fn has_method(&self, method_name: &str) -> bool {
        method_name == "shutdown"
    }

    fn call(&self, method_name: &str, _message: &PubSubMessage) -> HydrusResult<()> {
        if method_name == "shutdown" {
            self.shutdown();
        }

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Jobs
// ---------------------------------------------------------------------------

/// Work executed by a [`SchedulableJob`].
pub type JobCall = Arc<dyn Fn() -> HydrusResult<()> + Send + Sync>;

/// Convenience wrapper for a closure that cannot fail.
pub fn job_call<F>(func: F) -> JobCall
where
    F: Fn() + Send + Sync + 'static,
{
    Arc::new(move || {
        func();

        Ok(())
    })
}

enum JobKind {
    Single {
        work_complete: AtomicBool,
    },
    Repeating {
        period: f64,
        stop_repeating: AtomicBool,
    },
}

/// `class SchedulableJob` (plus `SingleJob` and `RepeatingJob`)
pub struct SchedulableJob {
    controller: Arc<dyn ThreadingController>,
    scheduler: Weak<JobScheduler>,
    work_callable: JobCall,
    label: String,
    pretty_class_name: &'static str,
    next_work_time: Mutex<f64>,
    thread_slot_type: Mutex<Option<String>>,
    work_lock: Mutex<()>,
    currently_working: AtomicBool,
    actual_work_started: AtomicBool,
    is_cancelled: AtomicBool,
    should_delay_on_wakeup: AtomicBool,
    kind: JobKind,
}

impl SchedulableJob {
    fn new(
        controller: Arc<dyn ThreadingController>,
        scheduler: &Arc<JobScheduler>,
        initial_delay: f64,
        work_callable: JobCall,
        label: impl Into<String>,
        pretty_class_name: &'static str,
        kind: JobKind,
    ) -> Arc<Self> {
        Arc::new(Self {
            controller,
            scheduler: Arc::downgrade(scheduler),
            work_callable,
            label: label.into(),
            pretty_class_name,
            next_work_time: Mutex::new(get_now_float() + initial_delay),
            thread_slot_type: Mutex::new(None),
            work_lock: Mutex::new(()),
            currently_working: AtomicBool::new(false),
            actual_work_started: AtomicBool::new(false),
            is_cancelled: AtomicBool::new(false),
            should_delay_on_wakeup: AtomicBool::new(false),
            kind,
        })
    }

    /// `SingleJob( controller, scheduler, initial_delay, work_callable )`
    pub fn single(
        controller: Arc<dyn ThreadingController>,
        scheduler: &Arc<JobScheduler>,
        initial_delay: f64,
        work_callable: JobCall,
        label: impl Into<String>,
    ) -> Arc<Self> {
        Self::new(
            controller,
            scheduler,
            initial_delay,
            work_callable,
            label,
            "single job",
            JobKind::Single {
                work_complete: AtomicBool::new(false),
            },
        )
    }

    /// `RepeatingJob( controller, scheduler, initial_delay, period, work_callable )`
    pub fn repeating(
        controller: Arc<dyn ThreadingController>,
        scheduler: &Arc<JobScheduler>,
        initial_delay: f64,
        period: f64,
        work_callable: JobCall,
        label: impl Into<String>,
    ) -> Arc<Self> {
        Self::new(
            controller,
            scheduler,
            initial_delay,
            work_callable,
            label,
            "repeating job",
            JobKind::Repeating {
                period,
                stop_repeating: AtomicBool::new(false),
            },
        )
    }

    pub fn get_label(&self) -> &str {
        &self.label
    }

    pub fn get_next_work_time(&self) -> f64 {
        *lock(&self.next_work_time)
    }

    pub fn get_time_delta_until_due(&self) -> f64 {
        get_timedelta_until_time_float(self.get_next_work_time())
    }

    pub fn get_pretty_job(&self) -> &str {
        &self.label
    }

    pub fn get_due_string(&self) -> String {
        let due_delta = self.get_next_work_time() - get_now_float();

        let due_string = time_delta_to_pretty_timedelta(Some(due_delta), true, false);

        if due_delta < 0.0 {
            format!("was due {due_string} ago")
        } else {
            format!("due in {due_string}")
        }
    }

    pub fn is_cancelled(&self) -> bool {
        self.is_cancelled.load(Ordering::Relaxed)
    }

    pub fn is_dead(&self) -> bool {
        // Matches Python: only the base implementation exists, so a job is
        // never considered dead. Repeating jobs re-add themselves.
        false
    }

    pub fn is_due(&self) -> bool {
        time_has_passed_float(self.get_next_work_time())
    }

    pub fn currently_working(&self) -> bool {
        if self.is_cancelled() && !self.actual_work_started.load(Ordering::Relaxed) {
            return false;
        }

        self.currently_working.load(Ordering::Relaxed)
    }

    /// `SingleJob.is_work_complete()`
    pub fn is_work_complete(&self) -> bool {
        match &self.kind {
            JobKind::Single { work_complete } => work_complete.load(Ordering::Relaxed),
            JobKind::Repeating { .. } => false,
        }
    }

    /// `cancel()`
    pub fn cancel(&self) {
        self.is_cancelled.store(true, Ordering::Relaxed);

        if let JobKind::Repeating { stop_repeating, .. } = &self.kind {
            stop_repeating.store(true, Ordering::Relaxed);
        }

        if let Some(scheduler) = self.scheduler.upgrade() {
            scheduler.job_cancelled();
        }
    }

    /// `delay( delay )`
    pub fn delay(&self, delay: f64) {
        *lock(&self.next_work_time) = get_now_float() + delay;

        if let Some(scheduler) = self.scheduler.upgrade() {
            scheduler.work_times_have_changed();
        }
    }

    /// `set_thread_slot_type( thread_type )`
    pub fn set_thread_slot_type(&self, thread_type: impl Into<String>) {
        *lock(&self.thread_slot_type) = Some(thread_type.into());
    }

    /// `should_delay_on_wakeup( value )`
    pub fn should_delay_on_wakeup(&self, value: bool) {
        self.should_delay_on_wakeup.store(value, Ordering::Relaxed);
    }

    /// `slot_ok()`
    pub fn slot_ok(&self) -> bool {
        let slot_type = lock(&self.thread_slot_type).clone();

        if let Some(slot_type) = slot_type {
            if self.controller.acquire_thread_slot(&slot_type) {
                return true;
            }

            *lock(&self.next_work_time) =
                get_now_float() + 10.0 + rand::thread_rng().gen_range(0.0..1.0);

            return false;
        }

        true
    }

    /// `start_work()`
    pub fn start_work(self: &Arc<Self>) -> HydrusResult<()> {
        if self.is_cancelled() {
            return Ok(());
        }

        self.currently_working.store(true, Ordering::Relaxed);

        let job = Arc::clone(self);
        let label = job.label.clone();
        let call = ThreadCall::new(label, move || job.work());

        if let Err(e) = self.controller.call_to_thread(call) {
            self.currently_working.store(false, Ordering::Relaxed);

            return Err(e);
        }

        Ok(())
    }

    /// `wake( next_work_time = None )`
    pub fn wake(&self, next_work_time: Option<f64>) {
        *lock(&self.next_work_time) = next_work_time.unwrap_or_else(get_now_float);

        if let Some(scheduler) = self.scheduler.upgrade() {
            scheduler.work_times_have_changed();
        }
    }

    /// `wake_on_pub_sub( topic )`
    ///
    /// Python registers the method name `PubSubWake` but the method is called
    /// `pub_sub_wake`, so its subscription never fires. The port uses the
    /// correct name; this is an intentional bug-fix.
    pub fn wake_on_pub_sub(self: &Arc<Self>, topic: &str) {
        self.controller.subscribe(
            Arc::clone(self) as Arc<dyn Subscriber>,
            "pub_sub_wake",
            topic,
        );
    }

    /// `waiting_on_work_slot()`
    pub fn waiting_on_work_slot(&self) -> bool {
        let slot_type = lock(&self.thread_slot_type).clone();

        if let Some(slot_type) = slot_type {
            return !self.currently_working()
                && self.is_due()
                && !self.controller.thread_slots_are_available(&slot_type);
        }

        false
    }

    fn run_work(&self) -> HydrusResult<()> {
        if self.should_delay_on_wakeup.load(Ordering::Relaxed) {
            while self.controller.just_woke_from_sleep() {
                check_if_thread_shutting_down()?;

                std::thread::sleep(Duration::from_secs(1));
            }
        }

        let result = {
            let _guard = lock(&self.work_lock);

            self.actual_work_started.store(true, Ordering::Relaxed);

            (self.work_callable)()
        };

        if let Some(slot_type) = lock(&self.thread_slot_type).clone() {
            self.controller.release_thread_slot(&slot_type);
        }

        self.actual_work_started.store(false, Ordering::Relaxed);
        self.currently_working.store(false, Ordering::Relaxed);

        result
    }

    /// `work()`
    pub fn work(self: &Arc<Self>) -> HydrusResult<()> {
        let result = self.run_work();

        if result.is_ok() {
            match &self.kind {
                JobKind::Single { work_complete } => {
                    work_complete.store(true, Ordering::Relaxed);
                }
                JobKind::Repeating {
                    period,
                    stop_repeating,
                } => {
                    if !stop_repeating.load(Ordering::Relaxed) {
                        *lock(&self.next_work_time) = get_now_float() + *period;

                        if let Some(scheduler) = self.scheduler.upgrade() {
                            scheduler.add_job(Arc::clone(self));
                        }
                    }
                }
            }
        }

        result
    }
}

impl std::fmt::Display for SchedulableJob {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "{}: {} {}",
            self.pretty_class_name,
            self.get_pretty_job(),
            self.get_due_string()
        )
    }
}

impl Subscriber for SchedulableJob {
    fn has_method(&self, method_name: &str) -> bool {
        matches!(method_name, "pub_sub_wake" | "PubSubWake")
    }

    fn call(&self, _method_name: &str, _message: &PubSubMessage) -> HydrusResult<()> {
        self.wake(None);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::atomic::AtomicUsize;

    struct TestController {
        calls: AtomicUsize,
        slots: Mutex<HashMap<String, (usize, usize)>>,
        subscribers: Mutex<Vec<(String, String)>>,
        fast_exit: AtomicBool,
        woke: AtomicBool,
    }

    impl TestController {
        fn new() -> Arc<Self> {
            Arc::new(Self {
                calls: AtomicUsize::new(0),
                slots: Mutex::new(HashMap::new()),
                subscribers: Mutex::new(Vec::new()),
                fast_exit: AtomicBool::new(false),
                woke: AtomicBool::new(false),
            })
        }

        fn as_dyn(self: &Arc<Self>) -> Arc<dyn ThreadingController> {
            Arc::clone(self) as Arc<dyn ThreadingController>
        }
    }

    impl ThreadingController for TestController {
        fn doing_fast_exit(&self) -> bool {
            self.fast_exit.load(Ordering::Relaxed)
        }

        fn subscribe(&self, _subscriber: Arc<dyn Subscriber>, method_name: &str, topic: &str) {
            self.subscribers
                .lock()
                .unwrap()
                .push((method_name.to_string(), topic.to_string()));
        }

        fn call_to_thread(&self, call: ThreadCall) -> HydrusResult<()> {
            self.calls.fetch_add(1, Ordering::SeqCst);

            std::thread::spawn(move || {
                let _ = call.call();
            });

            Ok(())
        }

        fn acquire_thread_slot(&self, thread_type: &str) -> bool {
            let mut slots = self.slots.lock().unwrap();

            match slots.get_mut(thread_type) {
                None => true,
                Some((current, max)) => {
                    if *current < *max {
                        *current += 1;

                        true
                    } else {
                        false
                    }
                }
            }
        }

        fn release_thread_slot(&self, thread_type: &str) {
            if let Some((current, _)) = self.slots.lock().unwrap().get_mut(thread_type) {
                *current = current.saturating_sub(1);
            }
        }

        fn thread_slots_are_available(&self, thread_type: &str) -> bool {
            match self.slots.lock().unwrap().get(thread_type) {
                None => true,
                Some((current, max)) => current < max,
            }
        }

        fn just_woke_from_sleep(&self) -> bool {
            self.woke.swap(false, Ordering::SeqCst)
        }

        fn good_time_to_start_background_work(&self) -> bool {
            true
        }

        fn good_time_to_start_foreground_work(&self) -> bool {
            true
        }
    }

    fn wait_for(condition: impl Fn() -> bool) -> bool {
        let started = std::time::Instant::now();

        while started.elapsed() < Duration::from_secs(5) {
            if condition() {
                return true;
            }

            std::thread::sleep(Duration::from_millis(5));
        }

        condition()
    }

    #[test]
    fn event_round_trip() {
        let event = Event::new();

        assert!(!event.is_set());
        assert!(!event.wait(Duration::from_millis(5)));

        event.set();

        assert!(event.is_set());
        assert!(event.wait(Duration::from_millis(5)));

        event.clear();

        assert!(!event.is_set());
    }

    #[test]
    fn regular_job_checker_is_due_immediately_then_waits() {
        let checker = RegularJobChecker::new(60.0);

        assert!(checker.due());
        assert!(!checker.due());
    }

    #[test]
    fn single_job_runs_once() {
        let controller = TestController::new();
        let scheduler = JobScheduler::new(controller.as_dyn());

        scheduler.start().unwrap();

        let ran = Arc::new(AtomicUsize::new(0));

        let job = SchedulableJob::single(
            controller.as_dyn(),
            &scheduler,
            0.0,
            job_call({
                let ran = Arc::clone(&ran);

                move || {
                    ran.fetch_add(1, Ordering::SeqCst);
                }
            }),
            "test single",
        );

        scheduler.add_job(Arc::clone(&job));

        assert!(wait_for(|| job.is_work_complete()));
        assert_eq!(ran.load(Ordering::SeqCst), 1);
        assert_eq!(scheduler.get_jobs().len(), 0);
    }

    #[test]
    fn repeating_job_reschedules_until_cancelled() {
        let controller = TestController::new();
        let scheduler = JobScheduler::new(controller.as_dyn());

        scheduler.start().unwrap();

        let ran = Arc::new(AtomicUsize::new(0));

        let job = SchedulableJob::repeating(
            controller.as_dyn(),
            &scheduler,
            0.0,
            0.02,
            job_call({
                let ran = Arc::clone(&ran);

                move || {
                    ran.fetch_add(1, Ordering::SeqCst);
                }
            }),
            "test repeating",
        );

        scheduler.add_job(Arc::clone(&job));

        assert!(wait_for(|| ran.load(Ordering::SeqCst) >= 3));

        job.cancel();

        let count = ran.load(Ordering::SeqCst);

        std::thread::sleep(Duration::from_millis(60));

        assert_eq!(ran.load(Ordering::SeqCst), count);
        assert!(job.is_cancelled());
    }

    #[test]
    fn cancelled_job_does_not_start() {
        let controller = TestController::new();
        let scheduler = JobScheduler::new(controller.as_dyn());

        let ran = Arc::new(AtomicUsize::new(0));

        let job = SchedulableJob::single(
            controller.as_dyn(),
            &scheduler,
            0.0,
            job_call({
                let ran = Arc::clone(&ran);

                move || {
                    ran.fetch_add(1, Ordering::SeqCst);
                }
            }),
            "cancelled",
        );

        job.cancel();

        assert!(job.start_work().is_ok());
        assert_eq!(ran.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn scheduler_sorts_jobs_by_due_time() {
        let controller = TestController::new();
        let scheduler = JobScheduler::new(controller.as_dyn());

        let first = SchedulableJob::single(
            controller.as_dyn(),
            &scheduler,
            100.0,
            job_call(|| {}),
            "later",
        );

        let second = SchedulableJob::single(
            controller.as_dyn(),
            &scheduler,
            0.0,
            job_call(|| {}),
            "sooner",
        );

        scheduler.add_job(Arc::clone(&first));
        scheduler.add_job(Arc::clone(&second));

        let jobs = scheduler.get_jobs();

        assert_eq!(jobs[0].get_label(), "sooner");
        assert_eq!(jobs[1].get_label(), "later");

        first.cancel();
        second.cancel();
    }

    #[test]
    fn thread_worker_pool_runs_calls() {
        let controller = TestController::new();
        let pool = ThreadWorkerPool::new(controller.as_dyn());

        let ran = Arc::new(AtomicBool::new(false));

        let thread = pool.get_call_to_thread().unwrap();

        thread.put(ThreadCall::new("test call", {
            let ran = Arc::clone(&ran);

            move || {
                ran.store(true, Ordering::SeqCst);

                Ok(())
            }
        }));

        assert!(wait_for(|| ran.load(Ordering::SeqCst)));
        assert!(wait_for(|| !thread.currently_working()));

        pool.shutdown();
    }

    #[test]
    fn daemon_worker_runs_and_stops() {
        let controller = TestController::new();

        let ran = Arc::new(AtomicUsize::new(0));

        let daemon = DaemonWorker::spawn(
            controller.as_dyn(),
            "test daemon",
            Arc::new({
                let ran = Arc::clone(&ran);

                move |_controller| {
                    ran.fetch_add(1, Ordering::SeqCst);

                    Ok(())
                }
            }),
            "test callable",
            &[],
            0.01,
            0.0,
            0.0,
        )
        .unwrap();

        assert!(wait_for(|| ran.load(Ordering::SeqCst) >= 2));

        daemon.shutdown();

        let count = ran.load(Ordering::SeqCst);

        std::thread::sleep(Duration::from_millis(40));

        assert_eq!(ran.load(Ordering::SeqCst), count);
        assert_eq!(daemon.get_name(), "test daemon");
    }

    #[test]
    fn thread_slots_gate_jobs() {
        let controller = TestController::new();

        controller
            .slots
            .lock()
            .unwrap()
            .insert("important".to_string(), (0, 1));

        let scheduler = JobScheduler::new(controller.as_dyn());

        let job = SchedulableJob::single(
            controller.as_dyn(),
            &scheduler,
            0.0,
            job_call(|| {}),
            "slotted",
        );

        job.set_thread_slot_type("important");

        assert!(job.slot_ok());
        assert!(!job.slot_ok());
        assert!(job.waiting_on_work_slot() || !job.is_due());

        job.cancel();
    }
}
