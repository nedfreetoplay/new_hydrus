impl ServerDB {
    /// `_modify_services( account, services )`
    pub fn modify_services(
        &self,
        services: &[ServerService],
    ) -> HydrusResult<std::collections::HashMap<Vec<u8>, Vec<u8>>> {
        let current_service_keys: std::collections::HashSet<Vec<u8>> =
            self.get_service_keys(None)?.into_iter().collect();
        let future_service_keys: std::collections::HashSet<Vec<u8>> = services
            .iter()
            .map(|service| service.get_service_key().to_vec())
            .collect();

        for service_key in &current_service_keys {
            if !future_service_keys.contains(service_key) {
                self.delete_service(service_key)?;
            }
        }

        let mut service_keys_to_access_keys = std::collections::HashMap::new();

        for service in services {
            let service_key = service.get_service_key().to_vec();

            if current_service_keys.contains(&service_key) {
                self.save_service(service)?;
            } else {
                let (_, access_key) = self.add_service_with_admin_access_key(service)?;
                service_keys_to_access_keys.insert(service_key, access_key);
            }
        }

        Ok(service_keys_to_access_keys)
    }

    /// `_delete_service( service_key )`
    pub fn delete_service(&self, service_key: &[u8]) -> HydrusResult<()> {
        let service_id = self.get_service_id(service_key)?;
        let service_type = self.get_service_type(service_id)?;
        let db = self.db.db_base()?;

        for statement in [
            "DELETE FROM accounts WHERE service_id = ?;",
            "DELETE FROM account_types WHERE service_id = ?;",
            "DELETE FROM sessions WHERE service_id = ?;",
            "DELETE FROM registration_keys WHERE service_id = ?;",
            "DELETE FROM service_info WHERE service_id = ?;",
            "DELETE FROM account_scores WHERE service_id = ?;",
            "DELETE FROM services WHERE service_id = ?;",
        ] {
            db.execute(statement, &[&service_id])?;
        }

        if REPOSITORIES.contains(&service_type) {
            for table in self.repository_table_names(service_id) {
                db.execute(&format!("DROP TABLE IF EXISTS {table};"), &[])?;
            }
        }

        Ok(())
    }

    fn repository_table_names(&self, service_id: i64) -> Vec<String> {
        let (hash_map, tag_map) =
            generate_repository_master_map_table_names(service_id);
        let (current_files, deleted_files, pending_files, petitioned_files, ip_addresses) =
            generate_repository_files_table_names(service_id);
        let (current_mappings, deleted_mappings, pending_mappings, petitioned_mappings) =
            generate_repository_mappings_table_names(service_id);
        let (current_parents, deleted_parents, pending_parents, petitioned_parents) =
            generate_repository_tag_parents_table_names(service_id);
        let (current_siblings, deleted_siblings, pending_siblings, petitioned_siblings) =
            generate_repository_tag_siblings_table_names(service_id);

        vec![
            hash_map,
            tag_map,
            current_files,
            deleted_files,
            pending_files,
            petitioned_files,
            ip_addresses,
            current_mappings,
            deleted_mappings,
            pending_mappings,
            petitioned_mappings,
            current_parents,
            deleted_parents,
            pending_parents,
            petitioned_parents,
            current_siblings,
            deleted_siblings,
            pending_siblings,
            petitioned_siblings,
            generate_repository_update_table_name(service_id),
        ]
    }
}
