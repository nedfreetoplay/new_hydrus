impl ServerDB {
    /// `_repository_process_client_to_server_update`
    pub fn repository_process_client_to_server_update(
        &self,
        service_key: &[u8],
        account: &NetworkAccount,
        update: &ClientToServerUpdate,
        timestamp: i64,
    ) -> HydrusResult<()> {
        let service_id = self.get_service_id(service_key)?;
        let account_id = self.get_account_id(&account.get_account_key())?;

        for (action, content, reason) in update.iterate_all_actions_and_contents_and_reasons() {
            let reason_id = self.get_reason_id(reason)?;
            let overwrite_deleted = account.has_permission(
                content.get_content_type(),
                hydrus_core::hydrus_constants::PermissionAction::Moderate,
            );

            match content.get_content_data() {
                ContentData::Files(hashes) => self.process_file_update(
                    service_id,
                    account_id,
                    account,
                    hashes,
                    action,
                    reason_id,
                    timestamp,
                )?,
                ContentData::Mapping(tag, hash) => self.process_mapping_update(
                    service_id,
                    account_id,
                    account,
                    tag,
                    &[hash.clone()],
                    action,
                    reason_id,
                    timestamp,
                    overwrite_deleted,
                )?,
                ContentData::Mappings(tag, hashes) => self.process_mapping_update(
                    service_id,
                    account_id,
                    account,
                    tag,
                    hashes,
                    action,
                    reason_id,
                    timestamp,
                    overwrite_deleted,
                )?,
                ContentData::TagParents(child, parent) => self.process_tag_parent_update(
                    service_id,
                    account_id,
                    account,
                    child,
                    parent,
                    action,
                    reason_id,
                    timestamp,
                    overwrite_deleted,
                )?,
                ContentData::TagSiblings(bad, good) => self.process_tag_sibling_update(
                    service_id,
                    account_id,
                    account,
                    bad,
                    good,
                    action,
                    reason_id,
                    timestamp,
                    overwrite_deleted,
                )?,
                _ => {
                    return Err(HydrusException::new(
                        ErrorKind::BadRequest,
                        format!("Unsupported content type for update: {action}"),
                    ));
                }
            }
        }

        Ok(())
    }

    fn process_file_update(
        &self,
        service_id: i64,
        account_id: i64,
        account: &NetworkAccount,
        hashes: &[Vec<u8>],
        action: ContentUpdate,
        reason_id: i64,
        timestamp: i64,
    ) -> HydrusResult<()> {
        let master_hash_ids = self.get_master_hash_ids(hashes)?;
        let service_hash_ids =
            self.repository_get_existing_service_hash_ids(service_id, &master_hash_ids)?;

        match action {
            ContentUpdate::Delete => {
                account.check_permission(
                    ContentType::Files,
                    hydrus_core::hydrus_constants::PermissionAction::Moderate,
                )?;
                self.repository_delete_files(
                    service_id,
                    account_id,
                    &service_hash_ids,
                    timestamp,
                )
            }
            ContentUpdate::Petition => {
                account.check_permission(
                    ContentType::Files,
                    hydrus_core::hydrus_constants::PermissionAction::Petition,
                )?;
                self.repository_petition_files(
                    service_id,
                    account_id,
                    &service_hash_ids,
                    reason_id,
                )
            }
            ContentUpdate::DenyPetition => {
                account.check_permission(
                    ContentType::Files,
                    hydrus_core::hydrus_constants::PermissionAction::Moderate,
                )?;
                self.repository_deny_file_petition(service_id, &service_hash_ids)
            }
            _ => Err(HydrusException::new(
                ErrorKind::BadRequest,
                format!("Unsupported file update action: {action}"),
            )),
        }
    }
}
