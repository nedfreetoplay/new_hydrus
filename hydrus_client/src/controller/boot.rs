//! Controller boot/init helpers.

use std::sync::RwLock;

use hydrus_core::hydrus_exceptions::HydrusResult;

use crate::client_application_command::{ApplicationCommand, SIMPLE_REFRESH, SIMPLE_UNDO};
use crate::client_defaults::{apply_default_bandwidth_rules, apply_default_domain_manager_data};
use crate::client_options::ClientOptions;
use crate::db::files_physical_storage;
use crate::files::{ClientFilesManager, FilesStorageBaseLocation, FilesStorageSubfolder};
use crate::gui::shortcuts::{
    SHORTCUT_MODIFIER_CTRL, SHORTCUT_PRESS_TYPE_PRESS, SHORTCUT_TYPE_KEYBOARD_CHARACTER, Shortcut,
    ShortcutSet,
};
use crate::networking::client_networking_bandwidth::NetworkBandwidthManager;
use crate::networking::client_networking_domain::DomainManager;
use crate::services::ServicesManager;

use super::*;

impl Controller {
    pub(super) fn init_domain_manager(&self) {
        if let Ok(mut manager) = self.domain_manager.write() {
            apply_default_domain_manager_data(&mut manager);
        }

        if let Ok(mut manager) = self.bandwidth_manager.write() {
            apply_default_bandwidth_rules(&mut manager);
        }
    }

    pub fn domain_manager(&self) -> &RwLock<DomainManager> {
        &self.domain_manager
    }

    pub fn bandwidth_manager(&self) -> &RwLock<NetworkBandwidthManager> {
        &self.bandwidth_manager
    }

    pub fn bandwidth_manager_arc(&self) -> std::sync::Arc<RwLock<NetworkBandwidthManager>> {
        self.bandwidth_manager.clone()
    }

    pub(super) fn init_client_files_manager(&self) -> HydrusResult<()> {
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

    pub(super) fn init_shortcuts(&self) {
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

    pub(super) fn load_options(&self) -> HydrusResult<()> {
        if let Some(object) = self.db.get_serialisable(OPTIONS_NAME)? {
            if let Some(options) = object.as_any().downcast_ref::<ClientOptions>() {
                let mut options = options.clone();
                options.merge_missing(&ClientOptions::new());
                *self.options.write().unwrap() = options;
                return Ok(());
            }
        }

        *self.options.write().unwrap() = ClientOptions::new();

        Ok(())
    }

    pub(super) fn ensure_local_services(&self) -> HydrusResult<()> {
        let manager = ServicesManager::ensure_local_services(self.db.db())?;
        *self.services.write().unwrap() = manager;

        Ok(())
    }
}
