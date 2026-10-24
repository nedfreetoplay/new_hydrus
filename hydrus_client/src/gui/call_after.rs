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
mod tests;
