pub(crate) mod bridge;
pub(crate) mod panels;

pub use bridge::run_application;

pub(crate) use bridge::apply_application_settings;

pub(crate) fn quit() {
    bridge::quit_application();
}
