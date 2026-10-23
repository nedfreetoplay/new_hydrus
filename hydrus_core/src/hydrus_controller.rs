//! Rust port of `hydrus/core/HydrusController.py`.
//!
//! Python's controller owns the pub/sub hub, the two job schedulers, the thread
//! worker pool, the database handle, the temp dir and the timestamps dictionary,
//! and reaches all of that through `HG.controller`. The Rust port keeps the same
//! responsibilities but:
//!
//! * splits the subclass overrides (`_init_db`, `_get_pubsub_valid_callable`,
//!   `maintain_db`, ...) into the [`ControllerHooks`] trait;
//! * implements [`ThreadingController`] so schedulers, daemons and worker pools
//!   can keep talking to "the controller" without a global;
//! * builds the self-reference and worker pool just after `Arc::new`, because
//!   `Arc::new_cyclic` does not allow upgrading its weak reference;
//! * takes a [`JobCall`] plus a label for scheduled work, since Rust closures
//!   have no useful name for the job-summary UI;
//! * keeps the per-instance `doing_fast_exit` flag but also honours the
//!   process-global [`hydrus_globals::doing_fast_exit`] flag used by
//!   `HydrusThreading`.
//!
//! `start_twisted_if_needed`/`stop_twisted_if_running` have no Rust equivalent;
//! they are hooks with no-op defaults so a client/server layer can plug in its
//! own event loop later. `initialise_mimes_to_default_thumbnail_paths` is
//! supplied by [`crate::files::hydrus_file_handling`] but remains a hook so a client
//! can replace the static-directory lookup.

use std::any::Any;
use std::collections::HashMap;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock, RwLock, Weak};
use std::time::Duration;

use crate::hydrus_constants::{MaintenanceMode, HYDRUS_KEY_LENGTH, UNICODE_ELLIPSIS};
use crate::hydrus_data::{clean_running_file, debug_print, generate_key, last_shutdown_was_bad, safe_print};
use crate::hydrus_db::HydrusDB;
use crate::hydrus_exceptions::{ErrorKind, HydrusException, HydrusResult};
use crate::hydrus_globals;
use crate::hydrus_logger::HydrusLogger;
use crate::hydrus_paths::delete_path;
use crate::processes::hydrus_process;
use crate::hydrus_pubsub::{PubSub, PubSubMessage, Subscriber};
use crate::processes::hydrus_subprocess;
use crate::hydrus_temp;
use crate::processes::hydrus_threading::{
    JobCall, JobScheduler, SchedulableJob, ThreadCall, ThreadCallToThread, ThreadWorkerPool,
    ThreadingController,
};
use crate::hydrus_time::{get_now, get_now_ms, time_has_passed, time_has_passed_ms};

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|error| error.into_inner())
}

/// The subclass surface of `HydrusController`.
///
/// Python's `HydrusController` is a base class that client and server override
/// with `super().method()` calls. The Rust port keeps the base behaviour here
/// and lets an implementation of this trait replace/extend it. All methods have
/// defaults, so a test or a headless application only implements what it needs.
pub trait ControllerHooks: Send + Sync {
    /// `_get_pubsub_valid_callable()`
    ///
    /// The client returns a "is this Qt object still alive" check here.
    fn pubsub_valid(&self, _object: &dyn Subscriber) -> bool {
        true
    }

    /// `_get_wake_delay_period_ms()`
    fn get_wake_delay_period_ms(&self) -> i64 {
        15 * 1000
    }

    /// `_init_db()`
    ///
    /// Return the database to install, or `None` for a controller without a
    /// database (useful for tests and for the headless tools in `core/`).
    fn init_db(
        &self,
        _controller: &HydrusController,
    ) -> HydrusResult<Option<Arc<HydrusDB>>> {
        Ok(None)
    }

    /// `initialise_mimes_to_default_thumbnail_paths()`
    fn initialise_mimes_to_default_thumbnail_paths(&self) {
        crate::files::hydrus_file_handling::initialise_mimes_to_default_thumbnail_paths();
    }

    /// Extra setup run at the end of `init_model`.
    fn after_init_model(&self, _controller: &Arc<HydrusController>) -> HydrusResult<()> {
        Ok(())
    }

    /// Extra daemons/setup run at the end of `init_view`.
    fn init_view(&self, _controller: &Arc<HydrusController>) -> HydrusResult<()> {
        Ok(())
    }

    /// `_publish_shutdown_subtext( text )`
    fn publish_shutdown_subtext(&self, _text: &str) {}

    /// `_report_shutdown_daemons_status()`
    fn report_shutdown_daemons_status(&self) {}

    /// `_show_just_woke_to_user()`
    fn show_just_woke_to_user(&self) {
        safe_print("Just woke from sleep.");
    }

    /// `currently_idle()`
    fn currently_idle(&self) -> bool {
        true
    }

    /// `clear_caches()`
    fn clear_caches(&self) {}

    /// `maintain_db( maintenance_mode, stop_time )`
    fn maintain_db(
        &self,
        _controller: &HydrusController,
        _maintenance_mode: MaintenanceMode,
        _stop_time: Option<i64>,
    ) {
    }

    /// `report_data_used( num_bytes )`
    fn report_data_used(&self, _num_bytes: u64) {}

    /// `report_request_used()`
    fn report_request_used(&self) {}

    /// `shutdown_from_server()`
    fn shutdown_from_server(&self) -> HydrusResult<()> {
        Err(HydrusException::new(
            ErrorKind::Unknown,
            "This hydrus application cannot be shut down from the server!",
        ))
    }

    /// `start_twisted_if_needed()`
    ///
    /// The Rust port has no twisted reactor; client/server layers start their
    /// own I/O runtime here.
    fn start_twisted_if_needed(&self) -> HydrusResult<()> {
        Ok(())
    }

    /// `stop_twisted_if_running()`
    fn stop_twisted_if_running(&self) {}
}

/// A snapshot of the workers the UI/debug views show.
#[derive(Default)]
pub struct ThreadsSnapshot {
    pub call_to_threads: Vec<Arc<ThreadCallToThread>>,
    pub long_running_call_to_threads: Vec<Arc<ThreadCallToThread>>,
    pub fast_job_scheduler: Option<Arc<JobScheduler>>,
    pub slow_job_scheduler: Option<Arc<JobScheduler>>,
}

