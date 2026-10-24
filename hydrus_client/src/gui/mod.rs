pub mod frame_gui;

mod qt;

use cxx_qt_lib as _;

/// Starts the Qt application with the initial client window.
///
/// The controller boot sequence is wired in a later milestone; this is the
/// GUI smoke path used by `main` and by `gui::run`.
pub fn run() {
    qt::run_application();
}
