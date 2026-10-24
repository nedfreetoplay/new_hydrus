//! Schema tables for files storage.

pub(super) const TABLES: &str = r#"
CREATE TABLE IF NOT EXISTS has_software_source ( hash_id INTEGER PRIMARY KEY );

CREATE TABLE IF NOT EXISTS has_transparency ( hash_id INTEGER PRIMARY KEY );

CREATE TABLE IF NOT EXISTS blurhashes (
            hash_id INTEGER PRIMARY KEY,
            blurhash TEXT
        );

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
"#;
