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

        CREATE TABLE IF NOT EXISTS file_inbox ( hash_id INTEGER PRIMARY KEY );

        CREATE TABLE IF NOT EXISTS archive_timestamps (
            hash_id INTEGER PRIMARY KEY,
            timestamp INTEGER
        );
        CREATE TABLE IF NOT EXISTS file_modified_timestamps (
            hash_id INTEGER PRIMARY KEY,
            timestamp INTEGER
        );
        CREATE TABLE IF NOT EXISTS file_domain_modified_timestamps (
            service_id INTEGER,
            hash_id INTEGER,
            timestamp INTEGER,
            PRIMARY KEY ( service_id, hash_id )
        );

        CREATE TABLE IF NOT EXISTS file_viewing_stats (
            hash_id INTEGER,
            canvas_type INTEGER,
            last_viewed_timestamp_ms INTEGER,
            views INTEGER,
            viewtime_ms INTEGER,
            PRIMARY KEY ( hash_id, canvas_type )
        );

        CREATE TABLE IF NOT EXISTS current_client_files_locations (
            path TEXT PRIMARY KEY,
            weight INTEGER,
            max_num_bytes INTEGER
        );
        CREATE TABLE IF NOT EXISTS ideal_client_files_locations (
            path TEXT PRIMARY KEY,
            weight INTEGER,
            max_num_bytes INTEGER
        );
        CREATE TABLE IF NOT EXISTS client_files_subfolders (
            prefix TEXT PRIMARY KEY,
            location TEXT
        );
        CREATE TABLE IF NOT EXISTS ideal_thumbnail_override_location ( path TEXT );
        CREATE TABLE IF NOT EXISTS current_storage_granularity ( granularity INTEGER );

        CREATE TABLE IF NOT EXISTS labels (
            label_id INTEGER PRIMARY KEY,
            label TEXT UNIQUE
        );
        CREATE TABLE IF NOT EXISTS notes (
            note_id INTEGER PRIMARY KEY,
            note TEXT UNIQUE
        );
        CREATE TABLE IF NOT EXISTS file_notes (
            hash_id INTEGER,
            name_id INTEGER,
            note_id INTEGER,
            PRIMARY KEY ( hash_id, name_id )
        );

        CREATE TABLE IF NOT EXISTS local_ratings (
            service_id INTEGER,
            hash_id INTEGER,
            rating REAL,
            PRIMARY KEY ( service_id, hash_id )
        );
        CREATE TABLE IF NOT EXISTS local_incdec_ratings (
            service_id INTEGER,
            hash_id INTEGER,
            rating INTEGER,
            PRIMARY KEY ( service_id, hash_id )
        );

        CREATE TABLE IF NOT EXISTS url_domains (
            domain_id INTEGER PRIMARY KEY,
            domain TEXT UNIQUE
        );
        CREATE TABLE IF NOT EXISTS urls (
            url_id INTEGER PRIMARY KEY,
            domain_id INTEGER,
            url TEXT UNIQUE
        );
        CREATE TABLE IF NOT EXISTS url_map (
            hash_id INTEGER,
            url_id INTEGER,
            PRIMARY KEY ( hash_id, url_id )
        );

        CREATE TABLE IF NOT EXISTS namespaces (
            namespace_id INTEGER PRIMARY KEY,
            namespace TEXT UNIQUE
        );
        CREATE TABLE IF NOT EXISTS subtags (
            subtag_id INTEGER PRIMARY KEY,
            subtag TEXT UNIQUE
        );
        CREATE TABLE IF NOT EXISTS tags (
            tag_id INTEGER PRIMARY KEY,
            namespace_id INTEGER,
            subtag_id INTEGER,
            UNIQUE ( namespace_id, subtag_id )
        );

        CREATE TABLE IF NOT EXISTS last_shutdown_work_time (
            name TEXT PRIMARY KEY,
            timestamp INTEGER
        );
        CREATE TABLE IF NOT EXISTS analyze_timestamps (
            table_name TEXT PRIMARY KEY,
            timestamp INTEGER
        );
        CREATE TABLE IF NOT EXISTS vacuum_timestamps (
            table_name TEXT PRIMARY KEY,
            timestamp INTEGER
        );
        CREATE TABLE IF NOT EXISTS deferred_delete_tables (
            table_name TEXT PRIMARY KEY,
            priority INTEGER
        );

        CREATE TABLE IF NOT EXISTS local_hashes_cache (
            hash_id INTEGER PRIMARY KEY,
            hash BLOB_BYTES UNIQUE
        );
        CREATE TABLE IF NOT EXISTS local_tags_cache (
            tag_id INTEGER PRIMARY KEY,
            tag TEXT UNIQUE
        );

        CREATE TABLE IF NOT EXISTS service_filenames (
            service_id INTEGER,
            hash_id INTEGER,
            filename TEXT,
            PRIMARY KEY ( service_id, hash_id )
        );
        CREATE TABLE IF NOT EXISTS service_directories (
            service_id INTEGER,
            directory_id INTEGER,
            dirname TEXT,
            num_files INTEGER,
            total_size INTEGER,
            note TEXT,
            PRIMARY KEY ( service_id, directory_id )
        );
        CREATE TABLE IF NOT EXISTS service_directory_file_map (
            service_id INTEGER,
            directory_id INTEGER,
            hash_id INTEGER,
            PRIMARY KEY ( service_id, directory_id, hash_id )
        );

        CREATE TABLE IF NOT EXISTS potential_duplicate_pairs (
            smaller_hash_id INTEGER,
            larger_hash_id INTEGER,
            distance INTEGER,
            PRIMARY KEY ( smaller_hash_id, larger_hash_id )
        );
        CREATE TABLE IF NOT EXISTS duplicate_false_positives (
            smaller_hash_id INTEGER,
            larger_hash_id INTEGER,
            PRIMARY KEY ( smaller_hash_id, larger_hash_id )
        );
        CREATE TABLE IF NOT EXISTS alternate_file_groups (
            alternates_group_id INTEGER PRIMARY KEY
        );
        CREATE TABLE IF NOT EXISTS alternate_file_group_members (
            alternates_group_id INTEGER,
            hash_id INTEGER UNIQUE,
            PRIMARY KEY ( alternates_group_id, hash_id )
        );
        CREATE TABLE IF NOT EXISTS duplicate_files (
            media_id INTEGER PRIMARY KEY,
            king_hash_id INTEGER UNIQUE
        );
        CREATE TABLE IF NOT EXISTS duplicate_file_members (
            media_id INTEGER,
            hash_id INTEGER UNIQUE,
            PRIMARY KEY ( media_id, hash_id )
        );

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
