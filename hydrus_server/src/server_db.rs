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
use sha2::{Digest, Sha256};

use hydrus_core::hydrus_constants::{
    DEFAULT_SERVER_ADMIN_PORT, FILE_REPOSITORY_SERVICE_INFO_TYPES, Mime, REPOSITORIES,
    SERVER_ADMIN_KEY, SOFTWARE_VERSION, ServiceInfoType, ServiceType,
    TAG_REPOSITORY_SERVICE_INFO_TYPES, TIMEZONE_LOCAL,
};
use hydrus_core::hydrus_data::generate_key;
use hydrus_core::hydrus_db::{DBController, DBHooks, HydrusDB, HydrusDBConfig};
use hydrus_core::hydrus_db_base::{DBBase, DBCursorTransactionWrapper};
use hydrus_core::hydrus_exceptions::{ErrorKind, HydrusException, HydrusResult};
use hydrus_core::hydrus_pubsub::Subscriber;
use hydrus_core::hydrus_serialisable::{
    SerialisableDictionary, SerialisableObject, create_from_string,
};
use hydrus_core::hydrus_sessions::{
    Account as SessionAccountTrait, ExistingSession, SessionController,
};
use hydrus_core::hydrus_time;
use hydrus_core::networking::hydrus_network::{
    Account as NetworkAccount, AccountType, ServerService, generate_service,
    generate_service_from_serialisable_tuple,
};

use crate::server_files::ServerFiles;

/// The file dictionary Python passes to `_add_file`.
#[derive(Clone, Debug)]
pub struct ServerFileInput {
    pub hash: Vec<u8>,
    pub size: u64,
    pub mime: Mime,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub duration_ms: Option<i64>,
    pub num_frames: Option<u32>,
    pub num_words: Option<i64>,
    pub path: PathBuf,
    pub thumbnail: Option<Vec<u8>>,
}

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

/// One row from `read( 'sessions' )`.
pub struct ServerSessionRow {
    pub session_key: Vec<u8>,
    pub service_key: Vec<u8>,
    pub account: Arc<NetworkAccount>,
    pub hashed_access_key: Vec<u8>,
    pub expires: i64,
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

    /// `_hash_exists( hash )`
    pub fn hash_exists(&self, hash: &[u8]) -> HydrusResult<bool> {
        let db = self.db.db_base()?;

        Ok(db
            .query_one(
                "SELECT 1 FROM external_master.hashes WHERE hash = ?;",
                &[&hash],
                |row| row.get::<_, i64>(0),
            )?
            .is_some())
    }

    /// `_master_hash_exists( hash )`
    pub fn master_hash_exists(&self, hash: &[u8]) -> HydrusResult<bool> {
        self.hash_exists(hash)
    }

    /// `_get_master_hash_id( hash )`
    pub fn get_master_hash_id(&self, hash: &[u8]) -> HydrusResult<i64> {
        let db = self.db.db_base()?;

        if let Some(master_hash_id) = db.query_one(
            "SELECT master_hash_id FROM external_master.hashes WHERE hash = ?;",
            &[&hash],
            |row| row.get::<_, i64>(0),
        )? {
            return Ok(master_hash_id);
        }

        db.execute(
            "INSERT OR IGNORE INTO external_master.hashes ( hash ) VALUES ( ? );",
            &[&hash],
        )?;

        db.query_one(
            "SELECT master_hash_id FROM external_master.hashes WHERE hash = ?;",
            &[&hash],
            |row| row.get::<_, i64>(0),
        )?
        .ok_or_else(|| {
            HydrusException::new(ErrorKind::DataMissing, "Could not create the master hash id.")
        })
    }

    /// `_get_master_hash_ids( hashes )`
    pub fn get_master_hash_ids(&self, hashes: &[Vec<u8>]) -> HydrusResult<Vec<i64>> {
        hashes
            .iter()
            .map(|hash| self.get_master_hash_id(hash))
            .collect()
    }

    /// `_get_hash( master_hash_id )`
    pub fn get_hash(&self, master_hash_id: i64) -> HydrusResult<Vec<u8>> {
        let db = self.db.db_base()?;

        db.query_one(
            "SELECT hash FROM external_master.hashes WHERE master_hash_id = ?;",
            &[&master_hash_id],
            |row| row.get::<_, Vec<u8>>(0),
        )?
        .ok_or_else(|| HydrusException::new(ErrorKind::DataMissing, "Hash error in database"))
    }

    /// `_get_hashes( master_hash_ids )`
    pub fn get_hashes(&self, master_hash_ids: &[i64]) -> HydrusResult<Vec<Vec<u8>>> {
        master_hash_ids
            .iter()
            .map(|master_hash_id| self.get_hash(*master_hash_id))
            .collect()
    }

    /// `_master_tag_exists( tag )`
    pub fn master_tag_exists(&self, tag: &str) -> HydrusResult<bool> {
        let db = self.db.db_base()?;

        Ok(db
            .query_one(
                "SELECT 1 FROM external_master.tags WHERE tag = ?;",
                &[&tag],
                |row| row.get::<_, i64>(0),
            )?
            .is_some())
    }

    /// `_get_master_tag_id( tag )`
    pub fn get_master_tag_id(&self, tag: &str) -> HydrusResult<i64> {
        let db = self.db.db_base()?;

        if let Some(master_tag_id) = db.query_one(
            "SELECT master_tag_id FROM external_master.tags WHERE tag = ?;",
            &[&tag],
            |row| row.get::<_, i64>(0),
        )? {
            return Ok(master_tag_id);
        }

        db.execute(
            "INSERT OR IGNORE INTO external_master.tags ( tag ) VALUES ( ? );",
            &[&tag],
        )?;

        db.query_one(
            "SELECT master_tag_id FROM external_master.tags WHERE tag = ?;",
            &[&tag],
            |row| row.get::<_, i64>(0),
        )?
        .ok_or_else(|| {
            HydrusException::new(ErrorKind::DataMissing, "Could not create the master tag id.")
        })
    }

    /// `_get_tag( master_tag_id )`
    pub fn get_tag(&self, master_tag_id: i64) -> HydrusResult<String> {
        let db = self.db.db_base()?;

        db.query_one(
            "SELECT tag FROM external_master.tags WHERE master_tag_id = ?;",
            &[&master_tag_id],
            |row| row.get::<_, String>(0),
        )?
        .ok_or_else(|| HydrusException::new(ErrorKind::DataMissing, "Tag error in database"))
    }

    /// `_add_file( file_dict )`
    pub fn add_file(&self, file: &ServerFileInput) -> HydrusResult<i64> {
        let master_hash_id = self.get_master_hash_id(&file.hash)?;
        let db = self.db.db_base()?;

        if db
            .query_one(
                "SELECT 1 FROM files_info WHERE master_hash_id = ?;",
                &[&master_hash_id],
                |row| row.get::<_, i64>(0),
            )?
            .is_none()
        {
            db.execute(
                "INSERT OR IGNORE INTO files_info ( master_hash_id, size, mime, width, height, duration, num_frames, num_words ) VALUES ( ?, ?, ?, ?, ?, ?, ?, ? );",
                &[
                    &master_hash_id,
                    &(file.size as i64),
                    &i64::from(file.mime.as_int()),
                    &file.width.map(i64::from),
                    &file.height.map(i64::from),
                    &file.duration_ms,
                    &file.num_frames.map(i64::from),
                    &file.num_words,
                ],
            )?;
        }

        let dest_path = self.files.get_expected_file_path(&file.hash);

        if !dest_path.exists() {
            hydrus_core::hydrus_paths::mirror_file(&file.path, &dest_path)?;
        }

        if let Some(thumbnail) = &file.thumbnail {
            let thumbnail_path = self.files.get_expected_thumbnail_path(&file.hash);

            if !thumbnail_path.exists() {
                if let Some(parent) = thumbnail_path.parent() {
                    hydrus_core::hydrus_paths::make_sure_directory_exists(parent)?;
                }

                std::fs::write(&thumbnail_path, thumbnail).map_err(|error| {
                    HydrusException::new(
                        ErrorKind::FileMissing,
                        format!(
                            "Could not write the thumbnail \"{}\": {error}",
                            thumbnail_path.display()
                        ),
                    )
                })?;
            }
        }

        Ok(master_hash_id)
    }

    /// `_get_deferred_physical_delete()`
    pub fn get_deferred_physical_delete(
        &self,
    ) -> HydrusResult<(Option<Vec<u8>>, Option<Vec<u8>>)> {
        let db = self.db.db_base()?;

        let file_hash = db
            .query_one(
                "SELECT master_hash_id FROM deferred_physical_file_deletes LIMIT 1;",
                &[],
                |row| row.get::<_, i64>(0),
            )?
            .map(|master_hash_id| self.get_hash(master_hash_id))
            .transpose()?;

        let thumbnail_hash = db
            .query_one(
                "SELECT master_hash_id FROM deferred_physical_thumbnail_deletes LIMIT 1;",
                &[],
                |row| row.get::<_, i64>(0),
            )?
            .map(|master_hash_id| self.get_hash(master_hash_id))
            .transpose()?;

        Ok((file_hash, thumbnail_hash))
    }

    /// `_clear_deferred_physical_delete( file_hash, thumbnail_hash )`
    pub fn clear_deferred_physical_delete(
        &self,
        file_hash: Option<&[u8]>,
        thumbnail_hash: Option<&[u8]>,
    ) -> HydrusResult<()> {
        let db = self.db.db_base()?;

        if let Some(file_hash) = file_hash {
            let master_hash_id = self.get_master_hash_id(file_hash)?;

            db.execute(
                "DELETE FROM deferred_physical_file_deletes WHERE master_hash_id = ?;",
                &[&master_hash_id],
            )?;
        }

        if let Some(thumbnail_hash) = thumbnail_hash {
            let master_hash_id = self.get_master_hash_id(thumbnail_hash)?;

            db.execute(
                "DELETE FROM deferred_physical_thumbnail_deletes WHERE master_hash_id = ?;",
                &[&master_hash_id],
            )?;
        }

        Ok(())
    }

    /// `_defer_files_delete_if_now_orphan( master_hash_ids, definitely_no_thumbnails, ignore_service_id )`
    pub fn defer_files_delete_if_now_orphan(
        &self,
        master_hash_ids: &[i64],
        definitely_no_thumbnails: bool,
        ignore_service_id: Option<i64>,
    ) -> HydrusResult<usize> {
        let orphan_master_hash_ids =
            self.filter_orphan_master_hash_ids(master_hash_ids, ignore_service_id)?;

        if orphan_master_hash_ids.is_empty() {
            return Ok(0);
        }

        let db = self.db.db_base()?;

        for master_hash_id in &orphan_master_hash_ids {
            db.execute(
                "INSERT OR IGNORE INTO deferred_physical_file_deletes ( master_hash_id ) VALUES ( ? );",
                &[master_hash_id],
            )?;

            if !definitely_no_thumbnails {
                db.execute(
                    "INSERT OR IGNORE INTO deferred_physical_thumbnail_deletes ( master_hash_id ) VALUES ( ? );",
                    &[master_hash_id],
                )?;
            }
        }

        Ok(orphan_master_hash_ids.len())
    }

