//! Service collection from `hydrus/client/ClientServices.py`.

use std::collections::BTreeMap;

use hydrus_core::hydrus_constants::ServiceType;
use hydrus_core::hydrus_db_base::DBBase;
use hydrus_core::hydrus_exceptions::HydrusResult;

use crate::db::services::{self, Service};

#[derive(Debug, Clone, Default)]
pub struct ServicesManager {
    services: BTreeMap<Vec<u8>, Service>,
}

impl ServicesManager {
    pub fn load(db: &DBBase) -> HydrusResult<Self> {
        let mut services = BTreeMap::new();

        for service in services::get_services(db)? {
            services.insert(service.service_key.clone(), service);
        }

        Ok(Self { services })
    }

    /// Creates the default local services when they are missing.
    pub fn ensure_local_services(db: &DBBase) -> HydrusResult<Self> {
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
            services::get_or_add_service(db, key, service_type, name)?;
        }

        Self::load(db)
    }

    pub fn get_service(&self, service_key: &[u8]) -> Option<&Service> {
        self.services.get(service_key)
    }

    pub fn get_name(&self, service_key: &[u8]) -> String {
        self.services
            .get(service_key)
            .map(|service| service.name.clone())
            .unwrap_or_else(|| hex_encode(service_key))
    }

    pub fn get_service_key_from_name(&self, name: &str) -> Option<&[u8]> {
        self.services
            .values()
            .find(|service| service.name == name)
            .map(|service| service.service_key.as_slice())
    }

    pub fn get_service_type(&self, service_key: &[u8]) -> Option<ServiceType> {
        self.services
            .get(service_key)
            .map(|service| service.service_type)
    }

    pub fn get_services(&self, limited_types: Option<&[ServiceType]>) -> Vec<&Service> {
        self.services
            .values()
            .filter(|service| {
                limited_types.is_none_or(|types| types.contains(&service.service_type))
            })
            .collect()
    }

    pub fn get_service_keys(&self, limited_types: Option<&[ServiceType]>) -> Vec<Vec<u8>> {
        self.get_services(limited_types)
            .into_iter()
            .map(|service| service.service_key.clone())
            .collect()
    }

    pub fn get_num_services(&self) -> usize {
        self.services.len()
    }

    pub fn names(&self) -> Vec<&str> {
        self.services
            .values()
            .map(|service| service.name.as_str())
            .collect()
    }
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::ClientDB;

    fn database() -> (ClientDB, std::path::PathBuf) {
        let directory = std::env::temp_dir().join(format!(
            "hydrus_client_services_{}_{}",
            std::process::id(),
            rand::random::<u64>()
        ));

        (ClientDB::new(&directory).unwrap(), directory)
    }

    #[test]
    fn ensure_local_services_creates_and_loads() {
        let (db, directory) = database();
        let manager = ServicesManager::ensure_local_services(db.db()).unwrap();

        assert_eq!(manager.get_num_services(), 3);
        assert_eq!(manager.get_name(b"my files"), "my files");
        assert_eq!(
            manager.get_service_key_from_name("local tags"),
            Some(b"local tags".as_slice())
        );
        assert_eq!(
            manager.get_service_type(b"trash"),
            Some(ServiceType::LocalFileTrashDomain)
        );

        drop(db);
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn filtering_by_type_matches_python() {
        let (db, directory) = database();
        let manager = ServicesManager::ensure_local_services(db.db()).unwrap();

        let file_domains = manager.get_services(Some(&[ServiceType::LocalFileDomain]));

        assert_eq!(file_domains.len(), 1);
        assert_eq!(file_domains[0].name, "my files");

        drop(db);
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn missing_service_names_fall_back_to_hex() {
        let manager = ServicesManager::default();

        assert_eq!(manager.get_name(&[0xab, 0xcd]), "abcd");
    }
}
