//! Call-after queue and GUI singleton state (`ClientGUICallAfter.py`,
//! `ClientGUICore.py`, `ClientGUIAsync.py`).
//!
//! Qt delivers callbacks on its own thread. The Rust port keeps a thread-safe
//! queue that the Qt event loop drains from a timer.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard};

pub type CallAfter = Box<dyn FnOnce() + Send + 'static>;

struct ScheduledCall {
    id: u64,
    run_at_ms: i64,
    call: CallAfter,
}

#[derive(Default)]
pub struct CallAfterQueue {
    next_id: AtomicU64,
    calls: Mutex<Vec<ScheduledCall>>,
}

impl CallAfterQueue {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn schedule(&self, run_at_ms: i64, call: CallAfter) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        self.calls().push(ScheduledCall {
            id,
            run_at_ms,
            call,
        });

        id
    }

    pub fn schedule_immediate(&self, call: CallAfter) -> u64 {
        self.schedule(0, call)
    }

    pub fn schedule_in(&self, now_ms: i64, delay_ms: i64, call: CallAfter) -> u64 {
        self.schedule(now_ms + delay_ms, call)
    }

    pub fn cancel(&self, id: u64) -> bool {
        let mut calls = self.calls();
        let previous_len = calls.len();

        calls.retain(|scheduled| scheduled.id != id);

        previous_len != calls.len()
    }

    pub fn run_due(&self, now_ms: i64) -> usize {
        let due: Vec<CallAfter> = {
            let mut calls = self.calls();
            let mut due = Vec::new();
            let mut remaining = Vec::new();

            for scheduled in calls.drain(..) {
                if scheduled.run_at_ms <= now_ms {
                    due.push(scheduled.call);
                } else {
                    remaining.push(scheduled);
                }
            }

            *calls = remaining;

            due
        };

        let count = due.len();

        for call in due {
            call();
        }

        count
    }

    pub fn len(&self) -> usize {
        self.calls().len()
    }

    pub fn is_empty(&self) -> bool {
        self.calls().is_empty()
    }

    fn calls(&self) -> MutexGuard<'_, Vec<ScheduledCall>> {
        self.calls
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

/// `GUICore`: the tiny amount of global UI state the Python singleton keeps.
#[derive(Debug, Default)]
pub struct GuiCore {
    menu_open: AtomicBool,
}

impl GuiCore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn menu_is_open(&self) -> bool {
        self.menu_open.load(Ordering::Relaxed)
    }

    pub fn menubar_menu_is_open(&self) -> bool {
        self.menu_is_open()
    }

    pub fn menubar_menu_is_closed(&self) {
        self.menu_open.store(false, Ordering::Relaxed);
    }

    pub fn set_menu_open(&self) {
        self.menu_open.store(true, Ordering::Relaxed);
    }

    /// `PopupMenu`
    pub fn popup_menu(&self) {
        self.set_menu_open();
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AsyncState {
    Pending,
    Running,
    Finished,
    Failed(String),
}

/// One background GUI job. `ClientGUIAsync` uses this to gate preview/canvas
/// work while the user changes selection.
#[derive(Debug)]
pub struct AsyncJob {
    state: Mutex<AsyncState>,
}

impl Default for AsyncJob {
    fn default() -> Self {
        Self::new()
    }
}

impl AsyncJob {
    pub fn new() -> Self {
        Self {
            state: Mutex::new(AsyncState::Pending),
        }
    }

    pub fn start(&self) {
        *self.state.lock().unwrap() = AsyncState::Running;
    }

    pub fn finish(&self) {
        *self.state.lock().unwrap() = AsyncState::Finished;
    }

    pub fn fail(&self, error: impl Into<String>) {
        *self.state.lock().unwrap() = AsyncState::Failed(error.into());
    }

    pub fn state(&self) -> AsyncState {
        self.state.lock().unwrap().clone()
    }

    pub fn is_running(&self) -> bool {
        matches!(*self.state.lock().unwrap(), AsyncState::Running)
    }

    pub fn is_finished(&self) -> bool {
        matches!(*self.state.lock().unwrap(), AsyncState::Finished)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::AtomicUsize;

    #[test]
    fn due_calls_run_in_schedule_order() {
        let queue = CallAfterQueue::new();
        let counter = Arc::new(AtomicUsize::new(0));
        let first = counter.clone();
        let second = counter.clone();

        queue.schedule(
            10,
            Box::new(move || {
                first.fetch_add(1, Ordering::SeqCst);
            }),
        );
        queue.schedule(
            20,
            Box::new(move || {
                second.fetch_add(10, Ordering::SeqCst);
            }),
        );

        assert_eq!(queue.run_due(9), 0);
        assert_eq!(queue.run_due(10), 1);
        assert_eq!(counter.load(Ordering::SeqCst), 1);
        assert_eq!(queue.run_due(20), 1);
        assert_eq!(counter.load(Ordering::SeqCst), 11);
    }

    #[test]
    fn cancelling_removes_a_call() {
        let queue = CallAfterQueue::new();

        let id = queue.schedule_immediate(Box::new(|| {}));

        assert!(queue.cancel(id));
        assert!(queue.is_empty());
        assert!(!queue.cancel(id));
    }

    #[test]
    fn relative_schedule_uses_now() {
        let queue = CallAfterQueue::new();
        queue.schedule_in(100, 50, Box::new(|| {}));

        assert_eq!(queue.run_due(149), 0);
        assert_eq!(queue.run_due(150), 1);
    }

    #[test]
    fn gui_core_tracks_menu_state() {
        let core = GuiCore::new();

        assert!(!core.menu_is_open());
        core.set_menu_open();

        assert!(core.menu_is_open());

        core.menubar_menu_is_closed();

        assert!(!core.menu_is_open());
    }

    #[test]
    fn async_job_tracks_lifecycle() {
        let job = AsyncJob::new();

        assert_eq!(job.state(), AsyncState::Pending);

        job.start();
        assert!(job.is_running());

        job.finish();
        assert!(job.is_finished());

        job.fail("nope");
        assert_eq!(job.state(), AsyncState::Failed("nope".to_string()));
    }
}
