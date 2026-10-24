impl ServerDB {
    /// `_repository_get_tag_parent_pends_summary`
    pub fn repository_get_tag_parent_pends_summary(
        &self,
        service_id: i64,
        limit: usize,
        account_id: Option<i64>,
        reason_id: Option<i64>,
    ) -> HydrusResult<Vec<PetitionHeader>> {
        let (_, _, pending_parents, petitioned_parents) =
            generate_repository_tag_parents_table_names(service_id);

        self.pending_petition_summary(
            &pending_parents,
            &petitioned_parents,
            ContentType::TagParents,
            ContentStatus::Pending,
            limit,
            account_id,
            reason_id,
        )
    }

    /// `_repository_get_tag_sibling_pends_summary`
    pub fn repository_get_tag_sibling_pends_summary(
        &self,
        service_id: i64,
        limit: usize,
        account_id: Option<i64>,
        reason_id: Option<i64>,
    ) -> HydrusResult<Vec<PetitionHeader>> {
        let (_, _, pending_siblings, petitioned_siblings) =
            generate_repository_tag_siblings_table_names(service_id);

        self.pending_petition_summary(
            &pending_siblings,
            &petitioned_siblings,
            ContentType::TagSiblings,
            ContentStatus::Pending,
            limit,
            account_id,
            reason_id,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn pending_petition_summary(
        &self,
        pending_table: &str,
        petitioned_table: &str,
        content_type: ContentType,
        status: ContentStatus,
        limit: usize,
        account_id: Option<i64>,
        reason_id: Option<i64>,
    ) -> HydrusResult<Vec<PetitionHeader>> {
        let rows = self.db.db_base()?.query_all(
            &format!(
                "SELECT DISTINCT account_id, reason_id FROM {pending_table} AS pending \
                 WHERE NOT EXISTS ( SELECT 1 FROM {petitioned_table} AS petitioned \
                 WHERE petitioned.account_id = pending.account_id \
                 AND petitioned.reason_id = pending.reason_id ) \
                 ORDER BY account_id;"
            ),
            &[],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
        )?;

        let mut headers = Vec::new();

        for (row_account_id, row_reason_id) in rows {
            if account_id.is_some_and(|id| row_account_id != id) {
                continue;
            }

            if reason_id.is_some_and(|id| row_reason_id != id) {
                continue;
            }

            if headers.len() >= limit {
                break;
            }

            headers.push(PetitionHeader::new(
                content_type,
                status,
                self.get_account_key_from_account_id(row_account_id)?,
                self.get_reason(row_reason_id)?,
            ));
        }

        Ok(headers)
    }
}
