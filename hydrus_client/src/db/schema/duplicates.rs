//! Schema tables for duplicates storage.

pub(super) const TABLES: &str = r#"
CREATE TABLE IF NOT EXISTS service_directory_file_map (
            service_id INTEGER,
            directory_id INTEGER,
            hash_id INTEGER,
            PRIMARY KEY ( service_id, directory_id, hash_id )
        );

CREATE TABLE IF NOT EXISTS remote_thumbnails (
            service_id INTEGER,
            hash_id INTEGER,
            thumbnail BLOB_BYTES,
            PRIMARY KEY ( service_id, hash_id )
        );

CREATE TABLE IF NOT EXISTS repository_updates (
            service_id INTEGER,
            update_index INTEGER,
            update_hash BLOB_BYTES,
            begin_timestamp INTEGER,
            end_timestamp INTEGER,
            processed INTEGER DEFAULT 0,
            PRIMARY KEY ( service_id, update_index )
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

CREATE TABLE IF NOT EXISTS shape_perceptual_hashes (
            hash_id INTEGER,
            perceptual_hash BLOB_BYTES,
            PRIMARY KEY ( hash_id, perceptual_hash )
        );

CREATE TABLE IF NOT EXISTS shape_perceptual_hash_map (
            perceptual_hash BLOB_BYTES,
            hash_id INTEGER,
            PRIMARY KEY ( perceptual_hash, hash_id )
        );

CREATE TABLE IF NOT EXISTS pixel_hash_map (
            pixel_hash BLOB_BYTES,
            hash_id INTEGER,
            PRIMARY KEY ( pixel_hash, hash_id )
        );

CREATE TABLE IF NOT EXISTS shape_search_cache (
            hash_id INTEGER PRIMARY KEY,
            searched_distance INTEGER
        );

CREATE TABLE IF NOT EXISTS json_dumps (
            name TEXT PRIMARY KEY,
            dump_type INTEGER,
            version INTEGER,
            dump TEXT
        );

CREATE TABLE IF NOT EXISTS json_dict (
            name TEXT PRIMARY KEY,
            dump TEXT
        );

CREATE TABLE IF NOT EXISTS json_dumps_named (
            dump_type INTEGER,
            dump_name TEXT,
            version INTEGER,
            timestamp_ms INTEGER,
            dump TEXT,
            PRIMARY KEY ( dump_type, dump_name )
        );

CREATE TABLE IF NOT EXISTS json_dumps_hashed (
            hash BLOB_BYTES,
            dump_type INTEGER,
            version INTEGER,
            dump TEXT,
            PRIMARY KEY ( hash, dump_type )
        );
"#;
