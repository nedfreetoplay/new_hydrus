impl ServerDB {


    /// `_modify_account_expires`
    pub fn modify_account_expires(
        &self,
        service_key: &[u8],
        subject_account_key: &[u8],
        expires: Option<i64>,
    ) -> HydrusResult<()> {
        let service_id = self.get_service_id(service_key)?;
        let account_id = self.get_account_id(subject_account_key)?;

        if self.is_null_account(service_id, account_id)? {
            return Err(HydrusException::new(
                ErrorKind::BadRequest,
                "You cannot modify the null account!",
            ));
        }

        self.db.db_base()?.execute(
            "UPDATE accounts SET expires = ? WHERE account_id = ?;",
            &[&expires, &account_id],
        )?;

        Ok(())
    }

    /// `_modify_account_set_message`
    pub fn modify_account_set_message(
        &self,
        service_key: &[u8],
        subject_account_key: &[u8],
        message: &str,
    ) -> HydrusResult<()> {
        let service_id = self.get_service_id(service_key)?;
        let account_id = self.get_account_id(subject_account_key)?;

        if self.is_null_account(service_id, account_id)? {
            return Err(HydrusException::new(
                ErrorKind::BadRequest,
                "You cannot tell the null account anything!",
            ));
        }

        let account = self.get_account(service_id, account_id)?;
        account.set_message(message, hydrus_time::get_now());
        self.save_account(&account)?;

        Ok(())
    }

    /// `_modify_account_delete_all_content`
    pub fn modify_account_delete_all_content(
        &self,
        service_key: &[u8],
        admin_account_key: &[u8],
        subject_account_key: &[u8],
    ) -> HydrusResult<bool> {
        let service_id = self.get_service_id(service_key)?;
        let subject_account_id = self.get_account_id(subject_account_key)?;
        let admin_account_id = self.get_account_id(admin_account_key)?;

        if self.is_null_account(service_id, subject_account_id)? {
            return Err(HydrusException::new(
                ErrorKind::BadRequest,
                "You cannot delete the null account's content!",
            ));
        }

        self.delete_repository_petitions(service_id, &[subject_account_id])?;

        if !REPOSITORIES.contains(&self.get_service_type(service_id)?) {
            return Ok(true);
        }

        loop {
            let (current_files, _, _, _, _) =
                generate_repository_files_table_names(service_id);
            let service_hash_ids = self.db.db_base()?.query_all(
                &format!(
                    "SELECT service_hash_id FROM {current_files} WHERE account_id = ? LIMIT 500;"
                ),
                &[&subject_account_id],
                |row| row.get::<_, i64>(0),
            )?;

            if service_hash_ids.is_empty() {
                break;
            }

            self.repository_delete_files(
                service_id,
                admin_account_id,
                &service_hash_ids,
                hydrus_time::get_now(),
            )?;
        }

        Ok(true)
    }

    fn delete_repository_petitions(
        &self,
        service_id: i64,
        account_ids: &[i64],
    ) -> HydrusResult<()> {
        if !REPOSITORIES.contains(&self.get_service_type(service_id)?) {
            return Ok(());
        }

        let (_, _, _, petitioned_files, _) =
            generate_repository_files_table_names(service_id);
        let (_, _, _, petitioned_mappings) =
            generate_repository_mappings_table_names(service_id);
        let (_, _, _, petitioned_parents) =
            generate_repository_tag_parents_table_names(service_id);
        let (_, _, _, petitioned_siblings) =
            generate_repository_tag_siblings_table_names(service_id);

        for table in [
            petitioned_files,
            petitioned_mappings,
            petitioned_parents,
            petitioned_siblings,
        ] {
            for account_id in account_ids {
                self.db.db_base()?.execute(
                    &format!("DELETE FROM {table} WHERE account_id = ?;"),
                    &[account_id],
                )?;
            }
        }

        self.regenerate_service_info(Some(service_id), None)?;

        Ok(())
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
}
