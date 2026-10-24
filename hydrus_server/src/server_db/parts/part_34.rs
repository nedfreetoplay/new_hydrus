impl ServerDB {
    fn generate_content_updates(
        &self,
        service_id: i64,
        begin: i64,
        end: i64,
    ) -> HydrusResult<Vec<ContentUpdatePackage>> {
        const MAX_CONTENT_ROWS: usize = 250_000;

        let mut builder: UpdateBuilder<ContentUpdatePackage> =
            UpdateBuilder::new(MAX_CONTENT_ROWS);

        self.add_file_content_rows(&mut builder, service_id, begin, end)?;
        self.add_mapping_content_rows(&mut builder, service_id, begin, end)?;
        self.add_tag_relation_content_rows(&mut builder, service_id, begin, end)?;

        builder.finish();

        Ok(builder.get_updates().to_vec())
    }

    fn add_file_content_rows(
        &self,
        builder: &mut UpdateBuilder<ContentUpdatePackage>,
        service_id: i64,
        begin: i64,
        end: i64,
    ) -> HydrusResult<()> {
        let db = self.db.db_base()?;
        let (current_files, deleted_files, _, _, _) =
            generate_repository_files_table_names(service_id);
        let (hash_map, _) = generate_repository_master_map_table_names(service_id);

        let rows = db.query_all(
            &format!(
                "SELECT service_hash_id, size, mime, file_timestamp, width, height, duration, num_frames, num_words FROM files_info NATURAL JOIN {hash_map} NATURAL JOIN {current_files} WHERE file_timestamp BETWEEN ? AND ?;"
            ),
            &[&begin, &end],
            |row| {
                Ok(json!([
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, Option<i64>>(4)?,
                    row.get::<_, Option<i64>>(5)?,
                    row.get::<_, Option<i64>>(6)?,
                    row.get::<_, Option<i64>>(7)?,
                    row.get::<_, Option<i64>>(8)?,
                ]))
            },
        )?;

        for row in rows {
            builder.add_row(
                ContentUpdateRow {
                    content_type: ContentType::Files,
                    action: ContentUpdate::Add,
                    data: row,
                },
                1,
            );
        }

        let service_hash_ids = db.query_all(
            &format!(
                "SELECT service_hash_id FROM {deleted_files} WHERE file_timestamp BETWEEN ? AND ?;"
            ),
            &[&begin, &end],
            |row| row.get::<_, i64>(0),
        )?;

        for service_hash_id in service_hash_ids {
            builder.add_row(
                ContentUpdateRow {
                    content_type: ContentType::Files,
                    action: ContentUpdate::Delete,
                    data: json!(service_hash_id),
                },
                1,
            );
        }

        Ok(())
    }

    fn add_mapping_content_rows(
        &self,
        builder: &mut UpdateBuilder<ContentUpdatePackage>,
        service_id: i64,
        begin: i64,
        end: i64,
    ) -> HydrusResult<()> {
        const MAX_CONTENT_CHUNK: usize = 25_000;

        let db = self.db.db_base()?;
        let (current_mappings, deleted_mappings, _, _) =
            generate_repository_mappings_table_names(service_id);

        for (table, action) in [
            (current_mappings, ContentUpdate::Add),
            (deleted_mappings, ContentUpdate::Delete),
        ] {
            let rows = db.query_all(
                &format!(
                    "SELECT service_tag_id, service_hash_id FROM {table} WHERE mapping_timestamp BETWEEN ? AND ?;"
                ),
                &[&begin, &end],
                |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
            )?;

            let mut grouped: std::collections::HashMap<i64, Vec<i64>> =
                std::collections::HashMap::new();

            for (service_tag_id, service_hash_id) in rows {
                grouped.entry(service_tag_id).or_default().push(service_hash_id);
            }

            let mut grouped: Vec<(i64, Vec<i64>)> = grouped.into_iter().collect();
            grouped.sort_by_key(|(tag_id, _)| *tag_id);

            for (service_tag_id, service_hash_ids) in grouped {
                for block in service_hash_ids.chunks(MAX_CONTENT_CHUNK) {
                    builder.add_row(
                        ContentUpdateRow {
                            content_type: ContentType::Mappings,
                            action,
                            data: json!([service_tag_id, block]),
                        },
                        block.len(),
                    );
                }
            }
        }

        Ok(())
    }
}
