impl ServerDB {


    /// `_repository_petition_tag_parent( service_id, account_id, child_service_tag_id, parent_service_tag_id, reason_id )`
    pub fn repository_petition_tag_parent(
        &self,
        service_id: i64,
        account_id: i64,
        child_service_tag_id: i64,
        parent_service_tag_id: i64,
        reason_id: i64,
    ) -> HydrusResult<()> {
        let (current_parents, _, _, petitioned_parents) =
            generate_repository_tag_parents_table_names(service_id);
        let db = self.db.db_base()?;

        if db
            .query_one(
                &format!(
                    "SELECT 1 FROM {current_parents} WHERE child_service_tag_id = ? AND parent_service_tag_id = ?;"
                ),
                &[&child_service_tag_id, &parent_service_tag_id],
                |row| row.get::<_, i64>(0),
            )?
            .is_none()
        {
            return Ok(());
        }

        let deleted = db.execute(
            &format!(
                "DELETE FROM {petitioned_parents} WHERE child_service_tag_id = ? AND parent_service_tag_id = ? AND account_id = ?;"
            ),
            &[&child_service_tag_id, &parent_service_tag_id, &account_id],
        )?;
        let added = db.execute(
            &format!(
                "INSERT OR IGNORE INTO {petitioned_parents} ( child_service_tag_id, parent_service_tag_id, account_id, reason_id ) VALUES ( ?, ?, ?, ? );"
            ),
            &[
                &child_service_tag_id,
                &parent_service_tag_id,
                &account_id,
                &reason_id,
            ],
        )?;

        self.update_service_info(
            service_id,
            ServiceInfoType::NumPetitionedTagParents,
            added as i64 - deleted as i64,
        )?;
        self.regenerate_service_info(
            Some(service_id),
            Some(ServiceInfoType::NumActionableParentDeletePetitions),
        )?;
        self.regenerate_service_info(
            Some(service_id),
            Some(ServiceInfoType::NumActionableParentAddPetitions),
        )?;

        Ok(())
    }

    /// `_repository_petition_tag_sibling( service_id, account_id, bad_service_tag_id, good_service_tag_id, reason_id )`
    pub fn repository_petition_tag_sibling(
        &self,
        service_id: i64,
        account_id: i64,
        bad_service_tag_id: i64,
        good_service_tag_id: i64,
        reason_id: i64,
    ) -> HydrusResult<()> {
        let (current_siblings, _, _, petitioned_siblings) =
            generate_repository_tag_siblings_table_names(service_id);
        let db = self.db.db_base()?;

        if db
            .query_one(
                &format!(
                    "SELECT 1 FROM {current_siblings} WHERE bad_service_tag_id = ? AND good_service_tag_id = ?;"
                ),
                &[&bad_service_tag_id, &good_service_tag_id],
                |row| row.get::<_, i64>(0),
            )?
            .is_none()
        {
            return Ok(());
        }

        let deleted = db.execute(
            &format!(
                "DELETE FROM {petitioned_siblings} WHERE bad_service_tag_id = ? AND good_service_tag_id = ? AND account_id = ?;"
            ),
            &[&bad_service_tag_id, &good_service_tag_id, &account_id],
        )?;
        let added = db.execute(
            &format!(
                "INSERT OR IGNORE INTO {petitioned_siblings} ( bad_service_tag_id, good_service_tag_id, account_id, reason_id ) VALUES ( ?, ?, ?, ? );"
            ),
            &[
                &bad_service_tag_id,
                &good_service_tag_id,
                &account_id,
                &reason_id,
            ],
        )?;

        self.update_service_info(
            service_id,
            ServiceInfoType::NumPetitionedTagSiblings,
            added as i64 - deleted as i64,
        )?;
        self.regenerate_service_info(
            Some(service_id),
            Some(ServiceInfoType::NumActionableSiblingDeletePetitions),
        )?;
        self.regenerate_service_info(
            Some(service_id),
            Some(ServiceInfoType::NumActionableSiblingAddPetitions),
        )?;

        Ok(())
    }

    /// `_repository_deny_tag_parent_pend`
    pub fn repository_deny_tag_parent_pend(
        &self,
        service_id: i64,
        child_master_tag_id: i64,
        parent_master_tag_id: i64,
    ) -> HydrusResult<()> {
        let (_, _, pending_parents, _) =
            generate_repository_tag_parents_table_names(service_id);
        let deleted = self.db.db_base()?.execute(
            &format!(
                "DELETE FROM {pending_parents} WHERE child_master_tag_id = ? AND parent_master_tag_id = ?;"
            ),
            &[&child_master_tag_id, &parent_master_tag_id],
        )?;

        if deleted > 0 {
            self.update_service_info(
                service_id,
                ServiceInfoType::NumPendingTagParents,
                -(deleted as i64),
            )?;
        }

        self.regenerate_service_info(
            Some(service_id),
            Some(ServiceInfoType::NumActionableParentAddPetitions),
        )?;

        Ok(())
    }
}
