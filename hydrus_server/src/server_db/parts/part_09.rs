impl ServerDB {


    /// `_repository_petition_files( service_id, account_id, service_hash_ids, reason_id )`
    pub fn repository_petition_files(
        &self,
        service_id: i64,
        account_id: i64,
        service_hash_ids: &[i64],
        reason_id: i64,
    ) -> HydrusResult<()> {
        let (current_files, _, _, petitioned_files, _) =
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

        let mut deleted = 0_usize;
        let mut added = 0_usize;

        for service_hash_id in &valid_service_hash_ids {
            deleted += db.execute(
                &format!(
                    "DELETE FROM {petitioned_files} WHERE service_hash_id = ? AND account_id = ?;"
                ),
                &[service_hash_id, &account_id],
            )?;
            added += db.execute(
                &format!(
                    "INSERT OR IGNORE INTO {petitioned_files} ( service_hash_id, account_id, reason_id ) VALUES ( ?, ?, ? );"
                ),
                &[service_hash_id, &account_id, &reason_id],
            )?;
        }

        self.update_service_info(
            service_id,
            ServiceInfoType::NumPetitionedFiles,
            added as i64 - deleted as i64,
        )?;
        self.regenerate_service_info(
            Some(service_id),
            Some(ServiceInfoType::NumActionableFileDeletePetitions),
        )?;

        Ok(())
    }

    /// `_repository_deny_file_petition( service_id, service_hash_ids )`
    pub fn repository_deny_file_petition(
        &self,
        service_id: i64,
        service_hash_ids: &[i64],
    ) -> HydrusResult<()> {
        let (_, _, _, petitioned_files, _) =
            generate_repository_files_table_names(service_id);
        let db = self.db.db_base()?;
        let mut deleted = 0_usize;

        for service_hash_id in service_hash_ids {
            deleted += db.execute(
                &format!(
                    "DELETE FROM {petitioned_files} WHERE service_hash_id = ?;"
                ),
                &[service_hash_id],
            )?;
        }

        if deleted > 0 {
            self.update_service_info(
                service_id,
                ServiceInfoType::NumPetitionedFiles,
                -(deleted as i64),
            )?;
        }

        self.regenerate_service_info(
            Some(service_id),
            Some(ServiceInfoType::NumActionableFileDeletePetitions),
        )?;

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
}
