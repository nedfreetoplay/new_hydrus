//! Rust port of the server database layer from
//! `hydrus/server/ServerDB.py`.
//!
//! This first slice ports the schema, service registry and repository table
//! creation. Account registration keys, sessions and the repository content
//! update machinery are kept for the next layers; the table layout is already
//! the one those layers expect.

#![allow(dead_code)] // The remaining API is wired up by the resource/session layers.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use chrono::Datelike;
use serde_json::json;

use hydrus_core::hydrus_constants::{
    DEFAULT_SERVER_ADMIN_PORT, REPOSITORIES, SERVER_ADMIN_KEY, SOFTWARE_VERSION, ServiceType,
    TIMEZONE_LOCAL,
};
use hydrus_core::hydrus_db::{DBController, DBHooks, HydrusDB, HydrusDBConfig};
use hydrus_core::hydrus_db_base::{DBBase, DBCursorTransactionWrapper};
use hydrus_core::hydrus_exceptions::{ErrorKind, HydrusException, HydrusResult};
use hydrus_core::hydrus_serialisable::SerialisableObject;
use hydrus_core::hydrus_time;
use hydrus_core::networking::hydrus_network::{
    ServerService, generate_service, generate_service_from_serialisable_tuple,
};

use crate::server_files::ServerFiles;

/// `generate_repository_master_map_table_names( service_id )`
pub fn generate_repository_master_map_table_names(service_id: i64) -> (String, String) {
    let suffix = service_id.to_string();

    (
        format!("external_master.repository_hash_id_map_{suffix}"),
        format!("external_master.repository_tag_id_map_{suffix}"),
    )
}

/// `generate_repository_files_table_names( service_id )`
pub fn generate_repository_files_table_names(
    service_id: i64,
) -> (String, String, String, String, String) {
    let suffix = service_id.to_string();

    (
        format!("current_files_{suffix}"),
        format!("deleted_files_{suffix}"),
        format!("pending_files_{suffix}"),
        format!("petitioned_files_{suffix}"),
        format!("ip_addresses_{suffix}"),
    )
}

/// `generate_repository_mappings_table_names( service_id )`
pub fn generate_repository_mappings_table_names(
    service_id: i64,
) -> (String, String, String, String) {
    let suffix = service_id.to_string();

    (
        format!("external_mappings.current_mappings_{suffix}"),
        format!("external_mappings.deleted_mappings_{suffix}"),
        format!("external_mappings.pending_mappings_{suffix}"),
        format!("external_mappings.petitioned_mappings_{suffix}"),
    )
}

/// `generate_repository_tag_parents_table_names( service_id )`
pub fn generate_repository_tag_parents_table_names(
    service_id: i64,
) -> (String, String, String, String) {
    let suffix = service_id.to_string();

    (
        format!("current_tag_parents_{suffix}"),
        format!("deleted_tag_parents_{suffix}"),
        format!("pending_tag_parents_{suffix}"),
        format!("petitioned_tag_parents_{suffix}"),
    )
}

/// `generate_repository_tag_siblings_table_names( service_id )`
pub fn generate_repository_tag_siblings_table_names(
    service_id: i64,
) -> (String, String, String, String) {
    let suffix = service_id.to_string();

    (
        format!("current_tag_siblings_{suffix}"),
        format!("deleted_tag_siblings_{suffix}"),
        format!("pending_tag_siblings_{suffix}"),
        format!("petitioned_tag_siblings_{suffix}"),
    )
}

/// `generate_repository_update_table_name( service_id )`
pub fn generate_repository_update_table_name(service_id: i64) -> String {
    format!("updates_{service_id}")
}

/// A server database, backed by the core `HydrusDB` job/database machinery.
pub struct ServerDB {
    db: Arc<HydrusDB>,
    files: ServerFiles,
}

