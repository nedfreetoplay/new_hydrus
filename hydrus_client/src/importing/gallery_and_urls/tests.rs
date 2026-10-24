use super::*;

#[test]
fn import_queues_pause_and_finish() {
    let mut queue = ImportQueue::default();

    queue.pause_play();
    assert!(queue.is_paused());

    queue.pause_play();
    assert!(!queue.is_paused());

    queue.mark_finished();
    assert!(queue.is_finished());
}

#[test]
fn gallery_import_tracks_both_queues() {
    let mut gallery = GalleryImport::new("blue eyes", "example", 100);
    gallery.add_file_seed(FileSeed::url("https://example.com/a", 0));
    gallery.add_file_seed(FileSeed::url("https://example.com/a", 0));
    gallery.add_gallery_seed("https://example.com/page/2");

    assert_eq!(gallery.get_num_seeds(), 1);
    assert_eq!(gallery.get_num_gallery_seeds(), 1);
    assert_eq!(gallery.get_creation_time(), 100);

    gallery.file_queue.mark_finished();
    gallery.gallery_queue.mark_finished();

    assert!(gallery.files_finished());
    assert!(gallery.gallery_finished());
    assert_eq!(gallery.get_status(), &ImportStatus::Finished);
}

#[test]
fn simple_downloader_advances_jobs() {
    let mut import = SimpleDownloaderImport::new("all images");
    import.set_pending_jobs([
        "https://example.com/a".to_string(),
        "https://example.com/b".to_string(),
    ]);

    assert_eq!(import.get_num_pending_jobs(), 2);

    let job = import.advance_job().unwrap().clone();

    assert_eq!(job.url, "https://example.com/a");
    assert_eq!(import.get_num_pending_jobs(), 1);

    import.delay_job();
    assert_eq!(import.get_num_pending_jobs(), 2);
}

#[test]
fn deleting_a_job_removes_it() {
    let mut import = SimpleDownloaderImport::new("formula");
    import.pend_job("https://example.com/a");
    import.advance_job();

    import.delete_job();

    assert!(import.jobs.is_empty());
    assert_eq!(import.get_status(), ImportStatus::Finished);
}

struct MockTransport {
    body: Vec<u8>,
}

impl crate::networking::client_networking::HttpTransport for MockTransport {
    fn execute(
        &self,
        _request: &crate::networking::client_networking::NetworkRequest,
    ) -> HydrusResult<crate::networking::client_networking::NetworkResponse> {
        Ok(crate::networking::client_networking::NetworkResponse {
            status: 200,
            headers: BTreeMap::new(),
            body: self.body.clone(),
        })
    }
}

struct DummyFormula(Vec<String>);

impl ParseFormula for DummyFormula {
    fn parse(
        &self,
        _context: &ParsingContext,
        _text: &str,
        _collapse_newlines: bool,
    ) -> Vec<String> {
        self.0.clone()
    }

    fn name(&self) -> &str {
        "dummy"
    }
}

#[test]
fn gallery_pages_are_fetched_and_parsed() {
    let mut gallery = GalleryImport::new("blue eyes", "example", 0);
    gallery
        .initial_search_urls
        .push("https://example.com/gallery".to_string());

    let mut engine = NetworkEngine::new(MockTransport {
        body: b"<html>page</html>".to_vec(),
    });

    let (url, body) = gallery.run_next_gallery_page(&mut engine).unwrap();
    assert_eq!(url, "https://example.com/gallery");
    assert_eq!(body.unwrap(), "<html>page</html>");
    assert!(gallery.gallery_finished());

    let formula = DummyFormula(vec![
        "https://example.com/a.jpg".to_string(),
        "https://example.com/b.jpg".to_string(),
    ]);

    assert_eq!(gallery.apply_gallery_formula(&formula, "text", 10), 2);
    assert_eq!(gallery.get_num_seeds(), 2);
}

#[test]
fn simple_downloader_jobs_use_the_network() {
    let mut import = SimpleDownloaderImport::new("formula");
    import.pend_job("https://example.com/page");

    let mut engine = NetworkEngine::new(MockTransport {
        body: b"page body".to_vec(),
    });

    let (url, body) = import.run_next_job(&mut engine).unwrap();
    assert_eq!(url, "https://example.com/page");
    assert_eq!(body.unwrap(), "page body");
    assert_eq!(import.get_status(), ImportStatus::Finished);

    let formula = DummyFormula(vec!["https://example.com/file.png".to_string()]);
    assert_eq!(import.apply_downloader_formula(&formula, "text", 5), 1);
    assert_eq!(import.get_num_seeds(), 1);
}
