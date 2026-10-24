use super::*;

use std::path::PathBuf;
use std::sync::atomic::AtomicU64;
use hydrus_core::hydrus_db::NullDBController;
use hydrus_core::hydrus_sessions::HydrusSessionManagerServer;
use hydrus_core::networking::hydrus_network_variable_handling::parse_network_bytes_to_parsed_hydrus_args;
use hydrus_core::networking::hydrus_server::REMOTE_DOMAIN;
use crate::server_db::ServerSessionController;

static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

pub(super) struct TestDir {
    pub(super) path: PathBuf,
}

impl TestDir {
    pub(super) fn new() -> Self {
        let suffix = format!(
            "{}_{}",
            std::process::id(),
            TEST_COUNTER.fetch_add(1, Ordering::Relaxed)
        );

        let path = std::env::temp_dir()
            .join("hydrus-server-resource-tests")
            .join(suffix);

        std::fs::create_dir_all(&path).unwrap();

        Self { path }
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

pub(super) struct Fixture {
    _dir: TestDir,
    pub(super) db: Arc<ServerDB>,
    pub(super) runtime: Arc<ServerResourceRuntime>,
    pub(super) admin_key: Vec<u8>,
}

impl Fixture {
    pub(super) fn new() -> Self {
        hydrus_core::initialise();

        let dir = TestDir::new();
        let db = ServerDB::new(Arc::new(NullDBController), &dir.path, "server").unwrap();
        let session_manager = HydrusSessionManagerServer::new(Arc::new(
            ServerSessionController::new(Arc::clone(&db)),
        ))
        .unwrap();

        let service = db.get_services(None).unwrap().remove(0);
        let admin_key = service.get_service_key().to_vec();

        let runtime = Arc::new(ServerResourceRuntime {
            db: Arc::clone(&db),
            session_manager,
            service: Arc::new(service),
            busy: Arc::new(AtomicBool::new(false)),
            shutdown_requested: Arc::new(AtomicBool::new(false)),
        });

        Self {
            _dir: dir,
            db,
            runtime,
            admin_key,
        }
    }

    pub(super) fn render(
        &self,
        kind: ServerResourceKind,
        request: &mut HydrusRequest,
    ) -> HydrusResult<ResponseContext> {
        render_resource(
            kind,
            ServerRoute {
                kind,
                domain: REMOTE_DOMAIN,
            },
            &self.runtime,
            request,
        )
    }

    pub(super) fn request(&self, method: &str) -> HydrusRequest {
        HydrusRequest::new(method, "/").with_client_ip("127.0.0.1")
    }
}

pub(super) fn body_args(response: &ResponseContext) -> ParsedRequestArguments {
    parse_network_bytes_to_parsed_hydrus_args(
        response.get_body_bytes().expect("the response has a body"),
    )
    .expect("the response is a hydrus network dictionary")
}