impl ServerDB {
    /// `DB( controller, db_dir, db_name )`
    pub fn new(
        controller: Arc<dyn DBController>,
        db_dir: impl Into<PathBuf>,
        db_name: impl Into<String>,
    ) -> HydrusResult<Arc<Self>> {
        let db_dir = db_dir.into();
        let db_name = db_name.into();
        let files = ServerFiles::new(db_dir.join("server_files"));

        let config = HydrusDBConfig {
            db_dir: db_dir.clone(),
            db_name,
            external_databases: vec![
                ("external_mappings".to_string(), "server.mappings.db".to_string()),
                ("external_master".to_string(), "server.master.db".to_string()),
            ],
            additional_db_filenames: vec!["server.master.db".to_string(), "server.mappings.db".to_string()],
            durable_temp_db_filename: None,
            ssl_cert_filename: None,
            ssl_key_filename: None,
            read_write_actions: vec![
                "access_key".to_string(),
                "immediate_content_update".to_string(),
                "registration_keys".to_string(),
            ],
            update_wait_seconds: 2,
        };

        let hooks = Arc::new(ServerDBHooks {
            files: files.clone(),
        });

        let db = HydrusDB::new(config, hooks, controller)?;

        Ok(Arc::new(Self { db, files }))
    }

    pub fn db(&self) -> &Arc<HydrusDB> {
        &self.db
    }

    /// `get_files_dir()`
    pub fn get_files_dir(&self) -> &Path {
        self.files.files_dir()
    }

    pub fn table_exists(&self, table_name: &str) -> HydrusResult<bool> {
        self.db.db_base()?.table_exists(table_name)
    }

    /// `_get_service_ids( limited_types = HC.ALL_SERVICES )`
    pub fn get_service_ids(
        &self,
        limited_types: Option<&[ServiceType]>,
    ) -> HydrusResult<Vec<i64>> {
        let services = self.get_services(limited_types)?;

        let mut ids = Vec::with_capacity(services.len());

        for service in services {
            ids.push(self.get_service_id(service.get_service_key())?);
        }

        Ok(ids)
    }

    /// `_get_service_keys( limited_types = HC.ALL_SERVICES )`
    pub fn get_service_keys(
        &self,
        limited_types: Option<&[ServiceType]>,
    ) -> HydrusResult<Vec<Vec<u8>>> {
        Ok(self
            .get_services(limited_types)?
            .into_iter()
            .map(|service| service.get_service_key().to_vec())
            .collect())
    }

    /// `_get_services( limited_types = HC.ALL_SERVICES )`
    pub fn get_services(
        &self,
        limited_types: Option<&[ServiceType]>,
    ) -> HydrusResult<Vec<ServerService>> {
        let db = self.db.db_base()?;

        let rows = db.query_all(
            "SELECT service_key, service_type, name, port, dictionary_string FROM services ORDER BY service_id;",
            &[],
            |row| {
                Ok((
                    row.get::<_, Vec<u8>>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, String>(4)?,
                ))
            },
        )?;

        let mut services = Vec::with_capacity(rows.len());

        for (service_key, service_type, name, port, dictionary_string) in rows {
            let service_type = u16::try_from(service_type)
                .ok()
                .and_then(ServiceType::try_from_int)
                .ok_or_else(|| {
                    HydrusException::new(
                        ErrorKind::Serialisation,
                        format!("Unknown server service type {service_type}."),
                    )
                })?;

            if let Some(limited_types) = limited_types
                && !limited_types.contains(&service_type)
            {
                continue;
            }

            let info = json!([
                hex_encode(&service_key),
                service_type.as_int(),
                name,
                port,
                dictionary_string,
            ]);

            services.push(generate_service_from_serialisable_tuple(&info)?);
        }

        Ok(services)
    }

