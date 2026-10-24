impl ServerDB {


    /// `_repository_pend_tag_parent( service_id, account_id, child_master_tag_id, parent_master_tag_id, reason_id )`
    pub fn repository_pend_tag_parent(
        &self,
        service_id: i64,
        account_id: i64,
        child_master_tag_id: i64,
        parent_master_tag_id: i64,
        reason_id: i64,
    ) -> HydrusResult<()> {
        let (current_parents, _, pending_parents, _) =
            generate_repository_tag_parents_table_names(service_id);

        if self.repository_service_tag_id_exists(service_id, child_master_tag_id)?
            && self.repository_service_tag_id_exists(service_id, parent_master_tag_id)?
        {
            let timestamp = hydrus_time::get_now();
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
            let db = self.db.db_base()?;

            if db
                .query_one(
                    &format!(
                        "SELECT 1 FROM {current_parents} WHERE child_service_tag_id = ? AND parent_service_tag_id = ?;"
                    ),
                    &[&child_service_tag_id, &parent_service_tag_id],
                    |row| row.get::<_, i64>(0),
                )?
                .is_some()
            {
                return Ok(());
            }
        }

        let db = self.db.db_base()?;
        let deleted = db.execute(
            &format!(
                "DELETE FROM {pending_parents} WHERE child_master_tag_id = ? AND parent_master_tag_id = ? AND account_id = ?;"
            ),
            &[&child_master_tag_id, &parent_master_tag_id, &account_id],
        )?;
        let added = db.execute(
            &format!(
                "INSERT OR IGNORE INTO {pending_parents} ( child_master_tag_id, parent_master_tag_id, account_id, reason_id ) VALUES ( ?, ?, ?, ? );"
            ),
            &[
                &child_master_tag_id,
                &parent_master_tag_id,
                &account_id,
                &reason_id,
            ],
        )?;

        self.update_service_info(
            service_id,
            ServiceInfoType::NumPendingTagParents,
            added as i64 - deleted as i64,
        )?;
        self.regenerate_service_info(
            Some(service_id),
            Some(ServiceInfoType::NumActionableParentAddPetitions),
        )?;

        Ok(())
    }

    /// `_repository_pend_tag_sibling( service_id, account_id, bad_master_tag_id, good_master_tag_id, reason_id )`
    pub fn repository_pend_tag_sibling(
        &self,
        service_id: i64,
        account_id: i64,
        bad_master_tag_id: i64,
        good_master_tag_id: i64,
        reason_id: i64,
    ) -> HydrusResult<()> {
        let (current_siblings, _, pending_siblings, _) =
            generate_repository_tag_siblings_table_names(service_id);

        if self.repository_service_tag_id_exists(service_id, bad_master_tag_id)?
            && self.repository_service_tag_id_exists(service_id, good_master_tag_id)?
        {
            let timestamp = hydrus_time::get_now();
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
            let db = self.db.db_base()?;

            if db
                .query_one(
                    &format!(
                        "SELECT 1 FROM {current_siblings} WHERE bad_service_tag_id = ? AND good_service_tag_id = ?;"
                    ),
                    &[&bad_service_tag_id, &good_service_tag_id],
                    |row| row.get::<_, i64>(0),
                )?
                .is_some()
            {
                return Ok(());
            }
        }

        let db = self.db.db_base()?;
        let deleted = db.execute(
            &format!(
                "DELETE FROM {pending_siblings} WHERE bad_master_tag_id = ? AND good_master_tag_id = ? AND account_id = ?;"
            ),
            &[&bad_master_tag_id, &good_master_tag_id, &account_id],
        )?;
        let added = db.execute(
            &format!(
                "INSERT OR IGNORE INTO {pending_siblings} ( bad_master_tag_id, good_master_tag_id, account_id, reason_id ) VALUES ( ?, ?, ?, ? );"
            ),
            &[
                &bad_master_tag_id,
                &good_master_tag_id,
                &account_id,
                &reason_id,
            ],
        )?;

        self.update_service_info(
            service_id,
            ServiceInfoType::NumPendingTagSiblings,
            added as i64 - deleted as i64,
        )?;
        self.regenerate_service_info(
            Some(service_id),
            Some(ServiceInfoType::NumActionableSiblingAddPetitions),
        )?;

        Ok(())
    }
}
