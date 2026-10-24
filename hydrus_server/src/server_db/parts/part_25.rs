impl ServerDB {


    /// `_add_session( session_key, service_key, account_key, expires )`
    pub fn add_session(
        &self,
        session_key: &[u8],
        service_key: &[u8],
        account_key: &[u8],
        expires: i64,
    ) -> HydrusResult<()> {
        let service_id = self.get_service_id(service_key)?;
        let account_id = self.get_account_id(account_key)?;
        let db = self.db.db_base()?;

        db.execute(
            "INSERT INTO sessions ( session_key, service_id, account_id, expires ) VALUES ( ?, ?, ?, ? );",
            &[&session_key, &service_id, &account_id, &expires],
        )?;

        Ok(())
    }

    /// `_get_sessions( service_key = None )`
    pub fn get_sessions(
        &self,
        service_key: Option<&[u8]>,
    ) -> HydrusResult<Vec<ServerSessionRow>> {
        let db = self.db.db_base()?;
        let now = hydrus_time::get_now();

        db.execute("DELETE FROM sessions WHERE ? > expires;", &[&now])?;

        let rows = match service_key {
            Some(service_key) => {
                let service_id = self.get_service_id(service_key)?;

                db.query_all(
                    "SELECT session_key, service_id, account_id, expires FROM sessions WHERE service_id = ?;",
                    &[&service_id],
                    |row| {
                        Ok((
                            row.get::<_, Vec<u8>>(0)?,
                            row.get::<_, i64>(1)?,
                            row.get::<_, i64>(2)?,
                            row.get::<_, i64>(3)?,
                        ))
                    },
                )?
            }
            None => db.query_all(
                "SELECT session_key, service_id, account_id, expires FROM sessions;",
                &[],
                |row| {
                    Ok((
                        row.get::<_, Vec<u8>>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, i64>(2)?,
                        row.get::<_, i64>(3)?,
                    ))
                },
            )?,
        };

        let mut service_ids_to_keys: std::collections::HashMap<i64, Vec<u8>> =
            std::collections::HashMap::new();
        let mut account_ids_to_accounts: std::collections::HashMap<i64, Arc<NetworkAccount>> =
            std::collections::HashMap::new();
        let mut account_ids_to_hashed_access_keys: std::collections::HashMap<i64, Vec<u8>> =
            std::collections::HashMap::new();
        let mut sessions = Vec::with_capacity(rows.len());

        for (session_key, service_id, account_id, expires) in rows {
            let service_key = match service_ids_to_keys.get(&service_id) {
                Some(service_key) => service_key.clone(),
                None => {
                    let service_key = self.get_service_key(service_id)?;
                    service_ids_to_keys.insert(service_id, service_key.clone());
                    service_key
                }
            };

            let account = match account_ids_to_accounts.get(&account_id) {
                Some(account) => account.clone(),
                None => {
                    let account = Arc::new(self.get_account(service_id, account_id)?);
                    account_ids_to_accounts.insert(account_id, Arc::clone(&account));
                    account
                }
            };

            let hashed_access_key = match account_ids_to_hashed_access_keys.get(&account_id) {
                Some(hashed_access_key) => hashed_access_key.clone(),
                None => {
                    let hashed_access_key = db
                        .query_one(
                            "SELECT hashed_access_key FROM accounts WHERE account_id = ?;",
                            &[&account_id],
                            |row| row.get::<_, Vec<u8>>(0),
                        )?
                        .ok_or_else(|| {
                            HydrusException::new(
                                ErrorKind::DataMissing,
                                "Could not find the session account's access key.",
                            )
                        })?;
                    account_ids_to_hashed_access_keys
                        .insert(account_id, hashed_access_key.clone());
                    hashed_access_key
                }
            };

            sessions.push(ServerSessionRow {
                session_key,
                service_key,
                account,
                hashed_access_key,
                expires,
            });
        }

        Ok(sessions)
    }

    /// Insert a service, create its account setup, and return the admin access
    /// key. Python's `_add_service` performs the same sequence and returns the
    /// key from the forced `init` registration token for the admin service.
    pub fn add_service_with_admin_access_key(
        &self,
        service: &ServerService,
    ) -> HydrusResult<(i64, Vec<u8>)> {
        add_service_full(&self.db.db_base()?, service)
    }

    /// Insert a service and return its database id.
    pub fn add_service(&self, service: &ServerService) -> HydrusResult<i64> {
        self.add_service_with_admin_access_key(service)
            .map(|(service_id, _)| service_id)
    }
}
/// Wrapper that gives the core session manager the narrow `Account` interface
/// without changing the network `Account` type.
pub struct ServerSessionAccount {
    account: Arc<NetworkAccount>,
}

impl SessionAccountTrait for ServerSessionAccount {
    fn get_account_key(&self) -> Vec<u8> {
        self.account.get_account_key()
    }

    fn is_dirty(&self) -> bool {
        self.account.is_dirty()
    }
}

/// `SessionController` implementation backed by `ServerDB`.
pub struct ServerSessionController {
    db: Arc<ServerDB>,
}

impl ServerSessionController {
    pub fn new(db: Arc<ServerDB>) -> Self {
        Self { db }
    }
}