    /// `_get_service_id( service_key )`
    pub fn get_service_id(&self, service_key: &[u8]) -> HydrusResult<i64> {
        let db = self.db.db_base()?;

        db.query_one(
            "SELECT service_id FROM services WHERE service_key = ?;",
            &[&service_key],
            |row| row.get::<_, i64>(0),
        )?
        .ok_or_else(|| {
            HydrusException::new(ErrorKind::DataMissing, "Service id error in database")
        })
    }

    /// `_get_service_key( service_id )`
    pub fn get_service_key(&self, service_id: i64) -> HydrusResult<Vec<u8>> {
        let db = self.db.db_base()?;

        db.query_one(
            "SELECT service_key FROM services WHERE service_id = ?;",
            &[&service_id],
            |row| row.get::<_, Vec<u8>>(0),
        )?
        .ok_or_else(|| {
            HydrusException::new(ErrorKind::DataMissing, "Service id error in database")
        })
    }

    /// `_get_service_name( service_id )`
    pub fn get_service_name(&self, service_id: i64) -> HydrusResult<String> {
        let db = self.db.db_base()?;

        db.query_one(
            "SELECT name FROM services WHERE service_id = ?;",
            &[&service_id],
            |row| row.get::<_, String>(0),
        )?
        .ok_or_else(|| {
            HydrusException::new(ErrorKind::DataMissing, "Service id error in database")
        })
    }

    /// `_get_service_type( service_id )`
    pub fn get_service_type(&self, service_id: i64) -> HydrusResult<ServiceType> {
        let db = self.db.db_base()?;

        let type_int = db
            .query_one(
                "SELECT service_type FROM services WHERE service_id = ?;",
                &[&service_id],
                |row| row.get::<_, i64>(0),
            )?
            .ok_or_else(|| {
                HydrusException::new(ErrorKind::DataMissing, "Service id error in database")
            })?;

        u16::try_from(type_int)
            .ok()
            .and_then(ServiceType::try_from_int)
            .ok_or_else(|| {
                HydrusException::new(
                    ErrorKind::Serialisation,
                    format!("Unknown server service type {type_int}."),
                )
            })
    }

    /// `_get_service_info( service_key )`
    #[allow(dead_code)]
    pub fn get_service_info(
        &self,
        service_key: &[u8],
    ) -> HydrusResult<std::collections::HashMap<i64, i64>> {
        let service_id = self.get_service_id(service_key)?;
        let db = self.db.db_base()?;

        let rows = db.query_all(
            "SELECT info_type, info FROM service_info WHERE service_id = ?;",
            &[&service_id],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
        )?;

        Ok(rows.into_iter().collect())
    }

    /// Insert a service row.
    ///
    /// Python's `_add_service` also creates the null/admin account types and
    /// their registration keys. That account setup belongs to the next layer;
    /// this method creates the service and its repository tables.
    pub fn add_service(&self, service: &ServerService) -> HydrusResult<i64> {
        let db = self.db.db_base()?;
        let service_id = insert_service_row(&db, service)?;

        if REPOSITORIES.contains(&service.get_service_type()) {
            create_repository_tables(&db, service_id)?;
        }

        Ok(service_id)
    }
}

struct ServerDBHooks {
    files: ServerFiles,
}