    /// `_Filter_orphan_master_hash_ids( master_hash_ids, ignore_service_id )`
    pub fn filter_orphan_master_hash_ids(
        &self,
        master_hash_ids: &[i64],
        ignore_service_id: Option<i64>,
    ) -> HydrusResult<Vec<i64>> {
        let file_repository_ids =
            self.get_service_ids(Some(&[ServiceType::FileRepository]))?;
        let repository_ids = self.get_service_ids(Some(REPOSITORIES))?;
        let db = self.db.db_base()?;

        let mut orphan_ids = Vec::new();

        for master_hash_id in master_hash_ids {
            let mut is_useful = false;

            for service_id in &file_repository_ids {
                if ignore_service_id == Some(*service_id) {
                    continue;
                }

                let (hash_map, _) =
                    generate_repository_master_map_table_names(*service_id);
                let (current_files, _, _, _, _) =
                    generate_repository_files_table_names(*service_id);

                let query = format!(
                    "SELECT 1 FROM {hash_map} AS hash_map CROSS JOIN {current_files} AS current_files USING ( service_hash_id ) WHERE hash_map.master_hash_id = ? LIMIT 1;"
                );

                if db
                    .query_one(&query, &[master_hash_id], |row| row.get::<_, i64>(0))?
                    .is_some()
                {
                    is_useful = true;
                    break;
                }
            }

            if !is_useful {
                for service_id in &repository_ids {
                    if ignore_service_id == Some(*service_id) {
                        continue;
                    }

                    let update_table = generate_repository_update_table_name(*service_id);
                    let query = format!(
                        "SELECT 1 FROM {update_table} WHERE master_hash_id = ? LIMIT 1;"
                    );

                    if db
                        .query_one(&query, &[master_hash_id], |row| row.get::<_, i64>(0))?
                        .is_some()
                    {
                        is_useful = true;
                        break;
                    }
                }
            }

            if !is_useful {
                orphan_ids.push(*master_hash_id);
            }
        }

        Ok(orphan_ids)
    }

    /// `_is_an_orphan( possible_hash )`
    pub fn is_an_orphan(&self, possible_hash: &[u8]) -> HydrusResult<bool> {
        if !self.hash_exists(possible_hash)? {
            return Ok(true);
        }

        let master_hash_id = self.get_master_hash_id(possible_hash)?;

        Ok(self
            .filter_orphan_master_hash_ids(&[master_hash_id], None)?
            .len()
            == 1)
    }

