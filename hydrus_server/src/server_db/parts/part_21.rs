impl ServerDB {


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

    fn is_null_account(&self, service_id: i64, account_id: i64) -> HydrusResult<bool> {
        Ok(self
            .get_account(service_id, account_id)?
            .get_account_type()
            .is_null_account())
    }

    fn save_account(&self, account: &NetworkAccount) -> HydrusResult<()> {
        let (account_key, _, _, _, dictionary) =
            NetworkAccount::generate_tuple_from_account(account)?;

        self.db.db_base()?.execute(
            "UPDATE accounts SET dictionary_string = ? WHERE account_key = ?;",
            &[&dictionary.dump_to_string(), &account_key],
        )?;

        account.set_clean();

        Ok(())
    }

    /// `_repository_get_account_info( service_id, account_id )`
    pub fn get_account_info(
        &self,
        service_key: &[u8],
        account_id: i64,
    ) -> HydrusResult<std::collections::HashMap<String, i64>> {
        let service_id = self.get_service_id(service_key)?;
        let service_type = self.get_service_type(service_id)?;
        let db = self.db.db_base()?;
        let mut info = std::collections::HashMap::new();

        if service_type == ServiceType::FileRepository {
            let (hash_map, _) = generate_repository_master_map_table_names(service_id);
            let (current_files, _, _, _, _) =
                generate_repository_files_table_names(service_id);
            let (num_files, num_bytes) = db
                .query_one(
                    &format!(
                        "SELECT COUNT( * ), COALESCE( SUM( size ), 0 ) FROM files_info NATURAL JOIN {hash_map} NATURAL JOIN {current_files} WHERE account_id = ?;"
                    ),
                    &[&account_id],
                    |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
                )?
                .unwrap_or((0, 0));

            info.insert("num_files".to_string(), num_files);
            info.insert("num_files_bytes".to_string(), num_bytes);
        } else if service_type == ServiceType::TagRepository {
            let (current_mappings, _, _, petitioned_mappings) =
                generate_repository_mappings_table_names(service_id);
            let (current_siblings, _, pending_siblings, petitioned_siblings) =
                generate_repository_tag_siblings_table_names(service_id);
            let (current_parents, _, pending_parents, petitioned_parents) =
                generate_repository_tag_parents_table_names(service_id);

            info.insert(
                "num_mappings".to_string(),
                db.query_one(
                    &format!(
                        "SELECT COUNT( * ) FROM {current_mappings} WHERE account_id = ?;"
                    ),
                    &[&account_id],
                    |row| row.get::<_, i64>(0),
                )?
                .unwrap_or(0),
            );
            info.insert(
                "num_mappings_petitioned".to_string(),
                db.query_one(
                    &format!(
                        "SELECT COUNT( * ) FROM {petitioned_mappings} WHERE account_id = ?;"
                    ),
                    &[&account_id],
                    |row| row.get::<_, i64>(0),
                )?
                .unwrap_or(0),
            );
            info.insert(
                "num_siblings".to_string(),
                db.query_one(
                    &format!(
                        "SELECT COUNT( * ) FROM {current_siblings} WHERE account_id = ?;"
                    ),
                    &[&account_id],
                    |row| row.get::<_, i64>(0),
                )?
                .unwrap_or(0),
            );
            info.insert(
                "num_siblings_pending".to_string(),
                db.query_one(
                    &format!(
                        "SELECT COUNT( * ) FROM {pending_siblings} WHERE account_id = ?;"
                    ),
                    &[&account_id],
                    |row| row.get::<_, i64>(0),
                )?
                .unwrap_or(0),
            );
            info.insert(
                "num_siblings_petitioned".to_string(),
                db.query_one(
                    &format!(
                        "SELECT COUNT( * ) FROM {petitioned_siblings} WHERE account_id = ?;"
                    ),
                    &[&account_id],
                    |row| row.get::<_, i64>(0),
                )?
                .unwrap_or(0),
            );
            info.insert(
                "num_parents".to_string(),
                db.query_one(
                    &format!(
                        "SELECT COUNT( * ) FROM {current_parents} WHERE account_id = ?;"
                    ),
                    &[&account_id],
                    |row| row.get::<_, i64>(0),
                )?
                .unwrap_or(0),
            );
            info.insert(
                "num_parents_pending".to_string(),
                db.query_one(
                    &format!(
                        "SELECT COUNT( * ) FROM {pending_parents} WHERE account_id = ?;"
                    ),
                    &[&account_id],
                    |row| row.get::<_, i64>(0),
                )?
                .unwrap_or(0),
            );
            info.insert(
                "num_parents_petitioned".to_string(),
                db.query_one(
                    &format!(
                        "SELECT COUNT( * ) FROM {petitioned_parents} WHERE account_id = ?;"
                    ),
                    &[&account_id],
                    |row| row.get::<_, i64>(0),
                )?
                .unwrap_or(0),
            );
        }

        let petition_score = db
            .query_one(
                "SELECT score FROM account_scores WHERE service_id = ? AND account_id = ? AND score_type = ?;",
                &[&service_id, &account_id, &i64::from(hydrus_core::hydrus_constants::SCORE_PETITION)],
                |row| row.get::<_, i64>(0),
            )?
            .unwrap_or(0);

        info.insert("petition_score".to_string(), petition_score);

        Ok(info)
    }
}
