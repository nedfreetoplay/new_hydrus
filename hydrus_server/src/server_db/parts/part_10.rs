impl ServerDB {


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
}
