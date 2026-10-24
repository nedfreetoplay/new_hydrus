impl ServerDB {


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

    /// `_repository_petition_mappings( service_id, account_id, service_tag_id, service_hash_ids, reason_id )`
    pub fn repository_petition_mappings(
        &self,
        service_id: i64,
        account_id: i64,
        service_tag_id: i64,
        service_hash_ids: &[i64],
        reason_id: i64,
    ) -> HydrusResult<()> {
        let (current_mappings, _, _, petitioned_mappings) =
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

        let mut deleted = 0_usize;
        let mut added = 0_usize;

        for service_hash_id in &valid_service_hash_ids {
            deleted += db.execute(
                &format!(
                    "DELETE FROM {petitioned_mappings} WHERE service_tag_id = ? AND service_hash_id = ? AND account_id = ?;"
                ),
                &[&service_tag_id, service_hash_id, &account_id],
            )?;
            added += db.execute(
                &format!(
                    "INSERT OR IGNORE INTO {petitioned_mappings} ( service_tag_id, service_hash_id, account_id, reason_id ) VALUES ( ?, ?, ?, ? );"
                ),
                &[
                    &service_tag_id,
                    service_hash_id,
                    &account_id,
                    &reason_id,
                ],
            )?;
        }

        self.update_service_info(
            service_id,
            ServiceInfoType::NumPetitionedMappings,
            added as i64 - deleted as i64,
        )?;
        self.regenerate_service_info(
            Some(service_id),
            Some(ServiceInfoType::NumActionableMappingDeletePetitions),
        )?;

        Ok(())
    }

    /// `_repository_deny_mapping_petition( service_id, service_tag_id, service_hash_ids )`
    pub fn repository_deny_mapping_petition(
        &self,
        service_id: i64,
        service_tag_id: i64,
        service_hash_ids: &[i64],
    ) -> HydrusResult<()> {
        let (_, _, _, petitioned_mappings) =
            generate_repository_mappings_table_names(service_id);
        let db = self.db.db_base()?;
        let mut deleted = 0_usize;

        for service_hash_id in service_hash_ids {
            deleted += db.execute(
                &format!(
                    "DELETE FROM {petitioned_mappings} WHERE service_tag_id = ? AND service_hash_id = ?;"
                ),
                &[&service_tag_id, service_hash_id],
            )?;
        }

        if deleted > 0 {
            self.update_service_info(
                service_id,
                ServiceInfoType::NumPetitionedMappings,
                -(deleted as i64),
            )?;
        }

        self.regenerate_service_info(
            Some(service_id),
            Some(ServiceInfoType::NumActionableMappingDeletePetitions),
        )?;

        Ok(())
    }
}