impl DBHooks for ServerDBHooks {
    fn create_db(
        &self,
        db: &DBBase,
        _cursor_transaction_wrapper: &mut DBCursorTransactionWrapper,
    ) -> HydrusResult<()> {
        self.files.prepare()?;

        db.execute_batch(
            "
            CREATE TABLE services ( service_id INTEGER PRIMARY KEY, service_key BLOB_BYTES, service_type INTEGER, name TEXT, port INTEGER, dictionary_string TEXT );

            CREATE TABLE accounts ( account_id INTEGER PRIMARY KEY, service_id INTEGER, account_key BLOB_BYTES, hashed_access_key BLOB_BYTES, account_type_id INTEGER, created INTEGER, expires INTEGER, dictionary_string TEXT );
            CREATE UNIQUE INDEX accounts_account_key_index ON accounts ( account_key );
            CREATE UNIQUE INDEX accounts_hashed_access_key_index ON accounts ( hashed_access_key );

            CREATE TABLE account_scores ( service_id INTEGER, account_id INTEGER, score_type INTEGER, score INTEGER, PRIMARY KEY ( service_id, account_id, score_type ) );

            CREATE TABLE account_types ( account_type_id INTEGER PRIMARY KEY, service_id INTEGER, dump TEXT );

            CREATE TABLE analyze_timestamps ( name TEXT, timestamp INTEGER );

            CREATE TABLE deferred_physical_file_deletes ( master_hash_id INTEGER PRIMARY KEY );
            CREATE TABLE deferred_physical_thumbnail_deletes ( master_hash_id INTEGER PRIMARY KEY );

            CREATE TABLE files_info ( master_hash_id INTEGER PRIMARY KEY, size INTEGER, mime INTEGER, width INTEGER, height INTEGER, duration INTEGER, num_frames INTEGER, num_words INTEGER );

            CREATE TABLE reasons ( reason_id INTEGER PRIMARY KEY, reason TEXT );
            CREATE UNIQUE INDEX reasons_reason_index ON reasons ( reason );

            CREATE TABLE registration_keys ( registration_key BLOB_BYTES PRIMARY KEY, service_id INTEGER, account_type_id INTEGER, account_key BLOB_BYTES, access_key BLOB_BYTES UNIQUE, expires INTEGER );

            CREATE TABLE IF NOT EXISTS service_info ( service_id INTEGER, info_type INTEGER, info INTEGER, PRIMARY KEY ( service_id, info_type ) );

            CREATE TABLE sessions ( session_key BLOB_BYTES, service_id INTEGER, account_id INTEGER, expires INTEGER );

            CREATE TABLE version ( version INTEGER, year INTEGER, month INTEGER );

            CREATE TABLE IF NOT EXISTS external_master.hashes ( master_hash_id INTEGER PRIMARY KEY, hash BLOB_BYTES UNIQUE );
            CREATE TABLE IF NOT EXISTS external_master.tags ( master_tag_id INTEGER PRIMARY KEY, tag TEXT UNIQUE );
            ",
        )?;

        let now = hydrus_time::timestamp_to_datetime(
            hydrus_time::get_now() as f64,
            TIMEZONE_LOCAL,
        );

        db.execute(
            "INSERT INTO version ( version, year, month ) VALUES ( ?, ?, ? );",
            &[
                &(SOFTWARE_VERSION as i64),
                &i64::from(now.year()),
                &i64::from(now.month()),
            ],
        )?;

        let admin_service = generate_service(
            SERVER_ADMIN_KEY.to_vec(),
            ServiceType::ServerAdmin,
            "server admin",
            DEFAULT_SERVER_ADMIN_PORT,
            None,
        )?;

        insert_service_row(db, &admin_service)?;

        Ok(())
    }

    fn update_db(
        &self,
        db: &DBBase,
        _cursor_transaction_wrapper: &mut DBCursorTransactionWrapper,
        version: i64,
    ) -> HydrusResult<()> {
        // The account/repository migrations will be added with those layers.
        // For now the version bump keeps `HydrusDB`'s update loop consistent.
        db.execute(
            "UPDATE version SET version = ?;",
            &[&(version + 1)],
        )?;

        Ok(())
    }
}

fn insert_service_row(db: &DBBase, service: &ServerService) -> HydrusResult<i64> {
    let (service_key, service_type, name, port, dictionary) = service.to_tuple()?;
    let dictionary_string = dictionary.dump_to_string();

    db.execute(
        "INSERT INTO services ( service_key, service_type, name, port, dictionary_string ) VALUES ( ?, ?, ?, ?, ? );",
        &[
            &service_key,
            &i64::from(service_type.as_int()),
            &name,
            &i64::from(port),
            &dictionary_string,
        ],
    )?;

    Ok(db.get_last_row_id())
}

