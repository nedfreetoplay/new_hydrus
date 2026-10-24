use super::*;
use super::support::*;

use std::sync::atomic::AtomicBool;

use hydrus_core::networking::hydrus_network::generate_service;
use hydrus_core::networking::hydrus_server::REMOTE_DOMAIN;

#[tokio::test]
async fn file_upload_stores_a_repository_file() {
    let fixture = Fixture::new();

    let service = generate_service(
        vec![0x71; 32],
        ServiceType::FileRepository,
        "upload repository",
        0,
        None,
    )
    .unwrap();
    let (service_id, access_key) = fixture
        .db
        .add_service_with_admin_access_key(&service)
        .unwrap();
    let service_key = service.get_service_key().to_vec();

    let runtime = Arc::new(ServerResourceRuntime {
        db: Arc::clone(&fixture.db),
        session_manager: Arc::clone(&fixture.runtime.session_manager),
        service: Arc::new(service),
        busy: Arc::new(AtomicBool::new(false)),
        shutdown_requested: Arc::new(AtomicBool::new(false)),
    });

    let upload_path = std::env::temp_dir().join(format!(
        "hydrus-server-upload-{}.gif",
        std::process::id()
    ));
    std::fs::write(
        &upload_path,
        b"GIF89a\x01\x00\x01\x00\x80\x00\x00\x00\x00\x00\xff\xff\xff!\xf9\x04\x01\x00\x00\x00\x00,\x00\x00\x00\x00\x01\x00\x01\x00\x00\x02\x02D\x01\x00;",
    )
    .unwrap();

    let mut request = HydrusRequest::new("POST", "/file").with_client_ip("127.0.0.1");
    request
        .request_headers
        .set("Hydrus-Key", hex_encode(&access_key));
    request.parsed_request_args =
        hydrus_core::networking::hydrus_network_variable_handling::parse_file_arguments(
            &upload_path,
            false,
        )
        .unwrap();

    let hash = request.parsed_request_args.get_bytes("hash").unwrap();

    let route = ServerRoute {
        kind: ServerResourceKind::RestrictedRepositoryFile,
        domain: REMOTE_DOMAIN,
    };

    let response = render_resource(
        ServerResourceKind::RestrictedRepositoryFile,
        route,
        &runtime,
        &mut request,
    )
    .unwrap();

    assert_eq!(response.get_status_code(), 200);

    let (valid, mime) = fixture
        .db
        .repository_has_file(&service_key, &hash)
        .unwrap();

    assert!(valid);
    assert!(mime.is_some());

    let _ = fixture.db.get_service_id(&service_key).map(|_| service_id);
    let _ = std::fs::remove_file(upload_path);
}
