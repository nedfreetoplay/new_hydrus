//! Availability / visibility predicates for WS-15.3.2b (R2: pure, tested).
//! Inputs are primitives/enums/slices; outputs are `bool`/decisions.
//! No Qt/FS/network/DB access.

/// #6: `max_pending` enabled unless `fully automatic`.
pub fn max_pending_enabled(operation: &str) -> bool {
    operation != "fully automatic"
}

/// #8: custom-action dialog: `better_side` combo enabled for `better` action.
pub fn custom_better_side_enabled(action: &str) -> bool {
    action == "better"
}

/// #8: `same` merge tab enabled for `same` action.
pub fn custom_same_tab_enabled(action: &str) -> bool {
    action == "same"
}

/// #8: `better` merge tab enabled for `better` action.
pub fn custom_better_tab_enabled(action: &str) -> bool {
    custom_better_side_enabled(action)
}

/// #9: permission checklist enabled unless `permits_everything`.
pub fn permission_list_enabled(permits_everything: bool) -> bool {
    !permits_everything
}

/// #10: comparator kind panels.
pub fn comparator_one_file_visible(kind: &str) -> bool {
    kind == "one_file"
}
pub fn comparator_relative_visible(kind: &str) -> bool {
    kind == "relative"
}
pub fn comparator_filetype_visible(kind: &str) -> bool {
    kind == "filetype"
}

/// #25/#26: manager buttons. Edit needs exactly one row selected.
pub fn manager_edit_enabled(selected: i64) -> bool {
    selected == 1
}
/// Delete needs at least one row selected.
pub fn manager_delete_enabled(selected: i64) -> bool {
    selected >= 1
}

/// #28: `no limit` zeroes the stored file limit.
pub fn file_limit_value(unlimited: bool, value: i64) -> i64 {
    if unlimited {
        0
    } else {
        value
    }
}

/// #29: file-limit spinbox enabled unless `unlimited`.
pub fn file_limit_enabled(unlimited: bool) -> bool {
    !unlimited
}

/// Downloader page kinds for #30/#31 visibility rows.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DownloaderKind {
    Urls,
    Gallery,
    Watcher,
    Simple,
    Unknown,
}

pub fn downloader_kind(kind: &str) -> DownloaderKind {
    match kind {
        "urls" => DownloaderKind::Urls,
        "gallery" => DownloaderKind::Gallery,
        "watcher" => DownloaderKind::Watcher,
        "simple" => DownloaderKind::Simple,
        _ => DownloaderKind::Unknown,
    }
}

/// #30: per-row visibility of the downloader sidebar form.
pub fn downloader_input_row_visible(kind: &str) -> bool {
    matches!(
        downloader_kind(kind),
        DownloaderKind::Urls | DownloaderKind::Simple
    )
}
pub fn downloader_query_row_visible(kind: &str) -> bool {
    downloader_kind(kind) == DownloaderKind::Gallery
}
pub fn downloader_url_row_visible(kind: &str) -> bool {
    downloader_kind(kind) == DownloaderKind::Watcher
}
pub fn downloader_site_row_visible(kind: &str) -> bool {
    matches!(
        downloader_kind(kind),
        DownloaderKind::Watcher | DownloaderKind::Simple
    )
}
pub fn downloader_subject_row_visible(kind: &str) -> bool {
    downloader_kind(kind) == DownloaderKind::Watcher
}
pub fn downloader_period_row_visible(kind: &str) -> bool {
    downloader_kind(kind) == DownloaderKind::Watcher
}
pub fn downloader_file_limit_row_visible(kind: &str) -> bool {
    downloader_kind(kind) == DownloaderKind::Gallery
}
pub fn downloader_gug_button_visible(kind: &str) -> bool {
    downloader_kind(kind) == DownloaderKind::Gallery
}
/// #31: highlight box visible for gallery/watcher only.
pub fn downloader_highlight_box_visible(kind: &str) -> bool {
    matches!(
        downloader_kind(kind),
        DownloaderKind::Gallery | DownloaderKind::Watcher
    )
}
pub fn downloader_start_button_visible(kind: &str) -> bool {
    downloader_kind(kind) != DownloaderKind::Gallery
}
pub fn downloader_start_button_enabled(running: bool) -> bool {
    !running
}
pub fn downloader_auto_button_visible(kind: &str) -> bool {
    downloader_kind(kind) == DownloaderKind::Watcher
}
pub fn downloader_highlight_subject_visible(page_url_empty: bool) -> bool {
    !page_url_empty
}

/// #41: second search row visible for `different_searches`.
pub fn duplicates_second_search_visible(dupe_search_type: &str) -> bool {
    dupe_search_type == "different_searches"
}

/// #42: distance spinbox disabled when pixel dupes are `required`.
pub fn duplicates_search_distance_enabled(pixel_dupes: &str) -> bool {
    pixel_dupes != "required"
}

/// #43: sort-order combo hidden for `random` sort.
pub fn duplicates_sort_ascending_visible(sort_type: &str) -> bool {
    sort_type != "random"
}

/// #44/#46: period button (and popup checkbox) enabled by `check regularly`.
pub fn period_button_enabled(check_regularly: bool) -> bool {
    check_regularly
}

/// #45: move destination widgets enabled only for `move` action.
pub fn move_destination_enabled(action: &str) -> bool {
    action == "move"
}

/// #47: `delete from client` disabled (and unchecked) for `synchronise`.
pub fn export_delete_from_client_enabled(export_type: &str) -> bool {
    export_type != "synchronise"
}

/// #48: `default:` containers cannot be renamed/deleted.
pub fn favourite_container_editable(key: &str) -> bool {
    !key.starts_with("default:")
}

/// #49: import/tag buttons need at least one good file.
pub fn local_import_can_import(good_count: i64) -> bool {
    good_count > 0
}

