impl ServerDB {


    /// `_repository_add_tag_parent( service_id, account_id, child_master_tag_id, parent_master_tag_id, overwrite_deleted, timestamp )`
    pub fn repository_add_tag_parent(
        &self,
        service_id: i64,
        account_id: i64,
        child_master_tag_id: i64,
        parent_master_tag_id: i64,
        overwrite_deleted: bool,
        timestamp: i64,
    ) -> HydrusResult<()> {
        let child_service_tag_id =
            self.repository_get_service_tag_id(service_id, child_master_tag_id, timestamp)?;
        let parent_service_tag_id =
            self.repository_get_service_tag_id(service_id, parent_master_tag_id, timestamp)?;
        let (current_parents, deleted_parents, pending_parents, _) =
            generate_repository_tag_parents_table_names(service_id);
        let db = self.db.db_base()?;

        if overwrite_deleted {
            let deleted = db.execute(
                &format!(
                    "DELETE FROM {deleted_parents} WHERE child_service_tag_id = ? AND parent_service_tag_id = ?;"
                ),
                &[&child_service_tag_id, &parent_service_tag_id],
            )?;

            if deleted > 0 {
                self.update_service_info(
                    service_id,
                    ServiceInfoType::NumDeletedTagParents,
                    -(deleted as i64),
                )?;
            }
        } else if db
            .query_one(
                &format!(
                    "SELECT 1 FROM {deleted_parents} WHERE child_service_tag_id = ? AND parent_service_tag_id = ?;"
                ),
                &[&child_service_tag_id, &parent_service_tag_id],
                |row| row.get::<_, i64>(0),
            )?
            .is_some()
        {
            return Ok(());
        }

        let pending_deleted = db.execute(
            &format!(
                "DELETE FROM {pending_parents} WHERE child_master_tag_id = ? AND parent_master_tag_id = ?;"
            ),
            &[&child_master_tag_id, &parent_master_tag_id],
        )?;

        if pending_deleted > 0 {
            self.update_service_info(
                service_id,
                ServiceInfoType::NumPendingTagParents,
                -(pending_deleted as i64),
            )?;
        }

        let inserted = db.execute(
            &format!(
                "INSERT OR IGNORE INTO {current_parents} ( child_service_tag_id, parent_service_tag_id, account_id, parent_timestamp ) VALUES ( ?, ?, ?, ? );"
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
                ServiceInfoType::NumTagParents,
                inserted as i64,
            )?;
        }

        Ok(())
    }

    /// `_repository_add_tag_sibling( service_id, account_id, bad_master_tag_id, good_master_tag_id, overwrite_deleted, timestamp )`
    pub fn repository_add_tag_sibling(
        &self,
        service_id: i64,
        account_id: i64,
        bad_master_tag_id: i64,
        good_master_tag_id: i64,
        overwrite_deleted: bool,
        timestamp: i64,
    ) -> HydrusResult<()> {
        let bad_service_tag_id =
            self.repository_get_service_tag_id(service_id, bad_master_tag_id, timestamp)?;
        let good_service_tag_id =
            self.repository_get_service_tag_id(service_id, good_master_tag_id, timestamp)?;
        let (current_siblings, deleted_siblings, pending_siblings, _) =
            generate_repository_tag_siblings_table_names(service_id);
        let db = self.db.db_base()?;

        if overwrite_deleted {
            let deleted = db.execute(
                &format!(
                    "DELETE FROM {deleted_siblings} WHERE bad_service_tag_id = ? AND good_service_tag_id = ?;"
                ),
                &[&bad_service_tag_id, &good_service_tag_id],
            )?;

            if deleted > 0 {
                self.update_service_info(
                    service_id,
                    ServiceInfoType::NumDeletedTagSiblings,
                    -(deleted as i64),
                )?;
            }
        } else if db
            .query_one(
                &format!(
                    "SELECT 1 FROM {deleted_siblings} WHERE bad_service_tag_id = ? AND good_service_tag_id = ?;"
                ),
                &[&bad_service_tag_id, &good_service_tag_id],
                |row| row.get::<_, i64>(0),
            )?
            .is_some()
        {
            return Ok(());
        }

        let pending_deleted = db.execute(
            &format!(
                "DELETE FROM {pending_siblings} WHERE bad_master_tag_id = ? AND good_master_tag_id = ?;"
            ),
            &[&bad_master_tag_id, &good_master_tag_id],
        )?;

        if pending_deleted > 0 {
            self.update_service_info(
                service_id,
                ServiceInfoType::NumPendingTagSiblings,
                -(pending_deleted as i64),
            )?;
        }

        let inserted = db.execute(
            &format!(
                "INSERT OR IGNORE INTO {current_siblings} ( bad_service_tag_id, good_service_tag_id, account_id, sibling_timestamp ) VALUES ( ?, ?, ?, ? );"
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
                ServiceInfoType::NumTagSiblings,
                inserted as i64,
            )?;
        }

        Ok(())
    }
}
