//! Schema tables for core storage.

pub(super) const TABLES: &str = r#"
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

CREATE TABLE IF NOT EXISTS files_info_forced_filetypes (
            hash_id INTEGER PRIMARY KEY,
            forced_mime INTEGER
        );

CREATE TABLE IF NOT EXISTS has_icc_profile ( hash_id INTEGER PRIMARY KEY );

CREATE TABLE IF NOT EXISTS has_exif ( hash_id INTEGER PRIMARY KEY );

CREATE TABLE IF NOT EXISTS has_xmp ( hash_id INTEGER PRIMARY KEY );

CREATE TABLE IF NOT EXISTS has_iptc ( hash_id INTEGER PRIMARY KEY );

CREATE TABLE IF NOT EXISTS has_human_readable_embedded_metadata ( hash_id INTEGER PRIMARY KEY );
"#;
