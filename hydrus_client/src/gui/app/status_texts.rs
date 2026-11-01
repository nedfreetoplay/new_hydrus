//! Status and empty-state text for WS-15.3.2a (R2: pure, tested).
//! All functions take primitives/enums and return `String`; no Qt/FS/network.

/// `operation_label`, with ` - paused` suffix when paused (#1).
pub fn operation_label_with_paused(label: &str, paused: bool) -> String {
    if paused {
        format!("{label} - paused")
    } else {
        label.to_string()
    }
}

/// Pending-pairs tooltip (#2).
pub fn pending_pairs_tooltip(pending: i64) -> String {
    if pending <= 0 {
        String::new()
    } else {
        format!("{pending} pair(s) are waiting for review")
    }
}

/// Defaults of `add_rule` dialog (#3).
pub fn new_rule_name() -> String {
    "new rule".to_string()
}
pub fn new_rule_operation_mode() -> String {
    "semi-automatic".to_string()
}
pub fn new_rule_max_pending_pairs() -> i64 {
    500
}
/// Pretty string of the default comparator (#4).
pub fn default_comparator_pretty() -> String {
    "A num_pixels > B".to_string()
}

/// Comparator number cell (#11).
pub fn comparator_number_text(value: f64) -> String {
    if value.fract() == 0.0 {
        format!("{}", value as i64)
    } else {
        format!("{value}")
    }
}

/// Page-tree fallbacks (#17-18).
pub fn page_tree_title_fallback(title: &str) -> String {
    if title.trim().is_empty() {
        "page".to_string()
    } else {
        title.to_string()
    }
}
pub fn page_tree_count_text(count: i64) -> String {
    count.to_string()
}

/// File hash label (#15).
pub fn file_hash_label(hash_hex: &str) -> String {
    format!("file: {hash_hex}")
}

/// Downloader button texts (#32-34).
pub fn start_button_text(watcher: bool) -> String {
    if watcher {
        "check now".to_string()
    } else {
        "start".to_string()
    }
}
pub fn auto_button_text(auto_watch: bool) -> String {
    if auto_watch {
        "automatic checks: on".to_string()
    } else {
        "automatic checks: off".to_string()
    }
}
pub fn gug_button_text(site_text: &str) -> String {
    if site_text.is_empty() {
        "no downloader set".to_string()
    } else {
        site_text.to_string()
    }
}

/// Highlight / watcher subject fallbacks (part of #36).
pub fn highlight_subject_text(subject: &str) -> String {
    if subject.is_empty() {
        "no subject".to_string()
    } else {
        subject.to_string()
    }
}
pub fn watcher_subject_text(subject: &str) -> String {
    if subject.is_empty() {
        "* unknown".to_string()
    } else {
        subject.to_string()
    }
}
pub fn gallery_source_text(site_text: &str) -> String {
    if site_text.is_empty() {
        "unknown downloader".to_string()
    } else {
        site_text.to_string()
    }
}

/// Files / checking cells (#36): thin wrappers so C++ never formats.
pub fn files_cell_text(done: i64, total: i64) -> String {
    super::render::downloader_files_status(done, total)
}
pub fn queue_count_text(count: i64) -> String {
    count.to_string()
}

/// Empty states (#39-40).
pub fn duplicates_media_empty() -> String {
    super::duplicates::duplicates_media_empty_status().to_string()
}
pub fn auto_rules_empty_status() -> String {
    "no auto-resolution rules yet".to_string()
}

/// Tag-service empty states (#51).
pub fn no_local_tag_service_tooltip() -> String {
    "no local tag service exists yet".to_string()
}
pub fn no_local_tag_service_label() -> String {
    "no local tag service".to_string()
}

/// Seed table headers, status colours and retry label (#52).
pub fn seed_headers() -> Vec<String> {
    ["status", "seed", "created", "modified", "note"]
        .iter()
        .map(|s| s.to_string())
        .collect()
}
pub fn seed_status_color(status: &str) -> String {
    match status {
        "error" | "failed" => "#800000".to_string(),
        "done" | "success" => "#008000".to_string(),
        "working" => "#000080".to_string(),
        _ => String::new(),
    }
}