fn create_repository_tables(db: &DBBase, service_id: i64) -> HydrusResult<()> {
    let (hash_map, tag_map) = generate_repository_master_map_table_names(service_id);

    db.execute(
        &format!("CREATE TABLE {hash_map} ( service_hash_id INTEGER PRIMARY KEY, master_hash_id INTEGER UNIQUE, hash_id_timestamp INTEGER );"),
        &[],
    )?;
    create_qualified_index(db, &hash_map, "hash_id_timestamp", "hash_id_timestamp")?;
    db.execute(
        &format!("CREATE TABLE {tag_map} ( service_tag_id INTEGER PRIMARY KEY, master_tag_id INTEGER UNIQUE, tag_id_timestamp INTEGER );"),
        &[],
    )?;
    create_qualified_index(db, &tag_map, "tag_id_timestamp", "tag_id_timestamp")?;

    let (current_files, deleted_files, pending_files, petitioned_files, ip_addresses) =
        generate_repository_files_table_names(service_id);

    db.execute(
        &format!("CREATE TABLE {current_files} ( service_hash_id INTEGER PRIMARY KEY, account_id INTEGER, file_timestamp INTEGER );"),
        &[],
    )?;
    db.execute(
        &format!("CREATE INDEX {current_files}_account_id_index ON {current_files} ( account_id );"),
        &[],
    )?;
    db.execute(
        &format!("CREATE INDEX {current_files}_file_timestamp_index ON {current_files} ( file_timestamp );"),
        &[],
    )?;

    db.execute(
        &format!("CREATE TABLE {deleted_files} ( service_hash_id INTEGER PRIMARY KEY, account_id INTEGER, file_timestamp INTEGER );"),
        &[],
    )?;
    db.execute(
        &format!("CREATE INDEX {deleted_files}_account_id_index ON {deleted_files} ( account_id );"),
        &[],
    )?;
    db.execute(
        &format!("CREATE INDEX {deleted_files}_file_timestamp_index ON {deleted_files} ( file_timestamp );"),
        &[],
    )?;

    db.execute(
        &format!("CREATE TABLE {pending_files} ( master_hash_id INTEGER, account_id INTEGER, reason_id INTEGER, PRIMARY KEY ( master_hash_id, account_id ) ) WITHOUT ROWID;"),
        &[],
    )?;
    db.execute(
        &format!("CREATE INDEX {pending_files}_account_id_reason_id_index ON {pending_files} ( account_id, reason_id );"),
        &[],
    )?;

    db.execute(
        &format!("CREATE TABLE {petitioned_files} ( service_hash_id INTEGER, account_id INTEGER, reason_id INTEGER, PRIMARY KEY ( service_hash_id, account_id ) ) WITHOUT ROWID;"),
        &[],
    )?;
    db.execute(
        &format!("CREATE INDEX {petitioned_files}_account_id_reason_id_index ON {petitioned_files} ( account_id, reason_id );"),
        &[],
    )?;

    db.execute(
        &format!("CREATE TABLE {ip_addresses} ( master_hash_id INTEGER, ip TEXT, ip_timestamp INTEGER );"),
        &[],
    )?;

    let (current_mappings, deleted_mappings, pending_mappings, petitioned_mappings) =
        generate_repository_mappings_table_names(service_id);

    db.execute(
        &format!("CREATE TABLE {current_mappings} ( service_tag_id INTEGER, service_hash_id INTEGER, account_id INTEGER, mapping_timestamp INTEGER, PRIMARY KEY ( service_tag_id, service_hash_id ) ) WITHOUT ROWID;"),
        &[],
    )?;
    create_qualified_index(db, &current_mappings, "account_id", "account_id")?;
    create_qualified_index(
        db,
        &current_mappings,
        "mapping_timestamp",
        "mapping_timestamp",
    )?;

    db.execute(
        &format!("CREATE TABLE {deleted_mappings} ( service_tag_id INTEGER, service_hash_id INTEGER, account_id INTEGER, mapping_timestamp INTEGER, PRIMARY KEY ( service_tag_id, service_hash_id ) ) WITHOUT ROWID;"),
        &[],
    )?;
    create_qualified_index(db, &deleted_mappings, "account_id", "account_id")?;
    create_qualified_index(
        db,
        &deleted_mappings,
        "mapping_timestamp",
        "mapping_timestamp",
    )?;

    db.execute(
        &format!("CREATE TABLE {pending_mappings} ( master_tag_id INTEGER, master_hash_id INTEGER, account_id INTEGER, reason_id INTEGER, PRIMARY KEY ( master_tag_id, master_hash_id, account_id ) ) WITHOUT ROWID;"),
        &[],
    )?;
    create_qualified_index(
        db,
        &pending_mappings,
        "account_id_reason_id",
        "account_id, reason_id",
    )?;

    db.execute(
        &format!("CREATE TABLE {petitioned_mappings} ( service_tag_id INTEGER, service_hash_id INTEGER, account_id INTEGER, reason_id INTEGER, PRIMARY KEY ( service_tag_id, service_hash_id, account_id ) ) WITHOUT ROWID;"),
        &[],
    )?;
    create_qualified_index(
        db,
        &petitioned_mappings,
        "account_id_reason_id",
        "account_id, reason_id",
    )?;

    let (current_parents, deleted_parents, pending_parents, petitioned_parents) =
        generate_repository_tag_parents_table_names(service_id);

    db.execute(
        &format!("CREATE TABLE {current_parents} ( child_service_tag_id INTEGER, parent_service_tag_id INTEGER, account_id INTEGER, parent_timestamp INTEGER, PRIMARY KEY ( child_service_tag_id, parent_service_tag_id ) ) WITHOUT ROWID;"),
        &[],
    )?;
    db.execute(
        &format!("CREATE INDEX {current_parents}_account_id_index ON {current_parents} ( account_id );"),
        &[],
    )?;
    db.execute(
        &format!("CREATE INDEX {current_parents}_parent_timestamp_index ON {current_parents} ( parent_timestamp );"),
        &[],
    )?;

    db.execute(
        &format!("CREATE TABLE {deleted_parents} ( child_service_tag_id INTEGER, parent_service_tag_id INTEGER, account_id INTEGER, parent_timestamp INTEGER, PRIMARY KEY ( child_service_tag_id, parent_service_tag_id ) ) WITHOUT ROWID;"),
        &[],
    )?;
    db.execute(
        &format!("CREATE INDEX {deleted_parents}_account_id_index ON {deleted_parents} ( account_id );"),
        &[],
    )?;
    db.execute(
        &format!("CREATE INDEX {deleted_parents}_parent_timestamp_index ON {deleted_parents} ( parent_timestamp );"),
        &[],
    )?;

    db.execute(
        &format!("CREATE TABLE {pending_parents} ( child_master_tag_id INTEGER, parent_master_tag_id INTEGER, account_id INTEGER, reason_id INTEGER, PRIMARY KEY ( child_master_tag_id, parent_master_tag_id, account_id ) ) WITHOUT ROWID;"),
        &[],
    )?;
    db.execute(
        &format!("CREATE INDEX {pending_parents}_account_id_reason_id_index ON {pending_parents} ( account_id, reason_id );"),
        &[],
    )?;

    db.execute(
        &format!("CREATE TABLE {petitioned_parents} ( child_service_tag_id INTEGER, parent_service_tag_id INTEGER, account_id INTEGER, reason_id INTEGER, PRIMARY KEY ( child_service_tag_id, parent_service_tag_id, account_id ) ) WITHOUT ROWID;"),
        &[],
    )?;
    db.execute(
        &format!("CREATE INDEX {petitioned_parents}_account_id_reason_id_index ON {petitioned_parents} ( account_id, reason_id );"),
        &[],
    )?;

    let (current_siblings, deleted_siblings, pending_siblings, petitioned_siblings) =
        generate_repository_tag_siblings_table_names(service_id);

    db.execute(
        &format!("CREATE TABLE {current_siblings} ( bad_service_tag_id INTEGER PRIMARY KEY, good_service_tag_id INTEGER, account_id INTEGER, sibling_timestamp INTEGER );"),
        &[],
    )?;
    db.execute(
        &format!("CREATE INDEX {current_siblings}_account_id_index ON {current_siblings} ( account_id );"),
        &[],
    )?;
    db.execute(
        &format!("CREATE INDEX {current_siblings}_sibling_timestamp_index ON {current_siblings} ( sibling_timestamp );"),
        &[],
    )?;

    db.execute(
        &format!("CREATE TABLE {deleted_siblings} ( bad_service_tag_id INTEGER PRIMARY KEY, good_service_tag_id INTEGER, account_id INTEGER, sibling_timestamp INTEGER );"),
        &[],
    )?;
    db.execute(
        &format!("CREATE INDEX {deleted_siblings}_account_id_index ON {deleted_siblings} ( account_id );"),
        &[],
    )?;
    db.execute(
        &format!("CREATE INDEX {deleted_siblings}_sibling_timestamp_index ON {deleted_siblings} ( sibling_timestamp );"),
        &[],
    )?;

    db.execute(
        &format!("CREATE TABLE {pending_siblings} ( bad_master_tag_id INTEGER, good_master_tag_id INTEGER, account_id INTEGER, reason_id INTEGER, PRIMARY KEY ( bad_master_tag_id, account_id ) ) WITHOUT ROWID;"),
        &[],
    )?;
    db.execute(
        &format!("CREATE INDEX {pending_siblings}_account_id_reason_id_index ON {pending_siblings} ( account_id, reason_id );"),
        &[],
    )?;

    db.execute(
        &format!("CREATE TABLE {petitioned_siblings} ( bad_service_tag_id INTEGER, good_service_tag_id INTEGER, account_id INTEGER, reason_id INTEGER, PRIMARY KEY ( bad_service_tag_id, account_id ) ) WITHOUT ROWID;"),
        &[],
    )?;
    db.execute(
        &format!("CREATE INDEX {petitioned_siblings}_account_id_reason_id_index ON {petitioned_siblings} ( account_id, reason_id );"),
        &[],
    )?;

    let update_table = generate_repository_update_table_name(service_id);

    db.execute(
        &format!("CREATE TABLE {update_table} ( master_hash_id INTEGER PRIMARY KEY );"),
        &[],
    )?;

    Ok(())
}

