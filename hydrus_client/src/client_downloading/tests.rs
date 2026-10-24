use super::*;

#[test]
fn gallery_identifier_round_trips() {
    let identifier = GalleryIdentifier::new(3, "page");
    let info = identifier.get_serialisable_info();
    let mut loaded = GalleryIdentifier::default();

    loaded.initialise_from_serialisable_info(&info).unwrap();

    assert_eq!(loaded, identifier);
    assert_eq!(identifier.to_string(), "unknown downloader");
}

#[test]
fn downloads_move_through_the_queue() {
    let mut manager = QuickDownloadManager::new();
    manager.download_files([vec![1], vec![2]]);

    let hash = manager.pop_next().unwrap();

    assert_eq!(hash, vec![1]);
    assert_eq!(manager.get_num_pending(), 1);
    assert_eq!(manager.get_num_active(), 1);

    manager.finish(&hash);

    assert!(manager.is_finished(&hash));
    assert_eq!(manager.get_num_finished(), 1);
}

#[test]
fn duplicate_requests_are_ignored() {
    let mut manager = QuickDownloadManager::new();
    manager.download_files([vec![1], vec![1]]);

    assert_eq!(manager.get_num_pending(), 1);

    let hash = manager.pop_next().unwrap();
    manager.download_files([hash.clone()]);

    assert_eq!(manager.get_num_pending(), 0);
}

#[test]
fn failed_downloads_are_tracked() {
    let mut manager = QuickDownloadManager::new();
    manager.download_files([vec![1]]);
    let hash = manager.pop_next().unwrap();

    manager.fail(&hash);

    assert!(manager.is_failed(&hash));
    assert!(!manager.is_finished(&hash));
    assert!(manager.is_idle());
}

struct MockTransport {
    body: Vec<u8>,
    fail: bool,
}

impl crate::networking::client_networking::HttpTransport for MockTransport {
    fn execute(
        &self,
        _request: &crate::networking::client_networking::NetworkRequest,
    ) -> HydrusResult<crate::networking::client_networking::NetworkResponse> {
        if self.fail {
            return Err(hydrus_core::hydrus_exceptions::unknown("mock failure"));
        }

        Ok(crate::networking::client_networking::NetworkResponse {
            status: 200,
            headers: BTreeMap::new(),
            body: self.body.clone(),
        })
    }
}

#[test]
fn queued_urls_download_through_network_engine() {
    let mut manager = QuickDownloadManager::new();
    manager.download_urls([(vec![1], "https://example.com/a".to_string())]);

    let mut engine = NetworkEngine::new(MockTransport {
        body: b"file bytes".to_vec(),
        fail: false,
    });

    let (hash, result) = manager.run_next_download(&mut engine).unwrap();

    assert_eq!(hash, vec![1]);
    assert_eq!(result.unwrap(), b"file bytes");
    assert!(manager.is_finished(&hash));
}

#[test]
fn download_errors_mark_the_hash_failed() {
    let mut manager = QuickDownloadManager::new();
    manager.download_urls([(vec![2], "https://example.com/b".to_string())]);

    let mut engine = NetworkEngine::new(MockTransport {
        body: Vec::new(),
        fail: true,
    });

    let (hash, result) = manager.run_next_download(&mut engine).unwrap();

    assert!(result.is_err());
    assert!(manager.is_failed(&hash));
}
