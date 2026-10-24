impl ServerDB {


    /// `_Filter_orphan_master_hash_ids( master_hash_ids, ignore_service_id )`
    pub fn filter_orphan_master_hash_ids(
        &self,
        master_hash_ids: &[i64],
        ignore_service_id: Option<i64>,
    ) -> HydrusResult<Vec<i64>> {
        let file_repository_ids =
            self.get_service_ids(Some(&[ServiceType::FileRepository]))?;
        let repository_ids = self.get_service_ids(Some(REPOSITORIES))?;
        let db = self.db.db_base()?;

        let mut orphan_ids = Vec::new();

        for master_hash_id in master_hash_ids {
            let mut is_useful = false;

            for service_id in &file_repository_ids {
                if ignore_service_id == Some(*service_id) {
                    continue;
                }

                let (hash_map, _) =
                    generate_repository_master_map_table_names(*service_id);
                let (current_files, _, _, _, _) =
                    generate_repository_files_table_names(*service_id);

                let query = format!(
                    "SELECT 1 FROM {hash_map} AS hash_map CROSS JOIN {current_files} AS current_files USING ( service_hash_id ) WHERE hash_map.master_hash_id = ? LIMIT 1;"
                );

                if db
                    .query_one(&query, &[master_hash_id], |row| row.get::<_, i64>(0))?
                    .is_some()
                {
                    is_useful = true;
                    break;
                }
            }

            if !is_useful {
                for service_id in &repository_ids {
                    if ignore_service_id == Some(*service_id) {
                        continue;
                    }

                    let update_table = generate_repository_update_table_name(*service_id);
                    let query = format!(
                        "SELECT 1 FROM {update_table} WHERE master_hash_id = ? LIMIT 1;"
                    );

                    if db
                        .query_one(&query, &[master_hash_id], |row| row.get::<_, i64>(0))?
                        .is_some()
                    {
                        is_useful = true;
                        break;
                    }
                }
            }

            if !is_useful {
                orphan_ids.push(*master_hash_id);
            }
        }

        Ok(orphan_ids)
    }

    /// `_is_an_orphan( possible_hash )`
    pub fn is_an_orphan(&self, possible_hash: &[u8]) -> HydrusResult<bool> {
        if !self.hash_exists(possible_hash)? {
            return Ok(true);
        }

        let master_hash_id = self.get_master_hash_id(possible_hash)?;

        Ok(self
            .filter_orphan_master_hash_ids(&[master_hash_id], None)?
            .len()
            == 1)
    }

    /// `_repository_get_ip_timestamp( service_key, account, hash )`
    pub fn get_ip_timestamp(
        &self,
        service_key: &[u8],
        hash: &[u8],
    ) -> HydrusResult<(String, i64)> {
        let service_id = self.get_service_id(service_key)?;
        let master_hash_id = self.get_master_hash_id(hash)?;
        let (_, _, _, _, ip_addresses) =
            generate_repository_files_table_names(service_id);
        let db = self.db.db_base()?;

        db.query_one(
            &format!(
                "SELECT ip, ip_timestamp FROM {ip_addresses} WHERE master_hash_id = ?;"
            ),
            &[&master_hash_id],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)),
        )?
        .ok_or_else(|| {
            HydrusException::new(
                ErrorKind::NotFound,
                "Did not find ip information for that hash.",
            )
        })
    }

    /// The write half of uploader-IP tracking.
    pub fn set_ip_timestamp(
        &self,
        service_key: &[u8],
        hash: &[u8],
        ip: &str,
        timestamp: i64,
    ) -> HydrusResult<()> {
        let service_id = self.get_service_id(service_key)?;
        let master_hash_id = self.get_master_hash_id(hash)?;
        let (_, _, _, _, ip_addresses) =
            generate_repository_files_table_names(service_id);
        let db = self.db.db_base()?;

        db.execute(
            &format!(
                "INSERT INTO {ip_addresses} ( master_hash_id, ip, ip_timestamp ) VALUES ( ?, ?, ? );"
            ),
            &[&master_hash_id, &ip, &timestamp],
        )?;

        Ok(())
    }

    /// `_repository_update_service_info( service_id, info_type, delta )`
    pub fn update_service_info(
        &self,
        service_id: i64,
        info_type: ServiceInfoType,
        delta: i64,
    ) -> HydrusResult<()> {
        if delta == 0 {
            return Ok(());
        }

        self.db.db_base()?.execute(
            "UPDATE service_info SET info = info + ? WHERE service_id = ? AND info_type = ?;",
            &[&delta, &service_id, &i64::from(info_type.as_int())],
        )?;

        Ok(())
    }
}
