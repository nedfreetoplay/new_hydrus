impl ServerDB {
    #[allow(clippy::too_many_arguments)]
    fn process_tag_sibling_update(
        &self,
        service_id: i64,
        account_id: i64,
        account: &NetworkAccount,
        bad_tag: &str,
        good_tag: &str,
        action: ContentUpdate,
        reason_id: i64,
        timestamp: i64,
        overwrite_deleted: bool,
    ) -> HydrusResult<()> {
        let bad_master_tag_id = self.get_master_tag_id(bad_tag)?;
        let good_master_tag_id = self.get_master_tag_id(good_tag)?;

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
                    format!("Unsupported tag sibling update action: {action}"),
                ));
            }
        };

        account.check_permission(ContentType::TagSiblings, permission)?;

        match action {
            ContentUpdate::Add => self.repository_add_tag_sibling(
                service_id,
                account_id,
                bad_master_tag_id,
                good_master_tag_id,
                overwrite_deleted,
                timestamp,
            ),
            ContentUpdate::Delete => {
                let bad_service_tag_id = self.repository_get_service_tag_id(
                    service_id,
                    bad_master_tag_id,
                    timestamp,
                )?;
                let good_service_tag_id = self.repository_get_service_tag_id(
                    service_id,
                    good_master_tag_id,
                    timestamp,
                )?;

                self.repository_delete_tag_sibling(
                    service_id,
                    account_id,
                    bad_service_tag_id,
                    good_service_tag_id,
                    timestamp,
                )
            }
            ContentUpdate::Pend => self.repository_pend_tag_sibling(
                service_id,
                account_id,
                bad_master_tag_id,
                good_master_tag_id,
                reason_id,
            ),
            ContentUpdate::DenyPend => self.repository_deny_tag_sibling_pend(
                service_id,
                bad_master_tag_id,
                good_master_tag_id,
            ),
            ContentUpdate::Petition => {
                let bad_service_tag_id = self.repository_get_service_tag_id(
                    service_id,
                    bad_master_tag_id,
                    timestamp,
                )?;
                let good_service_tag_id = self.repository_get_service_tag_id(
                    service_id,
                    good_master_tag_id,
                    timestamp,
                )?;

                self.repository_petition_tag_sibling(
                    service_id,
                    account_id,
                    bad_service_tag_id,
                    good_service_tag_id,
                    reason_id,
                )
            }
            ContentUpdate::DenyPetition => {
                let bad_service_tag_id = self.repository_get_service_tag_id(
                    service_id,
                    bad_master_tag_id,
                    timestamp,
                )?;
                let good_service_tag_id = self.repository_get_service_tag_id(
                    service_id,
                    good_master_tag_id,
                    timestamp,
                )?;

                self.repository_deny_tag_sibling_petition(
                    service_id,
                    bad_service_tag_id,
                    good_service_tag_id,
                )
            }
            _ => unreachable!(),
        }
    }
}
