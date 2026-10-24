mod qt;

use cxx_qt_lib as _;

/// Starts the Qt application with the initial client window.
///
/// The real controller boot sequence is ported in a later milestone; for now
/// this is the GUI smoke path used by `main` and by `gui::run`.
pub fn run() {
    qt::run_application();
}
