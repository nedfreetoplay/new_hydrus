impl ServerDB {


    /// `_repository_regenerate_service_info( service_id, info_type )`
    pub fn regenerate_service_info(
        &self,
        service_id: Option<i64>,
        info_type: Option<ServiceInfoType>,
    ) -> HydrusResult<()> {
        let service_ids = match service_id {
            Some(service_id) => vec![service_id],
            None => self.get_service_ids(Some(REPOSITORIES))?,
        };

        for service_id in service_ids {
            let service_type = self.get_service_type(service_id)?;

            if !REPOSITORIES.contains(&service_type) {
                continue;
            }

            let allowable: &[ServiceInfoType] = match service_type {
                ServiceType::FileRepository => FILE_REPOSITORY_SERVICE_INFO_TYPES,
                ServiceType::TagRepository => TAG_REPOSITORY_SERVICE_INFO_TYPES,
                _ => continue,
            };

            let info_types: Vec<ServiceInfoType> = match info_type {
                Some(info_type) => {
                    if allowable.contains(&info_type) {
                        vec![info_type]
                    } else {
                        continue;
                    }
                }
                None => allowable.to_vec(),
            };

            let db = self.db.db_base()?;

            if info_type.is_none() {
                db.execute(
                    "DELETE FROM service_info WHERE service_id = ?;",
                    &[&service_id],
                )?;
            } else {
                for info_type in &info_types {
                    db.execute(
                        "DELETE FROM service_info WHERE service_id = ? AND info_type = ?;",
                        &[&service_id, &i64::from(info_type.as_int())],
                    )?;
                }
            }

            for info_type in info_types {
                let count = self.service_info_count(service_id, service_type, info_type)?;

                db.execute(
                    "INSERT INTO service_info ( service_id, info_type, info ) VALUES ( ?, ?, ? );",
                    &[
                        &service_id,
                        &i64::from(info_type.as_int()),
                        &count,
                    ],
                )?;
            }
        }

        Ok(())
    }

    /// Count one repository service-info value.
    pub fn get_service_info_specific(
        &self,
        service_id: i64,
        info_type: ServiceInfoType,
    ) -> HydrusResult<i64> {
        let service_type = self.get_service_type(service_id)?;

        self.service_info_count(service_id, service_type, info_type)
    }

    fn service_info_count(
        &self,
        service_id: i64,
        service_type: ServiceType,
        info_type: ServiceInfoType,
    ) -> HydrusResult<i64> {
        let (hash_map, tag_map) = generate_repository_master_map_table_names(service_id);
        let (current_files, deleted_files, pending_files, petitioned_files, _) =
            generate_repository_files_table_names(service_id);
        let (current_mappings, deleted_mappings, pending_mappings, petitioned_mappings) =
            generate_repository_mappings_table_names(service_id);
        let (current_siblings, deleted_siblings, pending_siblings, petitioned_siblings) =
            generate_repository_tag_siblings_table_names(service_id);
        let (current_parents, deleted_parents, pending_parents, petitioned_parents) =
            generate_repository_tag_parents_table_names(service_id);

        let table = match info_type {
            ServiceInfoType::NumFileHashes => Some(hash_map.as_str()),
            ServiceInfoType::NumFiles => Some(current_files.as_str()),
            ServiceInfoType::NumDeletedFiles => Some(deleted_files.as_str()),
            ServiceInfoType::NumPendingFiles => Some(pending_files.as_str()),
            ServiceInfoType::NumPetitionedFiles => Some(petitioned_files.as_str()),
            ServiceInfoType::NumTags => Some(tag_map.as_str()),
            ServiceInfoType::NumMappings => Some(current_mappings.as_str()),
            ServiceInfoType::NumDeletedMappings => Some(deleted_mappings.as_str()),
            ServiceInfoType::NumPendingMappings => Some(pending_mappings.as_str()),
            ServiceInfoType::NumPetitionedMappings => Some(petitioned_mappings.as_str()),
            ServiceInfoType::NumTagSiblings => Some(current_siblings.as_str()),
            ServiceInfoType::NumDeletedTagSiblings => Some(deleted_siblings.as_str()),
            ServiceInfoType::NumPendingTagSiblings => Some(pending_siblings.as_str()),
            ServiceInfoType::NumPetitionedTagSiblings => Some(petitioned_siblings.as_str()),
            ServiceInfoType::NumTagParents => Some(current_parents.as_str()),
            ServiceInfoType::NumDeletedTagParents => Some(deleted_parents.as_str()),
            ServiceInfoType::NumPendingTagParents => Some(pending_parents.as_str()),
            ServiceInfoType::NumPetitionedTagParents => Some(petitioned_parents.as_str()),
            _ => None,
        };

        let db = self.db.db_base()?;

        if let Some(table) = table {
            return db
                .query_one(
                    &format!("SELECT COUNT( * ) FROM {table};"),
                    &[],
                    |row| row.get::<_, i64>(0),
                )?
                .ok_or_else(|| {
                    HydrusException::new(ErrorKind::DataMissing, "Could not count service info.")
                });
        }

        let query = match info_type {
            ServiceInfoType::NumActionableFileAddPetitions => format!(
                "SELECT COUNT( * ) FROM ( SELECT DISTINCT account_id, reason_id FROM {pending_files} );"
            ),
            ServiceInfoType::NumActionableFileDeletePetitions => format!(
                "SELECT COUNT( * ) FROM ( SELECT DISTINCT account_id, reason_id FROM {petitioned_files} );"
            ),
            ServiceInfoType::NumActionableMappingAddPetitions => format!(
                "SELECT COUNT( * ) FROM ( SELECT DISTINCT master_tag_id, reason_id FROM {pending_mappings} );"
            ),
            ServiceInfoType::NumActionableMappingDeletePetitions => format!(
                "SELECT COUNT( * ) FROM ( SELECT DISTINCT service_tag_id, reason_id FROM {petitioned_mappings} );"
            ),
            ServiceInfoType::NumActionableSiblingAddPetitions => format!(
                "SELECT COUNT( * ) FROM ( SELECT DISTINCT reason_id FROM {pending_siblings} EXCEPT SELECT DISTINCT reason_id FROM {petitioned_siblings} );"
            ),
            ServiceInfoType::NumActionableSiblingDeletePetitions => format!(
                "SELECT COUNT( * ) FROM ( SELECT DISTINCT reason_id FROM {petitioned_siblings} );"
            ),
            ServiceInfoType::NumActionableParentAddPetitions => format!(
                "SELECT COUNT( * ) FROM ( SELECT DISTINCT reason_id FROM {pending_parents} EXCEPT SELECT DISTINCT reason_id FROM {petitioned_parents} );"
            ),
            ServiceInfoType::NumActionableParentDeletePetitions => format!(
                "SELECT COUNT( * ) FROM ( SELECT DISTINCT reason_id FROM {petitioned_parents} );"
            ),
            _ => {
                return Err(HydrusException::new(
                    ErrorKind::DataMissing,
                    format!(
                        "Was asked to generate service info for an unsupported type: {}",
                        info_type.as_str()
                    ),
                ));
            }
        };

        let _ = service_type;

        db.query_one(&query, &[], |row| row.get::<_, i64>(0))?
            .ok_or_else(|| {
                HydrusException::new(ErrorKind::DataMissing, "Could not count service info.")
            })
    }
}
