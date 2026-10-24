impl ServerDB {


    /// `_modify_account_types`
    pub fn modify_account_types(
        &self,
        service_key: &[u8],
        account_types: &[AccountType],
        deletee_keys_to_replacements: &std::collections::HashMap<Vec<u8>, Vec<u8>>,
    ) -> HydrusResult<()> {
        let service_id = self.get_service_id(service_key)?;
        let current = self.get_account_types(service_key)?;

        let mut future: std::collections::HashMap<Vec<u8>, AccountType> = account_types
            .iter()
            .filter(|account_type| !account_type.is_null_account())
            .map(|account_type| {
                (
                    account_type.get_account_type_key().to_vec(),
                    account_type.clone(),
                )
            })
            .collect();

        for account_type in &current {
            if account_type.is_null_account() {
                future.insert(
                    account_type.get_account_type_key().to_vec(),
                    account_type.as_ref().clone(),
                );
            }
        }

        let current_by_key: std::collections::HashMap<Vec<u8>, Arc<AccountType>> = current
            .into_iter()
            .map(|account_type| {
                (
                    account_type.get_account_type_key().to_vec(),
                    account_type,
                )
            })
            .collect();

        let mut key_to_id: std::collections::HashMap<Vec<u8>, i64> =
            std::collections::HashMap::new();

        for (key, account_type) in &future {
            if let Some(existing) = current_by_key.get(key) {
                let id = self.get_account_type_id(service_id, key)?;

                self.db.db_base()?.execute(
                    "UPDATE account_types SET dump = ? WHERE service_id = ? AND account_type_id = ?;",
                    &[&account_type.dump_to_string(), &service_id, &id],
                )?;

                key_to_id.insert(key.clone(), id);

                let _ = existing;
            } else {
                let id = self.add_account_type(service_id, account_type)?;
                key_to_id.insert(key.clone(), id);
            }
        }

        for (deletee_key, deletee) in current_by_key {
            if future.contains_key(&deletee_key) {
                continue;
            }

            let Some(replacement_key) = deletee_keys_to_replacements.get(&deletee_key)
            else {
                return Err(HydrusException::new(
                    ErrorKind::BadRequest,
                    "Was missing a replacement account_type_key.",
                ));
            };

            let Some(replacement_id) = key_to_id.get(replacement_key) else {
                return Err(HydrusException::new(
                    ErrorKind::BadRequest,
                    "A replacement account_type_key was not in the future account types.",
                ));
            };

            let deletee_id = self.get_account_type_id(service_id, &deletee_key)?;

            self.db.db_base()?.execute(
                "UPDATE accounts SET account_type_id = ? WHERE service_id = ? AND account_type_id = ?;",
                &[replacement_id, &service_id, &deletee_id],
            )?;
            self.db.db_base()?.execute(
                "DELETE FROM account_types WHERE service_id = ? AND account_type_id = ?;",
                &[&service_id, &deletee_id],
            )?;

            let _ = deletee;
        }

        Ok(())
    }

    /// `_modify_account_account_type`
    pub fn modify_account_account_type(
        &self,
        service_key: &[u8],
        subject_account_key: &[u8],
        new_account_type_key: &[u8],
    ) -> HydrusResult<()> {
        let service_id = self.get_service_id(service_key)?;
        let account_id = self.get_account_id(subject_account_key)?;

        if self.is_null_account(service_id, account_id)? {
            return Err(HydrusException::new(
                ErrorKind::BadRequest,
                "You cannot reassign the null account!",
            ));
        }

        let new_account_type_id =
            self.get_account_type_id(service_id, new_account_type_key)?;
        let new_account_type = self.get_account_type(service_id, new_account_type_id)?;

        if new_account_type.is_null_account() {
            return Err(HydrusException::new(
                ErrorKind::BadRequest,
                "You cannot reassign anyone to the null account!",
            ));
        }

        self.db.db_base()?.execute(
            "UPDATE accounts SET account_type_id = ? WHERE account_id = ?;",
            &[&new_account_type_id, &account_id],
        )?;

        Ok(())
    }

    /// `_modify_account_ban`
    pub fn modify_account_ban(
        &self,
        service_key: &[u8],
        subject_account_key: &[u8],
        reason: &str,
        expires: Option<i64>,
    ) -> HydrusResult<()> {
        let service_id = self.get_service_id(service_key)?;
        let account_id = self.get_account_id(subject_account_key)?;

        if self.is_null_account(service_id, account_id)? {
            return Err(HydrusException::new(
                ErrorKind::BadRequest,
                "You cannot ban the null account!",
            ));
        }

        let account = self.get_account(service_id, account_id)?;
        account.ban(reason, hydrus_time::get_now(), expires);
        self.save_account(&account)?;
        self.delete_repository_petitions(service_id, &[account_id])?;

        Ok(())
    }

    /// `_modify_account_unban`
    pub fn modify_account_unban(
        &self,
        service_key: &[u8],
        subject_account_key: &[u8],
    ) -> HydrusResult<()> {
        let service_id = self.get_service_id(service_key)?;
        let account_id = self.get_account_id(subject_account_key)?;

        if self.is_null_account(service_id, account_id)? {
            return Err(HydrusException::new(
                ErrorKind::BadRequest,
                "You cannot unban the null account!",
            ));
        }

        let account = self.get_account(service_id, account_id)?;
        account.unban();
        self.save_account(&account)?;

        Ok(())
    }
}