/// Python's `HydrusController`.
pub struct HydrusController {
    name: RwLock<String>,
    boot_id: [u8; HYDRUS_KEY_LENGTH],
    last_shutdown_was_bad: AtomicBool,
    i_own_running_file: AtomicBool,

    db_dir: PathBuf,
    logger: Arc<HydrusLogger>,
    db: RwLock<Option<Arc<HydrusDB>>>,

    pubsub: PubSub,
    daemon_jobs: RwLock<HashMap<String, Arc<SchedulableJob>>>,
    managers: RwLock<HashMap<String, Arc<dyn Any + Send + Sync>>>,

    fast_job_scheduler: RwLock<Option<Arc<JobScheduler>>>,
    slow_job_scheduler: RwLock<Option<Arc<JobScheduler>>>,

    thread_slots: Mutex<HashMap<String, (usize, usize)>>,

    thread_worker_pool: OnceLock<ThreadWorkerPool>,

    timestamps: Mutex<HashMap<String, i64>>,

    sleep_lock: Mutex<()>,
    just_woke_from_sleep: AtomicBool,
    system_busy: AtomicBool,
    doing_fast_exit: AtomicBool,

    hydrus_temp_dir: Mutex<Option<PathBuf>>,

    self_ref: RwLock<Weak<HydrusController>>,

    hooks: Arc<dyn ControllerHooks>,
}

impl HydrusController {
    /// `HydrusController( db_dir, logger )`
    ///
    /// The hooks are the Rust stand-in for the Python subclass.
    pub fn new(
        db_dir: impl Into<PathBuf>,
        logger: Arc<HydrusLogger>,
        hooks: Arc<dyn ControllerHooks>,
    ) -> Arc<Self> {
        let db_dir = db_dir.into();

        let valid_hooks = Arc::clone(&hooks);

        let pubsub = PubSub::with_valid_callable(move |object| valid_hooks.pubsub_valid(object));

        let controller = Arc::new(Self {
            name: RwLock::new("hydrus".to_string()),
            boot_id: generate_key(),
            last_shutdown_was_bad: AtomicBool::new(false),
            i_own_running_file: AtomicBool::new(false),
            db_dir,
            logger,
            db: RwLock::new(None),
            pubsub,
            daemon_jobs: RwLock::new(HashMap::new()),
            managers: RwLock::new(HashMap::new()),
            fast_job_scheduler: RwLock::new(None),
            slow_job_scheduler: RwLock::new(None),
            thread_slots: Mutex::new(HashMap::new()),
            thread_worker_pool: OnceLock::new(),
            timestamps: Mutex::new(HashMap::new()),
            sleep_lock: Mutex::new(()),
            just_woke_from_sleep: AtomicBool::new(false),
            system_busy: AtomicBool::new(false),
            doing_fast_exit: AtomicBool::new(false),
            hydrus_temp_dir: Mutex::new(None),
            self_ref: RwLock::new(Weak::new()),
            hooks,
        });

        // `Arc::new_cyclic` refuses to upgrade its weak reference, so the
        // self-reference and the worker pool are wired up immediately after
        // construction instead.
        *controller
            .self_ref
            .write()
            .unwrap_or_else(|e| e.into_inner()) = Arc::downgrade(&controller);

        let threading_controller: Arc<dyn ThreadingController> =
            Arc::clone(&controller) as Arc<dyn ThreadingController>;

        let _ = controller
            .thread_worker_pool
            .set(ThreadWorkerPool::new(threading_controller));

        controller.touch_time("boot");
        controller.touch_time("last_sleep_check");

        controller
    }

    // -----------------------------------------------------------------------
    // Naming / identity
    // -----------------------------------------------------------------------

    pub fn get_name(&self) -> String {
        self.name.read().unwrap_or_else(|e| e.into_inner()).clone()
    }

    /// Python subclasses just assign `self._name = 'client'`.
    pub fn set_name(&self, name: impl Into<String>) {
        *self.name.write().unwrap_or_else(|e| e.into_inner()) = name.into();
    }

    pub fn get_boot_id(&self) -> &[u8; HYDRUS_KEY_LENGTH] {
        &self.boot_id
    }

    pub fn get_db_dir(&self) -> &Path {
        &self.db_dir
    }

    pub fn get_logger(&self) -> &Arc<HydrusLogger> {
        &self.logger
    }

    pub fn hooks(&self) -> &dyn ControllerHooks {
        self.hooks.as_ref()
    }

    // -----------------------------------------------------------------------
    // Timestamps and sleep detection
    // -----------------------------------------------------------------------

    pub fn get_timestamp_ms(&self, name: &str) -> i64 {
        *lock(&self.timestamps).get(name).unwrap_or(&0)
    }

    pub fn set_timestamp_ms(&self, name: &str, timestamp_ms: i64) {
        lock(&self.timestamps).insert(name.to_string(), timestamp_ms);
    }

    pub fn touch_time(&self, name: &str) {
        lock(&self.timestamps).insert(name.to_string(), get_now_ms());
    }

    pub fn get_boot_timestamp_ms(&self) -> i64 {
        self.get_timestamp_ms("boot")
    }

    /// `reset_idle_timer()`
    pub fn reset_idle_timer(&self) {
        self.touch_time("last_user_action");
    }

    /// `sleep_check()`
    pub fn sleep_check(&self) {
        let _guard = lock(&self.sleep_lock);

        if time_has_passed_ms(Some(self.get_timestamp_ms("last_sleep_check") + 60000)) {
            // It has been way too long since this method last fired, so we have
            // probably been asleep.
            self.just_woke_from_sleep.store(true, Ordering::SeqCst);

            // This stops background jobs kicking in as soon as the grace period
            // is over.
            self.reset_idle_timer();

            let wake_delay_period_ms = self.hooks.get_wake_delay_period_ms();

            // Enough time for ethernet to get back online and all that.
            self.set_timestamp_ms("now_awake", get_now_ms() + wake_delay_period_ms);

            self.hooks.show_just_woke_to_user();
        } else if self.just_woke_from_sleep.load(Ordering::SeqCst)
            && time_has_passed_ms(Some(self.get_timestamp_ms("now_awake")))
        {
            self.just_woke_from_sleep.store(false, Ordering::SeqCst);
        }

        self.touch_time("last_sleep_check");
    }

    /// `simulate_wake_from_sleep_event()`
    pub fn simulate_wake_from_sleep_event(&self) {
        {
            let _guard = lock(&self.sleep_lock);

            self.set_timestamp_ms("last_sleep_check", get_now_ms() - (3600 * 1000));
        }

        self.sleep_check();
    }