/// #50: `add tagging` needs at least one local tag service.
pub fn tagging_add_enabled(local_tag_service_count: i64) -> bool {
    local_tag_service_count > 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn max_pending() {
        assert!(max_pending_enabled("semi-automatic"));
        assert!(max_pending_enabled(""));
        assert!(!max_pending_enabled("fully automatic"));
    }

    #[test]
    fn custom_action_tabs() {
        assert!(custom_better_side_enabled("better"));
        assert!(!custom_better_side_enabled("same"));
        assert!(!custom_better_side_enabled(""));
        assert!(custom_same_tab_enabled("same"));
        assert!(!custom_same_tab_enabled("better"));
        assert!(custom_better_tab_enabled("better"));
        assert!(!custom_better_tab_enabled("same"));
    }

    #[test]
    fn permission_list() {
        assert!(permission_list_enabled(false));
        assert!(!permission_list_enabled(true));
    }

    #[test]
    fn comparator_panels() {
        assert!(comparator_one_file_visible("one_file"));
        assert!(!comparator_one_file_visible("relative"));
        assert!(comparator_relative_visible("relative"));
        assert!(!comparator_relative_visible("filetype"));
        assert!(comparator_filetype_visible("filetype"));
        assert!(!comparator_filetype_visible("one_file"));
        assert!(!comparator_one_file_visible(""));
    }

    #[test]
    fn manager_buttons() {
        assert!(!manager_edit_enabled(0));
        assert!(manager_edit_enabled(1));
        assert!(!manager_edit_enabled(2));
        assert!(!manager_edit_enabled(-1));
        assert!(!manager_delete_enabled(0));
        assert!(manager_delete_enabled(1));
        assert!(manager_delete_enabled(5));
        assert!(!manager_delete_enabled(-1));
    }

    #[test]
    fn file_limit() {
        assert_eq!(file_limit_value(true, 2000), 0);
        assert_eq!(file_limit_value(false, 2000), 2000);
        assert_eq!(file_limit_value(false, 0), 0);
        assert!(file_limit_enabled(false));
        assert!(!file_limit_enabled(true));
    }

    #[test]
    fn downloader_rows() {
        assert!(downloader_input_row_visible("urls"));
        assert!(downloader_input_row_visible("simple"));
        assert!(!downloader_input_row_visible("gallery"));
        assert!(!downloader_input_row_visible("watcher"));
        assert!(!downloader_input_row_visible("bogus"));
        assert!(downloader_query_row_visible("gallery"));
        assert!(!downloader_query_row_visible("watcher"));
        assert!(downloader_url_row_visible("watcher"));
        assert!(!downloader_url_row_visible("gallery"));
        assert!(downloader_site_row_visible("watcher"));
        assert!(downloader_site_row_visible("simple"));
        assert!(!downloader_site_row_visible("urls"));
        assert!(downloader_subject_row_visible("watcher"));
        assert!(!downloader_subject_row_visible("simple"));
        assert!(downloader_period_row_visible("watcher"));
        assert!(!downloader_period_row_visible("gallery"));
        assert!(downloader_file_limit_row_visible("gallery"));
        assert!(!downloader_file_limit_row_visible("watcher"));
        assert!(downloader_gug_button_visible("gallery"));
        assert!(!downloader_gug_button_visible("simple"));
        assert!(downloader_highlight_box_visible("gallery"));
        assert!(downloader_highlight_box_visible("watcher"));
        assert!(!downloader_highlight_box_visible("urls"));
        assert!(!downloader_highlight_box_visible("simple"));
        assert!(!downloader_highlight_box_visible(""));
        assert!(!downloader_start_button_visible("gallery"));
        assert!(downloader_start_button_visible("watcher"));
        assert!(downloader_start_button_visible("urls"));
        assert!(!downloader_start_button_enabled(true));
        assert!(downloader_start_button_enabled(false));
        assert!(downloader_auto_button_visible("watcher"));
        assert!(!downloader_auto_button_visible("gallery"));
        assert!(!downloader_highlight_subject_visible(true));
        assert!(downloader_highlight_subject_visible(false));
    }

    #[test]
    fn duplicates_controls() {
        assert!(duplicates_second_search_visible("different_searches"));
        assert!(!duplicates_second_search_visible("both_files"));
        assert!(!duplicates_second_search_visible(""));
        assert!(!duplicates_search_distance_enabled("required"));
        assert!(duplicates_search_distance_enabled("allowed"));
        assert!(duplicates_search_distance_enabled("excluded"));
        assert!(duplicates_search_distance_enabled(""));
        assert!(!duplicates_sort_ascending_visible("random"));
        assert!(duplicates_sort_ascending_visible("largest_first"));
        assert!(duplicates_sort_ascending_visible(""));
    }

    #[test]
    fn folder_editors() {
        assert!(period_button_enabled(true));
        assert!(!period_button_enabled(false));
        assert!(move_destination_enabled("move"));
        assert!(!move_destination_enabled("delete"));
        assert!(!move_destination_enabled(""));
        assert!(!export_delete_from_client_enabled("synchronise"));
        assert!(export_delete_from_client_enabled("regular"));
        assert!(export_delete_from_client_enabled(""));
        assert!(!favourite_container_editable("default:global"));
        assert!(!favourite_container_editable("default:"));
        assert!(favourite_container_editable("my preset"));
        assert!(favourite_container_editable(""));
        assert!(local_import_can_import(1));
        assert!(local_import_can_import(100));
        assert!(!local_import_can_import(0));
        assert!(!local_import_can_import(-1));
        assert!(tagging_add_enabled(1));
        assert!(!tagging_add_enabled(0));
        assert!(!tagging_add_enabled(-2));
    }
}
