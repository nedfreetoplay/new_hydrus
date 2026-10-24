impl ServerDB {
    /// `_repository_get_file_petition( service_id, petitioner_account_id, reason_id )`
    pub fn repository_get_file_petition(
        &self,
        service_id: i64,
        petitioner_account_id: i64,
        reason_id: i64,
    ) -> HydrusResult<Petition> {
        let (_, _, _, petitioned_files, _) =
            generate_repository_files_table_names(service_id);
        let db = self.db.db_base()?;

        let service_hash_ids = db.query_all(
            &format!(
                "SELECT service_hash_id FROM {petitioned_files} WHERE account_id = ? AND reason_id = ?;"
            ),
            &[&petitioner_account_id, &reason_id],
            |row| row.get::<_, i64>(0),
        )?;

        if service_hash_ids.is_empty() {
            return Err(HydrusException::new(
                ErrorKind::NotFound,
                "Sorry, did not find a petition for that account and reason!",
            ));
        }

        let master_hash_ids =
            self.repository_get_master_hash_ids(service_id, &service_hash_ids)?;
        let hashes = self.get_hashes(&master_hash_ids)?;

        self.build_petition(
            service_id,
            petitioner_account_id,
            reason_id,
            ContentType::Files,
            ContentStatus::Petitioned,
            vec![(ContentUpdate::Petition, vec![Content::files(hashes)])],
        )
    }

    /// `_repository_get_mapping_petition( service_id, petitioner_account_id, reason_id )`
    pub fn repository_get_mapping_petition(
        &self,
        service_id: i64,
        petitioner_account_id: i64,
        reason_id: i64,
    ) -> HydrusResult<Petition> {
        const MAX_MAPPINGS_PER_PETITION: usize = 500_000;
        const MAX_UNIQUE_TAG_IDS_PER_PETITION: usize = 10_000;

        let (_, _, _, petitioned_mappings) =
            generate_repository_mappings_table_names(service_id);
        let db = self.db.db_base()?;

        let rows = db.query_all(
            &format!(
                "SELECT service_tag_id, service_hash_id FROM {petitioned_mappings} WHERE account_id = ? AND reason_id = ? LIMIT ?;"
            ),
            &[
                &petitioner_account_id,
                &reason_id,
                &(MAX_MAPPINGS_PER_PETITION as i64),
            ],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
        )?;

        if rows.is_empty() {
            return Err(HydrusException::new(
                ErrorKind::NotFound,
                "Sorry, did not find a petition for that account and reason!",
            ));
        }

        let mut grouped: std::collections::HashMap<i64, Vec<i64>> =
            std::collections::HashMap::new();

        for (service_tag_id, service_hash_id) in rows {
            grouped.entry(service_tag_id).or_default().push(service_hash_id);
        }

        let mut grouped: Vec<(i64, Vec<i64>)> = grouped.into_iter().collect();
        grouped.sort_by_key(|(tag_id, hashes)| (std::cmp::Reverse(hashes.len()), *tag_id));

        let mut contents = Vec::new();

        for (service_tag_id, service_hash_ids) in
            grouped.into_iter().take(MAX_UNIQUE_TAG_IDS_PER_PETITION)
        {
            let master_tag_id =
                self.repository_get_master_tag_id(service_id, service_tag_id)?;
            let tag = self.get_tag(master_tag_id)?;
            let master_hash_ids =
                self.repository_get_master_hash_ids(service_id, &service_hash_ids)?;
            let hashes = self.get_hashes(&master_hash_ids)?;

            contents.push(Content::mappings(tag, hashes));
        }

        self.build_petition(
            service_id,
            petitioner_account_id,
            reason_id,
            ContentType::Mappings,
            ContentStatus::Petitioned,
            vec![(ContentUpdate::Petition, contents)],
        )
    }

    pub(crate) fn build_petition(
        &self,
        service_id: i64,
        petitioner_account_id: i64,
        reason_id: i64,
        content_type: ContentType,
        status: ContentStatus,
        actions_and_contents: Vec<(ContentUpdate, Vec<Content>)>,
    ) -> HydrusResult<Petition> {
        let petitioner_account = self.get_account(service_id, petitioner_account_id)?;
        let account_key = petitioner_account.get_account_key();
        let reason = self.get_reason(reason_id)?;

        let petition_header = PetitionHeader::new(
            content_type,
            status,
            account_key,
            reason,
        );

        Ok(Petition::new(
            Arc::new(petitioner_account),
            petition_header,
            actions_and_contents,
        ))
    }
}