    /// `just_woke_from_sleep()`
    pub fn just_woke_from_sleep(&self) -> bool {
        self.sleep_check();

        self.just_woke_from_sleep.load(Ordering::SeqCst)
    }

    /// `system_busy()`
    pub fn system_busy(&self) -> bool {
        self.system_busy.load(Ordering::SeqCst)
    }

    /// Python sets `self._system_busy` from the client/server layer.
    pub fn set_system_busy(&self, value: bool) {
        self.system_busy.store(value, Ordering::SeqCst);
    }

    /// `doing_fast_exit()`
    pub fn doing_fast_exit(&self) -> bool {
        self.doing_fast_exit.load(Ordering::SeqCst) || hydrus_globals::doing_fast_exit()
    }

    /// Python sets `self._doing_fast_exit = True` from shutdown code. The port
    /// keeps the process-global flag in sync so `HydrusThreading` sees it too.
    pub fn set_doing_fast_exit(&self, value: bool) {
        self.doing_fast_exit.store(value, Ordering::SeqCst);
        hydrus_globals::set_doing_fast_exit(value);
    }

    /// `good_time_to_start_background_work()`
    pub fn good_time_to_start_background_work(&self) -> bool {
        self.currently_idle() && !(self.just_woke_from_sleep() || self.system_busy())
    }

    /// `good_time_to_start_foreground_work()`
    pub fn good_time_to_start_foreground_work(&self) -> bool {
        !self.just_woke_from_sleep()
    }

    /// `currently_idle()`
    pub fn currently_idle(&self) -> bool {
        self.hooks.currently_idle()
    }

    // -----------------------------------------------------------------------
    // Pub/sub
    // -----------------------------------------------------------------------

    /// The underlying hub, for callers that need `process()`/`wait_on_pub()`.
    pub fn pubsub(&self) -> &PubSub {
        &self.pubsub
    }

    /// `pub( topic, *args, **kwargs )`
    ///
    /// Python passes arbitrary args/kwargs; the Rust port carries them in a
    /// [`PubSubMessage`]. Named `publish` because `pub` is a Rust keyword.
    pub fn publish(&self, message: PubSubMessage) {
        if hydrus_globals::model_shutdown() {
            self.pubsub.publish_immediate(message);
        } else {
            self.pubsub.publish(message);
        }
    }

    /// `pubimmediate( topic, *args, **kwargs )`
    pub fn pubimmediate(&self, message: PubSubMessage) {
        self.pubsub.publish_immediate(message);
    }

    /// `sub( object, method_name, topic )`
    pub fn sub<T>(&self, object: &Arc<T>, method_name: impl Into<String>, topic: impl Into<String>)
    where
        T: Subscriber + 'static,
    {
        self.pubsub.sub(object, method_name, topic);
    }

    /// `sub` for type-erased subscribers.
    pub fn sub_dyn(
        &self,
        object: &Arc<dyn Subscriber>,
        method_name: impl Into<String>,
        topic: impl Into<String>,
    ) {
        self.pubsub.sub_dyn(object, method_name, topic);
    }

    /// `currently_pub_subbing()`
    pub fn currently_pub_subbing(&self) -> bool {
        !self.pubsub.currently_idle()
    }

    // -----------------------------------------------------------------------
    // Thread slots
    // -----------------------------------------------------------------------

    /// `acquire_thread_slot( thread_type )`
    pub fn acquire_thread_slot(&self, thread_type: &str) -> bool {
        let mut slots = lock(&self.thread_slots);

        match slots.get_mut(thread_type) {
            None => true, // Assume no max if no max is set.
            Some((current_threads, max_threads)) => {
                if *current_threads < *max_threads {
                    *current_threads += 1;

                    true
                } else {
                    false
                }
            }
        }
    }

    /// `release_thread_slot( thread_type )`
    pub fn release_thread_slot(&self, thread_type: &str) {
        let mut slots = lock(&self.thread_slots);

        if let Some((current_threads, _max_threads)) = slots.get_mut(thread_type) {
            *current_threads = current_threads.saturating_sub(1);
        }
    }

    /// `thread_slots_are_available( thread_type )`
    pub fn thread_slots_are_available(&self, thread_type: &str) -> bool {
        let slots = lock(&self.thread_slots);

        match slots.get(thread_type) {
            None => true,
            Some((current_threads, max_threads)) => current_threads < max_threads,
        }
    }

    /// Python's client/server set `self._thread_slots[ lock_name ] = ( 0, max )`.
    pub fn set_thread_slot(&self, thread_type: impl Into<String>, max_threads: usize) {
        lock(&self.thread_slots).insert(thread_type.into(), (0, max_threads));
    }

    /// Python's client/server updates only the max, keeping the current count.
    pub fn set_thread_slot_max_threads(&self, thread_type: impl Into<String>, max_threads: usize) {
        lock(&self.thread_slots)
            .entry(thread_type.into())
            .and_modify(|slot| slot.1 = max_threads)
            .or_insert((0, max_threads));
    }

    // -----------------------------------------------------------------------
    // Managers
    // -----------------------------------------------------------------------

    /// `get_manager( name )`
    pub fn get_manager(&self, name: &str) -> Option<Arc<dyn Any + Send + Sync>> {
        self.managers
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .get(name)
            .cloned()
    }

    /// Python assigns `self._managers[ name ] = obj` directly.
    pub fn set_manager(&self, name: impl Into<String>, manager: Arc<dyn Any + Send + Sync>) {
        self.managers
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .insert(name.into(), manager);
    }

    pub fn delete_manager(&self, name: &str) {
        self.managers
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .remove(name);
    }

    // -----------------------------------------------------------------------
    // Thread pool
    // -----------------------------------------------------------------------

    /// `call_to_thread( callable, *args, **kwargs )`
    pub fn call_to_thread(&self, call: ThreadCall) -> HydrusResult<()> {
        if hydrus_globals::callto_report_mode() {
            safe_print(format!("CallToThread: {}", call.label()));
        }

        let call_to_thread = self.worker_pool().get_call_to_thread()?;

        call_to_thread.put(call);

        Ok(())
    }

    /// `call_to_thread_long_running( callable, *args, **kwargs )`
    pub fn call_to_thread_long_running(&self, call: ThreadCall) -> HydrusResult<()> {
        if hydrus_globals::callto_report_mode() {
            safe_print(format!("CallToThread (long running): {}", call.label()));
        }

        let call_to_thread = self.worker_pool().get_call_to_thread_long_running()?;

        call_to_thread.put(call);

        Ok(())
    }

