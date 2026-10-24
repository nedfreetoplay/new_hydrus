impl ServerDB {
    fn add_tag_relation_content_rows(
        &self,
        builder: &mut UpdateBuilder<ContentUpdatePackage>,
        service_id: i64,
        begin: i64,
        end: i64,
    ) -> HydrusResult<()> {
        let db = self.db.db_base()?;
        let (current_parents, deleted_parents, _, _) =
            generate_repository_tag_parents_table_names(service_id);

        for (table, action, timestamp_column) in [
            (current_parents, ContentUpdate::Add, "parent_timestamp"),
            (deleted_parents, ContentUpdate::Delete, "parent_timestamp"),
        ] {
            let rows = db.query_all(
                &format!(
                    "SELECT child_service_tag_id, parent_service_tag_id FROM {table} WHERE {timestamp_column} BETWEEN ? AND ?;"
                ),
                &[&begin, &end],
                |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
            )?;

            for (child_service_tag_id, parent_service_tag_id) in rows {
                builder.add_row(
                    ContentUpdateRow {
                        content_type: ContentType::TagParents,
                        action,
                        data: json!([child_service_tag_id, parent_service_tag_id]),
                    },
                    1,
                );
            }
        }

        let (current_siblings, deleted_siblings, _, _) =
            generate_repository_tag_siblings_table_names(service_id);

        for (table, action, timestamp_column) in [
            (current_siblings, ContentUpdate::Add, "sibling_timestamp"),
            (deleted_siblings, ContentUpdate::Delete, "sibling_timestamp"),
        ] {
            let rows = db.query_all(
                &format!(
                    "SELECT bad_service_tag_id, good_service_tag_id FROM {table} WHERE {timestamp_column} BETWEEN ? AND ?;"
                ),
                &[&begin, &end],
                |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
            )?;

            for (bad_service_tag_id, good_service_tag_id) in rows {
                builder.add_row(
                    ContentUpdateRow {
                        content_type: ContentType::TagSiblings,
                        action,
                        data: json!([bad_service_tag_id, good_service_tag_id]),
                    },
                    1,
                );
            }
        }

        Ok(())
    }
}
