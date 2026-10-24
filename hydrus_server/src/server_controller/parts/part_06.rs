use hydrus_core::hydrus_controller::{ControllerHooks, HydrusController};
use hydrus_core::hydrus_logger::HydrusLogger;

struct ServerControllerHooks {
    shutdown_requested: Arc<AtomicBool>,
}

impl ControllerHooks for ServerControllerHooks {
    fn shutdown_from_server(&self) -> HydrusResult<()> {
        self.shutdown_requested.store(true, Ordering::Relaxed);

        Ok(())
    }

    fn currently_idle(&self) -> bool {
        true
    }

    fn start_twisted_if_needed(&self) -> HydrusResult<()> {
        // The server starts its own tokio TLS listeners.
        Ok(())
    }

    fn stop_twisted_if_running(&self) {
        // Listeners are stopped by the controller run loop.
    }
}

#[allow(dead_code)]
impl Controller {
    fn init_core_controller(self: &Arc<Self>) -> HydrusResult<()> {
        let logger = Arc::new(HydrusLogger::new(&self.db_dir, "server")?);
        let hooks = Arc::new(ServerControllerHooks {
            shutdown_requested: Arc::clone(&self.shutdown_requested),
        });
        let core = HydrusController::new(&self.db_dir, logger, hooks);

        core.init_model()?;
        core.set_name("server");

        let _ = self.core_controller.set(core);

        Ok(())
    }

    fn shutdown_core_controller(&self) {
        if let Some(core) = self.core_controller.get() {
            core.shutdown_model();
        }
    }
}
