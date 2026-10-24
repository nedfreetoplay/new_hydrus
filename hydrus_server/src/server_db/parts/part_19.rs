impl ServerDB {


    /// Approve every item in the petition and apply the corresponding server
    /// update. Pending additions become adds; petitions become deletes.
    pub fn repository_approve_petition(
        &self,
        service_key: &[u8],
        petition: &mut Petition,
        timestamp: i64,
    ) -> HydrusResult<()> {
        let account = Arc::clone(petition.get_petitioner_account());
        let reason = petition.get_reason().to_string();

        for (action, contents) in petition.get_actions_and_contents() {
            let applied_action = match action {
                ContentUpdate::Pend => ContentUpdate::Add,
                ContentUpdate::Petition => ContentUpdate::Delete,
                action => *action,
            };

            for content in contents {
                let mut update = ClientToServerUpdate::new();
                update.add_content(applied_action, content.clone(), reason.clone());

                self.repository_process_client_to_server_update(
                    service_key,
                    account.as_ref(),
                    &update,
                    timestamp,
                )?;
            }
        }

        Ok(())
    }

    /// Deny every item in the petition and apply the corresponding deny update.
    pub fn repository_deny_petition(
        &self,
        service_key: &[u8],
        petition: &mut Petition,
        timestamp: i64,
    ) -> HydrusResult<()> {
        let account = Arc::clone(petition.get_petitioner_account());
        let reason = petition.get_reason().to_string();

        for (action, contents) in petition.get_actions_and_contents() {
            let applied_action = match action {
                ContentUpdate::Pend => ContentUpdate::DenyPend,
                ContentUpdate::Petition => ContentUpdate::DenyPetition,
                action => *action,
            };

            for content in contents {
                let mut update = ClientToServerUpdate::new();
                update.add_content(applied_action, content.clone(), reason.clone());

                self.repository_process_client_to_server_update(
                    service_key,
                    account.as_ref(),
                    &update,
                    timestamp,
                )?;
            }
        }

        Ok(())
    }

    fn null_account_id(&self, service_id: i64) -> HydrusResult<i64> {
        for account_type in self.get_account_types(&self.get_service_key(service_id)?)? {
            if account_type.is_null_account() {
                let id = self.get_account_type_id(
                    service_id,
                    account_type.get_account_type_key(),
                )?;

                if let Some(account_id) = self.db.db_base()?.query_one(
                    "SELECT account_id FROM accounts WHERE service_id = ? AND account_type_id = ? LIMIT 1;",
                    &[&service_id, &id],
                    |row| row.get::<_, i64>(0),
                )? {
                    return Ok(account_id);
                }
            }
        }

        Err(HydrusException::new(
            ErrorKind::DataMissing,
            "Could not find the null account.",
        ))
    }

    /// `_repository_nullify_history( service_key, begin, end )`
    pub fn repository_nullify_history(
        &self,
        service_key: &[u8],
        begin: i64,
        end: i64,
    ) -> HydrusResult<()> {
        let service_id = self.get_service_id(service_key)?;
        let null_id = self.null_account_id(service_id)?;
        let db = self.db.db_base()?;
        let (current_files, deleted_files, _, _, _) =
            generate_repository_files_table_names(service_id);
        let (current_mappings, deleted_mappings, _, _) =
            generate_repository_mappings_table_names(service_id);
        let (current_siblings, deleted_siblings, _, _) =
            generate_repository_tag_siblings_table_names(service_id);
        let (current_parents, deleted_parents, _, _) =
            generate_repository_tag_parents_table_names(service_id);

        for (table, column) in [
            (current_files.as_str(), "file_timestamp"),
            (deleted_files.as_str(), "file_timestamp"),
            (current_mappings.as_str(), "mapping_timestamp"),
            (deleted_mappings.as_str(), "mapping_timestamp"),
            (current_siblings.as_str(), "sibling_timestamp"),
            (deleted_siblings.as_str(), "sibling_timestamp"),
            (current_parents.as_str(), "parent_timestamp"),
            (deleted_parents.as_str(), "parent_timestamp"),
        ] {
            db.execute(
                &format!(
                    "UPDATE {table} SET account_id = ? WHERE {column} BETWEEN ? AND ?;"
                ),
                &[&null_id, &begin, &end],
            )?;
        }

        Ok(())
    }

    /// `_reward_accounts( service_id, score_type, scores )`
    pub fn reward_accounts(
        &self,
        service_id: i64,
        score_type: i64,
        scores: &[(i64, i64)],
    ) -> HydrusResult<()> {
        let db = self.db.db_base()?;

        for (account_id, score) in scores {
            db.execute(
                "INSERT OR IGNORE INTO account_scores ( service_id, account_id, score_type, score ) VALUES ( ?, ?, ?, 0 );",
                &[&service_id, account_id, &score_type],
            )?;
            db.execute(
                "UPDATE account_scores SET score = score + ? WHERE service_id = ? AND account_id = ? AND score_type = ?;",
                &[score, &service_id, account_id, &score_type],
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
}