fn create_qualified_index(
    db: &DBBase,
    qualified_table: &str,
    index_suffix: &str,
    columns: &str,
) -> HydrusResult<()> {
    let (schema, table) = qualified_table
        .split_once('.')
        .unwrap_or(("main", qualified_table));

    let index_name = format!("{table}_{index_suffix}_index");
    let qualified_index = if schema == "main" {
        index_name
    } else {
        format!("{schema}.{index_name}")
    };

    db.execute(
        &format!("CREATE INDEX {qualified_index} ON {table} ( {columns} );"),
        &[],
    )?;

    Ok(())
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";

    let mut text = String::with_capacity(bytes.len() * 2);

    for byte in bytes {
        text.push(char::from(HEX[usize::from(byte >> 4)]));
        text.push(char::from(HEX[usize::from(byte & 0x0F)]));
    }

    text
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::sync::atomic::{AtomicU64, Ordering};

    use hydrus_core::hydrus_constants::{DEFAULT_SERVICE_PORT, REPOSITORIES};
    use hydrus_core::hydrus_db::NullDBController;

    static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

    struct TestDir {
        path: PathBuf,
    }

    impl TestDir {
        fn new() -> Self {
            let suffix = format!(
                "{}_{}",
                std::process::id(),
                TEST_COUNTER.fetch_add(1, Ordering::Relaxed)
            );

            let path = std::env::temp_dir()
                .join("hydrus-server-db-tests")
                .join(suffix);

            std::fs::create_dir_all(&path).unwrap();

            Self { path }
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }

    fn make_db(dir: &TestDir) -> Arc<ServerDB> {
        hydrus_core::initialise();

        ServerDB::new(Arc::new(NullDBController), &dir.path, "server").unwrap()
    }

    #[test]
    fn table_name_helpers_match_python() {
        assert_eq!(
            generate_repository_master_map_table_names(7),
            (
                "external_master.repository_hash_id_map_7".to_string(),
                "external_master.repository_tag_id_map_7".to_string()
            )
        );
        assert_eq!(
            generate_repository_files_table_names(7).0,
            "current_files_7"
        );
        assert_eq!(
            generate_repository_mappings_table_names(7).0,
            "external_mappings.current_mappings_7"
        );
        assert_eq!(
            generate_repository_tag_parents_table_names(7).0,
            "current_tag_parents_7"
        );
        assert_eq!(
            generate_repository_tag_siblings_table_names(7).0,
            "current_tag_siblings_7"
        );
        assert_eq!(generate_repository_update_table_name(7), "updates_7");
    }

    #[test]
    fn fresh_server_db_creates_the_schema_and_admin_service() {
        let dir = TestDir::new();
        let db = make_db(&dir);

        assert!(db.table_exists("services").unwrap());
        assert!(db.table_exists("files_info").unwrap());
        assert!(db.table_exists("accounts").unwrap());

        let services = db.get_services(None).unwrap();

        assert_eq!(services.len(), 1);
        assert_eq!(services[0].get_name(), "server admin");
        assert_eq!(services[0].get_service_type(), ServiceType::ServerAdmin);
        assert_eq!(services[0].get_port(), DEFAULT_SERVER_ADMIN_PORT);

        assert!(db.get_files_dir().join("00").is_dir());
        assert!(db.get_files_dir().join("ff").is_dir());
    }

    #[test]
    fn service_queries_round_trip_and_filter() {
        let dir = TestDir::new();
        let db = make_db(&dir);

        let admin_key = db.get_service_keys(Some(&[ServiceType::ServerAdmin])).unwrap()[0].clone();
        let admin_id = db.get_service_id(&admin_key).unwrap();

        assert_eq!(db.get_service_name(admin_id).unwrap(), "server admin");
        assert_eq!(
            db.get_service_type(admin_id).unwrap(),
            ServiceType::ServerAdmin
        );
        assert_eq!(db.get_service_key(admin_id).unwrap(), admin_key);

        assert_eq!(
            db.get_service_ids(Some(&[ServiceType::ServerAdmin]))
                .unwrap(),
            vec![admin_id]
        );
        assert!(
            db.get_service_ids(Some(&[ServiceType::FileRepository]))
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn adding_a_repository_creates_its_tables() {
        let dir = TestDir::new();
        let db = make_db(&dir);

        let service = generate_service(
            vec![0xAB; 32],
            ServiceType::FileRepository,
            "test file repository",
            DEFAULT_SERVICE_PORT,
            None,
        )
        .unwrap();

        let service_id = db.add_service(&service).unwrap();

        assert_eq!(
            db.get_service_type(service_id).unwrap(),
            ServiceType::FileRepository
        );

        for table in [
            "current_files_2",
            "deleted_files_2",
            "pending_files_2",
            "petitioned_files_2",
            "ip_addresses_2",
            "current_tag_parents_2",
            "current_tag_siblings_2",
            "updates_2",
        ] {
            assert!(db.table_exists(table).unwrap(), "missing table {table}");
        }

        assert!(REPOSITORIES.contains(&ServiceType::FileRepository));
    }
}
