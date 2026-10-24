pub mod call_after;
pub mod frame_gui;
pub mod functions;
pub mod pages;
pub mod popup_messages;
pub mod qt_init;
pub mod shortcuts;
pub mod style;
pub mod top_level_windows;

mod qt;

use cxx_qt_lib as _;

/// Starts the Qt application with the initial client window.
///
/// The controller boot sequence provides the window title and status line; the
/// static shell content lives in [`frame_gui`].
pub fn run(title: &str, status: &str) {
    qt::run_application(title, status);
}
