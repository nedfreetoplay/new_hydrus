//! Import options manager from `importing/options/ImportOptionsManager.py`.
//!
//! The manager keeps a default container per caller type plus named favourites.
//! The effective options for an import are built by filling from global, then
//! the caller default, then the specific importer container.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::importing::options::{CALLER_TYPES, ImportOptionsContainer};

/// Options group and key used by the client database, mirroring the Python
/// serialisable manager that lives next to `duplicate_action_options`.
pub const IMPORT_OPTIONS_GROUP: &str = "import_options";
pub const IMPORT_OPTIONS_KEY: &str = "manager";

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ImportOptionsManager {
    defaults: BTreeMap<String, ImportOptionsContainer>,
    favourites: BTreeMap<String, ImportOptionsContainer>,
}

impl ImportOptionsManager {
    pub fn new() -> Self {
        let mut manager = Self::default();

        manager.defaults.insert(
            "global".to_string(),
            ImportOptionsContainer::with_defaults(),
        );

        for (caller, _label, _description) in CALLER_TYPES {
            manager.defaults.entry((*caller).to_string()).or_default();
        }

        manager
    }

    /// Ensures every caller type has a row and the global default is filled in.
    pub fn ensure_defaults(&mut self) {
        self.defaults
            .entry("global".to_string())
            .or_insert_with(ImportOptionsContainer::with_defaults);

        self.defaults
            .get_mut("global")
            .expect("global default exists")
            .fill_in_with_slice(&ImportOptionsContainer::with_defaults(), Some("global"));

        for (caller, _label, _description) in CALLER_TYPES {
            self.defaults.entry((*caller).to_string()).or_default();
        }
    }

    pub fn get_default(&self, caller: &str) -> &ImportOptionsContainer {
        self.defaults.get(caller).unwrap_or(&EMPTY_CONTAINER)
    }

    pub fn get_default_mut(&mut self, caller: &str) -> &mut ImportOptionsContainer {
        self.defaults.entry(caller.to_string()).or_default()
    }

    pub fn set_default(&mut self, caller: impl Into<String>, container: ImportOptionsContainer) {
        self.defaults.insert(caller.into(), container);
    }

    pub fn get_defaults(&self) -> &BTreeMap<String, ImportOptionsContainer> {
        &self.defaults
    }

    pub fn set_defaults(&mut self, defaults: BTreeMap<String, ImportOptionsContainer>) {
        self.defaults = defaults;
        self.ensure_defaults();
    }

    /// Builds the options an importer should use.
    pub fn get_effective(
        &self,
        caller: &str,
        specific: Option<&ImportOptionsContainer>,
    ) -> ImportOptionsContainer {
        let mut result = ImportOptionsContainer::default();

        if let Some(specific) = specific {
            result.fill_in_with_slice(specific, Some("specific importer"));
        }

        result.fill_in_with_slice(self.get_default(caller), Some(caller));
        result.fill_in_with_slice(self.get_default("global"), Some("global"));

        result
    }

    /// Builds the effective global options for callers that have no specific
    /// default yet.
    pub fn get_effective_global(&self) -> ImportOptionsContainer {
        self.get_effective("global", None)
    }

    pub fn add_favourite(&mut self, name: &str, container: ImportOptionsContainer) -> String {
        let name = unique_name(name, self.favourites.keys());
        self.favourites.insert(name.clone(), container);

        name
    }

    pub fn set_favourite(&mut self, name: impl Into<String>, container: ImportOptionsContainer) {
        self.favourites.insert(name.into(), container);
    }

    pub fn rename_favourite(&mut self, old_name: &str, new_name: &str) -> Option<String> {
        let container = self.favourites.remove(old_name)?;
        let name = self.add_favourite(new_name, container);

        Some(name)
    }

    pub fn delete_favourite(&mut self, name: &str) -> bool {
        self.favourites.remove(name).is_some()
    }

    pub fn get_favourite(&self, name: &str) -> Option<&ImportOptionsContainer> {
        self.favourites.get(name)
    }

    pub fn get_favourite_mut(&mut self, name: &str) -> Option<&mut ImportOptionsContainer> {
        self.favourites.get_mut(name)
    }

    pub fn get_favourites(&self) -> &BTreeMap<String, ImportOptionsContainer> {
        &self.favourites
    }

    pub fn set_favourites(&mut self, favourites: BTreeMap<String, ImportOptionsContainer>) {
        self.favourites = favourites;
    }

    pub fn get_names(&self) -> Vec<&str> {
        self.favourites.keys().map(String::as_str).collect()
    }

    pub fn clear_favourites(&mut self) {
        self.favourites.clear();
    }

    pub fn len(&self) -> usize {
        self.favourites.len()
    }

    pub fn is_empty(&self) -> bool {
        self.favourites.is_empty()
    }

    pub fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or_default()
    }

    pub fn from_json(value: serde_json::Value) -> Result<Self, serde_json::Error> {
        let mut manager: Self = serde_json::from_value(value)?;
        manager.ensure_defaults();

        Ok(manager)
    }

    pub fn from_json_str(value: &str) -> Result<Self, serde_json::Error> {
        let mut manager: Self = serde_json::from_str(value)?;
        manager.ensure_defaults();

        Ok(manager)
    }

    /// Loads the manager stored in the client options, creating defaults when
    /// this is the first run.
    pub fn load_from_options(options: &crate::client_options::ClientOptions) -> Self {
        options
            .get_group_value(IMPORT_OPTIONS_GROUP, IMPORT_OPTIONS_KEY)
            .and_then(|value| Self::from_json(value).ok())
            .unwrap_or_else(Self::new)
    }

    pub fn save_to_options(&self, options: &mut crate::client_options::ClientOptions) {
        options.set_group_value(IMPORT_OPTIONS_GROUP, IMPORT_OPTIONS_KEY, self.to_json());
    }
}

