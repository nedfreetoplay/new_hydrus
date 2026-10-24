impl ServerDB {
    /// `_repository_get_tag_parent_pend`
    pub fn repository_get_tag_parent_pend(
        &self,
        service_id: i64,
        petitioner_account_id: i64,
        reason_id: i64,
    ) -> HydrusResult<Petition> {
        let (_, _, pending_parents, _) =
            generate_repository_tag_parents_table_names(service_id);
        let db = self.db.db_base()?;

        let pairs = db.query_all(
            &format!(
                "SELECT child_master_tag_id, parent_master_tag_id FROM {pending_parents} WHERE account_id = ? AND reason_id = ?;"
            ),
            &[&petitioner_account_id, &reason_id],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
        )?;

        let contents = self.tag_parent_contents(&pairs)?;

        if contents.is_empty() {
            return Err(HydrusException::new(
                ErrorKind::NotFound,
                "Sorry, did not find a petition for that account and reason!",
            ));
        }

        self.build_petition(
            service_id,
            petitioner_account_id,
            reason_id,
            ContentType::TagParents,
            ContentStatus::Pending,
            vec![(ContentUpdate::Pend, contents)],
        )
    }

    /// `_repository_get_tag_parent_petition`
    pub fn repository_get_tag_parent_petition(
        &self,
        service_id: i64,
        petitioner_account_id: i64,
        reason_id: i64,
    ) -> HydrusResult<Petition> {
        let (_, _, pending_parents, petitioned_parents) =
            generate_repository_tag_parents_table_names(service_id);
        let db = self.db.db_base()?;

        let petitioned = db.query_all(
            &format!(
                "SELECT child_service_tag_id, parent_service_tag_id FROM {petitioned_parents} WHERE account_id = ? AND reason_id = ?;"
            ),
            &[&petitioner_account_id, &reason_id],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
        )?;

        let mut petitioned_pairs = Vec::new();

        for (child_service_tag_id, parent_service_tag_id) in petitioned {
            petitioned_pairs.push((
                self.repository_get_master_tag_id(service_id, child_service_tag_id)?,
                self.repository_get_master_tag_id(service_id, parent_service_tag_id)?,
            ));
        }

        let petitioned_contents = self.tag_parent_contents(&petitioned_pairs)?;

        if petitioned_contents.is_empty() {
            return Err(HydrusException::new(
                ErrorKind::NotFound,
                "Sorry, did not find a petition for that account and reason!",
            ));
        }

        let pending = db.query_all(
            &format!(
                "SELECT child_master_tag_id, parent_master_tag_id FROM {pending_parents} WHERE account_id = ? AND reason_id = ?;"
            ),
            &[&petitioner_account_id, &reason_id],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
        )?;

        let pending_contents = self.tag_parent_contents(&pending)?;

        let actions_and_contents = vec![
            (ContentUpdate::Petition, petitioned_contents),
            (ContentUpdate::Pend, pending_contents),
        ];

        self.build_petition(
            service_id,
            petitioner_account_id,
            reason_id,
            ContentType::TagParents,
            ContentStatus::Petitioned,
            actions_and_contents,
        )
    }

    fn tag_parent_contents(
        &self,
        pairs: &[(i64, i64)],
    ) -> HydrusResult<Vec<Content>> {
        let mut contents = Vec::with_capacity(pairs.len());

        for (child_master_tag_id, parent_master_tag_id) in pairs {
            let child_tag = self.get_tag(*child_master_tag_id)?;
            let parent_tag = self.get_tag(*parent_master_tag_id)?;

            contents.push(Content::tag_parents(child_tag, parent_tag));
        }

        Ok(contents)
    }
}
