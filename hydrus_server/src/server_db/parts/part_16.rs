impl ServerDB {


    /// `_repository_deny_tag_parent_petition`
    pub fn repository_deny_tag_parent_petition(
        &self,
        service_id: i64,
        child_service_tag_id: i64,
        parent_service_tag_id: i64,
    ) -> HydrusResult<()> {
        let (_, _, _, petitioned_parents) =
            generate_repository_tag_parents_table_names(service_id);
        let deleted = self.db.db_base()?.execute(
            &format!(
                "DELETE FROM {petitioned_parents} WHERE child_service_tag_id = ? AND parent_service_tag_id = ?;"
            ),
            &[&child_service_tag_id, &parent_service_tag_id],
        )?;

        if deleted > 0 {
            self.update_service_info(
                service_id,
                ServiceInfoType::NumPetitionedTagParents,
                -(deleted as i64),
            )?;
        }

        self.regenerate_service_info(
            Some(service_id),
            Some(ServiceInfoType::NumActionableParentDeletePetitions),
        )?;

        Ok(())
    }

    /// `_repository_deny_tag_sibling_pend`
    pub fn repository_deny_tag_sibling_pend(
        &self,
        service_id: i64,
        bad_master_tag_id: i64,
        good_master_tag_id: i64,
    ) -> HydrusResult<()> {
        let (_, _, pending_siblings, _) =
            generate_repository_tag_siblings_table_names(service_id);
        let deleted = self.db.db_base()?.execute(
            &format!(
                "DELETE FROM {pending_siblings} WHERE bad_master_tag_id = ? AND good_master_tag_id = ?;"
            ),
            &[&bad_master_tag_id, &good_master_tag_id],
        )?;

        if deleted > 0 {
            self.update_service_info(
                service_id,
                ServiceInfoType::NumPendingTagSiblings,
                -(deleted as i64),
            )?;
        }

        self.regenerate_service_info(
            Some(service_id),
            Some(ServiceInfoType::NumActionableSiblingAddPetitions),
        )?;

        Ok(())
    }

    /// `_repository_deny_tag_sibling_petition`
    pub fn repository_deny_tag_sibling_petition(
        &self,
        service_id: i64,
        bad_service_tag_id: i64,
        good_service_tag_id: i64,
    ) -> HydrusResult<()> {
        let (_, _, _, petitioned_siblings) =
            generate_repository_tag_siblings_table_names(service_id);
        let deleted = self.db.db_base()?.execute(
            &format!(
                "DELETE FROM {petitioned_siblings} WHERE bad_service_tag_id = ? AND good_service_tag_id = ?;"
            ),
            &[&bad_service_tag_id, &good_service_tag_id],
        )?;

        if deleted > 0 {
            self.update_service_info(
                service_id,
                ServiceInfoType::NumPetitionedTagSiblings,
                -(deleted as i64),
            )?;
        }

        self.regenerate_service_info(
            Some(service_id),
            Some(ServiceInfoType::NumActionableSiblingDeletePetitions),
        )?;

        Ok(())
    }

    /// `_repository_get_file_petitions_summary`
    pub fn repository_get_file_petitions_summary(
        &self,
        service_id: i64,
        limit: usize,
        account_id: Option<i64>,
        reason_id: Option<i64>,
    ) -> HydrusResult<Vec<PetitionHeader>> {
        let (_, _, _, petitioned_files, _) =
            generate_repository_files_table_names(service_id);

        self.petition_summary(
            petitioned_files,
            ContentType::Files,
            ContentStatus::Petitioned,
            limit,
            account_id,
            reason_id,
        )
    }

    /// `_repository_get_mapping_petitions_summary`
    pub fn repository_get_mapping_petitions_summary(
        &self,
        service_id: i64,
        limit: usize,
        account_id: Option<i64>,
        reason_id: Option<i64>,
    ) -> HydrusResult<Vec<PetitionHeader>> {
        let (_, _, _, petitioned_mappings) =
            generate_repository_mappings_table_names(service_id);

        self.petition_summary(
            petitioned_mappings,
            ContentType::Mappings,
            ContentStatus::Petitioned,
            limit,
            account_id,
            reason_id,
        )
    }

    /// `_repository_get_tag_parent_petitions_summary`
    pub fn repository_get_tag_parent_petitions_summary(
        &self,
        service_id: i64,
        limit: usize,
        account_id: Option<i64>,
        reason_id: Option<i64>,
    ) -> HydrusResult<Vec<PetitionHeader>> {
        let (_, _, _, petitioned_parents) =
            generate_repository_tag_parents_table_names(service_id);

        self.petition_summary(
            petitioned_parents,
            ContentType::TagParents,
            ContentStatus::Petitioned,
            limit,
            account_id,
            reason_id,
        )
    }

    /// `_repository_get_tag_sibling_petitions_summary`
    pub fn repository_get_tag_sibling_petitions_summary(
        &self,
        service_id: i64,
        limit: usize,
        account_id: Option<i64>,
        reason_id: Option<i64>,
    ) -> HydrusResult<Vec<PetitionHeader>> {
        let (_, _, _, petitioned_siblings) =
            generate_repository_tag_siblings_table_names(service_id);

        self.petition_summary(
            petitioned_siblings,
            ContentType::TagSiblings,
            ContentStatus::Petitioned,
            limit,
            account_id,
            reason_id,
        )
    }
}
