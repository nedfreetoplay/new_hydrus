use super::*;
use crate::media::MediaResult;
use crate::metadata::TagStatus;
use hydrus_core::files::hydrus_file_handling::FileInfo;
use hydrus_core::hydrus_constants::Mime;

fn media() -> MediaResult {
    let mut result = MediaResult::new(
        vec![1],
        FileInfo {
            size: 1,
            mime: Mime::Png,
            width: None,
            height: None,
            duration_ms: None,
            num_frames: None,
            has_audio: false,
            num_words: None,
        },
    );

    result.tags_manager.tags.add_tags(
        b"local tags",
        ["creator:someone", "blue eyes"],
        TagStatus::Current,
    );
    result.notes_manager.set_note("page", "hello");

    result
}

#[test]
fn gathers_tags_notes_and_namespaces() {
    let router = SingleFileMetadataRouter::new(
        vec![
            MetadataImporter::Tags {
                service_key: b"local tags".to_vec(),
            },
            MetadataImporter::Namespace {
                namespace: "creator".to_string(),
            },
            MetadataImporter::Notes,
        ],
        MetadataExporter::Json,
    );

    let gathered = router.gather(&media());

    assert_eq!(gathered["tags:6c6f63616c2074616773"].len(), 2);
    assert_eq!(gathered["namespace:creator"], vec!["someone"]);
    assert_eq!(gathered["notes"], vec!["page: hello"]);
}

#[test]
fn txt_export_is_one_value_per_line() {
    let router = SingleFileMetadataRouter::new(
        vec![MetadataImporter::Tags {
            service_key: b"local tags".to_vec(),
        }],
        MetadataExporter::Txt,
    );

    let text = router.export(&media());
    let mut lines: Vec<&str> = text.lines().collect();
    lines.sort();

    assert_eq!(lines, vec!["blue eyes", "creator:someone"]);
}

#[test]
fn tag_update_contains_matching_hashes() {
    let router = SingleFileMetadataRouter::new(
        vec![MetadataImporter::Tags {
            service_key: b"local tags".to_vec(),
        }],
        MetadataExporter::Txt,
    );

    let package = router.build_tag_update(b"dest tags".to_vec(), &media());

    assert_eq!(package.updates.len(), 1);
    assert_eq!(package.updates[0].hashes, vec![vec![1]]);
    assert_eq!(package.updates[0].tags.len(), 2);
}
