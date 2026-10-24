impl ServerDB {


    fn petition_summary(
        &self,
        table: String,
        content_type: ContentType,
        status: ContentStatus,
        limit: usize,
        account_id: Option<i64>,
        reason_id: Option<i64>,
    ) -> HydrusResult<Vec<PetitionHeader>> {
        let rows = self.db.db_base()?.query_all(
            &format!(
                "SELECT DISTINCT account_id, reason_id FROM {table} ORDER BY account_id;"
            ),
            &[],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
        )?;

        let mut headers = Vec::new();

        for (row_account_id, row_reason_id) in rows {
            if let Some(account_id) = account_id
                && row_account_id != account_id
            {
                continue;
            }

            if let Some(reason_id) = reason_id
                && row_reason_id != reason_id
            {
                continue;
            }

            if headers.len() >= limit {
                break;
            }

            headers.push(PetitionHeader::new(
                content_type,
                status,
                self.get_account_key_from_account_id(row_account_id)?,
                self.get_reason(row_reason_id)?,
            ));
        }

        Ok(headers)
    }

    /// Existing service-hash ids, without creating new map rows.
    pub fn repository_get_existing_service_hash_ids(
        &self,
        service_id: i64,
        master_hash_ids: &[i64],
    ) -> HydrusResult<Vec<i64>> {
        let (hash_map, _) = generate_repository_master_map_table_names(service_id);
        let db = self.db.db_base()?;
        let mut service_hash_ids = Vec::new();

        for master_hash_id in master_hash_ids {
            if let Some(service_hash_id) = db.query_one(
                &format!(
                    "SELECT service_hash_id FROM {hash_map} WHERE master_hash_id = ?;"
                ),
                &[master_hash_id],
                |row| row.get::<_, i64>(0),
            )? {
                service_hash_ids.push(service_hash_id);
            }
        }

        Ok(service_hash_ids)
    }

    #[allow(clippy::too_many_arguments)]
    fn process_mapping_update(
        &self,
        service_id: i64,
        account_id: i64,
        account: &NetworkAccount,
        tag: &str,
        hashes: &[Vec<u8>],
        action: ContentUpdate,
        reason_id: i64,
        timestamp: i64,
        overwrite_deleted: bool,
    ) -> HydrusResult<()> {
        let master_tag_id = self.get_master_tag_id(tag)?;
        let master_hash_ids = self.get_master_hash_ids(hashes)?;
        let service_tag_id =
            self.repository_get_service_tag_id(service_id, master_tag_id, timestamp)?;

        match action {
            ContentUpdate::Add => {
                account.check_permission(
                    ContentType::Mappings,
                    hydrus_core::hydrus_constants::PermissionAction::Create,
                )?;
                self.repository_add_mappings(
                    service_id,
                    account_id,
                    master_tag_id,
                    &master_hash_ids,
                    overwrite_deleted,
                    timestamp,
                )?;
            }
            ContentUpdate::Delete => {
                account.check_permission(
                    ContentType::Mappings,
                    hydrus_core::hydrus_constants::PermissionAction::Moderate,
                )?;
                let service_hash_ids = self
                    .repository_get_existing_service_hash_ids(service_id, &master_hash_ids)?;
                self.repository_delete_mappings(
                    service_id,
                    account_id,
                    service_tag_id,
                    &service_hash_ids,
                    timestamp,
                )?;
            }
            ContentUpdate::Petition => {
                account.check_permission(
                    ContentType::Mappings,
                    hydrus_core::hydrus_constants::PermissionAction::Petition,
                )?;
                let service_hash_ids = self
                    .repository_get_existing_service_hash_ids(service_id, &master_hash_ids)?;
                self.repository_petition_mappings(
                    service_id,
                    account_id,
                    service_tag_id,
                    &service_hash_ids,
                    reason_id,
                )?;
            }
            ContentUpdate::DenyPetition => {
                account.check_permission(
                    ContentType::Mappings,
                    hydrus_core::hydrus_constants::PermissionAction::Moderate,
                )?;
                let service_hash_ids = self
                    .repository_get_existing_service_hash_ids(service_id, &master_hash_ids)?;
                self.repository_deny_mapping_petition(
                    service_id,
                    service_tag_id,
                    &service_hash_ids,
                )?;
            }
            _ => {
                return Err(HydrusException::new(
                    ErrorKind::BadRequest,
                    format!("Unsupported mapping update action: {action}"),
                ));
            }
        }

        Ok(())
    }
}
