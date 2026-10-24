use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};

/// The subset of the controller surface that low-level modules may need.
///
/// Modules should prefer explicit context handles; this trait exists for the
/// few legacy call sites that publish user-facing messages.
pub trait ClientControllerHandle: Send + Sync {
    fn publish_message(&self, message: &str);
}

static CLIENT_CONTROLLER: OnceLock<Arc<dyn ClientControllerHandle>> = OnceLock::new();
static NO_QT_MULTIMEDIA: AtomicBool = AtomicBool::new(false);

/// `CG.client_controller`
pub fn client_controller() -> Option<&'static Arc<dyn ClientControllerHandle>> {
    CLIENT_CONTROLLER.get()
}

/// Installs the process-wide controller handle. Returns the existing handle if
/// one was already installed.
pub fn set_client_controller(
    controller: Arc<dyn ClientControllerHandle>,
) -> Result<(), Arc<dyn ClientControllerHandle>> {
    CLIENT_CONTROLLER.set(controller)
}

/// `CG.NO_QT_MULTIMEDIA`
pub fn no_qt_multimedia() -> bool {
    NO_QT_MULTIMEDIA.load(Ordering::Relaxed)
}

pub fn set_no_qt_multimedia(value: bool) {
    NO_QT_MULTIMEDIA.store(value, Ordering::Relaxed);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn multimedia_flag_round_trips() {
        let original = no_qt_multimedia();

        set_no_qt_multimedia(!original);
        assert_eq!(no_qt_multimedia(), !original);

        set_no_qt_multimedia(original);
    }
}
