//! Client database facade.
//!
//! The client opens `client.db` synchronously with `hydrus_core::DBBase` and
//! owns the initial schema. Migration steps from 619 to the current version
//! live in [`migrations`]; the threaded `HydrusDB` job queue is not needed by
//! the client's synchronous DB layer.

use std::path::{Path, PathBuf};

use hydrus_core::hydrus_db_base::DBBase;
use hydrus_core::hydrus_exceptions::HydrusResult;
use hydrus_core::hydrus_paths;

pub mod content_updates;
pub mod definitions_cache;
pub mod duplicates;
pub mod files_inbox;
pub mod files_metadata;
pub mod files_physical_storage;
pub mod files_storage;
pub mod files_timestamps;
pub mod files_viewing_stats;
pub mod maintenance;
pub mod mapping_counts;
pub mod mappings;
pub mod master;
pub mod master_tags;
pub mod media_results;
pub mod migrations;
pub mod module;
pub mod notes;
pub mod ratings;
pub mod repositories;
pub mod schema;
pub mod serialisable;
pub mod service_paths;
pub mod services;
pub mod similar_files;
pub mod tag_display;
pub mod tag_relations;
pub mod tag_search;
pub mod url_map;

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
        migrations::migrate(&self.db, self.get_version()?)
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
mod tests;