    /// `get_thread_pool_busy_status()`
    pub fn get_thread_pool_busy_status(&self) -> (String, String) {
        self.worker_pool().get_thread_pool_busy_status()
    }

    fn worker_pool(&self) -> &ThreadWorkerPool {
        self.thread_worker_pool
            .get()
            .expect("the worker pool is initialised by HydrusController::new")
    }

    /// `get_threads_snapshot()`
    pub fn get_threads_snapshot(&self) -> ThreadsSnapshot {
        let (call_to_threads, long_running_call_to_threads) =
            self.worker_pool().get_threads_snapshot();

        ThreadsSnapshot {
            call_to_threads,
            long_running_call_to_threads,
            fast_job_scheduler: self
                .fast_job_scheduler
                .read()
                .unwrap_or_else(|e| e.into_inner())
                .clone(),
            slow_job_scheduler: self
                .slow_job_scheduler
                .read()
                .unwrap_or_else(|e| e.into_inner())
                .clone(),
        }
    }

    // -----------------------------------------------------------------------
    // Job schedulers
    // -----------------------------------------------------------------------

    fn get_appropriate_job_scheduler(&self, time_delta: f64) -> Option<Arc<JobScheduler>> {
        if time_delta <= 1.0 {
            self.fast_job_scheduler
                .read()
                .unwrap_or_else(|e| e.into_inner())
                .clone()
        } else {
            self.slow_job_scheduler
                .read()
                .unwrap_or_else(|e| e.into_inner())
                .clone()
        }
    }

    fn require_job_scheduler(&self, time_delta: f64) -> HydrusResult<Arc<JobScheduler>> {
        self.get_appropriate_job_scheduler(time_delta)
            .ok_or_else(|| {
                HydrusException::new(
                    ErrorKind::Unknown,
                    "The job schedulers are not running! Call init_model() first.",
                )
            })
    }

    /// `call_later( initial_delay, func, *args, **kwargs )`
    ///
    /// `label` replaces Python's introspection of the callable's name; it is
    /// what the scheduler snapshots show.
    pub fn call_later(
        &self,
        initial_delay: f64,
        label: impl Into<String>,
        call: JobCall,
    ) -> HydrusResult<Arc<SchedulableJob>> {
        let scheduler = self.require_job_scheduler(initial_delay)?;

        let job = SchedulableJob::single(self.threading_arc(), &scheduler, initial_delay, call, label);

        scheduler.add_job(Arc::clone(&job));

        Ok(job)
    }

    /// `call_repeating( initial_delay, period, func, *args, **kwargs )`
    pub fn call_repeating(
        &self,
        initial_delay: f64,
        period: f64,
        label: impl Into<String>,
        call: JobCall,
    ) -> HydrusResult<Arc<SchedulableJob>> {
        let scheduler = self.require_job_scheduler(period)?;

        let job = SchedulableJob::repeating(
            self.threading_arc(),
            &scheduler,
            initial_delay,
            period,
            call,
            label,
        );

        scheduler.add_job(Arc::clone(&job));

        Ok(job)
    }

    /// `get_job_scheduler_snapshot( scheduler_name )`
    pub fn get_job_scheduler_snapshot(&self, scheduler_name: &str) -> Vec<Arc<SchedulableJob>> {
        let scheduler = if scheduler_name == "fast" {
            self.fast_job_scheduler
                .read()
                .unwrap_or_else(|e| e.into_inner())
                .clone()
        } else {
            self.slow_job_scheduler
                .read()
                .unwrap_or_else(|e| e.into_inner())
                .clone()
        };

        match scheduler {
            Some(scheduler) => scheduler.get_jobs(),
            None => Vec::new(),
        }
    }

    /// `debug_show_scheduled_jobs()`
    pub fn debug_show_scheduled_jobs(&self) {
        let fast_summary = self
            .fast_job_scheduler
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .as_ref()
            .map(|scheduler| scheduler.get_pretty_job_summary())
            .unwrap_or_else(|| "not running".to_string());

        safe_print("fast scheduler:");
        safe_print(fast_summary);

        let slow_summary = self
            .slow_job_scheduler
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .as_ref()
            .map(|scheduler| scheduler.get_pretty_job_summary())
            .unwrap_or_else(|| "not running".to_string());

        safe_print("slow scheduler:");
        safe_print(slow_summary);
    }

    // -----------------------------------------------------------------------
    // Daemons
    // -----------------------------------------------------------------------

    fn daemon_job(&self, name: &str) -> Option<Arc<SchedulableJob>> {
        self.daemon_jobs
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .get(name)
            .cloned()
    }

    /// `wake_daemon( name )`
    pub fn wake_daemon(&self, name: &str) {
        if let Some(job) = self.daemon_job(name) {
            job.wake(None);
        }
    }

    /// The daemon jobs currently registered, for status reporting.
    pub fn get_daemon_job_names(&self) -> Vec<String> {
        self.daemon_jobs
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .keys()
            .cloned()
            .collect()
    }

    /// `_shutdown_daemons()`
    fn shutdown_daemons(&self) {
        let jobs: Vec<Arc<SchedulableJob>> = self
            .daemon_jobs
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .values()
            .cloned()
            .collect();

        for job in &jobs {
            job.cancel();
        }

        if !self.doing_fast_exit() {
            let started = get_now();

            while jobs.iter().any(|job| job.currently_working()) {
                self.hooks.report_shutdown_daemons_status();

                std::thread::sleep(Duration::from_millis(100));

                if time_has_passed(Some(started + 30)) {
                    break;
                }
            }
        }

        self.daemon_jobs
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .clear();
    }

    // -----------------------------------------------------------------------
    // Database delegation
    // -----------------------------------------------------------------------

    pub fn db(&self) -> Option<Arc<HydrusDB>> {
        self.db.read().unwrap_or_else(|e| e.into_inner()).clone()
    }

    /// Python's client/server subclass assigns `self.db` in `_init_db`.
    pub fn set_db(&self, db: Option<Arc<HydrusDB>>) {
        *self.db.write().unwrap_or_else(|e| e.into_inner()) = db;
    }

