#[allow(dead_code)] // Lifecycle entry points are consumed by deployment tooling and tests.
impl Controller {
    /// `Controller( db_dir, logger )` for the filesystem layer.
    pub fn new(db_dir: impl Into<PathBuf>) -> Self {
        let db_dir = db_dir.into();
        let files_dir = db_dir.join("server_files");

        Self {
            db_dir,
            files_dir,
            db: OnceLock::new(),
            session_manager: OnceLock::new(),
            busy: Arc::new(AtomicBool::new(false)),
            shutdown_requested: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn db_dir(&self) -> &Path {
        &self.db_dir
    }

    pub fn files(&self) -> ServerFiles {
        ServerFiles::new(&self.files_dir)
    }

    /// Create the `server_files` root and its prefix directories.
    pub fn prepare_files_dir(&self) -> HydrusResult<()> {
        self.files().prepare()
    }

    /// Open (or return) the server database.
    pub fn open_database(self: &Arc<Self>) -> HydrusResult<Arc<ServerDB>> {
        if let Some(db) = self.db.get() {
            return Ok(Arc::clone(db));
        }

        let db_controller: Arc<dyn DBController> = Arc::new(ServerDBController {
            controller: Arc::downgrade(self),
        });

        let db = ServerDB::new(db_controller, &self.db_dir, "server")?;

        let _ = self.db.set(Arc::clone(&db));

        Ok(db)
    }

    /// Open the session manager and load existing sessions from the database.
    pub fn open_session_manager(
        self: &Arc<Self>,
    ) -> HydrusResult<Arc<HydrusSessionManagerServer>> {
        if let Some(manager) = self.session_manager.get() {
            return Ok(Arc::clone(manager));
        }

        let db = self.open_database()?;
        let session_controller = Arc::new(ServerSessionController::new(db));
        let manager = HydrusSessionManagerServer::new(session_controller)?;

        manager.refresh_all_accounts(None)?;

        let _ = self.session_manager.set(Arc::clone(&manager));

        Ok(manager)
    }

    #[allow(dead_code)]
    pub fn busy(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.busy)
    }

    #[allow(dead_code)]
    pub fn shutdown_requested(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.shutdown_requested)
    }

    /// Build the runtime object a service's resource handlers need.
    pub fn resource_runtime(
        self: &Arc<Self>,
        service_key: &[u8],
    ) -> HydrusResult<ServerResourceRuntime> {
        let db = self.open_database()?;
        let session_manager = self.open_session_manager()?;
        let service = db
            .get_services(None)?
            .into_iter()
            .find(|service| service.get_service_key() == service_key)
            .ok_or_else(|| {
                HydrusException::new(
                    ErrorKind::DataMissing,
                    "Could not find the requested server service.",
                )
            })?;

        Ok(ServerResourceRuntime {
            db,
            session_manager,
            service: Arc::new(service),
            busy: Arc::clone(&self.busy),
            shutdown_requested: Arc::clone(&self.shutdown_requested),
        })
    }
}