    /// `_repository_get_ip_timestamp( service_key, account, hash )`
    pub fn get_ip_timestamp(
        &self,
        service_key: &[u8],
        hash: &[u8],
    ) -> HydrusResult<(String, i64)> {
        let service_id = self.get_service_id(service_key)?;
        let master_hash_id = self.get_master_hash_id(hash)?;
        let (_, _, _, _, ip_addresses) =
            generate_repository_files_table_names(service_id);
        let db = self.db.db_base()?;

        db.query_one(
            &format!(
                "SELECT ip, ip_timestamp FROM {ip_addresses} WHERE master_hash_id = ?;"
            ),
            &[&master_hash_id],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)),
        )?
        .ok_or_else(|| {
            HydrusException::new(
                ErrorKind::NotFound,
                "Did not find ip information for that hash.",
            )
        })
    }

    /// The write half of uploader-IP tracking.
    pub fn set_ip_timestamp(
        &self,
        service_key: &[u8],
        hash: &[u8],
        ip: &str,
        timestamp: i64,
    ) -> HydrusResult<()> {
        let service_id = self.get_service_id(service_key)?;
        let master_hash_id = self.get_master_hash_id(hash)?;
        let (_, _, _, _, ip_addresses) =
            generate_repository_files_table_names(service_id);
        let db = self.db.db_base()?;

        db.execute(
            &format!(
                "INSERT INTO {ip_addresses} ( master_hash_id, ip, ip_timestamp ) VALUES ( ?, ?, ? );"
            ),
            &[&master_hash_id, &ip, &timestamp],
        )?;

        Ok(())
    }

    /// `_repository_update_service_info( service_id, info_type, delta )`
    pub fn update_service_info(
        &self,
        service_id: i64,
        info_type: ServiceInfoType,
        delta: i64,
    ) -> HydrusResult<()> {
        if delta == 0 {
            return Ok(());
        }

        self.db.db_base()?.execute(
            "UPDATE service_info SET info = info + ? WHERE service_id = ? AND info_type = ?;",
            &[&delta, &service_id, &i64::from(info_type.as_int())],
        )?;

        Ok(())
    }

    /// `_repository_regenerate_service_info( service_id, info_type )`
    pub fn regenerate_service_info(
        &self,
        service_id: Option<i64>,
        info_type: Option<ServiceInfoType>,
    ) -> HydrusResult<()> {
        let service_ids = match service_id {
            Some(service_id) => vec![service_id],
            None => self.get_service_ids(Some(REPOSITORIES))?,
        };

        for service_id in service_ids {
            let service_type = self.get_service_type(service_id)?;

            if !REPOSITORIES.contains(&service_type) {
                continue;
            }

            let allowable: &[ServiceInfoType] = match service_type {
                ServiceType::FileRepository => FILE_REPOSITORY_SERVICE_INFO_TYPES,
                ServiceType::TagRepository => TAG_REPOSITORY_SERVICE_INFO_TYPES,
                _ => continue,
            };

            let info_types: Vec<ServiceInfoType> = match info_type {
                Some(info_type) => {
                    if allowable.contains(&info_type) {
                        vec![info_type]
                    } else {
                        continue;
                    }
                }
                None => allowable.to_vec(),
            };

            let db = self.db.db_base()?;

            if info_type.is_none() {
                db.execute(
                    "DELETE FROM service_info WHERE service_id = ?;",
                    &[&service_id],
                )?;
            } else {
                for info_type in &info_types {
                    db.execute(
                        "DELETE FROM service_info WHERE service_id = ? AND info_type = ?;",
                        &[&service_id, &i64::from(info_type.as_int())],
                    )?;
                }
            }

            for info_type in info_types {
                let count = self.service_info_count(service_id, service_type, info_type)?;

                db.execute(
                    "INSERT INTO service_info ( service_id, info_type, info ) VALUES ( ?, ?, ? );",
                    &[
                        &service_id,
                        &i64::from(info_type.as_int()),
                        &count,
                    ],
                )?;
            }
        }

        Ok(())
    }

    /// Count one repository service-info value.
    pub fn get_service_info_specific(
        &self,
        service_id: i64,
        info_type: ServiceInfoType,
    ) -> HydrusResult<i64> {
        let service_type = self.get_service_type(service_id)?;

        self.service_info_count(service_id, service_type, info_type)
    }

    fn service_info_count(
        &self,
        service_id: i64,
        service_type: ServiceType,
        info_type: ServiceInfoType,
    ) -> HydrusResult<i64> {
        let (hash_map, tag_map) = generate_repository_master_map_table_names(service_id);
        let (current_files, deleted_files, pending_files, petitioned_files, _) =
            generate_repository_files_table_names(service_id);
        let (current_mappings, deleted_mappings, pending_mappings, petitioned_mappings) =
            generate_repository_mappings_table_names(service_id);
        let (current_siblings, deleted_siblings, pending_siblings, petitioned_siblings) =
            generate_repository_tag_siblings_table_names(service_id);
        let (current_parents, deleted_parents, pending_parents, petitioned_parents) =
            generate_repository_tag_parents_table_names(service_id);

        let table = match info_type {
            ServiceInfoType::NumFileHashes => Some(hash_map.as_str()),
            ServiceInfoType::NumFiles => Some(current_files.as_str()),
            ServiceInfoType::NumDeletedFiles => Some(deleted_files.as_str()),
            ServiceInfoType::NumPendingFiles => Some(pending_files.as_str()),
            ServiceInfoType::NumPetitionedFiles => Some(petitioned_files.as_str()),
            ServiceInfoType::NumTags => Some(tag_map.as_str()),
            ServiceInfoType::NumMappings => Some(current_mappings.as_str()),
            ServiceInfoType::NumDeletedMappings => Some(deleted_mappings.as_str()),
            ServiceInfoType::NumPendingMappings => Some(pending_mappings.as_str()),
            ServiceInfoType::NumPetitionedMappings => Some(petitioned_mappings.as_str()),
            ServiceInfoType::NumTagSiblings => Some(current_siblings.as_str()),
            ServiceInfoType::NumDeletedTagSiblings => Some(deleted_siblings.as_str()),
            ServiceInfoType::NumPendingTagSiblings => Some(pending_siblings.as_str()),
            ServiceInfoType::NumPetitionedTagSiblings => Some(petitioned_siblings.as_str()),
            ServiceInfoType::NumTagParents => Some(current_parents.as_str()),
            ServiceInfoType::NumDeletedTagParents => Some(deleted_parents.as_str()),
            ServiceInfoType::NumPendingTagParents => Some(pending_parents.as_str()),
            ServiceInfoType::NumPetitionedTagParents => Some(petitioned_parents.as_str()),
            _ => None,
        };

        let db = self.db.db_base()?;

        if let Some(table) = table {
            return db
                .query_one(
                    &format!("SELECT COUNT( * ) FROM {table};"),
                    &[],
                    |row| row.get::<_, i64>(0),
                )?
                .ok_or_else(|| {
                    HydrusException::new(ErrorKind::DataMissing, "Could not count service info.")
                });
        }

        let query = match info_type {
            ServiceInfoType::NumActionableFileAddPetitions => format!(
                "SELECT COUNT( * ) FROM ( SELECT DISTINCT account_id, reason_id FROM {pending_files} );"
            ),
            ServiceInfoType::NumActionableFileDeletePetitions => format!(
                "SELECT COUNT( * ) FROM ( SELECT DISTINCT account_id, reason_id FROM {petitioned_files} );"
            ),
            ServiceInfoType::NumActionableMappingAddPetitions => format!(
                "SELECT COUNT( * ) FROM ( SELECT DISTINCT master_tag_id, reason_id FROM {pending_mappings} );"
            ),
            ServiceInfoType::NumActionableMappingDeletePetitions => format!(
                "SELECT COUNT( * ) FROM ( SELECT DISTINCT service_tag_id, reason_id FROM {petitioned_mappings} );"
            ),
            ServiceInfoType::NumActionableSiblingAddPetitions => format!(
                "SELECT COUNT( * ) FROM ( SELECT DISTINCT reason_id FROM {pending_siblings} EXCEPT SELECT DISTINCT reason_id FROM {petitioned_siblings} );"
            ),
            ServiceInfoType::NumActionableSiblingDeletePetitions => format!(
                "SELECT COUNT( * ) FROM ( SELECT DISTINCT reason_id FROM {petitioned_siblings} );"
            ),
            ServiceInfoType::NumActionableParentAddPetitions => format!(
                "SELECT COUNT( * ) FROM ( SELECT DISTINCT reason_id FROM {pending_parents} EXCEPT SELECT DISTINCT reason_id FROM {petitioned_parents} );"
            ),
            ServiceInfoType::NumActionableParentDeletePetitions => format!(
                "SELECT COUNT( * ) FROM ( SELECT DISTINCT reason_id FROM {petitioned_parents} );"
            ),
            _ => {
                return Err(HydrusException::new(
                    ErrorKind::DataMissing,
                    format!(
                        "Was asked to generate service info for an unsupported type: {}",
                        info_type.as_str()
                    ),
                ));
            }
        };

        let _ = service_type;

        db.query_one(&query, &[], |row| row.get::<_, i64>(0))?
            .ok_or_else(|| {
                HydrusException::new(ErrorKind::DataMissing, "Could not count service info.")
            })
    }

    /// `_repository_get_service_hash_id( service_id, master_hash_id, timestamp )`
    pub fn repository_get_service_hash_id(
        &self,
        service_id: i64,
        master_hash_id: i64,
        timestamp: i64,
    ) -> HydrusResult<i64> {
        let (hash_map, _) = generate_repository_master_map_table_names(service_id);
        let db = self.db.db_base()?;

        if let Some(service_hash_id) = db.query_one(
            &format!(
                "SELECT service_hash_id FROM {hash_map} WHERE master_hash_id = ?;"
            ),
            &[&master_hash_id],
            |row| row.get::<_, i64>(0),
        )? {
            return Ok(service_hash_id);
        }

        db.execute(
            &format!(
                "INSERT INTO {hash_map} ( master_hash_id, hash_id_timestamp ) VALUES ( ?, ? );"
            ),
            &[&master_hash_id, &timestamp],
        )?;

        self.update_service_info(service_id, ServiceInfoType::NumFileHashes, 1)?;

        Ok(db.get_last_row_id())
    }

    /// `_repository_get_service_hash_ids( service_id, master_hash_ids, timestamp )`
    pub fn repository_get_service_hash_ids(
        &self,
        service_id: i64,
        master_hash_ids: &[i64],
        timestamp: i64,
    ) -> HydrusResult<Vec<i64>> {
        let mut service_hash_ids = Vec::with_capacity(master_hash_ids.len());

        for master_hash_id in master_hash_ids {
            service_hash_ids.push(self.repository_get_service_hash_id(
                service_id,
                *master_hash_id,
                timestamp,
            )?);
        }

        Ok(service_hash_ids)
    }

    /// `_repository_get_master_hash_ids( service_id, service_hash_ids )`
    pub fn repository_get_master_hash_ids(
        &self,
        service_id: i64,
        service_hash_ids: &[i64],
    ) -> HydrusResult<Vec<i64>> {
        let (hash_map, _) = generate_repository_master_map_table_names(service_id);
        let db = self.db.db_base()?;
        let mut master_hash_ids = Vec::with_capacity(service_hash_ids.len());

        for service_hash_id in service_hash_ids {
            let master_hash_id = db
                .query_one(
                    &format!(
                        "SELECT master_hash_id FROM {hash_map} WHERE service_hash_id = ?;"
                    ),
                    &[service_hash_id],
                    |row| row.get::<_, i64>(0),
                )?
                .ok_or_else(|| {
                    HydrusException::new(
                        ErrorKind::DataMissing,
                        "Missing master_hash_id map error!",
                    )
                })?;

            master_hash_ids.push(master_hash_id);
        }

        Ok(master_hash_ids)
    }

    /// `_repository_service_hash_id_exists( service_id, master_hash_id )`
    pub fn repository_service_hash_id_exists(
        &self,
        service_id: i64,
        master_hash_id: i64,
    ) -> HydrusResult<bool> {
        let (hash_map, _) = generate_repository_master_map_table_names(service_id);

        Ok(self
            .db
            .db_base()?
            .query_one(
                &format!(
                    "SELECT 1 FROM {hash_map} WHERE master_hash_id = ?;"
                ),
                &[&master_hash_id],
                |row| row.get::<_, i64>(0),
            )?
            .is_some())
    }

    /// `_repository_add_file( service_id, account_id, file_dict, overwrite_deleted, timestamp )`
    pub fn repository_add_file(
        &self,
        service_id: i64,
        account_id: i64,
        file: &ServerFileInput,
        overwrite_deleted: bool,
        timestamp: i64,
        ip: Option<&str>,
    ) -> HydrusResult<()> {
        let master_hash_id = self.add_file(file)?;
        let service_hash_id =
            self.repository_get_service_hash_id(service_id, master_hash_id, timestamp)?;
        let (current_files, deleted_files, _, _, ip_addresses) =
            generate_repository_files_table_names(service_id);
        let db = self.db.db_base()?;

        if let Some(ip) = ip {
            db.execute(
                &format!(
                    "INSERT INTO {ip_addresses} ( master_hash_id, ip, ip_timestamp ) VALUES ( ?, ?, ? );"
                ),
                &[&master_hash_id, &ip, &timestamp],
            )?;
        }

        if db
            .query_one(
                &format!(
                    "SELECT 1 FROM {current_files} WHERE service_hash_id = ?;"
                ),
                &[&service_hash_id],
                |row| row.get::<_, i64>(0),
            )?
            .is_some()
        {
            return Ok(());
        }

        if overwrite_deleted {
            let deleted = db.execute(
                &format!(
                    "DELETE FROM {deleted_files} WHERE service_hash_id = ?;"
                ),
                &[&service_hash_id],
            )?;

            if deleted > 0 {
                self.update_service_info(
                    service_id,
                    ServiceInfoType::NumDeletedFiles,
                    -(deleted as i64),
                )?;
            }
        } else if db
            .query_one(
                &format!(
                    "SELECT 1 FROM {deleted_files} WHERE service_hash_id = ?;"
                ),
                &[&service_hash_id],
                |row| row.get::<_, i64>(0),
            )?
            .is_some()
        {
            return Ok(());
        }

        db.execute(
            &format!(
                "INSERT INTO {current_files} ( service_hash_id, account_id, file_timestamp ) VALUES ( ?, ?, ? );"
            ),
            &[&service_hash_id, &account_id, &timestamp],
        )?;

        self.update_service_info(service_id, ServiceInfoType::NumFiles, 1)?;

        self.clear_deferred_physical_delete(Some(&file.hash), Some(&file.hash))?;

        Ok(())
    }

    /// `_repository_delete_files( service_id, account_id, service_hash_ids, timestamp )`
    pub fn repository_delete_files(
        &self,
        service_id: i64,
        account_id: i64,
        service_hash_ids: &[i64],
        timestamp: i64,
    ) -> HydrusResult<()> {
        let (current_files, deleted_files, _, petitioned_files, _) =
            generate_repository_files_table_names(service_id);
        let db = self.db.db_base()?;
        let mut valid_service_hash_ids = Vec::new();

        for service_hash_id in service_hash_ids {
            if db
                .query_one(
                    &format!(
                        "SELECT 1 FROM {current_files} WHERE service_hash_id = ?;"
                    ),
                    &[service_hash_id],
                    |row| row.get::<_, i64>(0),
                )?
                .is_some()
            {
                valid_service_hash_ids.push(*service_hash_id);
            }
        }

        for service_hash_id in &valid_service_hash_ids {
            db.execute(
                &format!(
                    "DELETE FROM {current_files} WHERE service_hash_id = ?;"
                ),
                &[service_hash_id],
            )?;
            db.execute(
                &format!(
                    "DELETE FROM {petitioned_files} WHERE service_hash_id = ?;"
                ),
                &[service_hash_id],
            )?;
        }

        if !valid_service_hash_ids.is_empty() {
            self.update_service_info(
                service_id,
                ServiceInfoType::NumFiles,
                -(valid_service_hash_ids.len() as i64),
            )?;
        }

        let mut inserted_deleted = 0_i64;

        for service_hash_id in &valid_service_hash_ids {
            inserted_deleted += db.execute(
                &format!(
                    "INSERT OR IGNORE INTO {deleted_files} ( service_hash_id, account_id, file_timestamp ) VALUES ( ?, ?, ? );"
                ),
                &[service_hash_id, &account_id, &timestamp],
            )? as i64;
        }

        if inserted_deleted > 0 {
            self.update_service_info(
                service_id,
                ServiceInfoType::NumDeletedFiles,
                inserted_deleted,
            )?;
        }

        let master_hash_ids =
            self.repository_get_master_hash_ids(service_id, &valid_service_hash_ids)?;

        self.defer_files_delete_if_now_orphan(&master_hash_ids, false, Some(service_id))?;

        Ok(())
    }

    /// `_repository_has_file( service_key, hash )`
    pub fn repository_has_file(
        &self,
        service_key: &[u8],
        hash: &[u8],
    ) -> HydrusResult<(bool, Option<Mime>)> {
        if !self.master_hash_exists(hash)? {
            return Ok((false, None));
        }

        let service_id = self.get_service_id(service_key)?;
        let master_hash_id = self.get_master_hash_id(hash)?;
        let (hash_map, _) = generate_repository_master_map_table_names(service_id);
        let (current_files, _, _, _, _) =
            generate_repository_files_table_names(service_id);
        let db = self.db.db_base()?;

        let mime = db
            .query_one(
                &format!(
                    "SELECT mime FROM files_info NATURAL JOIN {hash_map} NATURAL JOIN {current_files} WHERE master_hash_id = ?;"
                ),
                &[&master_hash_id],
                |row| row.get::<_, i64>(0),
            )?
            .and_then(|mime| u16::try_from(mime).ok())
            .and_then(Mime::try_from_int);

        Ok((mime.is_some(), mime))
    }

    /// `_repository_get_service_tag_id( service_id, master_tag_id, timestamp )`
    pub fn repository_get_service_tag_id(
        &self,
        service_id: i64,
        master_tag_id: i64,
        timestamp: i64,
    ) -> HydrusResult<i64> {
        let (_, tag_map) = generate_repository_master_map_table_names(service_id);
        let db = self.db.db_base()?;

        if let Some(service_tag_id) = db.query_one(
            &format!(
                "SELECT service_tag_id FROM {tag_map} WHERE master_tag_id = ?;"
            ),
            &[&master_tag_id],
            |row| row.get::<_, i64>(0),
        )? {
            return Ok(service_tag_id);
        }

        db.execute(
            &format!(
                "INSERT INTO {tag_map} ( master_tag_id, tag_id_timestamp ) VALUES ( ?, ? );"
            ),
            &[&master_tag_id, &timestamp],
        )?;

        self.update_service_info(service_id, ServiceInfoType::NumTags, 1)?;

        Ok(db.get_last_row_id())
    }

    /// `_repository_get_service_tag_ids( service_id, master_tag_ids, timestamp )`
    pub fn repository_get_service_tag_ids(
        &self,
        service_id: i64,
        master_tag_ids: &[i64],
        timestamp: i64,
    ) -> HydrusResult<Vec<i64>> {
        master_tag_ids
            .iter()
            .map(|master_tag_id| {
                self.repository_get_service_tag_id(service_id, *master_tag_id, timestamp)
            })
            .collect()
    }

    /// `_repository_get_master_tag_id( service_id, service_tag_id )`
    pub fn repository_get_master_tag_id(
        &self,
        service_id: i64,
        service_tag_id: i64,
    ) -> HydrusResult<i64> {
        let (_, tag_map) = generate_repository_master_map_table_names(service_id);

        self.db
            .db_base()?
            .query_one(
                &format!(
                    "SELECT master_tag_id FROM {tag_map} WHERE service_tag_id = ?;"
                ),
                &[&service_tag_id],
                |row| row.get::<_, i64>(0),
            )?
            .ok_or_else(|| {
                HydrusException::new(
                    ErrorKind::DataMissing,
                    "Missing master_tag_id map error!",
                )
            })
    }

    /// `_repository_service_tag_id_exists( service_id, master_tag_id )`
    pub fn repository_service_tag_id_exists(
        &self,
        service_id: i64,
        master_tag_id: i64,
    ) -> HydrusResult<bool> {
        let (_, tag_map) = generate_repository_master_map_table_names(service_id);

        Ok(self
            .db
            .db_base()?
            .query_one(
                &format!(
                    "SELECT 1 FROM {tag_map} WHERE master_tag_id = ?;"
                ),
                &[&master_tag_id],
                |row| row.get::<_, i64>(0),
            )?
            .is_some())
    }

    /// `_repository_add_mappings( service_id, account_id, master_tag_id, master_hash_ids, overwrite_deleted, timestamp )`
    pub fn repository_add_mappings(
        &self,
        service_id: i64,
        account_id: i64,
        master_tag_id: i64,
        master_hash_ids: &[i64],
        overwrite_deleted: bool,
        timestamp: i64,
    ) -> HydrusResult<()> {
        let service_tag_id =
            self.repository_get_service_tag_id(service_id, master_tag_id, timestamp)?;
        let mut service_hash_ids =
            self.repository_get_service_hash_ids(service_id, master_hash_ids, timestamp)?;

        let (current_mappings, deleted_mappings, _, _) =
            generate_repository_mappings_table_names(service_id);
        let db = self.db.db_base()?;

        if overwrite_deleted {
            let mut deleted = 0_usize;

            for service_hash_id in &service_hash_ids {
                deleted += db.execute(
                    &format!(
                        "DELETE FROM {deleted_mappings} WHERE service_tag_id = ? AND service_hash_id = ?;"
                    ),
                    &[&service_tag_id, service_hash_id],
                )?;
            }

            if deleted > 0 {
                self.update_service_info(
                    service_id,
                    ServiceInfoType::NumDeletedMappings,
                    -(deleted as i64),
                )?;
            }
        } else {
            service_hash_ids.retain(|service_hash_id| {
                !db.query_one(
                    &format!(
                        "SELECT 1 FROM {deleted_mappings} WHERE service_tag_id = ? AND service_hash_id = ?;"
                    ),
                    &[&service_tag_id, service_hash_id],
                    |row| row.get::<_, i64>(0),
                )
                .map(|result| result.is_some())
                .unwrap_or(false)
            });
        }

        let mut inserted = 0_i64;

        for service_hash_id in &service_hash_ids {
            inserted += db.execute(
                &format!(
                    "INSERT OR IGNORE INTO {current_mappings} ( service_tag_id, service_hash_id, account_id, mapping_timestamp ) VALUES ( ?, ?, ?, ? );"
                ),
                &[&service_tag_id, service_hash_id, &account_id, &timestamp],
            )? as i64;
        }

        if inserted > 0 {
            self.update_service_info(
                service_id,
                ServiceInfoType::NumMappings,
                inserted,
            )?;
        }

        Ok(())
    }

    /// `_repository_delete_mappings( service_id, account_id, service_tag_id, service_hash_ids, timestamp )`
    pub fn repository_delete_mappings(
        &self,
        service_id: i64,
        account_id: i64,
        service_tag_id: i64,
        service_hash_ids: &[i64],
        timestamp: i64,
    ) -> HydrusResult<()> {
        let (current_mappings, deleted_mappings, _, petitioned_mappings) =
            generate_repository_mappings_table_names(service_id);
        let db = self.db.db_base()?;
        let mut valid_service_hash_ids = Vec::new();

        for service_hash_id in service_hash_ids {
            if db
                .query_one(
                    &format!(
                        "SELECT 1 FROM {current_mappings} WHERE service_tag_id = ? AND service_hash_id = ?;"
                    ),
                    &[&service_tag_id, service_hash_id],
                    |row| row.get::<_, i64>(0),
                )?
                .is_some()
            {
                valid_service_hash_ids.push(*service_hash_id);
            }
        }

        for service_hash_id in &valid_service_hash_ids {
            db.execute(
                &format!(
                    "DELETE FROM {current_mappings} WHERE service_tag_id = ? AND service_hash_id = ?;"
                ),
                &[&service_tag_id, service_hash_id],
            )?;
            db.execute(
                &format!(
                    "DELETE FROM {petitioned_mappings} WHERE service_tag_id = ? AND service_hash_id = ?;"
                ),
                &[&service_tag_id, service_hash_id],
            )?;
        }

        if !valid_service_hash_ids.is_empty() {
            self.update_service_info(
                service_id,
                ServiceInfoType::NumMappings,
                -(valid_service_hash_ids.len() as i64),
            )?;
        }

        let mut inserted_deleted = 0_i64;

        for service_hash_id in &valid_service_hash_ids {
            inserted_deleted += db.execute(
                &format!(
                    "INSERT OR IGNORE INTO {deleted_mappings} ( service_tag_id, service_hash_id, account_id, mapping_timestamp ) VALUES ( ?, ?, ?, ? );"
                ),
                &[&service_tag_id, service_hash_id, &account_id, &timestamp],
            )? as i64;
        }

        if inserted_deleted > 0 {
            self.update_service_info(
                service_id,
                ServiceInfoType::NumDeletedMappings,
                inserted_deleted,
            )?;
        }

        Ok(())
    }

    /// `_repository_add_tag_parent( service_id, account_id, child_master_tag_id, parent_master_tag_id, overwrite_deleted, timestamp )`
    pub fn repository_add_tag_parent(
        &self,
        service_id: i64,
        account_id: i64,
        child_master_tag_id: i64,
        parent_master_tag_id: i64,
        overwrite_deleted: bool,
        timestamp: i64,
    ) -> HydrusResult<()> {
        let child_service_tag_id =
            self.repository_get_service_tag_id(service_id, child_master_tag_id, timestamp)?;
        let parent_service_tag_id =
            self.repository_get_service_tag_id(service_id, parent_master_tag_id, timestamp)?;
        let (current_parents, deleted_parents, pending_parents, _) =
            generate_repository_tag_parents_table_names(service_id);
        let db = self.db.db_base()?;

        if overwrite_deleted {
            let deleted = db.execute(
                &format!(
                    "DELETE FROM {deleted_parents} WHERE child_service_tag_id = ? AND parent_service_tag_id = ?;"
                ),
                &[&child_service_tag_id, &parent_service_tag_id],
            )?;

            if deleted > 0 {
                self.update_service_info(
                    service_id,
                    ServiceInfoType::NumDeletedTagParents,
                    -(deleted as i64),
                )?;
            }
        } else if db
            .query_one(
                &format!(
                    "SELECT 1 FROM {deleted_parents} WHERE child_service_tag_id = ? AND parent_service_tag_id = ?;"
                ),
                &[&child_service_tag_id, &parent_service_tag_id],
                |row| row.get::<_, i64>(0),
            )?
            .is_some()
        {
            return Ok(());
        }

        let pending_deleted = db.execute(
            &format!(
                "DELETE FROM {pending_parents} WHERE child_master_tag_id = ? AND parent_master_tag_id = ?;"
            ),
            &[&child_master_tag_id, &parent_master_tag_id],
        )?;

        if pending_deleted > 0 {
            self.update_service_info(
                service_id,
                ServiceInfoType::NumPendingTagParents,
                -(pending_deleted as i64),
            )?;
        }

        let inserted = db.execute(
            &format!(
                "INSERT OR IGNORE INTO {current_parents} ( child_service_tag_id, parent_service_tag_id, account_id, parent_timestamp ) VALUES ( ?, ?, ?, ? );"
            ),
            &[
                &child_service_tag_id,
                &parent_service_tag_id,
                &account_id,
                &timestamp,
            ],
        )?;

        if inserted > 0 {
            self.update_service_info(
                service_id,
                ServiceInfoType::NumTagParents,
                inserted as i64,
            )?;
        }

        Ok(())
    }

    /// `_repository_add_tag_sibling( service_id, account_id, bad_master_tag_id, good_master_tag_id, overwrite_deleted, timestamp )`
    pub fn repository_add_tag_sibling(
        &self,
        service_id: i64,
        account_id: i64,
        bad_master_tag_id: i64,
        good_master_tag_id: i64,
        overwrite_deleted: bool,
        timestamp: i64,
    ) -> HydrusResult<()> {
        let bad_service_tag_id =
            self.repository_get_service_tag_id(service_id, bad_master_tag_id, timestamp)?;
        let good_service_tag_id =
            self.repository_get_service_tag_id(service_id, good_master_tag_id, timestamp)?;
        let (current_siblings, deleted_siblings, pending_siblings, _) =
            generate_repository_tag_siblings_table_names(service_id);
        let db = self.db.db_base()?;

        if overwrite_deleted {
            let deleted = db.execute(
                &format!(
                    "DELETE FROM {deleted_siblings} WHERE bad_service_tag_id = ? AND good_service_tag_id = ?;"
                ),
                &[&bad_service_tag_id, &good_service_tag_id],
            )?;

            if deleted > 0 {
                self.update_service_info(
                    service_id,
                    ServiceInfoType::NumDeletedTagSiblings,
                    -(deleted as i64),
                )?;
            }
        } else if db
            .query_one(
                &format!(
                    "SELECT 1 FROM {deleted_siblings} WHERE bad_service_tag_id = ? AND good_service_tag_id = ?;"
                ),
                &[&bad_service_tag_id, &good_service_tag_id],
                |row| row.get::<_, i64>(0),
            )?
            .is_some()
        {
            return Ok(());
        }

        let pending_deleted = db.execute(
            &format!(
                "DELETE FROM {pending_siblings} WHERE bad_master_tag_id = ? AND good_master_tag_id = ?;"
            ),
            &[&bad_master_tag_id, &good_master_tag_id],
        )?;

        if pending_deleted > 0 {
            self.update_service_info(
                service_id,
                ServiceInfoType::NumPendingTagSiblings,
                -(pending_deleted as i64),
            )?;
        }

        let inserted = db.execute(
            &format!(
                "INSERT OR IGNORE INTO {current_siblings} ( bad_service_tag_id, good_service_tag_id, account_id, sibling_timestamp ) VALUES ( ?, ?, ?, ? );"
            ),
            &[
                &bad_service_tag_id,
                &good_service_tag_id,
                &account_id,
                &timestamp,
            ],
        )?;

        if inserted > 0 {
            self.update_service_info(
                service_id,
                ServiceInfoType::NumTagSiblings,
                inserted as i64,
            )?;
        }

        Ok(())
    }

    /// `_repository_delete_tag_parent( service_id, account_id, child_service_tag_id, parent_service_tag_id, timestamp )`
    pub fn repository_delete_tag_parent(
        &self,
        service_id: i64,
        account_id: i64,
        child_service_tag_id: i64,
        parent_service_tag_id: i64,
        timestamp: i64,
    ) -> HydrusResult<()> {
        let (current_parents, deleted_parents, _, petitioned_parents) =
            generate_repository_tag_parents_table_names(service_id);
        let db = self.db.db_base()?;

        let deleted = db.execute(
            &format!(
                "DELETE FROM {current_parents} WHERE child_service_tag_id = ? AND parent_service_tag_id = ?;"
            ),
            &[&child_service_tag_id, &parent_service_tag_id],
        )?;

        if deleted > 0 {
            self.update_service_info(
                service_id,
                ServiceInfoType::NumTagParents,
                -(deleted as i64),
            )?;
        }

        db.execute(
            &format!(
                "DELETE FROM {petitioned_parents} WHERE child_service_tag_id = ? AND parent_service_tag_id = ?;"
            ),
            &[&child_service_tag_id, &parent_service_tag_id],
        )?;

        let inserted = db.execute(
            &format!(
                "INSERT OR IGNORE INTO {deleted_parents} ( child_service_tag_id, parent_service_tag_id, account_id, parent_timestamp ) VALUES ( ?, ?, ?, ? );"
            ),
            &[
                &child_service_tag_id,
                &parent_service_tag_id,
                &account_id,
                &timestamp,
            ],
        )?;

        if inserted > 0 {
            self.update_service_info(
                service_id,
                ServiceInfoType::NumDeletedTagParents,
                inserted as i64,
            )?;
        }

        Ok(())
    }

    /// `_repository_delete_tag_sibling( service_id, account_id, bad_service_tag_id, good_service_tag_id, timestamp )`
    pub fn repository_delete_tag_sibling(
        &self,
        service_id: i64,
        account_id: i64,
        bad_service_tag_id: i64,
        good_service_tag_id: i64,
        timestamp: i64,
    ) -> HydrusResult<()> {
        let (current_siblings, deleted_siblings, _, petitioned_siblings) =
            generate_repository_tag_siblings_table_names(service_id);
        let db = self.db.db_base()?;

        let deleted = db.execute(
            &format!(
                "DELETE FROM {current_siblings} WHERE bad_service_tag_id = ? AND good_service_tag_id = ?;"
            ),
            &[&bad_service_tag_id, &good_service_tag_id],
        )?;

        if deleted > 0 {
            self.update_service_info(
                service_id,
                ServiceInfoType::NumTagSiblings,
                -(deleted as i64),
            )?;
        }

        db.execute(
            &format!(
                "DELETE FROM {petitioned_siblings} WHERE bad_service_tag_id = ? AND good_service_tag_id = ?;"
            ),
            &[&bad_service_tag_id, &good_service_tag_id],
        )?;

        let inserted = db.execute(
            &format!(
                "INSERT OR IGNORE INTO {deleted_siblings} ( bad_service_tag_id, good_service_tag_id, account_id, sibling_timestamp ) VALUES ( ?, ?, ?, ? );"
            ),
            &[
                &bad_service_tag_id,
                &good_service_tag_id,
                &account_id,
                &timestamp,
            ],
        )?;

        if inserted > 0 {
            self.update_service_info(
                service_id,
                ServiceInfoType::NumDeletedTagSiblings,
                inserted as i64,
            )?;
        }

        Ok(())
    }

    /// `_repository_get_current_mappings_count( service_id, service_tag_id )`
    pub fn repository_get_current_mappings_count(
        &self,
        service_id: i64,
        service_tag_id: i64,
    ) -> HydrusResult<i64> {
        let (current_mappings, _, _, _) =
            generate_repository_mappings_table_names(service_id);

        self.db
            .db_base()?
            .query_one(
                &format!(
                    "SELECT COUNT( * ) FROM {current_mappings} WHERE service_tag_id = ?;"
                ),
                &[&service_tag_id],
                |row| row.get::<_, i64>(0),
            )?
            .ok_or_else(|| {
                HydrusException::new(ErrorKind::DataMissing, "Could not count mappings.")
            })
    }

    /// `_add_account_type( service_id, account_type )`
    pub fn add_account_type(
        &self,
        service_id: i64,
        account_type: &AccountType,
    ) -> HydrusResult<i64> {
        add_account_type(&self.db.db_base()?, service_id, account_type)
    }

    /// `_get_account_type( service_id, account_type_id )`
    pub fn get_account_type(
        &self,
        service_id: i64,
        account_type_id: i64,
    ) -> HydrusResult<Arc<AccountType>> {
        get_account_type(&self.db.db_base()?, service_id, account_type_id)
    }

    /// `_get_account_types( service_key, account )`
    pub fn get_account_types(&self, service_key: &[u8]) -> HydrusResult<Vec<Arc<AccountType>>> {
        let service_id = self.get_service_id(service_key)?;

        get_account_types(&self.db.db_base()?, service_id)
    }

    /// `_get_account_type_id( service_id, account_type_key )`
    pub fn get_account_type_id(
        &self,
        service_id: i64,
        account_type_key: &[u8],
    ) -> HydrusResult<i64> {
        get_account_type_id(&self.db.db_base()?, service_id, account_type_key)
    }

    /// `_generate_registration_keys( service_id, num, account_type_id, expires, force_registration_key = None )`
    pub fn generate_registration_keys(
        &self,
        service_id: i64,
        num: usize,
        account_type_id: i64,
        expires: Option<i64>,
        force_registration_key: Option<&[u8]>,
    ) -> HydrusResult<Vec<Vec<u8>>> {
        generate_registration_keys(
            &self.db.db_base()?,
            service_id,
            num,
            account_type_id,
            expires,
            force_registration_key,
        )
    }

    /// `_get_access_key( service_key, registration_key )`
    pub fn get_access_key(
        &self,
        service_key: &[u8],
        registration_key: &[u8],
    ) -> HydrusResult<Vec<u8>> {
        let service_id = self.get_service_id(service_key)?;

        get_access_key(&self.db.db_base()?, service_id, registration_key)
    }

    /// `_get_account_key_from_access_key( service_key, access_key )`
    pub fn get_account_key_from_access_key(
        &self,
        service_key: &[u8],
        access_key: &[u8],
    ) -> HydrusResult<Vec<u8>> {
        let service_id = self.get_service_id(service_key)?;

        get_account_key_from_access_key(&self.db.db_base()?, service_id, access_key)
    }

    /// `_get_all_accounts( service_key, admin_account )`
    pub fn get_all_accounts(&self, service_key: &[u8]) -> HydrusResult<Vec<NetworkAccount>> {
        let service_id = self.get_service_id(service_key)?;
        let db = self.db.db_base()?;

        let account_ids = db.query_all(
            "SELECT account_id FROM accounts WHERE service_id = ? ORDER BY account_id;",
            &[&service_id],
            |row| row.get::<_, i64>(0),
        )?;

        account_ids
            .into_iter()
            .map(|account_id| get_account(&db, service_id, account_id))
            .collect()
    }

    /// `_get_account( service_id, account_id )`
    pub fn get_account(&self, service_id: i64, account_id: i64) -> HydrusResult<NetworkAccount> {
        get_account(&self.db.db_base()?, service_id, account_id)
    }

    /// `_get_account_from_account_key( service_key, account_key )`
    pub fn get_account_from_account_key(
        &self,
        service_key: &[u8],
        account_key: &[u8],
    ) -> HydrusResult<NetworkAccount> {
        let service_id = self.get_service_id(service_key)?;
        let account_id = self.get_account_id(account_key)?;

        self.get_account(service_id, account_id)
    }

    /// `_get_account_id( account_key )`
    pub fn get_account_id(&self, account_key: &[u8]) -> HydrusResult<i64> {
        get_account_id(&self.db.db_base()?, account_key)
    }

    /// `_get_account_key_from_account_id( account_id )`
    pub fn get_account_key_from_account_id(&self, account_id: i64) -> HydrusResult<Vec<u8>> {
        get_account_key_from_account_id(&self.db.db_base()?, account_id)
    }

    /// `_verify_access_key( service_key, access_key )`
    pub fn verify_access_key(
        &self,
        service_key: &[u8],
        access_key: &[u8],
    ) -> HydrusResult<bool> {
        let service_id = self.get_service_id(service_key)?;

        verify_access_key(&self.db.db_base()?, service_id, access_key)
    }

    /// `_add_session( session_key, service_key, account_key, expires )`
    pub fn add_session(
        &self,
        session_key: &[u8],
        service_key: &[u8],
        account_key: &[u8],
        expires: i64,
    ) -> HydrusResult<()> {
        let service_id = self.get_service_id(service_key)?;
        let account_id = self.get_account_id(account_key)?;
        let db = self.db.db_base()?;

        db.execute(
            "INSERT INTO sessions ( session_key, service_id, account_id, expires ) VALUES ( ?, ?, ?, ? );",
            &[&session_key, &service_id, &account_id, &expires],
        )?;

        Ok(())
    }

    /// `_get_sessions( service_key = None )`
    pub fn get_sessions(
        &self,
        service_key: Option<&[u8]>,
    ) -> HydrusResult<Vec<ServerSessionRow>> {
        let db = self.db.db_base()?;
        let now = hydrus_time::get_now();

        db.execute("DELETE FROM sessions WHERE ? > expires;", &[&now])?;

        let rows = match service_key {
            Some(service_key) => {
                let service_id = self.get_service_id(service_key)?;

                db.query_all(
                    "SELECT session_key, service_id, account_id, expires FROM sessions WHERE service_id = ?;",
                    &[&service_id],
                    |row| {
                        Ok((
                            row.get::<_, Vec<u8>>(0)?,
                            row.get::<_, i64>(1)?,
                            row.get::<_, i64>(2)?,
                            row.get::<_, i64>(3)?,
                        ))
                    },
                )?
            }
            None => db.query_all(
                "SELECT session_key, service_id, account_id, expires FROM sessions;",
                &[],
                |row| {
                    Ok((
                        row.get::<_, Vec<u8>>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, i64>(2)?,
                        row.get::<_, i64>(3)?,
                    ))
                },
            )?,
        };

        let mut service_ids_to_keys: std::collections::HashMap<i64, Vec<u8>> =
            std::collections::HashMap::new();
        let mut account_ids_to_accounts: std::collections::HashMap<i64, Arc<NetworkAccount>> =
            std::collections::HashMap::new();
        let mut account_ids_to_hashed_access_keys: std::collections::HashMap<i64, Vec<u8>> =
            std::collections::HashMap::new();
        let mut sessions = Vec::with_capacity(rows.len());

        for (session_key, service_id, account_id, expires) in rows {
            let service_key = match service_ids_to_keys.get(&service_id) {
                Some(service_key) => service_key.clone(),
                None => {
                    let service_key = self.get_service_key(service_id)?;
                    service_ids_to_keys.insert(service_id, service_key.clone());
                    service_key
                }
            };

            let account = match account_ids_to_accounts.get(&account_id) {
                Some(account) => account.clone(),
                None => {
                    let account = Arc::new(self.get_account(service_id, account_id)?);
                    account_ids_to_accounts.insert(account_id, Arc::clone(&account));
                    account
                }
            };

            let hashed_access_key = match account_ids_to_hashed_access_keys.get(&account_id) {
                Some(hashed_access_key) => hashed_access_key.clone(),
                None => {
                    let hashed_access_key = db
                        .query_one(
                            "SELECT hashed_access_key FROM accounts WHERE account_id = ?;",
                            &[&account_id],
                            |row| row.get::<_, Vec<u8>>(0),
                        )?
                        .ok_or_else(|| {
                            HydrusException::new(
                                ErrorKind::DataMissing,
                                "Could not find the session account's access key.",
                            )
                        })?;
                    account_ids_to_hashed_access_keys
                        .insert(account_id, hashed_access_key.clone());
                    hashed_access_key
                }
            };

            sessions.push(ServerSessionRow {
                session_key,
                service_key,
                account,
                hashed_access_key,
                expires,
            });
        }

        Ok(sessions)
    }

    /// Insert a service, create its account setup, and return the admin access
    /// key. Python's `_add_service` performs the same sequence and returns the
    /// key from the forced `init` registration token for the admin service.
    pub fn add_service_with_admin_access_key(
        &self,
        service: &ServerService,
    ) -> HydrusResult<(i64, Vec<u8>)> {
        add_service_full(&self.db.db_base()?, service)
    }

    /// Insert a service and return its database id.
    pub fn add_service(&self, service: &ServerService) -> HydrusResult<i64> {
        self.add_service_with_admin_access_key(service)
            .map(|(service_id, _)| service_id)
    }
}

