/// The core database only needs a few controller hooks. Keeping this adapter
/// separate avoids a strong `Controller -> DB -> Controller` cycle.
struct ServerDBController {
    controller: Weak<Controller>,
}

impl DBController for ServerDBController {
    fn currently_idle(&self) -> bool {
        self.controller.upgrade().is_none()
    }

    fn last_shutdown_was_bad(&self) -> bool {
        false
    }

    fn blocking_safe_show_critical_message(&self, title: &str, message: &str) {
        eprintln!("{title}\n\n{message}");
    }

    fn publish(&self, _message: PubSubMessage) {
        // The server's pub/sub daemon is added with the controller lifecycle.
    }
}

impl ServerController for Controller {
    fn get_files_dir(&self) -> &Path {
        &self.files_dir
    }
}
