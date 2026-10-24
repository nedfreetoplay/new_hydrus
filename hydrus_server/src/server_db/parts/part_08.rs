impl ServerDB {


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
}