/// Wrapper that gives the core session manager the narrow `Account` interface
/// without changing the network `Account` type.
pub struct ServerSessionAccount {
    account: Arc<NetworkAccount>,
}

impl SessionAccountTrait for ServerSessionAccount {
    fn get_account_key(&self) -> Vec<u8> {
        self.account.get_account_key()
    }

    fn is_dirty(&self) -> bool {
        self.account.is_dirty()
    }
}

/// `SessionController` implementation backed by `ServerDB`.
pub struct ServerSessionController {
    db: Arc<ServerDB>,
}

impl ServerSessionController {
    pub fn new(db: Arc<ServerDB>) -> Self {
        Self { db }
    }
}

impl SessionController for ServerSessionController {
    fn read_sessions(&self, service_key: Option<&[u8]>) -> HydrusResult<Vec<ExistingSession>> {
        Ok(self
            .db
            .get_sessions(service_key)?
            .into_iter()
            .map(|session| ExistingSession {
                session_key: session.session_key,
                service_key: session.service_key,
                account: Arc::new(ServerSessionAccount {
                    account: Arc::clone(&session.account),
                }),
                hashed_access_key: session.hashed_access_key,
                expires: session.expires,
            })
            .collect())
    }

    fn read_account(
        &self,
        service_key: &[u8],
        account_key: &[u8],
    ) -> HydrusResult<Arc<dyn SessionAccountTrait>> {
        let account = Arc::new(
            self.db
                .get_account_from_account_key(service_key, account_key)?,
        );

        Ok(Arc::new(ServerSessionAccount { account }))
    }

