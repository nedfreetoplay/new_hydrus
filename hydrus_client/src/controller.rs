//! Client controller boot/model layer.
//!
//! The full Python controller owns dozens of managers. This milestone wires the
//! pieces that exist: database, options, local services and the GUI shell.

use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use hydrus_core::hydrus_exceptions::HydrusResult;
use hydrus_core::hydrus_serialisable::SerialisableObject;

use crate::client_options::ClientOptions;
use crate::db::ClientDB;
use crate::files::{ClientFilesManager, FilesMaintenanceManager, TrashManager};
use crate::gui::session::GuiSession;
use crate::gui::shortcuts::ShortcutSet;
use crate::gui::style::ClientGUIStyle;
use crate::networking::client_networking_bandwidth::NetworkBandwidthManager;
use crate::networking::client_networking_domain::DomainManager;
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
    domain_manager: RwLock<DomainManager>,
    bandwidth_manager: RwLock<NetworkBandwidthManager>,
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
            domain_manager: RwLock::new(DomainManager::new()),
            bandwidth_manager: RwLock::new(NetworkBandwidthManager::new()),
        })
    }

    /// `init_model`
    pub fn init_model(&self) -> HydrusResult<()> {
        self.db.update_db()?;
        self.load_options()?;
        self.ensure_local_services()?;
        self.init_client_files_manager()?;
        self.init_shortcuts();
        self.init_domain_manager();

        Ok(())
    }
}

mod boot;

impl Controller {
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

    pub fn save_options(&self) -> HydrusResult<()> {
        let options = self.options.read().unwrap();

        self.db
            .set_serialisable(OPTIONS_NAME, &*options as &dyn SerialisableObject)?;

        Ok(())
    }

    pub fn save(&self) -> HydrusResult<()> {
        self.save_options()?;

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

mod runtime;
use runtime::default_session;

#[cfg(test)]
mod tests;
