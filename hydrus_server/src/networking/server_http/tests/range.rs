use super::*;

use http_body_util::BodyExt;

#[tokio::test]
async fn file_responses_honour_range_headers() {
    hydrus_core::initialise();

    let dir = TestDir::new();
    let file_path = dir.path.join("range.bin");
    std::fs::write(&file_path, b"0123456789").unwrap();

    let controller = Arc::new(Controller::new(&dir.path));
    let db = controller.open_database().unwrap();
    let admin_key = db
        .get_services(None)
        .unwrap()
        .into_iter()
        .find(|service| service.get_service_type() == ServiceType::ServerAdmin)
        .unwrap()
        .get_service_key()
        .to_vec();
    let runtime = controller.resource_runtime(&admin_key).unwrap();

    let response_context = ResponseContext::new(
        200,
        hydrus_core::hydrus_constants::Mime::OctetStream,
        None,
        Some(file_path),
        Vec::new(),
        false,
        None,
    )
    .unwrap();

    let mut request = HydrusRequest::new("GET", "/file");
    request.request_headers.set("Range", "bytes=2-5");

    let response = response_from_context(&request, &response_context, &runtime.service)
        .await
        .unwrap();

    assert_eq!(response.status(), 206);

    let body = response.into_body().collect().await.unwrap().to_bytes();

    assert_eq!(&body[..], b"2345");
}

#[tokio::test]
async fn large_file_ranges_are_clipped_to_the_file_end() {
    hydrus_core::initialise();

    let dir = TestDir::new();
    let file_path = dir.path.join("large.bin");
    let data = vec![0x5A; 1024 * 1024];
    std::fs::write(&file_path, &data).unwrap();

    let controller = Arc::new(Controller::new(&dir.path));
    let db = controller.open_database().unwrap();
    let admin_key = db
        .get_services(None)
        .unwrap()
        .into_iter()
        .find(|service| service.get_service_type() == ServiceType::ServerAdmin)
        .unwrap()
        .get_service_key()
        .to_vec();
    let runtime = controller.resource_runtime(&admin_key).unwrap();

    let response_context = ResponseContext::new(
        200,
        hydrus_core::hydrus_constants::Mime::OctetStream,
        None,
        Some(file_path),
        Vec::new(),
        false,
        None,
    )
    .unwrap();

    let mut request = HydrusRequest::new("GET", "/file");
    request.request_headers.set("Range", "bytes=1000000-2000000");

    let response = response_from_context(&request, &response_context, &runtime.service)
        .await
        .unwrap();

    assert_eq!(response.status(), 206);

    let body = response.into_body().collect().await.unwrap().to_bytes();

    assert_eq!(body.len(), 1024 * 1024 - 1_000_000);
}
