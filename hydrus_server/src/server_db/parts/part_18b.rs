impl ServerDB {
    #[allow(clippy::too_many_arguments)]
    fn process_tag_parent_update(
        &self,
        service_id: i64,
        account_id: i64,
        account: &NetworkAccount,
        child_tag: &str,
        parent_tag: &str,
        action: ContentUpdate,
        reason_id: i64,
        timestamp: i64,
        overwrite_deleted: bool,
    ) -> HydrusResult<()> {
        let child_master_tag_id = self.get_master_tag_id(child_tag)?;
        let parent_master_tag_id = self.get_master_tag_id(parent_tag)?;

        let permission = match action {
            ContentUpdate::Add | ContentUpdate::Delete | ContentUpdate::DenyPend
            | ContentUpdate::DenyPetition => {
                hydrus_core::hydrus_constants::PermissionAction::Moderate
            }
            ContentUpdate::Pend | ContentUpdate::Petition => {
                hydrus_core::hydrus_constants::PermissionAction::Petition
            }
            _ => {
                return Err(HydrusException::new(
                    ErrorKind::BadRequest,
                    format!("Unsupported tag parent update action: {action}"),
                ));
            }
        };

        account.check_permission(ContentType::TagParents, permission)?;

        match action {
            ContentUpdate::Add => self.repository_add_tag_parent(
                service_id,
                account_id,
                child_master_tag_id,
                parent_master_tag_id,
                overwrite_deleted,
                timestamp,
            ),
            ContentUpdate::Delete => {
                let child_service_tag_id = self.repository_get_service_tag_id(
                    service_id,
                    child_master_tag_id,
                    timestamp,
                )?;
                let parent_service_tag_id = self.repository_get_service_tag_id(
                    service_id,
                    parent_master_tag_id,
                    timestamp,
                )?;

                self.repository_delete_tag_parent(
                    service_id,
                    account_id,
                    child_service_tag_id,
                    parent_service_tag_id,
                    timestamp,
                )
            }
            ContentUpdate::Pend => self.repository_pend_tag_parent(
                service_id,
                account_id,
                child_master_tag_id,
                parent_master_tag_id,
                reason_id,
            ),
            ContentUpdate::DenyPend => self.repository_deny_tag_parent_pend(
                service_id,
                child_master_tag_id,
                parent_master_tag_id,
            ),
            ContentUpdate::Petition => {
                let child_service_tag_id = self.repository_get_service_tag_id(
                    service_id,
                    child_master_tag_id,
                    timestamp,
                )?;
                let parent_service_tag_id = self.repository_get_service_tag_id(
                    service_id,
                    parent_master_tag_id,
                    timestamp,
                )?;

                self.repository_petition_tag_parent(
                    service_id,
                    account_id,
                    child_service_tag_id,
                    parent_service_tag_id,
                    reason_id,
                )
            }
            ContentUpdate::DenyPetition => {
                let child_service_tag_id = self.repository_get_service_tag_id(
                    service_id,
                    child_master_tag_id,
                    timestamp,
                )?;
                let parent_service_tag_id = self.repository_get_service_tag_id(
                    service_id,
                    parent_master_tag_id,
                    timestamp,
                )?;

                self.repository_deny_tag_parent_petition(
                    service_id,
                    child_service_tag_id,
                    parent_service_tag_id,
                )
            }
            _ => unreachable!(),
        }
    }
}