    fn read_account_key_from_access_key(
        &self,
        service_key: &[u8],
        access_key: &[u8],
    ) -> HydrusResult<Vec<u8>> {
        self.db
            .get_account_key_from_access_key(service_key, access_key)
    }

    fn write_session(
        &self,
        session_key: &[u8],
        service_key: &[u8],
        account_key: &[u8],
        expires: i64,
    ) -> HydrusResult<()> {
        self.db
            .add_session(session_key, service_key, account_key, expires)
    }

    fn subscribe(&self, _subscriber: Arc<dyn Subscriber>, _method_name: &str, _topic: &str) {
        // Wired to the controller's pub/sub daemon with the controller layer.
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

        add_service_full(db, &admin_service)?;

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

fn add_service_full(
    db: &DBBase,
    service: &ServerService,
) -> HydrusResult<(i64, Vec<u8>)> {
    let service_id = insert_service_row(db, service)?;

    let null_account_type = AccountType::generate_null_account_type();
    let null_account_type_id = add_account_type(db, service_id, &null_account_type)?;

    let null_registration_keys =
        generate_registration_keys(db, service_id, 1, null_account_type_id, None, None)?;
    let null_access_key = get_access_key(db, service_id, &null_registration_keys[0])?;

    // The null access key is consumed here and never shown again.
    let _ = get_account_key_from_access_key(db, service_id, &null_access_key)?;

    let admin_account_type = AccountType::generate_admin_account_type(service.get_service_type())?;
    let admin_account_type_id = add_account_type(db, service_id, &admin_account_type)?;

    let force_registration_key = if service.get_service_type() == ServiceType::ServerAdmin {
        Some(b"init".as_slice())
    } else {
        None
    };

    let admin_registration_keys = generate_registration_keys(
        db,
        service_id,
        1,
        admin_account_type_id,
        None,
        force_registration_key,
    )?;
    let admin_access_key = get_access_key(db, service_id, &admin_registration_keys[0])?;

    if REPOSITORIES.contains(&service.get_service_type()) {
        create_repository_tables(db, service_id)?;
    }

    Ok((service_id, admin_access_key))
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

fn add_account_type(
    db: &DBBase,
    service_id: i64,
    account_type: &AccountType,
) -> HydrusResult<i64> {
    db.execute(
        "INSERT INTO account_types ( service_id, dump ) VALUES ( ?, ? );",
        &[&service_id, &account_type.dump_to_string()],
    )?;

    Ok(db.get_last_row_id())
}

fn parse_account_type(dump: &str) -> HydrusResult<Arc<AccountType>> {
    let object = create_from_string(dump, false)?;

    let account_type = object
        .as_any()
        .downcast_ref::<AccountType>()
        .ok_or_else(|| {
            HydrusException::new(
                ErrorKind::Serialisation,
                "The account type did not deserialise as an AccountType.",
            )
        })?;

    Ok(Arc::new(account_type.clone()))
}

fn get_account_type(
    db: &DBBase,
    service_id: i64,
    account_type_id: i64,
) -> HydrusResult<Arc<AccountType>> {
    let dump = db
        .query_one(
            "SELECT dump FROM account_types WHERE service_id = ? AND account_type_id = ?;",
            &[&service_id, &account_type_id],
            |row| row.get::<_, String>(0),
        )?
        .ok_or_else(|| {
            HydrusException::new(
                ErrorKind::DataMissing,
                "Could not find the given account type for that service!",
            )
        })?;

    parse_account_type(&dump)
}

fn get_account_types(db: &DBBase, service_id: i64) -> HydrusResult<Vec<Arc<AccountType>>> {
    let dumps = db.query_all(
        "SELECT dump FROM account_types WHERE service_id = ? ORDER BY account_type_id;",
        &[&service_id],
        |row| row.get::<_, String>(0),
    )?;

    dumps.iter().map(|dump| parse_account_type(dump)).collect()
}

fn get_account_type_id(
    db: &DBBase,
    service_id: i64,
    account_type_key: &[u8],
) -> HydrusResult<i64> {
    let rows = db.query_all(
        "SELECT account_type_id, dump FROM account_types WHERE service_id = ?;",
        &[&service_id],
        |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)),
    )?;

    for (account_type_id, dump) in rows {
        let account_type = parse_account_type(&dump)?;

        if account_type.get_account_type_key() == account_type_key {
            return Ok(account_type_id);
        }
    }

    Err(HydrusException::new(
        ErrorKind::DataMissing,
        "Could not find the given account type key!",
    ))
}

fn generate_registration_keys(
    db: &DBBase,
    service_id: i64,
    num: usize,
    account_type_id: i64,
    expires: Option<i64>,
    force_registration_key: Option<&[u8]>,
) -> HydrusResult<Vec<Vec<u8>>> {
    let account_type = get_account_type(db, service_id, account_type_id)?;

    if account_type.is_null_account() {
        let existing = db.query_one(
            "SELECT 1 FROM accounts WHERE account_type_id = ?;",
            &[&account_type_id],
            |row| row.get::<_, i64>(0),
        )?;

        if existing.is_some() {
            return Err(HydrusException::new(
                ErrorKind::BadRequest,
                "You cannot create new null accounts!",
            ));
        }
    }

    let mut registration_keys = Vec::with_capacity(num);

    for _ in 0..num {
        let registration_key = match force_registration_key {
            Some(key) => key.to_vec(),
            None => generate_key().to_vec(),
        };
        let account_key = generate_key().to_vec();
        let access_key = generate_key().to_vec();
        let registration_key_hash = sha256(&registration_key);

        db.execute(
            "INSERT INTO registration_keys ( registration_key, service_id, account_type_id, account_key, access_key, expires ) VALUES ( ?, ?, ?, ?, ?, ? );",
            &[
                &registration_key_hash,
                &service_id,
                &account_type_id,
                &account_key,
                &access_key,
                &expires,
            ],
        )?;

        registration_keys.push(registration_key);
    }

    Ok(registration_keys)
}

fn get_access_key(
    db: &DBBase,
    service_id: i64,
    registration_key: &[u8],
) -> HydrusResult<Vec<u8>> {
    let registration_key_hash = sha256(registration_key);

    let exists = db.query_one(
        "SELECT 1 FROM registration_keys WHERE service_id = ? AND registration_key = ?;",
        &[&service_id, &registration_key_hash],
        |row| row.get::<_, i64>(0),
    )?;

    if exists.is_none() {
        return Err(HydrusException::new(
            ErrorKind::InsufficientCredentials,
            "The service could not find that registration token in its database.",
        ));
    }

    let access_key = generate_key().to_vec();

    db.execute(
        "UPDATE registration_keys SET access_key = ? WHERE service_id = ? AND registration_key = ?;",
        &[&access_key, &service_id, &registration_key_hash],
    )?;

    Ok(access_key)
}

fn get_account_key_from_access_key(
    db: &DBBase,
    service_id: i64,
    access_key: &[u8],
) -> HydrusResult<Vec<u8>> {
    let hashed_access_key = sha256(access_key);

    if let Some(account_key) = db.query_one(
        "SELECT account_key FROM accounts WHERE service_id = ? AND hashed_access_key = ?;",
        &[&service_id, &hashed_access_key],
        |row| row.get::<_, Vec<u8>>(0),
    )? {
        return Ok(account_key);
    }

    let Some((account_type_id, account_key, expires)) = db.query_one(
        "SELECT account_type_id, account_key, expires FROM registration_keys WHERE access_key = ?;",
        &[&access_key],
        |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, Vec<u8>>(1)?,
                row.get::<_, Option<i64>>(2)?,
            ))
        },
    )?
    else {
        return Err(HydrusException::new(
            ErrorKind::InsufficientCredentials,
            "The service could not find that account in its database.",
        ));
    };

    db.execute(
        "DELETE FROM registration_keys WHERE access_key = ?;",
        &[&access_key],
    )?;

    let account_type = get_account_type(db, service_id, account_type_id)?;
    let created = hydrus_time::get_now();
    let account = NetworkAccount::new(account_key.clone(), account_type, created, expires);
    let (stored_key, _, _, _, dictionary) =
        NetworkAccount::generate_tuple_from_account(&account)?;

    db.execute(
        "INSERT INTO accounts ( service_id, account_key, hashed_access_key, account_type_id, created, expires, dictionary_string ) VALUES ( ?, ?, ?, ?, ?, ?, ? );",
        &[
            &service_id,
            &stored_key,
            &hashed_access_key,
            &account_type_id,
            &created,
            &expires,
            &dictionary.dump_to_string(),
        ],
    )?;

    Ok(stored_key)
}

