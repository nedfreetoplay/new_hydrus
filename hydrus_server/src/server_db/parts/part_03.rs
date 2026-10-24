impl ServerDB {


    /// `_get_service_type( service_id )`
    pub fn get_service_type(&self, service_id: i64) -> HydrusResult<ServiceType> {
        let db = self.db.db_base()?;

        let type_int = db
            .query_one(
                "SELECT service_type FROM services WHERE service_id = ?;",
                &[&service_id],
                |row| row.get::<_, i64>(0),
            )?
            .ok_or_else(|| {
                HydrusException::new(ErrorKind::DataMissing, "Service id error in database")
            })?;

        u16::try_from(type_int)
            .ok()
            .and_then(ServiceType::try_from_int)
            .ok_or_else(|| {
                HydrusException::new(
                    ErrorKind::Serialisation,
                    format!("Unknown server service type {type_int}."),
                )
            })
    }

    /// `_get_service_info( service_key )`
    #[allow(dead_code)]
    pub fn get_service_info(
        &self,
        service_key: &[u8],
    ) -> HydrusResult<std::collections::HashMap<i64, i64>> {
        let service_id = self.get_service_id(service_key)?;
        let db = self.db.db_base()?;

        let rows = db.query_all(
            "SELECT info_type, info FROM service_info WHERE service_id = ?;",
            &[&service_id],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
        )?;

        Ok(rows.into_iter().collect())
    }

    /// `_hash_exists( hash )`
    pub fn hash_exists(&self, hash: &[u8]) -> HydrusResult<bool> {
        let db = self.db.db_base()?;

        Ok(db
            .query_one(
                "SELECT 1 FROM external_master.hashes WHERE hash = ?;",
                &[&hash],
                |row| row.get::<_, i64>(0),
            )?
            .is_some())
    }

    /// `_master_hash_exists( hash )`
    pub fn master_hash_exists(&self, hash: &[u8]) -> HydrusResult<bool> {
        self.hash_exists(hash)
    }

    /// `_get_master_hash_id( hash )`
    pub fn get_master_hash_id(&self, hash: &[u8]) -> HydrusResult<i64> {
        let db = self.db.db_base()?;

        if let Some(master_hash_id) = db.query_one(
            "SELECT master_hash_id FROM external_master.hashes WHERE hash = ?;",
            &[&hash],
            |row| row.get::<_, i64>(0),
        )? {
            return Ok(master_hash_id);
        }

        db.execute(
            "INSERT OR IGNORE INTO external_master.hashes ( hash ) VALUES ( ? );",
            &[&hash],
        )?;

        db.query_one(
            "SELECT master_hash_id FROM external_master.hashes WHERE hash = ?;",
            &[&hash],
            |row| row.get::<_, i64>(0),
        )?
        .ok_or_else(|| {
            HydrusException::new(ErrorKind::DataMissing, "Could not create the master hash id.")
        })
    }

    /// `_get_master_hash_ids( hashes )`
    pub fn get_master_hash_ids(&self, hashes: &[Vec<u8>]) -> HydrusResult<Vec<i64>> {
        hashes
            .iter()
            .map(|hash| self.get_master_hash_id(hash))
            .collect()
    }

    /// `_get_hash( master_hash_id )`
    pub fn get_hash(&self, master_hash_id: i64) -> HydrusResult<Vec<u8>> {
        let db = self.db.db_base()?;

        db.query_one(
            "SELECT hash FROM external_master.hashes WHERE master_hash_id = ?;",
            &[&master_hash_id],
            |row| row.get::<_, Vec<u8>>(0),
        )?
        .ok_or_else(|| HydrusException::new(ErrorKind::DataMissing, "Hash error in database"))
    }

    /// `_get_hashes( master_hash_ids )`
    pub fn get_hashes(&self, master_hash_ids: &[i64]) -> HydrusResult<Vec<Vec<u8>>> {
        master_hash_ids
            .iter()
            .map(|master_hash_id| self.get_hash(*master_hash_id))
            .collect()
    }

    /// `_master_tag_exists( tag )`
    pub fn master_tag_exists(&self, tag: &str) -> HydrusResult<bool> {
        let db = self.db.db_base()?;

        Ok(db
            .query_one(
                "SELECT 1 FROM external_master.tags WHERE tag = ?;",
                &[&tag],
                |row| row.get::<_, i64>(0),
            )?
            .is_some())
    }

    /// `_get_master_tag_id( tag )`
    pub fn get_master_tag_id(&self, tag: &str) -> HydrusResult<i64> {
        let db = self.db.db_base()?;

        if let Some(master_tag_id) = db.query_one(
            "SELECT master_tag_id FROM external_master.tags WHERE tag = ?;",
            &[&tag],
            |row| row.get::<_, i64>(0),
        )? {
            return Ok(master_tag_id);
        }

        db.execute(
            "INSERT OR IGNORE INTO external_master.tags ( tag ) VALUES ( ? );",
            &[&tag],
        )?;

        db.query_one(
            "SELECT master_tag_id FROM external_master.tags WHERE tag = ?;",
            &[&tag],
            |row| row.get::<_, i64>(0),
        )?
        .ok_or_else(|| {
            HydrusException::new(ErrorKind::DataMissing, "Could not create the master tag id.")
        })
    }

    /// `_get_tag( master_tag_id )`
    pub fn get_tag(&self, master_tag_id: i64) -> HydrusResult<String> {
        let db = self.db.db_base()?;

        db.query_one(
            "SELECT tag FROM external_master.tags WHERE master_tag_id = ?;",
            &[&master_tag_id],
            |row| row.get::<_, String>(0),
        )?
        .ok_or_else(|| HydrusException::new(ErrorKind::DataMissing, "Tag error in database"))
    }

    /// `_get_reason( reason_id )`
    pub fn get_reason(&self, reason_id: i64) -> HydrusResult<String> {
        self.db
            .db_base()?
            .query_one(
                "SELECT reason FROM reasons WHERE reason_id = ?;",
                &[&reason_id],
                |row| row.get::<_, String>(0),
            )?
            .ok_or_else(|| {
                HydrusException::new(ErrorKind::DataMissing, "Reason error in database")
            })
    }
}