    /// `read( action, *args, **kwargs )`
    pub fn read(&self, action: &str) -> HydrusResult<Box<dyn Any + Send>> {
        match self.db() {
            Some(db) => db.read(action),
            None => Err(HydrusException::new(
                ErrorKind::Unknown,
                "The database is not running!",
            )),
        }
    }

    /// `write( action, *args, **kwargs )`
    pub fn write(&self, action: &str) -> HydrusResult<()> {
        self.write_internal(action, false)?;

        Ok(())
    }

    /// `write_synchronous( action, *args, **kwargs )`
    pub fn write_synchronous(&self, action: &str) -> HydrusResult<Box<dyn Any + Send>> {
        self.write_internal(action, true)?
            .ok_or_else(|| HydrusException::new(ErrorKind::Unknown, "The database did not return a result!"))
    }

    fn write_internal(
        &self,
        action: &str,
        synchronous: bool,
    ) -> HydrusResult<Option<Box<dyn Any + Send>>> {
        match self.db() {
            Some(db) => db.write(action, synchronous),
            None => Err(HydrusException::new(
                ErrorKind::Unknown,
                "The database is not running!",
            )),
        }
    }

    /// `db_currently_doing_job()`
    pub fn db_currently_doing_job(&self) -> bool {
        self.db().is_some_and(|db| db.currently_doing_job())
    }

    /// `is_first_start()`
    pub fn is_first_start(&self) -> bool {
        self.db().is_some_and(|db| db.is_first_start())
    }

    /// `get_db_status()`
    pub fn get_db_status(&self) -> Option<(String, String)> {
        self.db().map(|db| db.get_status())
    }

    /// `force_database_commit()`
    pub fn force_database_commit(&self) -> HydrusResult<()> {
        match self.db() {
            Some(db) => db.force_a_commit(),
            None => Err(HydrusException::new(
                ErrorKind::Unknown,
                "Sorry, database does not seem to be alive at the moment!",
            )),
        }
    }

    /// `wait_until_model_free()`
    pub fn wait_until_model_free(&self) {
        self.pubsub.wait_until_free();

        if let Some(db) = self.db() {
            db.wait_until_free();
        }
    }

    // -----------------------------------------------------------------------
    // Temp dir
    // -----------------------------------------------------------------------

    /// `_init_hydrus_temp_dir()`
    pub fn init_hydrus_temp_dir(&self) -> HydrusResult<PathBuf> {
        let path = hydrus_temp::initialise_hydrus_temp_dir()?;

        // Python stores this on the controller and every `HydrusTemp` helper
        // asks `HG.controller`. The port also publishes it to the temp
        // module's process-global so those helpers work without a controller.
        hydrus_temp::set_hydrus_temp_dir(&path);

        *lock(&self.hydrus_temp_dir) = Some(path.clone());

        Ok(path)
    }

    /// `get_hydrus_temp_dir()`
    pub fn get_hydrus_temp_dir(&self) -> HydrusResult<PathBuf> {
        let existing = lock(&self.hydrus_temp_dir).clone();

        match existing {
            Some(path) if path.exists() => Ok(path),
            _ => self.init_hydrus_temp_dir(),
        }
    }

    // -----------------------------------------------------------------------
    // Running file
    // -----------------------------------------------------------------------

    /// `record_running_start()`
    pub fn record_running_start(&self) {
        self.last_shutdown_was_bad.store(
            last_shutdown_was_bad(&self.db_dir, &self.get_name()),
            Ordering::SeqCst,
        );

        self.i_own_running_file.store(true, Ordering::SeqCst);

        hydrus_process::record_running_start(&self.db_dir, &self.get_name());
    }

    /// `clean_running_file()`
    pub fn clean_running_file(&self) {
        if self.i_own_running_file.load(Ordering::SeqCst) {
            clean_running_file(&self.db_dir, &self.get_name());
        }
    }

    /// `last_shutdown_was_bad()`
    pub fn last_shutdown_was_bad(&self) -> bool {
        self.last_shutdown_was_bad.load(Ordering::SeqCst)
    }

    // -----------------------------------------------------------------------
    // Maintenance
    // -----------------------------------------------------------------------

    /// `maintain_db( maintenance_mode, stop_time )`
    pub fn maintain_db(&self, maintenance_mode: MaintenanceMode, stop_time: Option<i64>) {
        self.hooks.maintain_db(self, maintenance_mode, stop_time);
    }

    /// `maintain_memory_fast()`
    pub fn maintain_memory_fast(&self) {
        let _ = std::io::stdout().flush();
        let _ = std::io::stderr().flush();

        self.publish(PubSubMessage::new("memory_maintenance_pulse"));

        if let Some(scheduler) = self
            .fast_job_scheduler
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .as_ref()
        {
            scheduler.clear_out_dead();
        }

        if let Some(scheduler) = self
            .slow_job_scheduler
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .as_ref()
        {
            scheduler.clear_out_dead();
        }

        hydrus_subprocess::reap_dead_long_lived_external_processes();
    }

    /// `maintain_memory_slow()`
    pub fn maintain_memory_slow(&self) {
        hydrus_temp::clean_up_old_temp_paths();

        self.worker_pool().maintain_call_to_threads();
    }

    /// `should_stop_this_work( maintenance_mode, stop_time )`
    pub fn should_stop_this_work(
        &self,
        maintenance_mode: MaintenanceMode,
        stop_time: Option<i64>,
    ) -> bool {
        if maintenance_mode == MaintenanceMode::Idle && !self.currently_idle() {
            return true;
        }

        if maintenance_mode == MaintenanceMode::Shutdown && !hydrus_globals::do_idle_shutdown_work()
        {
            return true;
        }

        if let Some(stop_time) = stop_time {
            if time_has_passed(Some(stop_time)) {
                return true;
            }
        }

        false
    }

    /// `clear_caches()`
    pub fn clear_caches(&self) {
        self.hooks.clear_caches();
    }

    /// `report_data_used( num_bytes )`
    pub fn report_data_used(&self, num_bytes: u64) {
        self.hooks.report_data_used(num_bytes);
    }

    /// `report_request_used()`
    pub fn report_request_used(&self) {
        self.hooks.report_request_used();
    }

    // -----------------------------------------------------------------------
    // Model / view lifetime
    // -----------------------------------------------------------------------

