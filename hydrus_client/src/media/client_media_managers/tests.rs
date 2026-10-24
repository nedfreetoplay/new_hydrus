use super::*;
use hydrus_core::hydrus_constants::Mime;

fn file_info() -> FileInfo {
    FileInfo {
        size: 1024,
        mime: Mime::Png,
        width: Some(100),
        height: Some(200),
        duration_ms: None,
        num_frames: None,
        has_audio: false,
        num_words: None,
    }
}

#[test]
fn file_info_manager_exposes_basic_metadata() {
    let manager = FileInfoManager::new(vec![1, 2], file_info());

    assert_eq!(manager.get_size(), 1024);
    assert_eq!(manager.get_resolution(), Some((100, 200)));
    assert!(!manager.has_audio());
}

#[test]
fn locations_track_inbox_and_services() {
    let mut manager = LocationsManager::new(true);
    manager.set_current(b"my files", 100);

    assert!(manager.is_current(b"my files"));
    assert!(!manager.has_archive());
}

#[test]
fn notes_are_named_entries() {
    let mut manager = NotesManager::default();
    manager.set_note("page 1", "hello");

    assert!(manager.has_notes());
    assert_eq!(manager.get_note("page 1"), Some("hello"));
}
