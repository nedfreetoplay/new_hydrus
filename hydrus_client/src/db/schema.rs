//! Initial client schema.
//!
//! The Python schema is assembled from dozens of DB modules. The port starts
//! with the tables that the boot/controller/options milestone needs; each later
//! DB milestone adds its own module with generation metadata.

use hydrus_core::hydrus_constants::SOFTWARE_VERSION;
use hydrus_core::hydrus_db_base::DBBase;
use hydrus_core::hydrus_exceptions::HydrusResult;

pub fn create_db(db: &DBBase) -> HydrusResult<()> {
    db.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS version ( version INTEGER );
        CREATE TABLE IF NOT EXISTS options ( options TEXT );

        CREATE TABLE IF NOT EXISTS services (
            service_id INTEGER PRIMARY KEY,
            service_key BLOB_BYTES UNIQUE,
            service_type INTEGER,
            name TEXT,
            dictionary_string TEXT
        );
        CREATE UNIQUE INDEX IF NOT EXISTS services_service_key_index ON services ( service_key );
        CREATE INDEX IF NOT EXISTS services_service_type_index ON services ( service_type );

        CREATE TABLE IF NOT EXISTS service_info (
            service_id INTEGER,
            info_type INTEGER,
            info BLOB_BYTES,
            PRIMARY KEY ( service_id, info_type )
        );

        CREATE TABLE IF NOT EXISTS hashes (
            hash_id INTEGER PRIMARY KEY AUTOINCREMENT,
            hash BLOB_BYTES UNIQUE
        );
        CREATE UNIQUE INDEX IF NOT EXISTS hashes_hash_index ON hashes ( hash );

        CREATE TABLE IF NOT EXISTS files_info (
            hash_id INTEGER PRIMARY KEY,
            size INTEGER,
            mime INTEGER,
            width INTEGER,
            height INTEGER,
            duration INTEGER,
            num_frames INTEGER,
            has_audio INTEGER,
            num_words INTEGER
        );
        CREATE INDEX IF NOT EXISTS files_info_mime_index ON files_info ( mime );

        CREATE TABLE IF NOT EXISTS json_dumps (
            name TEXT PRIMARY KEY,
            dump_type INTEGER,
            version INTEGER,
            dump TEXT
        );
        ",
    )?;

    let count: i64 = db
        .query_one("SELECT COUNT(*) FROM version", &[], |row| row.get(0))?
        .unwrap_or(0);

    if count == 0 {
        db.execute(
            "INSERT INTO version ( version ) VALUES ( ? )",
            &[&SOFTWARE_VERSION],
        )?;
    }

    Ok(())
}
