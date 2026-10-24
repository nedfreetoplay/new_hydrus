impl ServerDB {

    /// `DB( controller, db_dir, db_name )`
    pub fn new(
        controller: Arc<dyn DBController>,
        db_dir: impl Into<PathBuf>,
        db_name: impl Into<String>,
    ) -> HydrusResult<Arc<Self>> {
        let db_dir = db_dir.into();
        let db_name = db_name.into();
        let files = ServerFiles::new(db_dir.join("server_files"));

        let config = HydrusDBConfig {
            db_dir: db_dir.clone(),
            db_name,
            external_databases: vec![
                ("external_mappings".to_string(), "server.mappings.db".to_string()),
                ("external_master".to_string(), "server.master.db".to_string()),
            ],
            additional_db_filenames: vec!["server.master.db".to_string(), "server.mappings.db".to_string()],
            durable_temp_db_filename: None,
            ssl_cert_filename: None,
            ssl_key_filename: None,
            read_write_actions: vec![
                "access_key".to_string(),
                "immediate_content_update".to_string(),
                "registration_keys".to_string(),
            ],
            update_wait_seconds: 2,
        };

        let hooks = Arc::new(ServerDBHooks {
            files: files.clone(),
        });

        let db = HydrusDB::new(config, hooks, controller)?;

        Ok(Arc::new(Self { db, db_dir, files }))
    }

    pub fn db(&self) -> &Arc<HydrusDB> {
        &self.db
    }

    /// `get_files_dir()`
    pub fn get_files_dir(&self) -> &Path {
        self.files.files_dir()
    }

    pub fn table_exists(&self, table_name: &str) -> HydrusResult<bool> {
        self.db.db_base()?.table_exists(table_name)
    }

    /// `_get_service_ids( limited_types = HC.ALL_SERVICES )`
    pub fn get_service_ids(
        &self,
        limited_types: Option<&[ServiceType]>,
    ) -> HydrusResult<Vec<i64>> {
        let services = self.get_services(limited_types)?;

        let mut ids = Vec::with_capacity(services.len());

        for service in services {
            ids.push(self.get_service_id(service.get_service_key())?);
        }

        Ok(ids)
    }

    /// `_get_service_keys( limited_types = HC.ALL_SERVICES )`
    pub fn get_service_keys(
        &self,
        limited_types: Option<&[ServiceType]>,
    ) -> HydrusResult<Vec<Vec<u8>>> {
        Ok(self
            .get_services(limited_types)?
            .into_iter()
            .map(|service| service.get_service_key().to_vec())
            .collect())
    }

    /// `_get_services( limited_types = HC.ALL_SERVICES )`
    pub fn get_services(
        &self,
        limited_types: Option<&[ServiceType]>,
    ) -> HydrusResult<Vec<ServerService>> {
        let db = self.db.db_base()?;

        let rows = db.query_all(
            "SELECT service_key, service_type, name, port, dictionary_string FROM services ORDER BY service_id;",
            &[],
            |row| {
                Ok((
                    row.get::<_, Vec<u8>>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, String>(4)?,
                ))
            },
        )?;

        let mut services = Vec::with_capacity(rows.len());

        for (service_key, service_type, name, port, dictionary_string) in rows {
            let service_type = u16::try_from(service_type)
                .ok()
                .and_then(ServiceType::try_from_int)
                .ok_or_else(|| {
                    HydrusException::new(
                        ErrorKind::Serialisation,
                        format!("Unknown server service type {service_type}."),
                    )
                })?;

            if let Some(limited_types) = limited_types
                && !limited_types.contains(&service_type)
            {
                continue;
            }

            let info = json!([
                hex_encode(&service_key),
                service_type.as_int(),
                name,
                port,
                dictionary_string,
            ]);

            services.push(generate_service_from_serialisable_tuple(&info)?);
        }

        Ok(services)
    }

    /// `_get_service_id( service_key )`
    pub fn get_service_id(&self, service_key: &[u8]) -> HydrusResult<i64> {
        let db = self.db.db_base()?;

        db.query_one(
            "SELECT service_id FROM services WHERE service_key = ?;",
            &[&service_key],
            |row| row.get::<_, i64>(0),
        )?
        .ok_or_else(|| {
            HydrusException::new(ErrorKind::DataMissing, "Service id error in database")
        })
    }

    /// `_get_service_key( service_id )`
    pub fn get_service_key(&self, service_id: i64) -> HydrusResult<Vec<u8>> {
        let db = self.db.db_base()?;

        db.query_one(
            "SELECT service_key FROM services WHERE service_id = ?;",
            &[&service_id],
            |row| row.get::<_, Vec<u8>>(0),
        )?
        .ok_or_else(|| {
            HydrusException::new(ErrorKind::DataMissing, "Service id error in database")
        })
    }

    /// `_get_service_name( service_id )`
    pub fn get_service_name(&self, service_id: i64) -> HydrusResult<String> {
        let db = self.db.db_base()?;

        db.query_one(
            "SELECT name FROM services WHERE service_id = ?;",
            &[&service_id],
            |row| row.get::<_, String>(0),
        )?
        .ok_or_else(|| {
            HydrusException::new(ErrorKind::DataMissing, "Service id error in database")
        })
    }
}
