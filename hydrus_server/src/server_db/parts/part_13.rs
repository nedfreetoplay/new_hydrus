impl ServerDB {


    /// `_repository_delete_tag_parent( service_id, account_id, child_service_tag_id, parent_service_tag_id, timestamp )`
    pub fn repository_delete_tag_parent(
        &self,
        service_id: i64,
        account_id: i64,
        child_service_tag_id: i64,
        parent_service_tag_id: i64,
        timestamp: i64,
    ) -> HydrusResult<()> {
        let (current_parents, deleted_parents, _, petitioned_parents) =
            generate_repository_tag_parents_table_names(service_id);
        let db = self.db.db_base()?;

        let deleted = db.execute(
            &format!(
                "DELETE FROM {current_parents} WHERE child_service_tag_id = ? AND parent_service_tag_id = ?;"
            ),
            &[&child_service_tag_id, &parent_service_tag_id],
        )?;

        if deleted > 0 {
            self.update_service_info(
                service_id,
                ServiceInfoType::NumTagParents,
                -(deleted as i64),
            )?;
        }

        db.execute(
            &format!(
                "DELETE FROM {petitioned_parents} WHERE child_service_tag_id = ? AND parent_service_tag_id = ?;"
            ),
            &[&child_service_tag_id, &parent_service_tag_id],
        )?;

        let inserted = db.execute(
            &format!(
                "INSERT OR IGNORE INTO {deleted_parents} ( child_service_tag_id, parent_service_tag_id, account_id, parent_timestamp ) VALUES ( ?, ?, ?, ? );"
            ),
            &[
                &child_service_tag_id,
                &parent_service_tag_id,
                &account_id,
                &timestamp,
            ],
        )?;

        if inserted > 0 {
            self.update_service_info(
                service_id,
                ServiceInfoType::NumDeletedTagParents,
                inserted as i64,
            )?;
        }

        Ok(())
    }

    /// `_repository_delete_tag_sibling( service_id, account_id, bad_service_tag_id, good_service_tag_id, timestamp )`
    pub fn repository_delete_tag_sibling(
        &self,
        service_id: i64,
        account_id: i64,
        bad_service_tag_id: i64,
        good_service_tag_id: i64,
        timestamp: i64,
    ) -> HydrusResult<()> {
        let (current_siblings, deleted_siblings, _, petitioned_siblings) =
            generate_repository_tag_siblings_table_names(service_id);
        let db = self.db.db_base()?;

        let deleted = db.execute(
            &format!(
                "DELETE FROM {current_siblings} WHERE bad_service_tag_id = ? AND good_service_tag_id = ?;"
            ),
            &[&bad_service_tag_id, &good_service_tag_id],
        )?;

        if deleted > 0 {
            self.update_service_info(
                service_id,
                ServiceInfoType::NumTagSiblings,
                -(deleted as i64),
            )?;
        }

        db.execute(
            &format!(
                "DELETE FROM {petitioned_siblings} WHERE bad_service_tag_id = ? AND good_service_tag_id = ?;"
            ),
            &[&bad_service_tag_id, &good_service_tag_id],
        )?;

        let inserted = db.execute(
            &format!(
                "INSERT OR IGNORE INTO {deleted_siblings} ( bad_service_tag_id, good_service_tag_id, account_id, sibling_timestamp ) VALUES ( ?, ?, ?, ? );"
            ),
            &[
                &bad_service_tag_id,
                &good_service_tag_id,
                &account_id,
                &timestamp,
            ],
        )?;

        if inserted > 0 {
            self.update_service_info(
                service_id,
                ServiceInfoType::NumDeletedTagSiblings,
                inserted as i64,
            )?;
        }

        Ok(())
    }
}
