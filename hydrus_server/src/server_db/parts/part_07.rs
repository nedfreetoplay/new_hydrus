impl ServerDB {


    /// `_repository_get_service_hash_id( service_id, master_hash_id, timestamp )`
    pub fn repository_get_service_hash_id(
        &self,
        service_id: i64,
        master_hash_id: i64,
        timestamp: i64,
    ) -> HydrusResult<i64> {
        let (hash_map, _) = generate_repository_master_map_table_names(service_id);
        let db = self.db.db_base()?;

        if let Some(service_hash_id) = db.query_one(
            &format!(
                "SELECT service_hash_id FROM {hash_map} WHERE master_hash_id = ?;"
            ),
            &[&master_hash_id],
            |row| row.get::<_, i64>(0),
        )? {
            return Ok(service_hash_id);
        }

        db.execute(
            &format!(
                "INSERT INTO {hash_map} ( master_hash_id, hash_id_timestamp ) VALUES ( ?, ? );"
            ),
            &[&master_hash_id, &timestamp],
        )?;

        self.update_service_info(service_id, ServiceInfoType::NumFileHashes, 1)?;

        Ok(db.get_last_row_id())
    }

    /// `_repository_get_service_hash_ids( service_id, master_hash_ids, timestamp )`
    pub fn repository_get_service_hash_ids(
        &self,
        service_id: i64,
        master_hash_ids: &[i64],
        timestamp: i64,
    ) -> HydrusResult<Vec<i64>> {
        let mut service_hash_ids = Vec::with_capacity(master_hash_ids.len());

        for master_hash_id in master_hash_ids {
            service_hash_ids.push(self.repository_get_service_hash_id(
                service_id,
                *master_hash_id,
                timestamp,
            )?);
        }

        Ok(service_hash_ids)
    }

    /// `_repository_get_master_hash_ids( service_id, service_hash_ids )`
    pub fn repository_get_master_hash_ids(
        &self,
        service_id: i64,
        service_hash_ids: &[i64],
    ) -> HydrusResult<Vec<i64>> {
        let (hash_map, _) = generate_repository_master_map_table_names(service_id);
        let db = self.db.db_base()?;
        let mut master_hash_ids = Vec::with_capacity(service_hash_ids.len());

        for service_hash_id in service_hash_ids {
            let master_hash_id = db
                .query_one(
                    &format!(
                        "SELECT master_hash_id FROM {hash_map} WHERE service_hash_id = ?;"
                    ),
                    &[service_hash_id],
                    |row| row.get::<_, i64>(0),
                )?
                .ok_or_else(|| {
                    HydrusException::new(
                        ErrorKind::DataMissing,
                        "Missing master_hash_id map error!",
                    )
                })?;

            master_hash_ids.push(master_hash_id);
        }

        Ok(master_hash_ids)
    }

    /// `_repository_service_hash_id_exists( service_id, master_hash_id )`
    pub fn repository_service_hash_id_exists(
        &self,
        service_id: i64,
        master_hash_id: i64,
    ) -> HydrusResult<bool> {
        let (hash_map, _) = generate_repository_master_map_table_names(service_id);

        Ok(self
            .db
            .db_base()?
            .query_one(
                &format!(
                    "SELECT 1 FROM {hash_map} WHERE master_hash_id = ?;"
                ),
                &[&master_hash_id],
                |row| row.get::<_, i64>(0),
            )?
            .is_some())
    }
}
