//! Import options manager from `importing/options/ImportOptionsManager.py`.

use std::collections::BTreeMap;

use crate::importing::options::ImportOptionsContainer;

#[derive(Debug, Default)]
pub struct ImportOptionsManager {
    containers: BTreeMap<String, ImportOptionsContainer>,
    default: ImportOptionsContainer,
}

impl ImportOptionsManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_default(&self) -> &ImportOptionsContainer {
        &self.default
    }

    pub fn set_default(&mut self, container: ImportOptionsContainer) {
        self.default = container;
    }

    pub fn set_container(&mut self, name: impl Into<String>, container: ImportOptionsContainer) {
        self.containers.insert(name.into(), container);
    }

    pub fn get_container(&self, name: &str) -> Option<&ImportOptionsContainer> {
        self.containers.get(name)
    }

    pub fn get_container_mut(&mut self, name: &str) -> Option<&mut ImportOptionsContainer> {
        self.containers.get_mut(name)
    }

    pub fn get_or_create(&mut self, name: impl Into<String>) -> &mut ImportOptionsContainer {
        self.containers.entry(name.into()).or_default()
    }

    pub fn delete_container(&mut self, name: &str) -> bool {
        self.containers.remove(name).is_some()
    }

    pub fn get_names(&self) -> Vec<&str> {
        self.containers.keys().map(String::as_str).collect()
    }

    pub fn clear_containers(&mut self) {
        self.containers.clear();
    }

    pub fn len(&self) -> usize {
        self.containers.len()
    }

    pub fn is_empty(&self) -> bool {
        self.containers.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::importing::options::{
        FileFilteringImportOptions, ServiceTagImportOptions, TagImportOptions,
    };

    #[test]
    fn containers_round_trip_by_name() {
        let mut manager = ImportOptionsManager::new();
        let mut container = ImportOptionsContainer::default();
        container.file_filtering.max_size = Some(1000);
        container.tag_import_options.set_service_options(
            b"local tags".to_vec(),
            ServiceTagImportOptions {
                get_tags: true,
                ..Default::default()
            },
        );

        manager.set_container("my options", container.clone());

        assert_eq!(manager.get_container("my options"), Some(&container));
        assert_eq!(manager.get_names(), vec!["my options"]);
    }

    #[test]
    fn get_or_create_creates_empty_container() {
        let mut manager = ImportOptionsManager::new();

        let container = manager.get_or_create("new");
        assert_eq!(*container, ImportOptionsContainer::default());

        assert_eq!(manager.len(), 1);
    }

    #[test]
    fn default_options_can_be_replaced() {
        let mut manager = ImportOptionsManager::new();
        let mut container = ImportOptionsContainer::default();
        container.file_filtering = FileFilteringImportOptions {
            max_size: Some(5),
            ..Default::default()
        };
        container.tag_import_options = TagImportOptions::new();

        manager.set_default(container.clone());

        assert_eq!(manager.get_default(), &container);
    }

    #[test]
    fn containers_can_be_deleted_and_cleared() {
        let mut manager = ImportOptionsManager::new();
        manager.set_container("a", ImportOptionsContainer::default());
        manager.set_container("b", ImportOptionsContainer::default());

        assert!(manager.delete_container("a"));
        assert!(!manager.delete_container("a"));

        manager.clear_containers();
        assert!(manager.is_empty());
    }
}
