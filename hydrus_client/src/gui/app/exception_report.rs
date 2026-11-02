//! Crash/exception report window logic, mirroring Hydrus' popup message manager.
//!
//! Python sources: `hydrus/client/ClientThreading.py` (`JobStatus.ToString`,
//! `SetExceptionTuple`), `hydrus/client/gui/ClientGUIPopupMessages.py`
//! (`PopupMessage`: `TEXT_CUTOFF`, `CopyTB`, `ShowTB`),
//! `hydrus/client/ClientData.py` (`ShowExceptionTupleClient`), and
//! `hydrus/client/gui/ClientGUIExceptionHandling.py`
//! (`HandlePaintEventException`). Qt only applies the ready-made strings.

use std::sync::Mutex;

use crate::gui::qt::api::ffi;

/// Maximum characters shown inline before the Python truncation notice kicks
/// in (`PopupMessage.TEXT_CUTOFF`).
pub const TEXT_CUTOFF: usize = 1024;

/// Verbatim truncation notice from `PopupMessage._ProcessText`.
pub const TRUNCATION_NOTICE: &str =
    "The text is too long to display here. Here is the start of it (the rest is printed to the log):";

/// Verbatim button labels from `PopupMessage`.
pub const SHOW_TRACEBACK_LABEL: &str = "show traceback";
pub const HIDE_TRACEBACK_LABEL: &str = "hide traceback";
pub const COPY_TRACEBACK_LABEL: &str = "copy traceback information";
pub const COPY_CLIPBOARD_LABEL: &str = "copy to clipboard";

/// Dialog/window title used for uncaught-error popups.
pub const EXCEPTION_DIALOG_TITLE: &str = "hydrus exception";

/// One-shot paint-event failsafe message from `HandlePaintEventException`.
/// `{window}` is the name of the offending window.
pub const PAINT_EVENT_MESSAGE_PREFIX: &str = "Hey, one of your windows raised an exception during a paint event. This is never ever supposed to happen, and this last-ditch failsafe is now handling it to forestall a crash.";

pub const PAINT_EVENT_MESSAGE_SUFFIX: &str = "Please send it to hydev! You will not see any more of these error popups this program boot, but it is probably still happening. Stuff is probably drawing bad somewhere.";

pub fn paint_event_message(window_name: &str, trace: &str) -> String {
    format!(
        "{PAINT_EVENT_MESSAGE_PREFIX}\n\nThe name of the window is \"{window_name}\", and the exception trace will follow. {PAINT_EVENT_MESSAGE_SUFFIX}\n\n{trace}"
    )
}

/// Human platform tag for the report header, mirroring
/// `sys.platform.lower()` in `PopupMessage.CopyTB`.
pub fn platform_tag() -> &'static str {
    if cfg!(target_os = "windows") {
        "win32"
    } else if cfg!(target_os = "macos") {
        "darwin"
    } else {
        "linux"
    }
}

/// A parsed uncaught error: `title` is the exception type name
/// (`SetStatusTitle`), `first_line` the first line of the value
/// (`SetStatusText`), `traceback` the nice trace (`ToString` tail).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExceptionReport {
    pub title: String,
    pub first_line: String,
    pub traceback: String,
}

impl ExceptionReport {
    pub fn new(
        title: impl Into<String>,
        value: impl Into<String>,
        traceback: impl Into<String>,
    ) -> Self {
        let value = value.into();
        Self {
            title: title.into(),
            first_line: first_line_of(&value),
            traceback: traceback.into(),
        }
    }
}

/// Mirrors `SetExceptionTuple`: empty value becomes `Unknown error`, the
/// status text is the first line of the value.
pub fn first_line_of(value: &str) -> String {
    if value.is_empty() {
        return "Unknown error".to_string();
    }

    value.lines().next().unwrap_or("Exception").to_string()
}

/// Mirrors `JobStatus.ToString`: title, first line, then the traceback.
pub fn report_body(report: &ExceptionReport) -> String {
    format!(
        "{}\n{}\n{}",
        report.title, report.first_line, report.traceback
    )
}

