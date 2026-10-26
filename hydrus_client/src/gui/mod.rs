pub mod app;
pub mod call_after;
pub mod canvas;
pub mod functions;
pub mod list_model;
pub mod pages;
pub mod pubsub_bridge;
pub mod qt_init;
pub mod scrolled_panels;
pub mod session;
pub mod shortcuts;
pub mod splash;
pub mod style;
pub mod time;
pub mod top_level_windows;

mod qt;

use std::sync::Arc;

use cxx_qt_lib as _;

/// Starts the Qt client. The controller is moved into the application state and
/// the Qt layer only renders that state.
pub fn run(controller: Arc<crate::controller::Controller>) {
    qt::run_application(controller);
}

/// Asks Qt to leave the main event loop (used by the SIGINT handler).
pub fn request_quit() {
    qt::quit();
}
