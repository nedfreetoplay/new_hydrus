/// One service listener and the runtime state shared by its requests.
struct ServiceContext {
    runtime: Arc<ServerResourceRuntime>,
}

impl ServiceContext {
    fn service(&self) -> &Arc<ServerService> {
        &self.runtime.service
    }

    fn service_name(&self) -> String {
        self.service().get_name()
    }
}

/// Adapter from the server's `ServerService` to the core resource contract.
struct ServiceAdapter {
    service: Arc<ServerService>,
}

impl HydrusResourceService for ServiceAdapter {
    fn get_name(&self) -> String {
        self.service.get_name()
    }

    fn get_service_type(&self) -> ServiceType {
        self.service.get_service_type()
    }

    fn get_service_key(&self) -> Vec<u8> {
        self.service.get_service_key().to_vec()
    }

    fn allows_non_local_connections(&self) -> bool {
        self.service.allows_non_local_connections()
    }

    fn supports_cors(&self) -> bool {
        self.service.supports_cors()
    }

    fn use_normie_eris(&self) -> bool {
        self.service.use_normie_eris()
    }

    fn logs_requests(&self) -> bool {
        self.service.logs_requests()
    }

    fn report_data_used(&self, num_bytes: i64) {
        self.service.report_data_used(num_bytes);
    }

    fn report_request_used(&self) {
        self.service.report_request_used();
    }
}
