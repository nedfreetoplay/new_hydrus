impl ServerDB {


    /// `_get_account_key_from_content`
    pub fn get_account_key_from_content(
        &self,
        service_key: &[u8],
        content: &Content,
    ) -> HydrusResult<Vec<u8>> {
        let service_id = self.get_service_id(service_key)?;
        let service_type = self.get_service_type(service_id)?;
        let account_id = match content.get_content_data() {
            ContentData::Files(hashes) => {
                if service_type != ServiceType::FileRepository {
                    return Err(HydrusException::new(
                        ErrorKind::NotFound,
                        "Only File Repositories support file account lookups!",
                    ));
                }

                let hash = hashes.first().ok_or_else(|| {
                    HydrusException::new(ErrorKind::NotFound, "No file hash in content!")
                })?;

                if !self.master_hash_exists(hash)? {
                    return Err(HydrusException::new(
                        ErrorKind::NotFound,
                        "The service could not find that hash in its database.",
                    ));
                }

                let master_hash_id = self.get_master_hash_id(hash)?;
                let (hash_map, _) = generate_repository_master_map_table_names(service_id);
                let (current_files, _, _, _, _) =
                    generate_repository_files_table_names(service_id);

                self.db
                    .db_base()?
                    .query_one(
                        &format!(
                            "SELECT account_id FROM {current_files} NATURAL JOIN {hash_map} WHERE master_hash_id = ?;"
                        ),
                        &[&master_hash_id],
                        |row| row.get::<_, i64>(0),
                    )?
                    .ok_or_else(|| {
                        HydrusException::new(
                            ErrorKind::NotFound,
                            "The service could not find that hash in its database.",
                        )
                    })?
            }
            ContentData::Mapping(tag, hash) => {
                if service_type != ServiceType::TagRepository {
                    return Err(HydrusException::new(
                        ErrorKind::NotFound,
                        "Only Tag Repositories support mapping account lookups!",
                    ));
                }

                if !self.master_hash_exists(hash)? || !self.master_tag_exists(tag)? {
                    return Err(HydrusException::new(
                        ErrorKind::NotFound,
                        "The service could not find that mapping in its database.",
                    ));
                }

                let master_hash_id = self.get_master_hash_id(hash)?;
                let master_tag_id = self.get_master_tag_id(tag)?;
                let (hash_map, tag_map) =
                    generate_repository_master_map_table_names(service_id);
                let (current_mappings, _, _, _) =
                    generate_repository_mappings_table_names(service_id);

                self.db
                    .db_base()?
                    .query_one(
                        &format!(
                            "SELECT account_id FROM {current_mappings} NATURAL JOIN {hash_map} NATURAL JOIN {tag_map} WHERE master_hash_id = ? AND master_tag_id = ?;"
                        ),
                        &[&master_hash_id, &master_tag_id],
                        |row| row.get::<_, i64>(0),
                    )?
                    .ok_or_else(|| {
                        HydrusException::new(
                            ErrorKind::NotFound,
                            "The service could not find that mapping in its database.",
                        )
                    })?
            }
            _ => {
                return Err(HydrusException::new(
                    ErrorKind::NotFound,
                    "The service could not understand the submitted content.",
                ));
            }
        };

        self.get_account_key_from_account_id(account_id)
    }
}
