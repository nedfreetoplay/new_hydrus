//! Client controller boot/model layer.
//!
//! The full Python controller owns dozens of managers. This milestone wires the
//! pieces that exist: database, options, local services and the GUI shell.

use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use hydrus_core::hydrus_exceptions::HydrusResult;
use hydrus_core::hydrus_serialisable::SerialisableObject;

use crate::client_application_command::{ApplicationCommand, SIMPLE_REFRESH, SIMPLE_UNDO};
use crate::client_globals::ClientControllerHandle;
use crate::client_options::ClientOptions;
use crate::db::{ClientDB, files_physical_storage, services};
use crate::files::{
    ClientFilesManager, FilesMaintenanceManager, FilesStorageBaseLocation, FilesStorageSubfolder,
    TrashManager,
};
use crate::gui::pages::{Page, PageType, PagesNotebook};
use crate::gui::session::GuiSession;
use crate::gui::shortcuts::{
    SHORTCUT_MODIFIER_CTRL, SHORTCUT_PRESS_TYPE_PRESS, SHORTCUT_TYPE_KEYBOARD_CHARACTER, Shortcut,
    ShortcutSet,
};
use crate::gui::style::ClientGUIStyle;
use crate::services::ServicesManager;

const OPTIONS_NAME: &str = "client options";
const SESSION_FILENAME: &str = "gui session.json";

pub struct Controller {
    db_dir: PathBuf,
    db: ClientDB,
    options: RwLock<ClientOptions>,
    session: RwLock<GuiSession>,
    services: RwLock<ServicesManager>,
    files_manager: RwLock<ClientFilesManager>,
    maintenance: FilesMaintenanceManager,
    trash: RwLock<TrashManager>,
    shortcuts: RwLock<ShortcutSet>,
    style: ClientGUIStyle,
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
            services: RwLock::new(ServicesManager::default()),
            files_manager: RwLock::new(ClientFilesManager::default()),
            maintenance: FilesMaintenanceManager::new(),
            trash: RwLock::new(TrashManager::new(72, 2048)),
            shortcuts: RwLock::new(ShortcutSet::new("global")),
            style: ClientGUIStyle::default(),
        })
    }

    /// `init_model`
    pub fn init_model(&self) -> HydrusResult<()> {
        self.db.update_db()?;
        self.load_options()?;
        self.ensure_local_services()?;
        self.init_client_files_manager()?;
        self.init_shortcuts();

        Ok(())
    }

    fn init_client_files_manager(&self) -> HydrusResult<()> {
        let locations = files_physical_storage::get_current_locations(self.db.db())?;
        let locations = if locations.is_empty() {
            vec![
                FilesStorageBaseLocation::new(self.db_dir.join("files"), 1, None),
                FilesStorageBaseLocation::new(self.db_dir.join("thumbs"), 1, None),
            ]
        } else {
            locations
        };

        let subfolders = files_physical_storage::get_subfolders(self.db.db(), &locations)?;
        let subfolders = if subfolders.is_empty() {
            vec![
                FilesStorageSubfolder::new("f", locations[0].clone()),
                FilesStorageSubfolder::new("t", locations[locations.len() - 1].clone()),
            ]
        } else {
            subfolders
        };

        let manager = ClientFilesManager::new(subfolders);
        manager.make_sure_exists()?;

        *self.files_manager.write().unwrap() = manager;

        Ok(())
    }

    fn init_shortcuts(&self) {
        let mut shortcuts = ShortcutSet::new("global");
        shortcuts.set_command(
            Shortcut::new(
                SHORTCUT_TYPE_KEYBOARD_CHARACTER,
                b'r' as i32,
                SHORTCUT_PRESS_TYPE_PRESS,
                vec![SHORTCUT_MODIFIER_CTRL],
            ),
            ApplicationCommand::simple(SIMPLE_REFRESH),
        );
        shortcuts.set_command(
            Shortcut::new(
                SHORTCUT_TYPE_KEYBOARD_CHARACTER,
                b'z' as i32,
                SHORTCUT_PRESS_TYPE_PRESS,
                vec![SHORTCUT_MODIFIER_CTRL],
            ),
            ApplicationCommand::simple(SIMPLE_UNDO),
        );

        *self.shortcuts.write().unwrap() = shortcuts;
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
        let manager = ServicesManager::ensure_local_services(self.db.db())?;
        *self.services.write().unwrap() = manager;

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
        let services = self.services.read().unwrap().get_num_services();
        let locations = self.files_manager.read().unwrap().locations().len();
        let shortcuts = self.shortcuts.read().unwrap().len();

        format!(
            "Hydrus Client - Rust/Qt 6 · db {version} · {services} services · {locations} file locations · {shortcuts} shortcuts · {}",
            self.db_dir.display()
        )
    }

    pub fn services(&self) -> ServicesManager {
        self.services.read().unwrap().clone()
    }

    pub fn files_manager(&self) -> ClientFilesManager {
        self.files_manager.read().unwrap().clone()
    }

    pub fn maintenance(&self) -> &FilesMaintenanceManager {
        &self.maintenance
    }

    pub fn trash(&self) -> TrashManager {
        self.trash.read().unwrap().clone()
    }

    pub fn shortcuts(&self) -> ShortcutSet {
        self.shortcuts.read().unwrap().clone()
    }

    pub fn style(&self) -> &ClientGUIStyle {
        &self.style
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
        assert_eq!(controller.services().get_num_services(), 3);
        assert_eq!(controller.files_manager().locations().len(), 2);
        assert_eq!(controller.shortcuts().len(), 2);

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
