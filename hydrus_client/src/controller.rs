//! Client controller boot/model layer.
//!
//! The full Python controller owns dozens of managers. This milestone wires the
//! pieces that exist: database, options, local services and the GUI shell.

use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use hydrus_core::hydrus_constants::ServiceType;
use hydrus_core::hydrus_exceptions::HydrusResult;
use hydrus_core::hydrus_serialisable::SerialisableObject;

use crate::client_globals::ClientControllerHandle;
use crate::client_options::ClientOptions;
use crate::db::{ClientDB, services};
use crate::gui::pages::{Page, PageType, PagesNotebook};
use crate::gui::session::GuiSession;

const OPTIONS_NAME: &str = "client options";
const SESSION_FILENAME: &str = "gui session.json";

pub struct Controller {
    db_dir: PathBuf,
    db: ClientDB,
    options: RwLock<ClientOptions>,
    session: RwLock<GuiSession>,
}

impl Controller {
    pub fn new(db_dir: impl AsRef<Path>) -> HydrusResult<Self> {
        let db_dir = db_dir.as_ref().to_path_buf();
        let session = GuiSession::load_from_path(db_dir.join(SESSION_FILENAME))
            .unwrap_or_else(|_| default_session());

        Ok(Self {
            db: ClientDB::new(&db_dir)?,
            db_dir,
            options: RwLock::new(ClientOptions::new()),
            session: RwLock::new(session),
        })
    }

    /// `init_model`
    pub fn init_model(&self) -> HydrusResult<()> {
        self.db.update_db()?;
        self.load_options()?;
        self.ensure_local_services()?;

        Ok(())
    }

    fn load_options(&self) -> HydrusResult<()> {
        if let Some(object) = self.db.get_serialisable(OPTIONS_NAME)? {
            if let Some(options) = object.as_any().downcast_ref::<ClientOptions>() {
                *self.options.write().unwrap() = options.clone();
                return Ok(());
            }
        }

        *self.options.write().unwrap() = ClientOptions::new();

        Ok(())
    }

    fn ensure_local_services(&self) -> HydrusResult<()> {
        for (key, service_type, name) in [
            (
                b"my files".as_slice(),
                ServiceType::LocalFileDomain,
                "my files",
            ),
            (
                b"local tags".as_slice(),
                ServiceType::LocalTag,
                "local tags",
            ),
            (
                b"trash".as_slice(),
                ServiceType::LocalFileTrashDomain,
                "trash",
            ),
        ] {
            services::get_or_add_service(self.db.db(), key, service_type, name)?;
        }

        Ok(())
    }

    pub fn options(&self) -> ClientOptions {
        self.options.read().unwrap().clone()
    }

    pub fn db(&self) -> &ClientDB {
        &self.db
    }

    pub fn db_dir(&self) -> &Path {
        &self.db_dir
    }

    pub fn window_title(&self) -> String {
        "Hydrus Client".to_string()
    }

    /// Human-readable summary used by the boot status bar.
    pub fn status_line(&self) -> String {
        let version = self.db.get_version().unwrap_or(0);
        let services = services::get_services(self.db.db())
            .map(|services| services.len())
            .unwrap_or(0);

        format!(
            "Hydrus Client — Rust/Qt 6 · db {version} · {services} services · {}",
            self.db_dir.display()
        )
    }

    pub fn save(&self) -> HydrusResult<()> {
        let options = self.options.read().unwrap();

        self.db
            .set_serialisable(OPTIONS_NAME, &*options as &dyn SerialisableObject)?;

        let session = self.session.read().unwrap();

        session
            .save_to_path(self.db_dir.join(SESSION_FILENAME))
            .map_err(|error| {
                hydrus_core::hydrus_exceptions::HydrusException::new(
                    hydrus_core::hydrus_exceptions::ErrorKind::FileMissing,
                    error.to_string(),
                )
            })
    }

    pub fn session(&self) -> GuiSession {
        self.session.read().unwrap().clone()
    }

    pub fn set_session(&self, session: GuiSession) {
        *self.session.write().unwrap() = session;
    }

    pub fn session_path(&self) -> PathBuf {
        self.db_dir.join(SESSION_FILENAME)
    }

    pub fn install_global(self: &Arc<Self>) -> bool {
        crate::client_globals::set_client_controller(self.clone()).is_ok()
    }
}

fn default_session() -> GuiSession {
    let mut notebook = PagesNotebook::new();
    notebook.add_page(Page::new("Welcome", PageType::Search));

    GuiSession::from_notebook(&notebook)
}

impl ClientControllerHandle for Controller {
    fn publish_message(&self, message: &str) {
        crate::gui::pubsub_bridge::post("message", message);

        if let Some(logger) = crate::client_boot::logger() {
            logger.write(message);
            logger.write("\n");
        } else {
            println!("{message}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temporary_controller() -> (Controller, PathBuf) {
        let directory = std::env::temp_dir().join(format!(
            "hydrus_client_controller_{}_{}",
            std::process::id(),
            rand::random::<u64>()
        ));

        let _ = crate::client_options::register_serialisable_type();

        let controller = Controller::new(&directory).unwrap();
        controller.init_model().unwrap();

        (controller, directory)
    }

    #[test]
    fn init_model_creates_local_services_and_loads_options() {
        let (controller, directory) = temporary_controller();
        let services = services::get_services(controller.db().db()).unwrap();

        assert!(services.iter().any(|service| service.name == "my files"));
        assert!(services.iter().any(|service| service.name == "local tags"));
        assert!(!controller.options().get_boolean("advanced_mode"));
        assert_eq!(controller.status_line().contains("services"), true);
        assert_eq!(controller.session().pages.len(), 1);
        assert_eq!(controller.session().pages[0].name, "Welcome");

        drop(controller);
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn options_are_saved_and_reloaded() {
        let (controller, directory) = temporary_controller();

        {
            let mut options = controller.options.write().unwrap();
            options.set_boolean("advanced_mode", true);
        }

        controller.save().unwrap();

        let reloaded = Controller::new(&directory).unwrap();
        reloaded.init_model().unwrap();

        assert!(reloaded.options().get_boolean("advanced_mode"));

        drop(reloaded);
        drop(controller);
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn session_is_saved_and_restored() {
        let (controller, directory) = temporary_controller();

        let mut notebook = PagesNotebook::new();
        notebook.add_page(Page::new("files", PageType::Import));
        controller.set_session(GuiSession::from_notebook(&notebook));
        controller.save().unwrap();

        let reloaded = Controller::new(&directory).unwrap();

        assert_eq!(reloaded.session().pages.len(), 1);
        assert_eq!(reloaded.session().pages[0].name, "files");

        drop(reloaded);
        drop(controller);
        let _ = std::fs::remove_dir_all(directory);
    }
}
