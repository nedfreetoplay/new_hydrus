//! Client controller boot/model layer.
//!
//! The full Python controller owns dozens of managers. This milestone wires the
//! pieces that exist: database, options, local services and the GUI shell.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, RwLock};

use hydrus_core::hydrus_constants::ServiceType;
use hydrus_core::hydrus_exceptions::HydrusResult;
use hydrus_core::hydrus_serialisable::SerialisableObject;
use serde::{Deserialize, Serialize};

use crate::api::{ApiManager, ApiServer, ApiServerHandle, ClientApiBackend};
use crate::client_defaults::CustomHeader;
use crate::client_options::ClientOptions;
use crate::db::ClientDB;
use crate::files::{ClientFilesManager, FilesMaintenanceManager, TrashManager};
use crate::gui::session::{GuiSession, UnifiedSession};
use crate::gui::shortcuts::ShortcutSet;
use crate::gui::style::ClientGUIStyle;
use crate::networking::client_networking_bandwidth::{
    BandwidthRuleEntry, NetworkBandwidthManager, default_rule_entries,
};
use crate::networking::client_networking_domain::DomainManager;
use crate::networking::client_networking_url_class::UrlClass;
use crate::networking::download_pools::DownloadPoolSettings;
use crate::services::ServicesManager;

const OPTIONS_NAME: &str = "client options";
const SESSION_FILENAME: &str = "gui session.json";
const NETWORK_FILENAME: &str = "network settings.json";

#[derive(Debug, Serialize, Deserialize)]
struct NetworkSettingsFile {
    #[serde(default)]
    headers: BTreeMap<String, CustomHeader>,
    #[serde(default)]
    url_classes: Vec<UrlClass>,
    #[serde(default = "default_rule_entries")]
    bandwidth_rules: Vec<BandwidthRuleEntry>,
    #[serde(default)]
    download_pools: DownloadPoolSettings,
}

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
    bandwidth_manager: Arc<RwLock<NetworkBandwidthManager>>,
    bandwidth_rules: RwLock<Vec<BandwidthRuleEntry>>,
    download_pools: RwLock<DownloadPoolSettings>,
    network_paused: Arc<AtomicBool>,
    api_server: Mutex<Option<ApiServerHandle>>,
}

