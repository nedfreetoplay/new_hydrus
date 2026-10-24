impl ServerDB {
    /// `_repository_generate_updates( service_id, begin, end )`
    pub fn repository_generate_updates(
        &self,
        service_id: i64,
        begin: i64,
        end: i64,
    ) -> HydrusResult<Vec<RepositoryUpdate>> {
        let mut updates = Vec::new();

        for definitions in self.generate_definitions_updates(service_id, begin, end)? {
            if definitions.get_num_rows() > 0 {
                updates.push(RepositoryUpdate::Definitions(definitions));
            }
        }

        for content in self.generate_content_updates(service_id, begin, end)? {
            if content.get_num_rows(None) > 0 {
                updates.push(RepositoryUpdate::Content(content));
            }
        }

        Ok(updates)
    }

    fn generate_definitions_updates(
        &self,
        service_id: i64,
        begin: i64,
        end: i64,
    ) -> HydrusResult<Vec<DefinitionsUpdate>> {
        const MAX_DEFINITIONS_ROWS: usize = 50_000;

        let db = self.db.db_base()?;
        let (hash_map, tag_map) = generate_repository_master_map_table_names(service_id);
        let mut builder: UpdateBuilder<DefinitionsUpdate> =
            UpdateBuilder::new(MAX_DEFINITIONS_ROWS);

        let hashes = db.query_all(
            &format!(
                "SELECT service_hash_id, hash FROM {hash_map} NATURAL JOIN external_master.hashes WHERE hash_id_timestamp BETWEEN ? AND ?;"
            ),
            &[&begin, &end],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, Vec<u8>>(1)?)),
        )?;

        for (service_hash_id, hash) in hashes {
            builder.add_row(
                DefinitionsUpdateRow {
                    definitions_type: DefinitionsType::Hashes,
                    key: service_hash_id,
                    value: DefinitionValue::Hash(hash),
                },
                1,
            );
        }

        let tags = db.query_all(
            &format!(
                "SELECT service_tag_id, tag FROM {tag_map} NATURAL JOIN external_master.tags WHERE tag_id_timestamp BETWEEN ? AND ?;"
            ),
            &[&begin, &end],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)),
        )?;

        for (service_tag_id, tag) in tags {
            builder.add_row(
                DefinitionsUpdateRow {
                    definitions_type: DefinitionsType::Tags,
                    key: service_tag_id,
                    value: DefinitionValue::Tag(tag),
                },
                1,
            );
        }

        builder.finish();

        Ok(builder.get_updates().to_vec())
    }
}
