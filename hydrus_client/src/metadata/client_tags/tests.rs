
use super::*;

#[test]
fn tags_are_grouped_by_service_and_status() {
    let mut tags = ServiceKeysToTags::new();
    tags.add_tags(
        b"local tags",
        ["blue eyes", "blonde hair"],
        TagStatus::Current,
    );
    tags.add_status(b"local tags", "deleted tag", TagStatus::Deleted);

    assert_eq!(
        tags.get_tags(b"local tags"),
        BTreeSet::from(["blue eyes", "blonde hair", "deleted tag"])
    );
    assert_eq!(
        tags.get_tags_with_status(b"local tags", TagStatus::Current),
        BTreeSet::from(["blue eyes", "blonde hair"])
    );
    assert_eq!(
        tags.get_status(b"local tags", "deleted tag"),
        Some(TagStatus::Deleted)
    );
}

#[test]
fn serialisation_round_trips() {
    let mut tags = ServiceKeysToTags::new();
    tags.add_status(b"local tags", "blue eyes", TagStatus::Current);

    let info = tags.get_serialisable_info();
    let mut loaded = ServiceKeysToTags::new();
    loaded.initialise_from_serialisable_info(&info).unwrap();

    assert_eq!(loaded, tags);
}

#[test]
fn removing_the_last_tag_removes_the_service() {
    let mut tags = ServiceKeysToTags::new();
    tags.add_status(b"local tags", "blue eyes", TagStatus::Current);
    tags.remove_tag(b"local tags", "blue eyes");

    assert!(tags.is_empty());
    assert!(tags.get_service_keys().is_empty());
}
