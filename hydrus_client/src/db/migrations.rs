//! Schema migrations for the client database.
//!
//! Python's `ClientDB._update_db` runs one block per version. Most of the
//! 619..689 steps schedule background maintenance or rewrite serialised
//! options; this module applies the durable SQL changes and treats the
//! maintenance steps as no-ops. Every step is idempotent.

use hydrus_core::hydrus_constants::SOFTWARE_VERSION;
use hydrus_core::hydrus_db_base::DBBase;
use hydrus_core::hydrus_exceptions::HydrusResult;

pub const FIRST_SUPPORTED_VERSION: i64 = 619;

/// Applies every step after `from_version` and returns the final version.
pub fn migrate(db: &DBBase, from_version: i64) -> HydrusResult<i64> {
    let mut version = from_version.max(FIRST_SUPPORTED_VERSION);
    let target = SOFTWARE_VERSION as i64;

    while version < target {
        apply_step(db, version)?;
        version += 1;
        db.execute("UPDATE version SET version = ?", &[&version])?;
    }

    Ok(version)
}

fn apply_step(db: &DBBase, version: i64) -> HydrusResult<()> {
    match version {
        // shape_search_cache counters
        638 => {
            db.execute(
                "CREATE TABLE IF NOT EXISTS shape_search_cache_numbers (
                    searched_distance INTEGER PRIMARY KEY,
                    count INTEGER
                )",
                &[],
            )?;
        }
        // service renames
        646 => {
            db.execute(
                "UPDATE services SET name = 'hydrus local file storage'
                 WHERE service_type = ? AND name = 'all local files'",
                &[
                    &(hydrus_core::hydrus_constants::ServiceType::HydrusLocalFileStorage.as_int()
                        as i64),
                ],
            )?;
        }
        648 => {
            db.execute(
                "UPDATE services SET name = 'combined local file domains'
                 WHERE service_type = ? AND name = 'all my files'",
                &[&(hydrus_core::hydrus_constants::ServiceType::CombinedFile.as_int() as i64)],
            )?;
        }
        // storage granularity table
        659 => {
            db.execute(
                "CREATE TABLE IF NOT EXISTS current_storage_granularity (
                    location_id INTEGER PRIMARY KEY,
                    granularity INTEGER
                )",
                &[],
            )?;
        }
        // alternate-pair uniqueness
        678 => {
            db.execute(
                "CREATE TABLE IF NOT EXISTS confirmed_alternate_pairs (
                    larger_media_id INTEGER,
                    smaller_media_id INTEGER,
                    PRIMARY KEY ( larger_media_id, smaller_media_id )
                )",
                &[],
            )?;
            db.execute(
                "CREATE UNIQUE INDEX IF NOT EXISTS confirmed_alternate_pairs_larger_smaller
                 ON confirmed_alternate_pairs ( larger_media_id, smaller_media_id )",
                &[],
            )?;
        }
        // new metadata flag tables
        681 => {
            db.execute(
                "CREATE TABLE IF NOT EXISTS has_xmp ( hash_id INTEGER PRIMARY KEY )",
                &[],
            )?;
            db.execute(
                "CREATE TABLE IF NOT EXISTS has_iptc ( hash_id INTEGER PRIMARY KEY )",
                &[],
            )?;
        }
        // Everything else is background work, option rewrites or a no-op for
        // databases created by this port.
        _ => {}
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::ClientDB;

    fn database() -> (ClientDB, std::path::PathBuf) {
        let directory = std::env::temp_dir().join(format!(
            "hydrus_client_migrations_{}_{}",
            std::process::id(),
            rand::random::<u64>()
        ));

        (ClientDB::new(&directory).unwrap(), directory)
    }

    #[test]
    fn migration_advances_to_the_software_version() {
        let (db, directory) = database();

        db.db()
            .execute("UPDATE version SET version = 619", &[])
            .unwrap();

        let version = migrate(db.db(), 619).unwrap();

        assert_eq!(version, SOFTWARE_VERSION as i64);
        assert_eq!(db.get_version().unwrap(), SOFTWARE_VERSION as i64);

        drop(db);
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn migration_steps_are_idempotent() {
        let (db, directory) = database();

        migrate(db.db(), 638).unwrap();
        migrate(db.db(), 681).unwrap();

        assert!(db.db().table_exists("shape_search_cache_numbers").unwrap());
        assert!(db.db().table_exists("has_xmp").unwrap());
        assert!(db.db().table_exists("has_iptc").unwrap());
        assert!(db.db().table_exists("current_storage_granularity").unwrap());

        drop(db);
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn service_renames_apply_once() {
        let (db, directory) = database();

        db.db()
            .execute(
                "INSERT INTO services ( service_key, service_type, name )
                 VALUES ( ?, ?, ? )",
                &[
                    &b"local files".to_vec(),
                    &(hydrus_core::hydrus_constants::ServiceType::HydrusLocalFileStorage.as_int()
                        as i64),
                    &"all local files",
                ],
            )
            .unwrap();

        migrate(db.db(), 646).unwrap();

        let name: String = db
            .db()
            .query_one(
                "SELECT name FROM services WHERE name = 'hydrus local file storage'",
                &[],
                |row| row.get(0),
            )
            .unwrap()
            .unwrap();

        assert_eq!(name, "hydrus local file storage");

        drop(db);
        let _ = std::fs::remove_dir_all(directory);
    }
}