    /// `init_model()`
    pub fn init_model(self: &Arc<Self>) -> HydrusResult<()> {
        if self.init_hydrus_temp_dir().is_err() {
            safe_print("Failed to initialise temp folder.");
        }

        self.hooks.initialise_mimes_to_default_thumbnail_paths();

        let fast_scheduler = JobScheduler::new(self.threading_arc());
        fast_scheduler.start()?;

        *self.fast_job_scheduler.write().unwrap_or_else(|e| e.into_inner()) =
            Some(Arc::clone(&fast_scheduler));

        let slow_scheduler = JobScheduler::new(self.threading_arc());
        slow_scheduler.start()?;

        *self.slow_job_scheduler.write().unwrap_or_else(|e| e.into_inner()) =
            Some(Arc::clone(&slow_scheduler));

        if let Some(db) = self.hooks.init_db(self)? {
            self.set_db(Some(Arc::clone(&db)));

            let commit_db = Arc::clone(&db);

            self.call_repeating(
                5.0,
                15.0,
                "db commit check",
                Arc::new(move || {
                    commit_db.schedule_commit_check();

                    Ok(())
                }),
            )?;
        }

        // Reset after a long db update.
        self.touch_time("last_sleep_check");

        self.hooks.after_init_model(self)
    }

    /// `init_view()`
    pub fn init_view(self: &Arc<Self>) -> HydrusResult<()> {
        let controller = Arc::clone(self);

        let job = self.call_repeating(
            60.0,
            300.0,
            "maintain db",
            Arc::new(move || {
                controller.maintain_db(MaintenanceMode::Idle, None);

                Ok(())
            }),
        )?;

        job.wake_on_pub_sub("wake_idle_workers");
        job.should_delay_on_wakeup(true);

        self.daemon_jobs
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .insert("maintain_db".to_string(), job);

        let controller = Arc::clone(self);

        let job = self.call_repeating(
            0.0,
            15.0,
            "sleep check",
            Arc::new(move || {
                controller.sleep_check();

                Ok(())
            }),
        )?;

        self.daemon_jobs
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .insert("sleep_check".to_string(), job);

        let controller = Arc::clone(self);

        let job = self.call_repeating(
            10.0,
            60.0,
            "maintain memory fast",
            Arc::new(move || {
                controller.maintain_memory_fast();

                Ok(())
            }),
        )?;

        self.daemon_jobs
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .insert("maintain_memory_fast".to_string(), job);

        let controller = Arc::clone(self);

        let job = self.call_repeating(
            10.0,
            300.0,
            "maintain memory slow",
            Arc::new(move || {
                controller.maintain_memory_slow();

                Ok(())
            }),
        )?;

        self.daemon_jobs
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .insert("maintain_memory_slow".to_string(), job);

        self.hooks.init_view(self)
    }

    /// Stop the fast and slow job schedulers.
    ///
    /// Python does this inline in `shutdown_model`; it is a method here so
    /// tests and partial shutdowns can stop them without touching the global
    /// `model_shutdown` flag.
    pub fn shutdown_job_schedulers(&self) {
        if let Some(scheduler) = self
            .fast_job_scheduler
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .take()
        {
            scheduler.shutdown();
        }

        if let Some(scheduler) = self
            .slow_job_scheduler
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .take()
        {
            scheduler.shutdown();
        }
    }

    /// `shutdown_model()`
    pub fn shutdown_model(&self) {
        self.hooks.stop_twisted_if_running();

        self.shutdown_job_schedulers();

        self.worker_pool().shutdown();

        self.pubsub.shutdown();

        if let Some(db) = self.db() {
            db.shutdown();

            if !self.doing_fast_exit() {
                while !db.loop_is_finished() {
                    self.hooks.publish_shutdown_subtext(&format!(
                        "waiting for db to finish up{UNICODE_ELLIPSIS}"
                    ));

                    std::thread::sleep(Duration::from_millis(100));
                }
            }
        }

        hydrus_temp::clean_up_old_temp_paths();

        let temp_dir = lock(&self.hydrus_temp_dir).take();

        if let Some(temp_dir) = temp_dir {
            if !delete_path(&temp_dir) {
                safe_print(format!(
                    "While shutting down, could not delete the temp dir \"{}\"!",
                    temp_dir.display()
                ));
            }
        }

        hydrus_globals::set_model_shutdown(true);
    }

    /// `shutdown_view()`
    pub fn shutdown_view(&self) {
        hydrus_globals::set_view_shutdown(true);

        self.shutdown_daemons();
    }

    /// `shutdown_from_server()`
    pub fn shutdown_from_server(&self) -> HydrusResult<()> {
        self.hooks.shutdown_from_server()
    }

    /// `start_twisted_if_needed()`
    pub fn start_twisted_if_needed(&self) -> HydrusResult<()> {
        self.hooks.start_twisted_if_needed()
    }

    /// `stop_twisted_if_running()`
    pub fn stop_twisted_if_running(&self) {
        self.hooks.stop_twisted_if_running();
    }

    // -----------------------------------------------------------------------
    // Console fallbacks
    // -----------------------------------------------------------------------

    fn wait_for_enter() {
        safe_print("Press Enter to continue.");

        let mut line = String::new();

        let _ = std::io::stdin().read_line(&mut line);
    }

    /// `blocking_safe_show_critical_message( title, message )`
    pub fn blocking_safe_show_critical_message(&self, title: &str, message: &str) {
        debug_print(title);
        debug_print(message);

        Self::wait_for_enter();
    }

    /// `blocking_safe_show_message( message )`
    pub fn blocking_safe_show_message(&self, message: &str) {
        debug_print(message);

        Self::wait_for_enter();
    }

    // -----------------------------------------------------------------------
    // Internals
    // -----------------------------------------------------------------------

    fn threading_arc(&self) -> Arc<dyn ThreadingController> {
        // Schedulers and scheduled jobs hold this Arc, which creates a
        // controller <-> job cycle (just as the Python object graph is cyclic).
        // A process has one long-lived controller, so the cycle is harmless;
        // `shutdown_model` still stops all the work.
        let controller = self
            .self_ref
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .upgrade()
            .expect("the controller is still alive while running jobs");

        controller as Arc<dyn ThreadingController>
    }
}

impl ThreadingController for HydrusController {
    fn doing_fast_exit(&self) -> bool {
        HydrusController::doing_fast_exit(self)
    }

    fn subscribe(&self, subscriber: Arc<dyn Subscriber>, method_name: &str, topic: &str) {
        self.pubsub.sub_dyn(&subscriber, method_name, topic);
    }

