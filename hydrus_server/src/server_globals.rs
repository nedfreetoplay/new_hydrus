//! Rust port of `hydrus/server/ServerGlobals.py`.
//!
//! Python stores the active `ServerController` in a module global. The Rust
//! port keeps the same process-wide handle, but stores a weak reference so the
//! global cannot keep a shutting-down controller alive.

use std::sync::{OnceLock, RwLock, Weak};

/// The small controller surface the file/network layers need.
///
/// The concrete [`crate::server_controller::Controller`] implements this; tests
/// can provide a lightweight fake without starting a database or HTTP stack.
pub trait ServerController: Send + Sync {
    /// `get_files_dir()`
    fn get_files_dir(&self) -> &std::path::Path;
}

static SERVER_CONTROLLER: OnceLock<RwLock<Option<Weak<dyn ServerController>>>> = OnceLock::new();

fn controller_slot() -> &'static RwLock<Option<Weak<dyn ServerController>>> {
    SERVER_CONTROLLER.get_or_init(|| RwLock::new(None))
}

/// `SG.server_controller = controller`
pub fn set_server_controller(controller: &std::sync::Arc<dyn ServerController>) {
    *controller_slot()
        .write()
        .unwrap_or_else(|error| error.into_inner()) = Some(std::sync::Arc::downgrade(controller));
}

/// Clear the global, matching a clean controller shutdown.
#[allow(dead_code)]
pub fn clear_server_controller() {
    *controller_slot()
        .write()
        .unwrap_or_else(|error| error.into_inner()) = None;
}

/// `SG.server_controller`
///
/// Returns `None` if no controller has been installed or the last controller
/// has already been dropped.
pub fn server_controller() -> Option<std::sync::Arc<dyn ServerController>> {
    controller_slot()
        .read()
        .unwrap_or_else(|error| error.into_inner())
        .as_ref()
        .and_then(Weak::upgrade)
}

/// Require an active controller, with the error the file layer uses when the
/// server is not booted.
pub fn require_server_controller() -> hydrus_core::hydrus_exceptions::HydrusResult<
    std::sync::Arc<dyn ServerController>,
> {
    server_controller().ok_or_else(|| {
        hydrus_core::hydrus_exceptions::HydrusException::new(
            hydrus_core::hydrus_exceptions::ErrorKind::Unknown,
            "The server controller is not set!",
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::path::{Path, PathBuf};
    use std::sync::{Arc, Mutex};

    static TEST_LOCK: Mutex<()> = Mutex::new(());

    struct FakeController {
        files_dir: PathBuf,
    }

    impl ServerController for FakeController {
        fn get_files_dir(&self) -> &Path {
            &self.files_dir
        }
    }

    #[test]
    fn global_round_trips_and_tracks_the_controller_lifetime() {
        let _guard = TEST_LOCK.lock().unwrap();

        clear_server_controller();

        let controller = Arc::new(FakeController {
            files_dir: PathBuf::from("server_files"),
        });
        let trait_controller: Arc<dyn ServerController> = controller.clone();

        set_server_controller(&trait_controller);

        assert_eq!(
            server_controller()
                .expect("the controller is installed")
                .get_files_dir(),
            Path::new("server_files")
        );

        drop(controller);
        drop(trait_controller);

        assert!(server_controller().is_none());
        assert!(require_server_controller().is_err());

        clear_server_controller();
    }
}
