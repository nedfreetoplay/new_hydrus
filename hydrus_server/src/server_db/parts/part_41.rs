impl ServerDB {
    /// Ensure tables introduced after the first server schema exist.
    ///
    /// Python handles these through versioned `_update_db` branches. The Rust
    /// port keeps a single idempotent pass: it adds missing global tables and
    /// any missing repository tables, then regenerates cached service info.
    pub fn ensure_schema_migrations(&self) -> HydrusResult<bool> {
        let db = self.db.db_base()?;
        let mut changed = false;

        for (table_name, statement) in [
            (
                "deferred_physical_file_deletes",
                "CREATE TABLE IF NOT EXISTS deferred_physical_file_deletes ( master_hash_id INTEGER PRIMARY KEY );",
            ),
            (
                "deferred_physical_thumbnail_deletes",
                "CREATE TABLE IF NOT EXISTS deferred_physical_thumbnail_deletes ( master_hash_id INTEGER PRIMARY KEY );",
            ),
            (
                "service_info",
                "CREATE TABLE IF NOT EXISTS service_info ( service_id INTEGER, info_type INTEGER, info INTEGER, PRIMARY KEY ( service_id, info_type ) );",
            ),
        ] {
            if !db.table_exists(table_name)? {
                db.execute(statement, &[])?;
                changed = true;
            }
        }

        let repositories = self.get_service_ids(Some(&REPOSITORIES))?;

        for service_id in repositories {
            let (hash_map, _) =
                generate_repository_master_map_table_names(service_id);

            if !db.table_exists(&hash_map)? {
                create_repository_tables(&db, service_id)?;
                changed = true;

                continue;
            }

            let update_table = generate_repository_update_table_name(service_id);

            if !db.table_exists(&update_table)? {
                db.execute(
                    &format!(
                        "CREATE TABLE {update_table} ( master_hash_id INTEGER PRIMARY KEY );"
                    ),
                    &[],
                )?;
                changed = true;
            }

            let (_, _, _, _, ip_addresses) =
                generate_repository_files_table_names(service_id);

            if !db.table_exists(&ip_addresses)? {
                db.execute(
                    &format!(
                        "CREATE TABLE {ip_addresses} ( master_hash_id INTEGER, ip TEXT, ip_timestamp INTEGER );"
                    ),
                    &[],
                )?;
                changed = true;
            }
        }

        if changed {
            self.regenerate_service_info(None, None)?;
        }

        Ok(changed)
    }
}