    fn call_to_thread(&self, call: ThreadCall) -> HydrusResult<()> {
        HydrusController::call_to_thread(self, call)
    }

    fn acquire_thread_slot(&self, thread_type: &str) -> bool {
        HydrusController::acquire_thread_slot(self, thread_type)
    }

    fn release_thread_slot(&self, thread_type: &str) {
        HydrusController::release_thread_slot(self, thread_type);
    }

    fn thread_slots_are_available(&self, thread_type: &str) -> bool {
        HydrusController::thread_slots_are_available(self, thread_type)
    }

    fn just_woke_from_sleep(&self) -> bool {
        HydrusController::just_woke_from_sleep(self)
    }

    fn good_time_to_start_background_work(&self) -> bool {
        HydrusController::good_time_to_start_background_work(self)
    }

    fn good_time_to_start_foreground_work(&self) -> bool {
        HydrusController::good_time_to_start_foreground_work(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::processes::hydrus_threading::thread_call;
    use std::sync::atomic::AtomicUsize;
    use std::sync::Mutex as StdMutex;

    /// The temp-dir tests touch a process-global path, so keep them apart.
    static TEMP_DIR_TEST_LOCK: StdMutex<()> = StdMutex::new(());

    struct TestHooks {
        thumbnails_initialised: AtomicBool,
        maintain_db_calls: AtomicUsize,
        wake_delay_ms: i64,
        shutdown_subtexts: Mutex<Vec<String>>,
    }

    impl TestHooks {
        fn new(wake_delay_ms: i64) -> Arc<Self> {
            Arc::new(Self {
                thumbnails_initialised: AtomicBool::new(false),
                maintain_db_calls: AtomicUsize::new(0),
                wake_delay_ms,
                shutdown_subtexts: Mutex::new(Vec::new()),
            })
        }
    }

    impl ControllerHooks for TestHooks {
        fn get_wake_delay_period_ms(&self) -> i64 {
            self.wake_delay_ms
        }

        fn initialise_mimes_to_default_thumbnail_paths(&self) {
            self.thumbnails_initialised.store(true, Ordering::SeqCst);
        }

        fn maintain_db(
            &self,
            _controller: &HydrusController,
            _maintenance_mode: MaintenanceMode,
            _stop_time: Option<i64>,
        ) {
            self.maintain_db_calls.fetch_add(1, Ordering::SeqCst);
        }

        fn publish_shutdown_subtext(&self, text: &str) {
            lock(&self.shutdown_subtexts).push(text.to_string());
        }
    }

    fn test_db_dir(label: &str) -> PathBuf {
        let suffix = format!(
            "{}-{}-{}",
            label,
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );

        std::env::temp_dir().join("hydrus-controller-tests").join(suffix)
    }

    fn make_controller(hooks: Arc<TestHooks>) -> Arc<HydrusController> {
        let db_dir = test_db_dir("controller");

        let logger = Arc::new(HydrusLogger::new(&db_dir, "controller-test").unwrap());

        HydrusController::new(&db_dir, logger, hooks)
    }

    struct CountingSubscriber {
        count: AtomicUsize,
    }

    impl Subscriber for CountingSubscriber {
        fn has_method(&self, method_name: &str) -> bool {
            method_name == "bump"
        }

        fn call(
            &self,
            _method_name: &str,
            _message: &PubSubMessage,
        ) -> HydrusResult<()> {
            self.count.fetch_add(1, Ordering::SeqCst);

            Ok(())
        }
    }

    #[test]
    fn timestamps_start_at_boot_and_can_be_touched() {
        let controller = make_controller(TestHooks::new(15000));

        assert!(controller.get_boot_timestamp_ms() > 0);
        assert_eq!(controller.get_boot_timestamp_ms(), controller.get_timestamp_ms("boot"));

        controller.set_timestamp_ms("custom", 12345);

        assert_eq!(controller.get_timestamp_ms("custom"), 12345);
        assert_eq!(controller.get_timestamp_ms("missing"), 0);

        controller.touch_time("custom");

        assert!(controller.get_timestamp_ms("custom") > 12345);
    }

    #[test]
    fn thread_slots_are_counted_and_released() {
        let controller = make_controller(TestHooks::new(15000));

        // No max set means unlimited.
        assert!(controller.acquire_thread_slot("unbounded"));
        assert!(controller.thread_slots_are_available("unbounded"));

        controller.set_thread_slot("import", 1);

        assert!(controller.thread_slots_are_available("import"));
        assert!(controller.acquire_thread_slot("import"));
        assert!(!controller.thread_slots_are_available("import"));
        assert!(!controller.acquire_thread_slot("import"));

        controller.release_thread_slot("import");

        assert!(controller.acquire_thread_slot("import"));

        // Releasing more than acquired does not go negative.
        controller.release_thread_slot("import");
        controller.release_thread_slot("import");
        controller.release_thread_slot("import");

        assert!(controller.acquire_thread_slot("import"));
    }

    #[test]
    fn sleep_check_detects_a_long_gap_and_clears_after_the_wake_delay() {
        let hooks = TestHooks::new(50);
        let controller = make_controller(hooks);

        controller.set_timestamp_ms("last_sleep_check", get_now_ms() - 3600 * 1000);

        assert!(!controller.just_woke_from_sleep.load(Ordering::SeqCst));

        controller.sleep_check();

        assert!(controller.just_woke_from_sleep.load(Ordering::SeqCst));
        assert!(controller.get_timestamp_ms("now_awake") >= get_now_ms());

        // After the wake delay has elapsed, the flag clears.
        controller.set_timestamp_ms("now_awake", get_now_ms() - 1);
        controller.sleep_check();

        assert!(!controller.just_woke_from_sleep.load(Ordering::SeqCst));
    }

    #[test]
    fn simulate_wake_from_sleep_sets_the_flag() {
        let hooks = TestHooks::new(0);
        let controller = make_controller(hooks);

        controller.simulate_wake_from_sleep_event();

        assert!(controller.just_woke_from_sleep.load(Ordering::SeqCst));
    }

    #[test]
    fn pub_and_sub_round_trip() {
        let controller = make_controller(TestHooks::new(15000));

        let subscriber = Arc::new(CountingSubscriber {
            count: AtomicUsize::new(0),
        });

        controller.sub(&subscriber, "bump", "test topic");

        controller.publish(PubSubMessage::new("test topic"));

        assert!(controller.pubsub().work_to_do());
        assert!(controller.pubsub().process());
        assert_eq!(subscriber.count.load(Ordering::SeqCst), 1);

        controller.pubimmediate(PubSubMessage::new("test topic"));

        assert_eq!(subscriber.count.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn managers_are_stored_by_name() {
        let controller = make_controller(TestHooks::new(15000));

        assert!(controller.get_manager("nothing").is_none());

        let manager: Arc<dyn Any + Send + Sync> = Arc::new(41u64);

        controller.set_manager("answer", manager);

        let stored = controller.get_manager("answer").unwrap();

        assert_eq!(*stored.downcast::<u64>().unwrap(), 41);

        controller.delete_manager("answer");

        assert!(controller.get_manager("answer").is_none());
    }

    #[test]
    fn call_later_and_repeating_need_running_schedulers() {
        let controller = make_controller(TestHooks::new(15000));

        let result = controller.call_later(0.0, "too early", Arc::new(|| Ok(())));

        assert!(result.is_err());
    }

    #[test]
    fn init_model_starts_schedulers_and_runs_queued_calls() {
        let _guard = TEMP_DIR_TEST_LOCK.lock().unwrap();

        let hooks = TestHooks::new(15000);
        let controller = make_controller(Arc::clone(&hooks));

        controller.init_model().unwrap();

        assert!(hooks.thumbnails_initialised.load(Ordering::SeqCst));

        let snapshot = controller.get_threads_snapshot();

        assert!(snapshot.fast_job_scheduler.is_some());
        assert!(snapshot.slow_job_scheduler.is_some());

        let counter = Arc::new(AtomicUsize::new(0));
        let counter_for_job = Arc::clone(&counter);

        controller
            .call_later(
                0.0,
                "test call later",
                Arc::new(move || {
                    counter_for_job.fetch_add(1, Ordering::SeqCst);

                    Ok(())
                }),
            )
            .unwrap();

        let deadline = std::time::Instant::now() + Duration::from_secs(5);

        while counter.load(Ordering::SeqCst) == 0 && std::time::Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(10));
        }

        assert_eq!(counter.load(Ordering::SeqCst), 1);

        controller.shutdown_job_schedulers();

        assert!(controller.get_threads_snapshot().fast_job_scheduler.is_none());
        assert!(controller.get_threads_snapshot().slow_job_scheduler.is_none());
    }

    #[test]
    fn init_view_registers_daemons_and_shutdown_view_clears_them() {
        let _guard = TEMP_DIR_TEST_LOCK.lock().unwrap();

        let hooks = TestHooks::new(15000);
        let controller = make_controller(hooks);

        controller.init_model().unwrap();
        controller.init_view().unwrap();

        let mut names = controller.get_daemon_job_names();
        names.sort();

        assert_eq!(
            names,
            vec![
                "maintain_db",
                "maintain_memory_fast",
                "maintain_memory_slow",
                "sleep_check"
            ]
        );

        controller.shutdown_view();

        assert!(hydrus_globals::view_shutdown());
        assert!(controller.get_daemon_job_names().is_empty());

        controller.shutdown_job_schedulers();

        // Restore the global for other tests.
        hydrus_globals::set_view_shutdown(false);
    }

    #[test]
    fn database_calls_fail_cleanly_without_a_database() {
        let controller = make_controller(TestHooks::new(15000));

        assert!(controller.db().is_none());
        assert!(controller.read("options").is_err());
        assert!(controller.write("options").is_err());
        assert!(controller.write_synchronous("options").is_err());
        assert!(controller.force_database_commit().is_err());
        assert!(!controller.db_currently_doing_job());
        assert!(!controller.is_first_start());
        assert!(controller.get_db_status().is_none());
        assert!(controller.db().is_none());
    }

    #[test]
    fn call_to_thread_runs_the_closure() {
        let controller = make_controller(TestHooks::new(15000));

        let counter = Arc::new(AtomicUsize::new(0));
        let counter_for_call = Arc::clone(&counter);

        let call = thread_call("test call", move || {
            counter_for_call.fetch_add(1, Ordering::SeqCst);
        });

        controller.call_to_thread(call).unwrap();

        let deadline = std::time::Instant::now() + Duration::from_secs(5);

        while counter.load(Ordering::SeqCst) == 0 && std::time::Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(10));
        }

        assert_eq!(counter.load(Ordering::SeqCst), 1);

        controller.worker_pool().shutdown();
    }

