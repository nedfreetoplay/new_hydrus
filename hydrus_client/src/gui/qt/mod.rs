pub(crate) mod api;
pub(crate) mod bridge;
pub(crate) mod panels;

#[cfg(test)]
mod seam_guard;

pub use bridge::run_application;

pub(crate) use bridge::apply_application_settings;

pub(crate) fn quit() {
    bridge::quit_application();
}