fn get_account(db: &DBBase, service_id: i64, account_id: i64) -> HydrusResult<NetworkAccount> {
    let (account_key, account_type_id, created, expires, dictionary_string) = db
        .query_one(
            "SELECT account_key, account_type_id, created, expires, dictionary_string FROM accounts WHERE service_id = ? AND account_id = ?;",
            &[&service_id, &account_id],
            |row| {
                Ok((
                    row.get::<_, Vec<u8>>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, Option<i64>>(3)?,
                    row.get::<_, String>(4)?,
                ))
            },
        )?
        .ok_or_else(|| {
            HydrusException::new(
                ErrorKind::InsufficientCredentials,
                "The service could not find that account in its database.",
            )
        })?;

    let account_type = get_account_type(db, service_id, account_type_id)?;
    let dictionary = parse_dictionary(&dictionary_string)?;

    NetworkAccount::generate_account_from_tuple(
        account_key,
        account_type,
        created,
        expires,
        dictionary,
    )
}

fn get_account_id(db: &DBBase, account_key: &[u8]) -> HydrusResult<i64> {
    db.query_one(
        "SELECT account_id FROM accounts WHERE account_key = ?;",
        &[&account_key],
        |row| row.get::<_, i64>(0),
    )?
    .ok_or_else(|| {
        HydrusException::new(
            ErrorKind::InsufficientCredentials,
            "The service could not find that account id in its database.",
        )
    })
}