    #[test]
    fn temp_dir_is_created_and_reused() {
        let _guard = TEMP_DIR_TEST_LOCK.lock().unwrap();

        let controller = make_controller(TestHooks::new(15000));

        let first = controller.get_hydrus_temp_dir().unwrap();

        assert!(first.exists());
        assert_eq!(first, hydrus_temp::get_hydrus_temp_dir());

        let second = controller.get_hydrus_temp_dir().unwrap();

        assert_eq!(first, second);

        // Remove it so the next call re-initialises.
        std::fs::remove_dir_all(&first).unwrap();

        let third = controller.get_hydrus_temp_dir().unwrap();

        assert!(third.exists());
        assert_ne!(first, third);

        // Leave `third` in place: it is the process-global temp dir that the
        // rest of the test binary uses. Deleting it here would make unrelated
        // tests race against a missing parent directory.
    }

    #[test]
    fn running_file_round_trip() {
        let controller = make_controller(TestHooks::new(15000));

        assert!(!controller.last_shutdown_was_bad());

        controller.record_running_start();

        assert!(!controller.last_shutdown_was_bad());
        assert!(controller.i_own_running_file.load(Ordering::SeqCst));

        // `record_running_start` is a no-op while psutil is unavailable, so
        // there is no file to assert on; `clean_running_file` must still be
        // safe to call.
        controller.clean_running_file();
    }

    #[test]
    fn shutdown_model_stops_everything_without_a_database() {
        let _guard = TEMP_DIR_TEST_LOCK.lock().unwrap();

        let controller = make_controller(TestHooks::new(15000));

        controller.init_model().unwrap();

        let temp_dir = controller.get_hydrus_temp_dir().unwrap();

        controller.shutdown_model();

        assert!(hydrus_globals::model_shutdown());
        assert!(!temp_dir.exists());
        assert!(controller.db().is_none());

        // Restore the globals for the other tests in this binary. Shutdown
        // removed the temp dir, so make a fresh one for them to use.
        hydrus_globals::set_model_shutdown(false);
        let _ = hydrus_temp::initialise_hydrus_temp_dir();
    }
}
