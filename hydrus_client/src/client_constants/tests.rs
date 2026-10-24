
use super::*;

#[test]
fn sort_type_lookup_covers_every_system_sort() {
    for sort_type in SYSTEM_SORT_TYPES {
        assert_ne!(sort_type_string(sort_type), "unknown");
    }
}

#[test]
fn media_viewer_canvas_types_match_python() {
    assert_eq!(
        CANVAS_MEDIA_VIEWER_TYPES,
        [0, 2, 3],
        "Python set is {{0, 2, 3}}"
    );
}
