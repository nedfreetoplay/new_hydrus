
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
