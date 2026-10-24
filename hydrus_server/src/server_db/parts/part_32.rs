/// One repository update produced by `_repository_generate_updates`.
pub enum RepositoryUpdate {
    Definitions(DefinitionsUpdate),
    Content(ContentUpdatePackage),
}

impl RepositoryUpdate {
    /// `get_num_rows`
    pub fn get_num_rows(&self) -> usize {
        match self {
            Self::Definitions(update) => update.get_num_rows(),
            Self::Content(update) => update.get_num_rows(None),
        }
    }

    /// `dump_to_network_bytes`
    pub fn dump_to_network_bytes(&self) -> HydrusResult<Vec<u8>> {
        match self {
            Self::Definitions(update) => {
                SerialisableObject::dump_to_network_bytes(update)
            }
            Self::Content(update) => {
                SerialisableObject::dump_to_network_bytes(update)
            }
        }
    }
}

impl ServerDB {
    /// `_repository_create_update( service_key, begin, end )`
    pub fn repository_create_update(
        &self,
        service_key: &[u8],
        begin: i64,
        end: i64,
    ) -> HydrusResult<Vec<Vec<u8>>> {
        let service_id = self.get_service_id(service_key)?;
        let updates = self.repository_generate_updates(service_id, begin, end)?;

        let mut update_hashes = Vec::new();

        if updates.is_empty() {
            return Ok(update_hashes);
        }

        for update in &updates {
            let update_bytes = update.dump_to_network_bytes()?;
            let update_hash = Sha256::digest(&update_bytes).to_vec();
            let dest_path = self.files.get_expected_file_path(&update_hash);

            if let Some(parent) = dest_path.parent() {
                hydrus_core::hydrus_paths::make_sure_directory_exists(parent)?;
            }

            std::fs::write(&dest_path, &update_bytes).map_err(|error| {
                HydrusException::new(
                    ErrorKind::FileMissing,
                    format!(
                        "Could not write the update \"{}\": {error}",
                        dest_path.display()
                    ),
                )
            })?;

            update_hashes.push(update_hash);
        }

        let update_table = generate_repository_update_table_name(service_id);
        let master_hash_ids = self.get_master_hash_ids(&update_hashes)?;

        for (update_hash, master_hash_id) in
            update_hashes.iter().zip(master_hash_ids.iter())
        {
            self.db.db_base()?.execute(
                &format!(
                    "INSERT OR IGNORE INTO {update_table} ( master_hash_id ) VALUES ( ? );"
                ),
                &[master_hash_id],
            )?;

            self.clear_deferred_physical_delete(Some(update_hash), Some(update_hash))?;
        }

        Ok(update_hashes)
    }
}