static EMPTY_CONTAINER: ImportOptionsContainer = ImportOptionsContainer {
    prefetch: None,
    file_filtering: None,
    tag_filtering: None,
    locations: None,
    tags: None,
    notes: None,
    external_programs: None,
    presentation: None,
    source_labels: BTreeMap::new(),
};

fn unique_name<'a>(name: &str, existing: impl Iterator<Item = &'a String>) -> String {
    let name = name.trim();

    let base = if name.is_empty() {
        "import options"
    } else {
        name
    };
    let mut candidate = base.to_string();
    let existing: Vec<&str> = existing.map(String::as_str).collect();

    if !existing.contains(&candidate.as_str()) {
        return candidate;
    }

    let mut counter = 2;

    loop {
        candidate = format!("{base} ({counter})");

        if !existing.contains(&candidate.as_str()) {
            return candidate;
        }

        counter += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::importing::options::{
        FileFilteringImportOptions, LocationImportOptions, ServiceTagImportOptions,
        TagImportOptions,
    };

    #[test]
    fn containers_round_trip_by_name() {
        let mut manager = ImportOptionsManager::new();
        let mut container = ImportOptionsContainer::default();
        container.file_filtering = Some(FileFilteringImportOptions {
            max_size: Some(1000),
            ..Default::default()
        });
        let mut tag_options = TagImportOptions::new();
        tag_options.set_service_options(
            b"local tags".to_vec(),
            ServiceTagImportOptions {
                get_tags: true,
                ..Default::default()
            },
        );
        container.tags = Some(tag_options);

        let name = manager.add_favourite("my options", container.clone());

        assert_eq!(manager.get_favourite(&name), Some(&container));
        assert_eq!(manager.get_names(), vec!["my options"]);
    }

    #[test]
    fn duplicate_favourite_names_are_made_unique() {
        let mut manager = ImportOptionsManager::new();

        assert_eq!(
            manager.add_favourite("new", ImportOptionsContainer::default()),
            "new"
        );
        assert_eq!(
            manager.add_favourite("new", ImportOptionsContainer::default()),
            "new (2)"
        );
    }

    #[test]
    fn effective_options_fill_from_global_then_caller() {
        let mut manager = ImportOptionsManager::new();
        let mut local = ImportOptionsContainer::default();
        local.locations = Some(LocationImportOptions {
            destination_services: vec!["archive".to_string()],
            ..Default::default()
        });
        manager.set_default("local_import", local);

        let effective = manager.get_effective("local_import", None);

        assert!(effective.file_filtering.is_some(), "global file filtering");
        assert!(effective.prefetch.is_some(), "global prefetch");
        assert_eq!(
            effective.locations_or_default().destination_services,
            vec!["archive".to_string()]
        );
        assert_eq!(effective.get_source_label("locations"), "local_import");
        assert_eq!(effective.get_source_label("prefetch"), "global");
    }

    #[test]
    fn specific_importer_wins_over_caller_default() {
        let mut manager = ImportOptionsManager::new();
        let mut local = ImportOptionsContainer::default();
        local.locations = Some(LocationImportOptions {
            automatically_archives: true,
            ..Default::default()
        });
        manager.set_default("local_import", local);

        let mut specific = ImportOptionsContainer::default();
        specific.locations = Some(LocationImportOptions {
            destination_services: vec!["inbox".to_string()],
            ..Default::default()
        });

        let effective = manager.get_effective("local_import", Some(&specific));

        assert_eq!(
            effective.locations_or_default().destination_services,
            vec!["inbox".to_string()]
        );
        assert!(!effective.locations_or_default().automatically_archives);
        assert_eq!(effective.get_source_label("locations"), "specific importer");
    }

    #[test]
    fn favourite_can_be_renamed_and_deleted() {
        let mut manager = ImportOptionsManager::new();
        manager.add_favourite("a", ImportOptionsContainer::default());

        let renamed = manager.rename_favourite("a", "b").unwrap();
        assert_eq!(renamed, "b");
        assert!(manager.get_favourite("a").is_none());

        assert!(manager.delete_favourite("b"));
        assert!(!manager.delete_favourite("b"));
    }

    #[test]
    fn manager_round_trips_through_json() {
        let mut manager = ImportOptionsManager::new();
        let mut container = ImportOptionsContainer::default();
        container.file_filtering = Some(FileFilteringImportOptions {
            max_size: Some(5),
            ..Default::default()
        });
        manager.add_favourite("saved", container);

        let json = manager.to_json();
        let loaded = ImportOptionsManager::from_json(json).unwrap();

        assert_eq!(loaded.get_names(), vec!["saved"]);
        assert_eq!(
            loaded
                .get_favourite("saved")
                .unwrap()
                .file_filtering
                .as_ref()
                .unwrap()
                .max_size,
            Some(5)
        );
        assert!(loaded.get_default("global").is_full());
    }
}