/// Mirrors `PopupMessage.CopyTB`: version/platform header plus the body.
pub fn clipboard_text(
    software_version: u32,
    platform: &str,
    frozen: bool,
    report: &ExceptionReport,
) -> String {
    let build = if frozen { "frozen" } else { "source" };
    let info = format!("v{software_version}, {platform}, {build}");
    format!("{}\n{}", info, report_body(report))
}

/// Mirrors `PopupMessage._ProcessText`: truncates long display text with the
/// verbatim notice; the full text still goes to the log.
pub fn display_text(full_text: &str) -> String {
    if full_text.len() > TEXT_CUTOFF {
        format!(
            "{TRUNCATION_NOTICE}\n{}",
            &full_text[..TEXT_CUTOFF.min(full_text.len())]
        )
    } else {
        full_text.to_string()
    }
}

/// Builds the typed seam state for the exception dialog. All strings are
/// final; C++ only assigns them.
pub fn dialog_state(
    software_version: u32,
    platform: &str,
    frozen: bool,
    report: &ExceptionReport,
) -> ffi::ExceptionReportState {
    let body = report_body(report);
    ffi::ExceptionReportState {
        title: EXCEPTION_DIALOG_TITLE.to_string(),
        display_text: display_text(&body),
        traceback_text: report.traceback.clone(),
        clipboard_text: clipboard_text(software_version, platform, frozen, report),
    }
}

static PENDING: Mutex<Vec<ExceptionReport>> = Mutex::new(Vec::new());
static PAINT_EVENT_SHOWN: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

/// Queue capacity: repeated errors update one dialog, but the queue itself is
/// bounded so a crash loop cannot grow memory without limit.
const PENDING_CAP: usize = 32;

fn queue_report(report: ExceptionReport) {
    if let Ok(mut pending) = PENDING.lock() {
        if pending.len() >= PENDING_CAP {
            pending.remove(0);
        }

        pending.push(report);
    }
}

/// Logs the report exactly like Python: full text goes to the client log
/// (`HydrusData.PrintExceptionTuple` -> `DebugPrint`), which is the same
/// `client - YYYY-MM.log` file the `HydrusLogger` writes to.
pub fn log_report(report: &ExceptionReport) {
    let body = report_body(report);
    let message = format!("\n{body}\n");

    if let Some(logger) = crate::client_boot::logger() {
        logger.write(&message);
        logger.flush();
    } else {
        eprintln!("{message}");
    }
}

/// Entry point for caught Rust errors: logs like Python and queues the popup.
pub fn report_exception(report: ExceptionReport) {
    log_report(&report);
    queue_report(report);
}

/// One-shot paint-event failsafe mirroring `HandlePaintEventException`.
/// Returns the message for the popup, or `None` if already shown this boot.
pub fn handle_paint_event_exception(window_name: &str, trace: &str) -> Option<String> {
    if PAINT_EVENT_SHOWN.swap(true, std::sync::atomic::Ordering::Relaxed) {
        return None;
    }

    let message = paint_event_message(window_name, trace);

    if let Some(logger) = crate::client_boot::logger() {
        logger.write(&format!("\n{message}\n"));
        logger.flush();
    }

    Some(message)
}

/// Drain queued reports into typed UI commands. Called from the Qt tick so a
/// Rust panic on any thread surfaces as the exception dialog instead of a
/// silent death. Repeated errors reuse the same dialog: C++ keeps a single
/// instance and refreshes its texts.
pub fn drain_pending_commands(
    state: &mut crate::gui::app::model::AppState,
) -> Vec<crate::gui::app::commands::UiCommand> {
    let reports: Vec<ExceptionReport> = match PENDING.lock() {
        Ok(mut pending) => pending.drain(..).collect(),
        Err(_) => Vec::new(),
    };

    reports
        .into_iter()
        .map(|report| {
            state.notifications.push(
                "error",
                format!("{}: {}", report.title, report.first_line),
            );

            crate::gui::app::commands::UiCommand::ExceptionReport(dialog_state(
                hydrus_core::hydrus_constants::SOFTWARE_VERSION,
                platform_tag(),
                is_frozen(),
                &report,
            ))
        })
        .collect()
}