/// Seed retry button label (#52).
pub fn seed_retry_label(failed: i64) -> String {
    if failed > 0 {
        format!("retry failed ({failed})")
    } else {
        "retry failed".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn operation_label_paused() {
        assert_eq!(operation_label_with_paused("semi-automatic", true), "semi-automatic - paused");
        assert_eq!(operation_label_with_paused("semi-automatic", false), "semi-automatic");
        assert_eq!(operation_label_with_paused("", true), " - paused");
    }

    #[test]
    fn pending_tooltip_states() {
        assert_eq!(pending_pairs_tooltip(0), "");
        assert_eq!(pending_pairs_tooltip(-3), "");
        assert_eq!(pending_pairs_tooltip(1), "1 pair(s) are waiting for review");
        assert_eq!(pending_pairs_tooltip(25), "25 pair(s) are waiting for review");
    }

    #[test]
    fn new_rule_defaults() {
        assert_eq!(new_rule_name(), "new rule");
        assert_eq!(new_rule_operation_mode(), "semi-automatic");
        assert_eq!(new_rule_max_pending_pairs(), 500);
        assert_eq!(default_comparator_pretty(), "A num_pixels > B");
    }

    #[test]
    fn comparator_numbers() {
        assert_eq!(comparator_number_text(2.0), "2");
        assert_eq!(comparator_number_text(1.5), "1.5");
        assert_eq!(comparator_number_text(0.0), "0");
    }

    #[test]
    fn page_tree_fallbacks() {
        assert_eq!(page_tree_title_fallback(""), "page");
        assert_eq!(page_tree_title_fallback("  "), "page");
        assert_eq!(page_tree_title_fallback("files"), "files");
        assert_eq!(page_tree_count_text(0), "0");
        assert_eq!(page_tree_count_text(42), "42");
    }

    #[test]
    fn file_hash() {
        assert_eq!(file_hash_label("ab12"), "file: ab12");
    }

    #[test]
    fn button_texts() {
        assert_eq!(start_button_text(true), "check now");
        assert_eq!(start_button_text(false), "start");
        assert_eq!(auto_button_text(true), "automatic checks: on");
        assert_eq!(auto_button_text(false), "automatic checks: off");
        assert_eq!(gug_button_text(""), "no downloader set");
        assert_eq!(gug_button_text("e-hentai"), "e-hentai");
    }

    #[test]
    fn subject_fallbacks() {
        assert_eq!(highlight_subject_text(""), "no subject");
        assert_eq!(highlight_subject_text("cats"), "cats");
        assert_eq!(watcher_subject_text(""), "* unknown");
        assert_eq!(watcher_subject_text("cats"), "cats");
        assert_eq!(gallery_source_text(""), "unknown downloader");
        assert_eq!(gallery_source_text("nhentai"), "nhentai");
    }

    #[test]
    fn files_cells() {
        assert_eq!(files_cell_text(3, 5), "3/5");
        assert_eq!(files_cell_text(0, 0), "");
        assert_eq!(queue_count_text(7), "7");
    }

    #[test]
    fn empty_states() {
        assert!(!duplicates_media_empty().is_empty());
        assert_eq!(auto_rules_empty_status(), "no auto-resolution rules yet");
        assert_eq!(no_local_tag_service_tooltip(), "no local tag service exists yet");
        assert_eq!(no_local_tag_service_label(), "no local tag service");
    }

    #[test]
    fn seed_headers_and_colors() {
        assert_eq!(seed_headers(), vec!["status", "seed", "created", "modified", "note"]);
        assert_eq!(seed_status_color("failed"), "#800000");
        assert_eq!(seed_status_color("error"), "#800000");
        assert_eq!(seed_status_color("done"), "#008000");
        assert_eq!(seed_status_color("success"), "#008000");
        assert_eq!(seed_status_color("working"), "#000080");
        assert_eq!(seed_status_color("pending"), "");
        assert_eq!(seed_status_color(""), "");
        assert_eq!(seed_retry_label(0), "retry failed");
        assert_eq!(seed_retry_label(3), "retry failed (3)");
    }
}
