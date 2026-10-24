#[allow(dead_code)] // Lifecycle and maintenance entry points are consumed by deployment tooling.
impl Controller {
    /// Render one path for a service through its route tree.
    pub fn render_service_request(
        self: &Arc<Self>,
        service_key: &[u8],
        path: &str,
        request: &mut HydrusRequest,
    ) -> HydrusResult<ResponseContext> {
        let runtime = self.resource_runtime(service_key)?;
        let routes = ServerResourceTree::for_service_type(runtime.service.get_service_type())?;

        let route = routes.get_child(path).copied().ok_or_else(|| {
            HydrusException::new(
                ErrorKind::NotFound,
                format!("No server resource is registered at \"{path}\"."),
            )
        })?;

        render_resource(route.kind, route, &runtime, request)
    }

    /// The administration service sets this from /shutdown.
    #[allow(dead_code)]
    pub fn shutdown_from_server(&self) -> HydrusResult<()> {
        self.shutdown_requested.store(true, Ordering::Relaxed);

        Ok(())
    }

    /// `init_model`
    pub fn init_model(self: &Arc<Self>) -> HydrusResult<()> {
        self.open_database()?;
        self.open_session_manager()?;
        hydrus_process::record_running_start(&self.db_dir, "server");

        let service_controller: Arc<
            dyn hydrus_core::networking::hydrus_network::ServerServiceController,
        > = self.clone();
        hydrus_core::networking::hydrus_network::set_server_service_controller(Some(
            service_controller,
        ));

        Ok(())
    }

    /// `init_view`
    pub fn init_view(self: &Arc<Self>) -> HydrusResult<()> {
        self.init_model()
    }

    /// `run`
    pub async fn run(self: &Arc<Self>) -> HydrusResult<()> {
        self.init_model()?;

        let daemon_controller = Arc::clone(self);

        tokio::spawn(async move {
            let mut interval =
                tokio::time::interval(std::time::Duration::from_secs(60));

            loop {
                interval.tick().await;

                if daemon_controller.shutdown_requested.load(Ordering::Relaxed) {
                    break;
                }

                let controller = Arc::clone(&daemon_controller);

                let _ = tokio::task::spawn_blocking(move || {
                    let _ = controller.run_daemons();
                })
                .await;
            }
        });

        server_http::serve(Arc::clone(self)).await
    }

    /// `exit`
    pub fn exit(&self) {
        self.shutdown_requested.store(true, Ordering::Relaxed);
        hydrus_core::hydrus_data::clean_running_file(&self.db_dir, "server");
    }

    pub fn backup(self: &Arc<Self>) -> HydrusResult<()> {
        self.open_database()?.backup()
    }

    pub fn analyze(self: &Arc<Self>) -> HydrusResult<()> {
        self.open_database()?.analyze(None)
    }

    pub fn vacuum(self: &Arc<Self>) -> HydrusResult<()> {
        self.open_database()?.vacuum()
    }

    /// `set_services`
    pub fn set_services(&self, services: &[Arc<hydrus_core::networking::hydrus_network::ServerService>]) -> HydrusResult<()> {
        for service in services {
            if service.get_port() == 0 {
                return Err(HydrusException::new(
                    ErrorKind::BadRequest,
                    "A server service cannot use port 0.",
                ));
            }
        }

        Ok(())
    }

    /// `restart_services`
    pub fn restart_services(&self) -> HydrusResult<()> {
        Ok(())
    }

    /// `server_bandwidth_ok`
    pub fn server_bandwidth_ok(self: &Arc<Self>) -> HydrusResult<bool> {
        Ok(self
            .open_database()?
            .get_services(Some(&[hydrus_core::hydrus_constants::ServiceType::ServerAdmin]))?
            .first()
            .map(|service| service.server_bandwidth_ok())
            .unwrap_or(false))
    }

    /// Install this controller in the `ServerGlobals` slot.
    pub fn install(self: &Arc<Self>) {
        let controller: Arc<dyn ServerController> = self.clone();

        server_globals::set_server_controller(&controller);
    }
}
