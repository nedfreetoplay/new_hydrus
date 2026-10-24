impl ServerDB {
    /// `_repository_get_tag_sibling_pend`
    pub fn repository_get_tag_sibling_pend(
        &self,
        service_id: i64,
        petitioner_account_id: i64,
        reason_id: i64,
    ) -> HydrusResult<Petition> {
        let (_, _, pending_siblings, _) =
            generate_repository_tag_siblings_table_names(service_id);
        let db = self.db.db_base()?;

        let pairs = db.query_all(
            &format!(
                "SELECT bad_master_tag_id, good_master_tag_id FROM {pending_siblings} WHERE account_id = ? AND reason_id = ?;"
            ),
            &[&petitioner_account_id, &reason_id],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
        )?;

        let contents = self.tag_sibling_contents(&pairs)?;

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
            ContentType::TagSiblings,
            ContentStatus::Pending,
            vec![(ContentUpdate::Pend, contents)],
        )
    }

    /// `_repository_get_tag_sibling_petition`
    pub fn repository_get_tag_sibling_petition(
        &self,
        service_id: i64,
        petitioner_account_id: i64,
        reason_id: i64,
    ) -> HydrusResult<Petition> {
        let (_, _, pending_siblings, petitioned_siblings) =
            generate_repository_tag_siblings_table_names(service_id);
        let db = self.db.db_base()?;

        let petitioned = db.query_all(
            &format!(
                "SELECT bad_service_tag_id, good_service_tag_id FROM {petitioned_siblings} WHERE account_id = ? AND reason_id = ?;"
            ),
            &[&petitioner_account_id, &reason_id],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
        )?;

        let mut petitioned_pairs = Vec::new();

        for (bad_service_tag_id, good_service_tag_id) in petitioned {
            petitioned_pairs.push((
                self.repository_get_master_tag_id(service_id, bad_service_tag_id)?,
                self.repository_get_master_tag_id(service_id, good_service_tag_id)?,
            ));
        }

        let petitioned_contents = self.tag_sibling_contents(&petitioned_pairs)?;

        if petitioned_contents.is_empty() {
            return Err(HydrusException::new(
                ErrorKind::NotFound,
                "Sorry, did not find a petition for that account and reason!",
            ));
        }

        let pending = db.query_all(
            &format!(
                "SELECT bad_master_tag_id, good_master_tag_id FROM {pending_siblings} WHERE account_id = ? AND reason_id = ?;"
            ),
            &[&petitioner_account_id, &reason_id],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
        )?;

        let pending_contents = self.tag_sibling_contents(&pending)?;

        let actions_and_contents = vec![
            (ContentUpdate::Petition, petitioned_contents),
            (ContentUpdate::Pend, pending_contents),
        ];

        self.build_petition(
            service_id,
            petitioner_account_id,
            reason_id,
            ContentType::TagSiblings,
            ContentStatus::Petitioned,
            actions_and_contents,
        )
    }

    fn tag_sibling_contents(
        &self,
        pairs: &[(i64, i64)],
    ) -> HydrusResult<Vec<Content>> {
        let mut contents = Vec::with_capacity(pairs.len());

        for (bad_master_tag_id, good_master_tag_id) in pairs {
            let bad_tag = self.get_tag(*bad_master_tag_id)?;
            let good_tag = self.get_tag(*good_master_tag_id)?;

            contents.push(Content::tag_siblings(bad_tag, good_tag));
        }

        Ok(contents)
    }
}