#[cfg(not(test))]
fn is_frozen() -> bool {
    std::env::current_exe()
        .ok()
        .and_then(|path| path.extension().map(|ext| ext.to_owned()))
        .is_some_and(|ext| ext == "exe")
}

#[cfg(test)]
fn is_frozen() -> bool {
    false
}

/// Installs the process-wide panic hook. Python installs
/// `CatchExceptionClient`/`ShowExceptionClient` as the uncaught handler; here
/// a Rust panic is logged to the client log and queued for the exception
/// dialog instead of killing the process silently.
pub fn install_panic_hook() {
    std::panic::set_hook(Box::new(|info| {
        let title = "Rust panic".to_string();
        let value = info.to_string();
        let traceback = format!("panic at {info}\n{}", backtrace_hint());
        report_exception(ExceptionReport::new(title, value, traceback));
    }));
}

fn backtrace_hint() -> &'static str {
    "Set RUST_BACKTRACE=1 for a native backtrace; see client log for details."
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn body_matches_python_to_string_order() {
        let report = ExceptionReport::new("ValueError", "bad value", "trace here");
        assert_eq!(report_body(&report), "ValueError\nbad value\ntrace here");
    }

    #[test]
    fn empty_value_becomes_unknown_error() {
        assert_eq!(first_line_of(""), "Unknown error");
        let report = ExceptionReport::new("TypeError", "", "trace");
        assert_eq!(report.first_line, "Unknown error");
    }

    #[test]
    fn first_line_is_first_line_only() {
        assert_eq!(first_line_of("one\ntwo\nthree"), "one");
    }

    #[test]
    fn clipboard_header_matches_python_copy_tb() {
        let report = ExceptionReport::new("ValueError", "bad", "trace");
        let text = clipboard_text(692, "win32", true, &report);
        assert_eq!(text, "v692, win32, frozen\nValueError\nbad\ntrace");
        let text = clipboard_text(692, "linux", false, &report);
        assert!(text.starts_with("v692, linux, source\n"));
    }

    #[test]
    fn long_text_uses_verbatim_truncation_notice() {
        let full = "x".repeat(TEXT_CUTOFF + 10);
        let shown = display_text(&full);
        assert!(shown.starts_with(TRUNCATION_NOTICE));
        assert!(shown.contains(&"x".repeat(TEXT_CUTOFF)));
        assert!(!shown.contains(&"x".repeat(TEXT_CUTOFF + 1)));
    }

    #[test]
    fn short_text_is_untouched() {
        assert_eq!(display_text("hello"), "hello");
    }

    #[test]
    fn paint_event_failsafe_message_mentions_window() {
        let message = paint_event_message("my window", "trace");
        assert!(message.starts_with(PAINT_EVENT_MESSAGE_PREFIX));
        assert!(message.contains("\"my window\""));
        assert!(message.contains("Please send it to hydev!"));
    }

    #[test]
    fn dialog_state_carries_final_strings() {
        let report = ExceptionReport::new("ValueError", "bad", "trace");
        let state = dialog_state(692, "win32", true, &report);
        assert_eq!(state.title, EXCEPTION_DIALOG_TITLE);
        assert_eq!(state.traceback_text, "trace");
        assert_eq!(state.clipboard_text, "v692, win32, frozen\nValueError\nbad\ntrace");
    }

    #[test]
    fn button_labels_match_python() {
        assert_eq!(SHOW_TRACEBACK_LABEL, "show traceback");
        assert_eq!(HIDE_TRACEBACK_LABEL, "hide traceback");
        assert_eq!(COPY_TRACEBACK_LABEL, "copy traceback information");
        assert_eq!(COPY_CLIPBOARD_LABEL, "copy to clipboard");
    }
}
