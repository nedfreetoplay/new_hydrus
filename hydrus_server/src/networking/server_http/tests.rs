use super::*;

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

mod range;

static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

struct TestDir {
    path: PathBuf,
}

impl TestDir {
    fn new() -> Self {
        let suffix = format!(
            "{}_{}",
            std::process::id(),
            TEST_COUNTER.fetch_add(1, Ordering::Relaxed)
        );

        let path = std::env::temp_dir()
            .join("hydrus-server-http-tests")
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

#[tokio::test]
async fn tls_listener_serves_busy_and_shutdown() {
    hydrus_core::initialise();

    let dir = TestDir::new();
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
    let runtime = Arc::new(controller.resource_runtime(&admin_key).unwrap());
    let (cert_path, key_path) = db.db().get_ssl_paths().unwrap();
    let tls_settings = generate_ssl_context_settings(&cert_path, &key_path).unwrap();
    let acceptor = Arc::new(build_tls_acceptor(&tls_settings).unwrap());

    let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let context = Arc::new(ServiceContext {
        runtime: Arc::clone(&runtime),
    });
    let handle = tokio::spawn(serve_listener(listener, context, acceptor));

    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .no_proxy()
        .build()
        .unwrap();
    let base = format!("https://127.0.0.1:{port}");

    let response = client.get(format!("{base}/busy")).send().await.unwrap();

    assert_eq!(response.status(), 200);
    assert_eq!(response.text().await.unwrap(), "0");

    let response = client
        .post(format!("{base}/shutdown"))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);
    assert!(runtime.shutdown_requested.load(Ordering::Relaxed));

    handle.abort();
}

#[test]
fn query_parser_handles_repeated_and_percent_encoded_values() {
    let args = parse_query_args(Some("hash=01ab&hash=02cd&reason=blue%20sky")).unwrap();

    assert_eq!(args.get_bytes("hash").unwrap(), vec![0x01, 0xAB]);
    assert_eq!(args.get_string("reason").unwrap(), "blue sky");
}

#[test]
fn cookie_formatter_matches_the_http_cookie_shape() {
    let cookie = ResponseCookie::new("session_key", "abcd")
        .with_option("max_age", "100")
        .with_option("path", "/");

    assert_eq!(
        format_cookie(&cookie),
        "session_key=abcd; max_age=100; path=/"
    );
}

#[tokio::test]
async fn tls_listener_handles_concurrent_requests() {
    hydrus_core::initialise();

    let dir = TestDir::new();
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
    let runtime = Arc::new(controller.resource_runtime(&admin_key).unwrap());
    let (cert_path, key_path) = db.db().get_ssl_paths().unwrap();
    let tls_settings = generate_ssl_context_settings(&cert_path, &key_path).unwrap();
    let acceptor = Arc::new(build_tls_acceptor(&tls_settings).unwrap());

    let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let context = Arc::new(ServiceContext {
        runtime: Arc::clone(&runtime),
    });
    let handle = tokio::spawn(serve_listener(listener, context, acceptor));

    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .no_proxy()
        .build()
        .unwrap();
    let base = format!("https://127.0.0.1:{port}");

    let mut tasks = tokio::task::JoinSet::new();

    for _ in 0..8 {
        let client = client.clone();
        let url = format!("{base}/busy");

        tasks.spawn(async move {
            let response = client.get(url).send().await.unwrap();

            (response.status().as_u16(), response.text().await.unwrap())
        });
    }

    while let Some(result) = tasks.join_next().await {
        let (status, body) = result.unwrap();

        assert_eq!(status, 200);
        assert_eq!(body, "0");
    }

    handle.abort();
}
