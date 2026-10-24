//! Client database facade.
//!
//! This milestone opens `client.db` synchronously with `hydrus_core::DBBase`
//! and owns the initial schema. The threaded `HydrusDB` facade and the
//! 619→689 migration chain are ported in the DB milestone that follows.

use std::path::{Path, PathBuf};

use hydrus_core::hydrus_constants::SOFTWARE_VERSION;
use hydrus_core::hydrus_db_base::DBBase;
use hydrus_core::hydrus_exceptions::HydrusResult;
use hydrus_core::hydrus_paths;

pub mod files_inbox;
pub mod files_metadata;
pub mod files_physical_storage;
pub mod files_storage;
pub mod files_timestamps;
pub mod files_viewing_stats;
pub mod master;
pub mod schema;
pub mod serialisable;
pub mod services;

pub use services::Service;

pub struct ClientDB {
    db: DBBase,
    path: PathBuf,
}

impl ClientDB {
    pub fn new(db_dir: impl AsRef<Path>) -> HydrusResult<Self> {
        let db_dir = db_dir.as_ref();

        hydrus_paths::make_sure_directory_exists(db_dir)?;

        Self::from_path(db_dir.join("client.db"))
    }

    pub fn from_path(path: impl AsRef<Path>) -> HydrusResult<Self> {
        let path = path.as_ref().to_path_buf();
        let db = DBBase::open(&path)?;

        schema::create_db(&db)?;

        Ok(Self { db, path })
    }

    pub fn db(&self) -> &DBBase {
        &self.db
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn get_version(&self) -> HydrusResult<i64> {
        self.db
            .query_one("SELECT version FROM version", &[], |row| row.get(0))?
            .ok_or_else(|| hydrus_core::hydrus_exceptions::data_missing("version table is empty"))
    }

    pub fn update_db(&self) -> HydrusResult<i64> {
        let mut version = self.get_version()?;

        // Individual schema/data migrations are added by later milestones.
        // Until then we only advance the recorded version so the boot path can
        // report a consistent database.
        if version < SOFTWARE_VERSION as i64 {
            version = SOFTWARE_VERSION as i64;
            self.db
                .execute("UPDATE version SET version = ?", &[&version])?;
        }

        Ok(version)
    }

    pub fn read_options_text(&self) -> HydrusResult<Option<String>> {
        self.db
            .query_one("SELECT options FROM options", &[], |row| row.get(0))
    }

    pub fn write_options_text(&self, options: &str) -> HydrusResult<()> {
        self.db.execute("DELETE FROM options", &[])?;
        self.db
            .execute("INSERT INTO options ( options ) VALUES ( ? )", &[&options])?;

        Ok(())
    }

    pub fn set_serialisable(
        &self,
        name: &str,
        object: &dyn hydrus_core::hydrus_serialisable::SerialisableObject,
    ) -> HydrusResult<()> {
        serialisable::set_serialisable(&self.db, name, object)
    }

    pub fn get_serialisable(
        &self,
        name: &str,
    ) -> HydrusResult<Option<Box<dyn hydrus_core::hydrus_serialisable::SerialisableObject>>> {
        serialisable::get_serialisable(&self.db, name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client_options::ClientOptions;
    use hydrus_core::files::hydrus_file_handling::FileInfo;
    use hydrus_core::hydrus_constants::{Mime, ServiceType};

    fn temporary_db() -> (ClientDB, PathBuf) {
        let directory = std::env::temp_dir().join(format!(
            "hydrus_client_db_{}_{}",
            std::process::id(),
            rand::random::<u64>()
        ));

        let db = ClientDB::new(&directory).unwrap();

        (db, directory)
    }

    #[test]
    fn creates_the_initial_schema_and_version() {
        let (db, directory) = temporary_db();

        assert!(db.db().table_exists("version").unwrap());
        assert!(db.db().table_exists("services").unwrap());
        assert_eq!(db.get_version().unwrap(), SOFTWARE_VERSION as i64);

        drop(db);
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn hashes_are_idempotent() {
        let (db, directory) = temporary_db();
        let hash = vec![0xaa, 0xbb];

        let first = master::get_hash_id(db.db(), &hash).unwrap();
        let second = master::get_hash_id(db.db(), &hash).unwrap();

        assert_eq!(first, second);
        assert_eq!(master::get_hash(db.db(), first).unwrap(), Some(hash));

        drop(db);
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn services_round_trip() {
        let (db, directory) = temporary_db();

        let service_id = services::get_or_add_service(
            db.db(),
            b"my files",
            ServiceType::LocalFileDomain,
            "my files",
        )
        .unwrap();
        let service = services::get_service(db.db(), b"my files")
            .unwrap()
            .unwrap();

        assert_eq!(service.service_id, service_id);
        assert_eq!(service.name, "my files");

        drop(db);
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn file_info_round_trips() {
        let (db, directory) = temporary_db();
        let hash_id = master::get_hash_id(db.db(), &[1, 2, 3]).unwrap();
        let info = FileInfo {
            size: 1024,
            mime: Mime::Png,
            width: Some(100),
            height: Some(200),
            duration_ms: None,
            num_frames: None,
            has_audio: false,
            num_words: None,
        };

        files_metadata::set_file_info(db.db(), hash_id, &info).unwrap();

        assert_eq!(
            files_metadata::get_file_info(db.db(), hash_id).unwrap(),
            Some(info)
        );

        drop(db);
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn client_options_round_trip() {
        let _ = crate::client_options::register_serialisable_type();
        let (db, directory) = temporary_db();
        let mut options = ClientOptions::new();
        options.set_boolean("advanced_mode", true);

        db.set_serialisable("client options", &options).unwrap();
        let loaded = db.get_serialisable("client options").unwrap().unwrap();
        let loaded = loaded.as_any().downcast_ref::<ClientOptions>().unwrap();

        assert!(loaded.get_boolean("advanced_mode"));

        drop(db);
        let _ = std::fs::remove_dir_all(directory);
    }
}
