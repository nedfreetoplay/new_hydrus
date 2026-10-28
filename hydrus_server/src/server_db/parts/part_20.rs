impl ServerDB {


    /// `_backup`
    pub fn backup(&self) -> HydrusResult<()> {
        let backup_dir = self.db_dir.join("server_backup");
        hydrus_core::hydrus_paths::make_sure_directory_exists(&backup_dir)?;

        for filename in [
            "server.db",
            "server.master.db",
            "server.mappings.db",
            "server.temp.db",
            "server.crt",
            "server.key",
        ] {
            let source = self.db_dir.join(filename);
            if source.exists() {
                hydrus_core::hydrus_paths::mirror_file(
                    &source,
                    &backup_dir.join(filename),
                )?;
            }
        }

        hydrus_core::hydrus_paths::mirror_tree(
            self.get_files_dir(),
            &backup_dir.join("server_files"),
            None,
            None,
        )?;

        Ok(())
    }

    /// `_vacuum`
    ///
    /// A live connection cannot be safely vacuumed. The core queue is committed
    /// first; the on-disk maintenance itself is performed by the database
    /// shutdown path.
    pub fn vacuum(&self) -> HydrusResult<()> {
        self.db.force_a_commit()
    }

    /// `_analyze`
    pub fn analyze(&self, _stop_time: Option<i64>) -> HydrusResult<()> {
        self.db.force_a_commit()
    }

    /// Save a service's name, port and serialisable dictionary back to the
    /// services table.
    pub fn save_service(&self, service: &ServerService) -> HydrusResult<()> {
        let (service_key, _, name, port, dictionary) = service.to_tuple()?;

        self.db.db_base()?.execute(
            "UPDATE services SET name = ?, port = ?, dictionary_string = ?
             WHERE service_key = ?;",
            &[&name, &i64::from(port), &dictionary.dump_to_string(), &service_key],
        )?;

        service.set_clean();

        Ok(())
    }

    /// `_save_services`
    pub fn save_services(&self, services: &[ServerService]) -> HydrusResult<()> {
        for service in services {
            self.save_service(service)?;
        }

        Ok(())
    }

    /// `_modify_account_types`'s supporting option mutation.
    pub fn set_service_nullification_period(
        &self,
        service_key: &[u8],
        period: i64,
    ) -> HydrusResult<()> {
        let service = self
            .get_services(None)?
            .into_iter()
            .find(|service| service.get_service_key() == service_key)
            .ok_or_else(|| {
                HydrusException::new(ErrorKind::DataMissing, "Service not found.")
            })?;

        service.set_nullification_period(period);
        self.save_service(&service)
    }

    /// `_modify_account_types`'s supporting option mutation.
    pub fn set_service_update_period(
        &self,
        service_key: &[u8],
        period: i64,
    ) -> HydrusResult<()> {
        let service = self
            .get_services(None)?
            .into_iter()
            .find(|service| service.get_service_key() == service_key)
            .ok_or_else(|| {
                HydrusException::new(ErrorKind::DataMissing, "Service not found.")
            })?;

        service.set_update_period(period)?;
        self.save_service(&service)
    }

    /// `_add_account_type( service_id, account_type )`
    pub fn add_account_type(
        &self,
        service_id: i64,
        account_type: &AccountType,
    ) -> HydrusResult<i64> {
        add_account_type(&self.db.db_base()?, service_id, account_type)
    }

    /// `_get_account_type( service_id, account_type_id )`
    pub fn get_account_type(
        &self,
        service_id: i64,
        account_type_id: i64,
    ) -> HydrusResult<Arc<AccountType>> {
        get_account_type(&self.db.db_base()?, service_id, account_type_id)
    }

    /// `_get_account_types( service_key, account )`
    pub fn get_account_types(&self, service_key: &[u8]) -> HydrusResult<Vec<Arc<AccountType>>> {
        let service_id = self.get_service_id(service_key)?;

        get_account_types(&self.db.db_base()?, service_id)
    }

    /// `_get_account_type_id( service_id, account_type_key )`
    pub fn get_account_type_id(
        &self,
        service_id: i64,
        account_type_key: &[u8],
    ) -> HydrusResult<i64> {
        get_account_type_id(&self.db.db_base()?, service_id, account_type_key)
    }

    /// `_generate_registration_keys( service_id, num, account_type_id, expires, force_registration_key = None )`
    pub fn generate_registration_keys(
        &self,
        service_id: i64,
        num: usize,
        account_type_id: i64,
        expires: Option<i64>,
        force_registration_key: Option<&[u8]>,
    ) -> HydrusResult<Vec<Vec<u8>>> {
        generate_registration_keys(
            &self.db.db_base()?,
            service_id,
            num,
            account_type_id,
            expires,
            force_registration_key,
        )
    }

    /// `_get_access_key( service_key, registration_key )`
    pub fn get_access_key(
        &self,
        service_key: &[u8],
        registration_key: &[u8],
    ) -> HydrusResult<Vec<u8>> {
        let service_id = self.get_service_id(service_key)?;

        get_access_key(&self.db.db_base()?, service_id, registration_key)
    }

    /// `_get_account_key_from_access_key( service_key, access_key )`
    pub fn get_account_key_from_access_key(
        &self,
        service_key: &[u8],
        access_key: &[u8],
    ) -> HydrusResult<Vec<u8>> {
        let service_id = self.get_service_id(service_key)?;

        get_account_key_from_access_key(&self.db.db_base()?, service_id, access_key)
    }
}