fn get_account_key_from_account_id(db: &DBBase, account_id: i64) -> HydrusResult<Vec<u8>> {
    db.query_one(
        "SELECT account_key FROM accounts WHERE account_id = ?;",
        &[&account_id],
        |row| row.get::<_, Vec<u8>>(0),
    )?
    .ok_or_else(|| {
        HydrusException::new(
            ErrorKind::InsufficientCredentials,
            "The service could not find that account_id in its database.",
        )
    })
}

fn verify_access_key(
    db: &DBBase,
    service_id: i64,
    access_key: &[u8],
) -> HydrusResult<bool> {
    let hashed_access_key = sha256(access_key);

    if db
        .query_one(
            "SELECT 1 FROM accounts WHERE service_id = ? AND hashed_access_key = ?;",
            &[&service_id, &hashed_access_key],
            |row| row.get::<_, i64>(0),
        )?
        .is_some()
    {
        return Ok(true);
    }

    Ok(db
        .query_one(
            "SELECT 1 FROM registration_keys WHERE service_id = ? AND access_key = ?;",
            &[&service_id, &access_key],
            |row| row.get::<_, i64>(0),
        )?
        .is_some())
}

fn parse_dictionary(dump: &str) -> HydrusResult<SerialisableDictionary> {
    let object = create_from_string(dump, false)?;

    let dictionary = object
        .as_any()
        .downcast_ref::<SerialisableDictionary>()
        .ok_or_else(|| {
            HydrusException::new(
                ErrorKind::Serialisation,
                "The account dictionary did not deserialise as a SerialisableDictionary.",
            )
        })?;

    Ok(dictionary.clone())
}