impl Controller {
    pub fn new(db_dir: impl AsRef<Path>) -> HydrusResult<Self> {
        let db_dir = db_dir.as_ref().to_path_buf();
        let session = UnifiedSession::load(db_dir.join(SESSION_FILENAME))
            .and_then(|session| session.legacy_session())
            .unwrap_or_else(default_session);

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
            bandwidth_manager: Arc::new(RwLock::new(NetworkBandwidthManager::new())),
            bandwidth_rules: RwLock::new(default_rule_entries()),
            download_pools: RwLock::new(DownloadPoolSettings::new()),
            network_paused: Arc::new(AtomicBool::new(false)),
            api_server: Mutex::new(None),
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
        self.load_network_settings();

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

    pub fn url_classes(&self) -> Vec<UrlClass> {
        self.domain_manager
            .read()
            .unwrap()
            .get_url_classes()
            .to_vec()
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

    pub fn replace_options(&self, options: ClientOptions) {
        *self.options.write().unwrap() = options;
    }

    pub fn reload_services(&self) -> HydrusResult<()> {
        let manager = ServicesManager::load(self.db.db())?;
        *self.services.write().unwrap() = manager;

        Ok(())
    }

    pub fn add_service(&self, name: &str, service_type: ServiceType) -> HydrusResult<i64> {
        use rand::RngCore;

        let name = name.trim();

        if name.is_empty() {
            return Err(hydrus_core::hydrus_exceptions::unknown(
                "a service needs a name",
            ));
        }

        if self
            .services()
            .names()
            .iter()
            .any(|existing| *existing == name)
        {
            return Err(hydrus_core::hydrus_exceptions::unknown(
                "a service with that name already exists",
            ));
        }

        let mut key = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut key);

        let service_id =
            crate::db::services::get_or_add_service(self.db.db(), &key, service_type, name)?;

        match service_type {
            ServiceType::LocalFileDomain
            | ServiceType::LocalFileTrashDomain
            | ServiceType::LocalFileUpdateDomain
            | ServiceType::HydrusLocalFileStorage => {
                crate::db::files_storage::create_tables(self.db.db(), service_id)?;
            }
            ServiceType::LocalTag | ServiceType::TagRepository | ServiceType::CombinedTag => {
                crate::db::mappings::create_tables(self.db.db(), service_id)?;
            }
            _ => {}
        }

        self.reload_services()?;

        Ok(service_id)
    }

    pub fn rename_service(&self, service_key_hex: &str, name: &str) -> HydrusResult<()> {
        let Some(key) = decode_hex(service_key_hex) else {
            return Err(hydrus_core::hydrus_exceptions::unknown(
                "invalid service key",
            ));
        };

        let name = name.trim();

        if name.is_empty() {
            return Err(hydrus_core::hydrus_exceptions::unknown(
                "a service needs a name",
            ));
        }

        let dictionary_string = self
            .services()
            .get_service(&key)
            .map(|service| service.dictionary_string.clone())
            .unwrap_or_default();

        crate::db::services::update_service(self.db.db(), &key, name, &dictionary_string)?;
        self.reload_services()?;

        Ok(())
    }

    pub fn delete_service(&self, service_key_hex: &str) -> HydrusResult<()> {
        let Some(key) = decode_hex(service_key_hex) else {
            return Err(hydrus_core::hydrus_exceptions::unknown(
                "invalid service key",
            ));
        };

        let Some(service) = self.services().get_service(&key).cloned() else {
            return Err(hydrus_core::hydrus_exceptions::unknown(
                "service is missing",
            ));
        };

        if matches!(service.name.as_str(), "my files" | "local tags" | "trash") {
            return Err(hydrus_core::hydrus_exceptions::unknown(
                "the built-in local services cannot be deleted",
            ));
        }

        let peer_count = self
            .services()
            .get_services(Some(&[service.service_type]))
            .len();

        if matches!(
            service.service_type,
            ServiceType::LocalFileDomain
                | ServiceType::LocalTag
                | ServiceType::LocalFileTrashDomain
        ) && peer_count <= 1
        {
            return Err(hydrus_core::hydrus_exceptions::unknown(
                "the client needs at least one service of this type",
            ));
        }

        let count: i64 = self
            .db
            .db()
            .query_one(
                &format!("SELECT COUNT(*) FROM current_files_{}", service.service_id),
                &[],
                |row| row.get(0),
            )
            .unwrap_or(Some(0))
            .unwrap_or(0);

        if count > 0 && service.service_type == ServiceType::LocalFileDomain {
            return Err(hydrus_core::hydrus_exceptions::unknown(
                "move or delete this service's files before removing it",
            ));
        }

        if service.service_type == ServiceType::LocalTag {
            let mappings: i64 = self
                .db
                .db()
                .query_one(
                    &format!(
                        "SELECT COUNT(*) FROM current_mappings_{}",
                        service.service_id
                    ),
                    &[],
                    |row| row.get(0),
                )
                .unwrap_or(Some(0))
                .unwrap_or(0);

            if mappings > 0 {
                return Err(hydrus_core::hydrus_exceptions::unknown(
                    "remove this service's tags before deleting it",
                ));
            }
        }

        for suffix in [
            "current_files",
            "pending_files",
            "deleted_files",
            "petitioned_files",
        ] {
            let _ = self.db.db().execute(
                &format!("DROP TABLE IF EXISTS {suffix}_{}", service.service_id),
                &[],
            );
        }

        for suffix in [
            "current_mappings",
            "pending_mappings",
            "deleted_mappings",
            "petitioned_mappings",
        ] {
            let _ = self.db.db().execute(
                &format!("DROP TABLE IF EXISTS {suffix}_{}", service.service_id),
                &[],
            );
        }

        crate::db::services::delete_service(self.db.db(), &key)?;
        self.reload_services()?;

        Ok(())
    }

    pub fn network_paused(&self) -> bool {
        self.network_paused.load(Ordering::Relaxed)
    }

    pub fn network_paused_flag(&self) -> Arc<AtomicBool> {
        self.network_paused.clone()
    }

    pub fn set_network_paused(&self, paused: bool) {
        self.network_paused.store(paused, Ordering::Relaxed);
    }

    pub fn api_status(&self) -> Option<String> {
        self.api_server
            .lock()
            .ok()
            .and_then(|guard| guard.as_ref().map(|handle| handle.address.to_string()))
    }

    pub fn stop_api(&self) {
        if let Ok(mut guard) = self.api_server.lock() {
            if let Some(handle) = guard.take() {
                handle.stop();
            }
        }
    }

    pub fn start_api(&self, port: u16, access_key: &str) -> HydrusResult<String> {
        self.stop_api();

        let backend = ClientApiBackend::new(
            self.db.db().clone(),
            self.files_manager(),
            crate::db::services::get_services(self.db.db())?,
        );
        let manager = Arc::new(ApiManager::with_access_key(backend, access_key));
        let server = ApiServer::bind(("127.0.0.1", port), manager)?;
        let handle = server.start()?;
        let address = handle.address.to_string();

        *self.api_server.lock().unwrap() = Some(handle);

        Ok(address)
    }

    /// Starts or stops the API according to the saved options and returns the
    /// current status line.
    pub fn refresh_api(&self) -> HydrusResult<String> {
        let options = self.options();
        let enabled = options.get_boolean("api_enabled");

        if !enabled {
            self.stop_api();
            return Ok("api disabled".to_string());
        }

        let port = options.get_integer("api_port");
        let port = if (1..=65535).contains(&port) {
            port as u16
        } else {
            45869
        };

        let mut key = options.get_string("api_access_key");

        if key.len() != 64 {
            key = random_hex_32();

            let mut options = options;
            options.set_string("api_access_key", &key);
            self.replace_options(options);
            let _ = self.save_options();
        }

        match self.start_api(port, &key) {
            Ok(address) => Ok(format!("api listening at http://{address}")),
            Err(error) => Ok(format!("api failed: {error}")),
        }
    }

    pub fn save_options(&self) -> HydrusResult<()> {
        let options = self.options.read().unwrap();

        self.db
            .set_serialisable(OPTIONS_NAME, &*options as &dyn SerialisableObject)?;

        let _ = self.save_network_settings();

        Ok(())
    }

    pub fn load_network_settings(&self) {
        let path = self.db_dir.join(NETWORK_FILENAME);
        let Ok(text) = std::fs::read_to_string(path) else {
            return;
        };
        let Ok(settings) = serde_json::from_str::<NetworkSettingsFile>(&text) else {
            return;
        };

        if let Ok(mut manager) = self.domain_manager.write() {
            manager.set_custom_headers(settings.headers);
            manager.set_url_classes(settings.url_classes);
        }

        *self.bandwidth_rules.write().unwrap() = settings.bandwidth_rules.clone();
        let mut pools = settings.download_pools;
        pools.normalize();
        *self.download_pools.write().unwrap() = pools;

        if let Ok(mut manager) = self.bandwidth_manager.write() {
            manager.clear_rules();
            manager.apply_rule_entries(&settings.bandwidth_rules);
        }
    }

    pub fn save_network_settings(&self) -> std::io::Result<()> {
        let manager = self.domain_manager.read().unwrap();
        let settings = NetworkSettingsFile {
            headers: manager.get_custom_headers().clone(),
            url_classes: manager.get_url_classes().to_vec(),
            bandwidth_rules: self.bandwidth_rules.read().unwrap().clone(),
            download_pools: self.download_pools.read().unwrap().clone(),
        };
        let dump = serde_json::to_string_pretty(&settings).map_err(std::io::Error::other)?;

        std::fs::write(self.db_dir.join(NETWORK_FILENAME), dump)
    }

    pub fn bandwidth_rule_entries(&self) -> Vec<BandwidthRuleEntry> {
        self.bandwidth_rules.read().unwrap().clone()
    }

    pub fn replace_bandwidth_rule_entries(&self, entries: Vec<BandwidthRuleEntry>) {
        *self.bandwidth_rules.write().unwrap() = entries.clone();

        if let Ok(mut manager) = self.bandwidth_manager.write() {
            manager.clear_rules();
            manager.apply_rule_entries(&entries);
        }
    }

    pub fn download_pool_settings(&self) -> DownloadPoolSettings {
        self.download_pools.read().unwrap().clone()
    }

    pub fn replace_download_pool_settings(&self, mut settings: DownloadPoolSettings) {
        settings.normalize();
        *self.download_pools.write().unwrap() = settings;
    }

    pub fn save(&self) -> HydrusResult<()> {
        self.save_options()?;

        let path = self.db_dir.join(SESSION_FILENAME);
        let mut unified = UnifiedSession::load(&path).unwrap_or_default();
        unified.version = UnifiedSession::VERSION;
        unified.legacy = Some(self.session.read().unwrap().clone());
        unified.save(path).map_err(|error| {
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

fn decode_hex(value: &str) -> Option<Vec<u8>> {
    if value.len() % 2 != 0 {
        return None;
    }

    (0..value.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(&value[index..index + 2], 16).ok())
        .collect()
}

fn random_hex_32() -> String {
    use rand::RngCore;

    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests;
