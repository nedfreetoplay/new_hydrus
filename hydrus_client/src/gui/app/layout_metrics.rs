//! Layout metrics for WS-15.3.2c (R2: pure, tested).
//! Column widths, clamps, gauge percents. Inputs are primitives/slices,
//! outputs are numbers. No Qt/FS/network.

/// #5: auto-resolution preview table column width (pass and fail tables).
pub fn auto_resolution_preview_column_width() -> i64 {
    116
}
/// #7: review table column width.
pub fn auto_resolution_review_column_width() -> i64 {
    116
}
/// #20: gallery queue table widths `{56,78,30,30,56,64,56}`.
pub fn gallery_column_width(index: i64) -> i64 {
    const WIDTHS: [i64; 7] = [56, 78, 30, 30, 56, 64, 56];
    if (0..WIDTHS.len() as i64).contains(&index) {
        WIDTHS[index as usize]
    } else {
        0
    }
}
pub fn gallery_column_count() -> i64 {
    7
}
/// #22: watcher table widths `{80,38,38,48,38,46}`.
pub fn watcher_column_width(index: i64) -> i64 {
    const WIDTHS: [i64; 6] = [80, 38, 38, 48, 38, 46];
    if (0..WIDTHS.len() as i64).contains(&index) {
        WIDTHS[index as usize]
    } else {
        0
    }
}
pub fn watcher_column_count() -> i64 {
    6
}
/// #23: export folders manager widths `{132,20,79,106,106,132}`.
pub fn export_manager_column_width(index: i64) -> i64 {
    const WIDTHS: [i64; 6] = [132, 20, 79, 106, 106, 132];
    if (0..WIDTHS.len() as i64).contains(&index) {
        WIDTHS[index as usize]
    } else {
        0
    }
}
pub fn export_manager_column_count() -> i64 {
    6
}
/// #24: import folders manager widths `{200,430,70,140}`.
pub fn import_manager_column_width(index: i64) -> i64 {
    const WIDTHS: [i64; 4] = [200, 430, 70, 140];
    if (0..WIDTHS.len() as i64).contains(&index) {
        WIDTHS[index as usize]
    } else {
        0
    }
}
pub fn import_manager_column_count() -> i64 {
    4
}
/// #27: `qBound(25, scale, 400)` thumbnail scale clamp.
pub fn thumbnail_scale_clamp(scale: i64) -> i64 {
    scale.clamp(25, 400)
}
/// #27: file limit clamp `qMax(1, value)`.
pub fn file_limit_clamp(value: i64) -> i64 {
    value.max(1)
}
/// #38: `files_progress` gauge percent (done/total, empty total -> 0).
pub fn files_progress_percent(done: i64, total: i64) -> i64 {
    if total <= 0 || done <= 0 {
        return 0;
    }
    let pct = (done as f64 * 100.0 / total as f64).floor() as i64;
    pct.clamp(0, 100)
}
/// Autocomplete popup visible rows: `qBound(1, count, 12)`.
pub fn autocomplete_visible_rows(count: i64) -> i64 {
    count.clamp(1, 12)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preview_widths() {
        assert_eq!(auto_resolution_preview_column_width(), 116);
        assert_eq!(auto_resolution_review_column_width(), 116);
    }

    #[test]
    fn gallery_widths() {
        assert_eq!(gallery_column_count(), 7);
        for (i, w) in [56, 78, 30, 30, 56, 64, 56].iter().enumerate() {
            assert_eq!(gallery_column_width(i as i64), *w);
        }
        assert_eq!(gallery_column_width(-1), 0);
        assert_eq!(gallery_column_width(7), 0);
        assert_eq!(gallery_column_width(1000), 0);
    }

    #[test]
    fn watcher_widths() {
        assert_eq!(watcher_column_count(), 6);
        for (i, w) in [80, 38, 38, 48, 38, 46].iter().enumerate() {
            assert_eq!(watcher_column_width(i as i64), *w);
        }
        assert_eq!(watcher_column_width(-1), 0);
        assert_eq!(watcher_column_width(6), 0);
    }

    #[test]
    fn manager_widths() {
        assert_eq!(export_manager_column_count(), 6);
        for (i, w) in [132, 20, 79, 106, 106, 132].iter().enumerate() {
            assert_eq!(export_manager_column_width(i as i64), *w);
        }
        assert_eq!(export_manager_column_width(-5), 0);
        assert_eq!(export_manager_column_width(6), 0);
        assert_eq!(export_manager_column_count(), 6);
        assert_eq!(import_manager_column_count(), 4);
        for (i, w) in [200, 430, 70, 140].iter().enumerate() {
            assert_eq!(import_manager_column_width(i as i64), *w);
        }
        assert_eq!(import_manager_column_width(-1), 0);
        assert_eq!(import_manager_column_width(4), 0);
        assert_eq!(import_manager_column_width(i64::MAX), 0);
    }

    #[test]
    fn clamps() {
        assert_eq!(thumbnail_scale_clamp(0), 25);
        assert_eq!(thumbnail_scale_clamp(-100), 25);
        assert_eq!(thumbnail_scale_clamp(25), 25);
        assert_eq!(thumbnail_scale_clamp(100), 100);
        assert_eq!(thumbnail_scale_clamp(400), 400);
        assert_eq!(thumbnail_scale_clamp(1000), 400);
        assert_eq!(thumbnail_scale_clamp(i64::MAX), 400);
        assert_eq!(thumbnail_scale_clamp(i64::MIN), 25);
        assert_eq!(file_limit_clamp(0), 1);
        assert_eq!(file_limit_clamp(-5), 1);
        assert_eq!(file_limit_clamp(1), 1);
        assert_eq!(file_limit_clamp(2000), 2000);
        assert_eq!(file_limit_clamp(i64::MAX), i64::MAX);
        assert_eq!(autocomplete_visible_rows(0), 1);
        assert_eq!(autocomplete_visible_rows(-3), 1);
        assert_eq!(autocomplete_visible_rows(5), 5);
        assert_eq!(autocomplete_visible_rows(12), 12);
        assert_eq!(autocomplete_visible_rows(99), 12);
    }

    #[test]
    fn progress_percent() {
        assert_eq!(files_progress_percent(0, 0), 0);
        assert_eq!(files_progress_percent(5, 0), 0);
        assert_eq!(files_progress_percent(0, 10), 0);
        assert_eq!(files_progress_percent(-1, 10), 0);
        assert_eq!(files_progress_percent(5, -2), 0);
        assert_eq!(files_progress_percent(1, 2), 50);
        assert_eq!(files_progress_percent(1, 3), 33);
        assert_eq!(files_progress_percent(10, 10), 100);
        assert_eq!(files_progress_percent(15, 10), 100);
        assert_eq!(files_progress_percent(1_000_000, 2_000_000), 50);
    }

    #[test]
    fn empty_and_huge_lists() {
        let empty: Vec<i64> = vec![];
        assert!(empty.is_empty());
        assert_eq!(files_progress_percent(empty.len() as i64, 0), 0);
        assert_eq!(gallery_column_width(empty.len() as i64 - 1), 0);
    }
}