fn sha256(data: &[u8]) -> Vec<u8> {
    Sha256::digest(data).to_vec()
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
    use hydrus_core::hydrus_sessions::HydrusSessionManagerServer;

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
    fn file_addition_stores_bytes_thumbnail_and_metadata() {
        let dir = TestDir::new();
        let db = make_db(&dir);

        let source_path = dir.path.join("source.bin");
        std::fs::write(&source_path, b"data").unwrap();

        let hash = vec![0xAB; 32];
        let file = ServerFileInput {
            hash: hash.clone(),
            size: 4,
            mime: Mime::OctetStream,
            width: Some(10),
            height: Some(20),
            duration_ms: None,
            num_frames: None,
            num_words: None,
            path: source_path,
            thumbnail: Some(b"thumbnail".to_vec()),
        };

        let master_hash_id = db.add_file(&file).unwrap();

        assert_eq!(db.get_master_hash_id(&hash).unwrap(), master_hash_id);
        assert_eq!(db.get_hash(master_hash_id).unwrap(), hash);
        assert!(db.hash_exists(&hash).unwrap());
        assert!(db.master_hash_exists(&hash).unwrap());

        let server_files = ServerFiles::new(db.get_files_dir());
        let stored_path = server_files.get_expected_file_path(&hash);
        let thumbnail_path = server_files.get_expected_thumbnail_path(&hash);

        assert_eq!(std::fs::read(stored_path).unwrap(), b"data");
        assert_eq!(std::fs::read(thumbnail_path).unwrap(), b"thumbnail");

        let db_base = db.db().db_base().unwrap();
        let (size, mime, width, height) = db_base
            .query_one(
                "SELECT size, mime, width, height FROM files_info WHERE master_hash_id = ?;",
                &[&master_hash_id],
                |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, i64>(2)?,
                        row.get::<_, i64>(3)?,
                    ))
                },
            )
            .unwrap()
            .unwrap();

        assert_eq!(size, 4);
        assert_eq!(mime, i64::from(Mime::OctetStream.as_int()));
        assert_eq!((width, height), (10, 20));
    }

    #[test]
    fn master_tags_are_created_once_and_read_back() {
        let dir = TestDir::new();
        let db = make_db(&dir);

        let tag_id = db.get_master_tag_id("blue sky").unwrap();

        assert_eq!(db.get_master_tag_id("blue sky").unwrap(), tag_id);
        assert_eq!(db.get_tag(tag_id).unwrap(), "blue sky");
        assert!(db.master_tag_exists("blue sky").unwrap());
        assert!(!db.master_tag_exists("green sky").unwrap());
    }

    fn add_file_to_new_repository(
        db: &ServerDB,
        dir: &TestDir,
        hash: Vec<u8>,
        label: &str,
    ) -> (i64, Vec<u8>, i64) {
        let source_path = dir.path.join(format!("{label}.bin"));
        std::fs::write(&source_path, b"repository file").unwrap();

        let file = ServerFileInput {
            hash: hash.clone(),
            size: 15,
            mime: Mime::OctetStream,
            width: None,
            height: None,
            duration_ms: None,
            num_frames: None,
            num_words: None,
            path: source_path,
            thumbnail: None,
        };

        let master_hash_id = db.add_file(&file).unwrap();
        let service = generate_service(
            vec![0x22; 32],
            ServiceType::FileRepository,
            format!("{label} repository"),
            DEFAULT_SERVICE_PORT,
            None,
        )
        .unwrap();
        let service_id = db.add_service(&service).unwrap();
        let service_key = service.get_service_key().to_vec();
        let (hash_map, _) = generate_repository_master_map_table_names(service_id);
        let (current_files, _, _, _, _) =
            generate_repository_files_table_names(service_id);
        let db_base = db.db().db_base().unwrap();

        db_base
            .execute(
                &format!(
                    "INSERT INTO {hash_map} ( master_hash_id, hash_id_timestamp ) VALUES ( ?, ? );"
                ),
                &[&master_hash_id, &hydrus_time::get_now()],
            )
            .unwrap();
        let service_hash_id = db_base.get_last_row_id();
        db_base
            .execute(
                &format!(
                    "INSERT INTO {current_files} ( service_hash_id, account_id, file_timestamp ) VALUES ( ?, ?, ? );"
                ),
                &[&service_hash_id, &1_i64, &hydrus_time::get_now()],
            )
            .unwrap();

        (service_id, service_key, master_hash_id)
    }

    #[test]
    fn service_info_regeneration_counts_repository_rows() {
        let dir = TestDir::new();
        let db = make_db(&dir);
        let (_service_id, service_key, _master_hash_id) =
            add_file_to_new_repository(&db, &dir, vec![0xA1; 32], "service_info");

        let service_id = db.get_service_id(&service_key).unwrap();

        db.regenerate_service_info(Some(service_id), None).unwrap();

        let info = db.get_service_info(&service_key).unwrap();

        assert_eq!(
            info.get(&i64::from(ServiceInfoType::NumFileHashes.as_int())),
            Some(&1)
        );
        assert_eq!(
            info.get(&i64::from(ServiceInfoType::NumFiles.as_int())),
            Some(&1)
        );
        assert_eq!(
            db.get_service_info_specific(service_id, ServiceInfoType::NumFiles)
                .unwrap(),
            1
        );

        db.update_service_info(service_id, ServiceInfoType::NumFiles, 2)
            .unwrap();

        let info = db.get_service_info(&service_key).unwrap();

        assert_eq!(
            info.get(&i64::from(ServiceInfoType::NumFiles.as_int())),
            Some(&3)
        );
    }

    #[test]
    fn ip_timestamp_round_trips_for_repository_files() {
        let dir = TestDir::new();
        let db = make_db(&dir);
        let (_service_id, service_key, _master_hash_id) =
            add_file_to_new_repository(&db, &dir, vec![0xA2; 32], "ip_timestamp");
        let hash = vec![0xA2; 32];

        db.set_ip_timestamp(&service_key, &hash, "127.0.0.1", 123456)
            .unwrap();

        assert_eq!(
            db.get_ip_timestamp(&service_key, &hash).unwrap(),
            ("127.0.0.1".to_string(), 123456)
        );
    }

    #[test]
    fn admin_registration_key_creates_a_usable_admin_account() {
        let dir = TestDir::new();
        let db = make_db(&dir);

        let admin_key = db
            .get_service_keys(Some(&[ServiceType::ServerAdmin]))
            .unwrap()[0]
            .clone();

        let account_types = db.get_account_types(&admin_key).unwrap();
        assert_eq!(account_types.len(), 2);
        assert!(
            account_types
                .iter()
                .any(|account_type| account_type.get_title() == "null account")
        );
        assert!(
            account_types
                .iter()
                .any(|account_type| account_type.get_title() == "administrator")
        );

        let access_key = db.get_access_key(&admin_key, b"init").unwrap();

        assert!(db.verify_access_key(&admin_key, &access_key).unwrap());

        let account_key = db
            .get_account_key_from_access_key(&admin_key, &access_key)
            .unwrap();

        assert!(db.verify_access_key(&admin_key, &access_key).unwrap());

        let account_id = db.get_account_id(&account_key).unwrap();

        assert!(account_id > 0);
        assert_eq!(
            db.get_account_key_from_account_id(account_id).unwrap(),
            account_key
        );

        let account = db
            .get_account_from_account_key(&admin_key, &account_key)
            .unwrap();

        assert_eq!(account.get_account_key(), account_key);
        assert_eq!(account.get_account_type().get_title(), "administrator");

        // The registration token is consumed when the account is created.
        assert!(db.get_access_key(&admin_key, b"init").is_err());
    }

    #[test]
    fn unknown_registration_key_is_rejected() {
        let dir = TestDir::new();
        let db = make_db(&dir);

        let admin_key = db
            .get_service_keys(Some(&[ServiceType::ServerAdmin]))
            .unwrap()[0]
            .clone();

        let error = db.get_access_key(&admin_key, b"not-the-token").unwrap_err();

        assert!(error.is_a(ErrorKind::InsufficientCredentials));
    }

    #[test]
    fn session_manager_can_add_and_restore_a_session() {
        let dir = TestDir::new();
        let db = make_db(&dir);

        let admin_key = db
            .get_service_keys(Some(&[ServiceType::ServerAdmin]))
            .unwrap()[0]
            .clone();
        let access_key = db.get_access_key(&admin_key, b"init").unwrap();
        let account_key = db
            .get_account_key_from_access_key(&admin_key, &access_key)
            .unwrap();

        let session_controller: Arc<dyn SessionController> =
            Arc::new(ServerSessionController::new(Arc::clone(&db)));
        let manager =
            HydrusSessionManagerServer::new(session_controller).expect("session manager");

        let (session_key, expires) = manager
            .add_session(&admin_key, &access_key)
            .expect("session is created");

        assert!(expires > hydrus_time::get_now());

        let account = manager
            .get_account(&admin_key, &session_key)
            .expect("session resolves to the account");

        assert_eq!(account.get_account_key(), account_key);

        manager
            .refresh_all_accounts(None)
            .expect("sessions reload from the database");

        let reloaded = manager
            .get_account(&admin_key, &session_key)
            .expect("reloaded session resolves");

        assert_eq!(reloaded.get_account_key(), account_key);

        let rows = db.get_sessions(Some(&admin_key)).unwrap();

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].session_key, session_key);
        assert_eq!(rows[0].account.get_account_key(), account_key);
    }

    #[test]
    fn orphan_files_are_deferred_and_cleared() {
        let dir = TestDir::new();
        let db = make_db(&dir);

        let source_path = dir.path.join("orphan.bin");
        std::fs::write(&source_path, b"orphan").unwrap();

        let hash = vec![0xCD; 32];
        let file = ServerFileInput {
            hash: hash.clone(),
            size: 6,
            mime: Mime::OctetStream,
            width: None,
            height: None,
            duration_ms: None,
            num_frames: None,
            num_words: None,
            path: source_path,
            thumbnail: Some(b"thumb".to_vec()),
        };

        let master_hash_id = db.add_file(&file).unwrap();

        assert_eq!(
            db.defer_files_delete_if_now_orphan(&[master_hash_id], false, None)
                .unwrap(),
            1
        );
        assert_eq!(
            db.get_deferred_physical_delete().unwrap(),
            (Some(hash.clone()), Some(hash.clone()))
        );

        db.clear_deferred_physical_delete(Some(&hash), Some(&hash))
            .unwrap();

        assert_eq!(db.get_deferred_physical_delete().unwrap(), (None, None));
    }

    #[test]
    fn current_repository_files_are_not_orphans() {
        let dir = TestDir::new();
        let db = make_db(&dir);

        let source_path = dir.path.join("current.bin");
        std::fs::write(&source_path, b"current").unwrap();

        let hash = vec![0xEF; 32];
        let file = ServerFileInput {
            hash: hash.clone(),
            size: 7,
            mime: Mime::OctetStream,
            width: None,
            height: None,
            duration_ms: None,
            num_frames: None,
            num_words: None,
            path: source_path,
            thumbnail: None,
        };

        let master_hash_id = db.add_file(&file).unwrap();
        let service = generate_service(
            vec![0x11; 32],
            ServiceType::FileRepository,
            "orphan test repository",
            DEFAULT_SERVICE_PORT,
            None,
        )
        .unwrap();
        let service_id = db.add_service(&service).unwrap();
        let (hash_map, _) = generate_repository_master_map_table_names(service_id);
        let (current_files, _, _, _, _) =
            generate_repository_files_table_names(service_id);

        let db_base = db.db().db_base().unwrap();
        db_base
            .execute(
                &format!(
                    "INSERT INTO {hash_map} ( master_hash_id, hash_id_timestamp ) VALUES ( ?, ? );"
                ),
                &[&master_hash_id, &hydrus_time::get_now()],
            )
            .unwrap();
        let service_hash_id = db_base.get_last_row_id();
        db_base
            .execute(
                &format!(
                    "INSERT INTO {current_files} ( service_hash_id, account_id, file_timestamp ) VALUES ( ?, ?, ? );"
                ),
                &[&service_hash_id, &1_i64, &hydrus_time::get_now()],
            )
            .unwrap();

        assert!(
            db.filter_orphan_master_hash_ids(&[master_hash_id], None)
                .unwrap()
                .is_empty()
        );
        assert!(!db.is_an_orphan(&hash).unwrap());
        assert_eq!(
            db.defer_files_delete_if_now_orphan(&[master_hash_id], false, None)
                .unwrap(),
            0
        );
    }

    fn create_file_repository(db: &ServerDB, label: &str) -> (i64, Vec<u8>) {
        let service = generate_service(
            vec![0x33; 32],
            ServiceType::FileRepository,
            format!("{label} repository"),
            DEFAULT_SERVICE_PORT,
            None,
        )
        .unwrap();
        let service_id = db.add_service(&service).unwrap();

        (service_id, service.get_service_key().to_vec())
    }

    #[test]
    fn repository_add_and_delete_files_updates_tables_and_info() {
        let dir = TestDir::new();
        let db = make_db(&dir);
        let (service_id, service_key) = create_file_repository(&db, "file ops");

        let source_path = dir.path.join("represented.bin");
        std::fs::write(&source_path, b"represented file").unwrap();

        let hash = vec![0xB1; 32];
        let file = ServerFileInput {
            hash: hash.clone(),
            size: 16,
            mime: Mime::OctetStream,
            width: None,
            height: None,
            duration_ms: None,
            num_frames: None,
            num_words: None,
            path: source_path,
            thumbnail: Some(b"thumbnail".to_vec()),
        };

        db.repository_add_file(
            service_id,
            1,
            &file,
            false,
            hydrus_time::get_now(),
            Some("127.0.0.1"),
        )
        .unwrap();

        assert_eq!(
            db.repository_has_file(&service_key, &hash).unwrap(),
            (true, Some(Mime::OctetStream))
        );
        assert_eq!(
            db.get_service_info_specific(service_id, ServiceInfoType::NumFiles)
                .unwrap(),
            1
        );
        assert_eq!(
            db.get_ip_timestamp(&service_key, &hash).unwrap().0,
            "127.0.0.1"
        );

        let (hash_map, _) = generate_repository_master_map_table_names(service_id);
        let master_hash_id = db.get_master_hash_id(&hash).unwrap();
        let service_hash_id = db
            .db()
            .db_base()
            .unwrap()
            .query_one(
                &format!(
                    "SELECT service_hash_id FROM {hash_map} WHERE master_hash_id = ?;"
                ),
                &[&master_hash_id],
                |row| row.get::<_, i64>(0),
            )
            .unwrap()
            .unwrap();

        db.repository_delete_files(
            service_id,
            1,
            &[service_hash_id],
            hydrus_time::get_now(),
        )
        .unwrap();

        assert_eq!(
            db.repository_has_file(&service_key, &hash).unwrap(),
            (false, None)
        );
        assert_eq!(
            db.get_service_info_specific(service_id, ServiceInfoType::NumFiles)
                .unwrap(),
            0
        );
        assert_eq!(
            db.get_service_info_specific(service_id, ServiceInfoType::NumDeletedFiles)
                .unwrap(),
            1
        );
        assert_eq!(
            db.get_deferred_physical_delete().unwrap(),
            (Some(hash.clone()), Some(hash))
        );
    }

    fn create_tag_repository(db: &ServerDB, label: &str) -> (i64, Vec<u8>) {
        let service = generate_service(
            vec![0x44; 32],
            ServiceType::TagRepository,
            format!("{label} repository"),
            DEFAULT_SERVICE_PORT,
            None,
        )
        .unwrap();
        let service_id = db.add_service(&service).unwrap();

        (service_id, service.get_service_key().to_vec())
    }

    #[test]
    fn repository_mappings_add_delete_and_count() {
        let dir = TestDir::new();
        let db = make_db(&dir);
        let (service_id, _service_key) = create_tag_repository(&db, "mappings");

        let master_tag_id = db.get_master_tag_id("blue sky").unwrap();
        let hashes = vec![vec![0xC1; 32], vec![0xC2; 32]];
        let master_hash_ids = db.get_master_hash_ids(&hashes).unwrap();
        let timestamp = hydrus_time::get_now();

        db.repository_add_mappings(
            service_id,
            1,
            master_tag_id,
            &master_hash_ids,
            false,
            timestamp,
        )
        .unwrap();

        let service_tag_id = db
            .repository_get_service_tag_id(service_id, master_tag_id, timestamp)
            .unwrap();

        assert_eq!(
            db.repository_get_master_tag_id(service_id, service_tag_id)
                .unwrap(),
            master_tag_id
        );
        assert!(
            db.repository_service_tag_id_exists(service_id, master_tag_id)
                .unwrap()
        );
        assert_eq!(
            db.repository_get_current_mappings_count(service_id, service_tag_id)
                .unwrap(),
            2
        );
        assert_eq!(
            db.get_service_info_specific(service_id, ServiceInfoType::NumMappings)
                .unwrap(),
            2
        );
        assert_eq!(
            db.get_service_info_specific(service_id, ServiceInfoType::NumTags)
                .unwrap(),
            1
        );

        let service_hash_ids = db
            .repository_get_service_hash_ids(service_id, &master_hash_ids, timestamp)
            .unwrap();

        db.repository_delete_mappings(
            service_id,
            1,
            service_tag_id,
            &[service_hash_ids[0]],
            timestamp,
        )
        .unwrap();

        assert_eq!(
            db.repository_get_current_mappings_count(service_id, service_tag_id)
                .unwrap(),
            1
        );
        assert_eq!(
            db.get_service_info_specific(service_id, ServiceInfoType::NumMappings)
                .unwrap(),
            1
        );
        assert_eq!(
            db.get_service_info_specific(service_id, ServiceInfoType::NumDeletedMappings)
                .unwrap(),
            1
        );
    }

    #[test]
    fn repository_tag_parents_and_siblings_add_delete_and_count() {
        let dir = TestDir::new();
        let db = make_db(&dir);
        let (service_id, _service_key) = create_tag_repository(&db, "tag relations");
        let timestamp = hydrus_time::get_now();

        let child_master_tag_id = db.get_master_tag_id("child").unwrap();
        let parent_master_tag_id = db.get_master_tag_id("parent").unwrap();
        let bad_master_tag_id = db.get_master_tag_id("bad").unwrap();
        let good_master_tag_id = db.get_master_tag_id("good").unwrap();

        db.repository_add_tag_parent(
            service_id,
            1,
            child_master_tag_id,
            parent_master_tag_id,
            false,
            timestamp,
        )
        .unwrap();
        db.repository_add_tag_sibling(
            service_id,
            1,
            bad_master_tag_id,
            good_master_tag_id,
            false,
            timestamp,
        )
        .unwrap();

        assert_eq!(
            db.get_service_info_specific(service_id, ServiceInfoType::NumTagParents)
                .unwrap(),
            1
        );
        assert_eq!(
            db.get_service_info_specific(service_id, ServiceInfoType::NumTagSiblings)
                .unwrap(),
            1
        );

        let child_service_tag_id = db
            .repository_get_service_tag_id(service_id, child_master_tag_id, timestamp)
            .unwrap();
        let parent_service_tag_id = db
            .repository_get_service_tag_id(service_id, parent_master_tag_id, timestamp)
            .unwrap();
        let bad_service_tag_id = db
            .repository_get_service_tag_id(service_id, bad_master_tag_id, timestamp)
            .unwrap();
        let good_service_tag_id = db
            .repository_get_service_tag_id(service_id, good_master_tag_id, timestamp)
            .unwrap();

        db.repository_delete_tag_parent(
            service_id,
            1,
            child_service_tag_id,
            parent_service_tag_id,
            timestamp,
        )
        .unwrap();
        db.repository_delete_tag_sibling(
            service_id,
            1,
            bad_service_tag_id,
            good_service_tag_id,
            timestamp,
        )
        .unwrap();

        assert_eq!(
            db.get_service_info_specific(service_id, ServiceInfoType::NumTagParents)
                .unwrap(),
            0
        );
        assert_eq!(
            db.get_service_info_specific(service_id, ServiceInfoType::NumTagSiblings)
                .unwrap(),
            0
        );
        assert_eq!(
            db.get_service_info_specific(service_id, ServiceInfoType::NumDeletedTagParents)
                .unwrap(),
            1
        );
        assert_eq!(
            db.get_service_info_specific(service_id, ServiceInfoType::NumDeletedTagSiblings)
                .unwrap(),
            1
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
