pub mod call_after;
pub mod canvas;
pub mod frame_gui;
pub mod functions;
pub mod import_panels;
pub mod list_model;
pub mod networking_panels;
pub mod options_panel;
pub mod pages;
pub mod popup_messages;
pub mod pubsub_bridge;
pub mod qt_init;
pub mod scrolled_panels;
pub mod session;
pub mod shortcuts;
pub mod splash;
pub mod string_panels;
pub mod style;
pub mod time;
pub mod top_level_windows;
pub mod view_model;

mod qt;

use cxx_qt_lib as _;

/// Starts the Qt application with the given view model.
///
/// Application state is turned into [`view_model::MainViewModel`] before this
/// call; the Qt layer only renders it.
pub fn run(model: &view_model::MainViewModel) {
    qt::run_application(model);
}

/// Asks Qt to leave the main event loop (used by the SIGINT handler).
pub fn request_quit() {
    qt::quit();
}
