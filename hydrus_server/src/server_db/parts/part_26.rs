impl SessionController for ServerSessionController {

    fn read_sessions(&self, service_key: Option<&[u8]>) -> HydrusResult<Vec<ExistingSession>> {
        Ok(self
            .db
            .get_sessions(service_key)?
            .into_iter()
            .map(|session| ExistingSession {
                session_key: session.session_key,
                service_key: session.service_key,
                account: Arc::new(ServerSessionAccount {
                    account: Arc::clone(&session.account),
                }),
                hashed_access_key: session.hashed_access_key,
                expires: session.expires,
            })
            .collect())
    }

    fn read_account(
        &self,
        service_key: &[u8],
        account_key: &[u8],
    ) -> HydrusResult<Arc<dyn SessionAccountTrait>> {
        let account = Arc::new(
            self.db
                .get_account_from_account_key(service_key, account_key)?,
        );

        Ok(Arc::new(ServerSessionAccount { account }))
    }

    fn read_account_key_from_access_key(
        &self,
        service_key: &[u8],
        access_key: &[u8],
    ) -> HydrusResult<Vec<u8>> {
        self.db
            .get_account_key_from_access_key(service_key, access_key)
    }

    fn write_session(
        &self,
        session_key: &[u8],
        service_key: &[u8],
        account_key: &[u8],
        expires: i64,
    ) -> HydrusResult<()> {
        self.db
            .add_session(session_key, service_key, account_key, expires)
    }

    fn subscribe(&self, _subscriber: Arc<dyn Subscriber>, _method_name: &str, _topic: &str) {
        // Wired to the controller's pub/sub daemon with the controller layer.
    }
}

struct ServerDBHooks {
    files: ServerFiles,
}

impl DBHooks for ServerDBHooks {
    fn create_db(
        &self,
        db: &DBBase,
        _cursor_transaction_wrapper: &mut DBCursorTransactionWrapper,
    ) -> HydrusResult<()> {
        self.files.prepare()?;

        db.execute_batch(
            "
            CREATE TABLE services ( service_id INTEGER PRIMARY KEY, service_key BLOB_BYTES, service_type INTEGER, name TEXT, port INTEGER, dictionary_string TEXT );

            CREATE TABLE accounts ( account_id INTEGER PRIMARY KEY, service_id INTEGER, account_key BLOB_BYTES, hashed_access_key BLOB_BYTES, account_type_id INTEGER, created INTEGER, expires INTEGER, dictionary_string TEXT );
            CREATE UNIQUE INDEX accounts_account_key_index ON accounts ( account_key );
            CREATE UNIQUE INDEX accounts_hashed_access_key_index ON accounts ( hashed_access_key );

            CREATE TABLE account_scores ( service_id INTEGER, account_id INTEGER, score_type INTEGER, score INTEGER, PRIMARY KEY ( service_id, account_id, score_type ) );

            CREATE TABLE account_types ( account_type_id INTEGER PRIMARY KEY, service_id INTEGER, dump TEXT );

            CREATE TABLE analyze_timestamps ( name TEXT, timestamp INTEGER );

            CREATE TABLE deferred_physical_file_deletes ( master_hash_id INTEGER PRIMARY KEY );
            CREATE TABLE deferred_physical_thumbnail_deletes ( master_hash_id INTEGER PRIMARY KEY );

            CREATE TABLE files_info ( master_hash_id INTEGER PRIMARY KEY, size INTEGER, mime INTEGER, width INTEGER, height INTEGER, duration INTEGER, num_frames INTEGER, num_words INTEGER );

            CREATE TABLE reasons ( reason_id INTEGER PRIMARY KEY, reason TEXT );
            CREATE UNIQUE INDEX reasons_reason_index ON reasons ( reason );

            CREATE TABLE registration_keys ( registration_key BLOB_BYTES PRIMARY KEY, service_id INTEGER, account_type_id INTEGER, account_key BLOB_BYTES, access_key BLOB_BYTES UNIQUE, expires INTEGER );

            CREATE TABLE IF NOT EXISTS service_info ( service_id INTEGER, info_type INTEGER, info INTEGER, PRIMARY KEY ( service_id, info_type ) );

            CREATE TABLE sessions ( session_key BLOB_BYTES, service_id INTEGER, account_id INTEGER, expires INTEGER );

            CREATE TABLE version ( version INTEGER, year INTEGER, month INTEGER );

            CREATE TABLE IF NOT EXISTS external_master.hashes ( master_hash_id INTEGER PRIMARY KEY, hash BLOB_BYTES UNIQUE );
            CREATE TABLE IF NOT EXISTS external_master.tags ( master_tag_id INTEGER PRIMARY KEY, tag TEXT UNIQUE );
            ",
        )?;

        let now = hydrus_time::timestamp_to_datetime(
            hydrus_time::get_now() as f64,
            Timezone::Local,
        );

        db.execute(
            "INSERT INTO version ( version, year, month ) VALUES ( ?, ?, ? );",
            &[
                &(SOFTWARE_VERSION as i64),
                &i64::from(now.year()),
                &i64::from(now.month()),
            ],
        )?;

        let admin_service = generate_service(
            SERVER_ADMIN_KEY.to_vec(),
            ServiceType::ServerAdmin,
            "server admin",
            DEFAULT_SERVER_ADMIN_PORT,
            None,
        )?;

        add_service_full(db, &admin_service)?;

        Ok(())
    }

    fn update_db(
        &self,
        db: &DBBase,
        _cursor_transaction_wrapper: &mut DBCursorTransactionWrapper,
        version: i64,
    ) -> HydrusResult<()> {
        // The account/repository migrations will be added with those layers.
        // For now the version bump keeps `HydrusDB`'s update loop consistent.
        db.execute(
            "UPDATE version SET version = ?;",
            &[&(version + 1)],
        )?;

        Ok(())
    }
}
