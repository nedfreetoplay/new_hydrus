//! Schema tables for tags storage.

pub(super) const TABLES: &str = r#"
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
"#;
