//! Pure file-history chart logic for `database -> view file history`
//! (WS-13.9 / WS-11.12, R2). Mirrors `ClientGUICharts.FileHistory` and the
//! `ReviewFileHistory` panel: series names, visibility, y/x auto-ranges, axis
//! label formats and the QtCharts default series colours. C++ only draws the
//! values produced here.

/// Series names exactly as in Python (`setName` calls).
pub const SERIES_ALL: &str = "all files";
pub const SERIES_INBOX: &str = "inbox";
pub const SERIES_ARCHIVE: &str = "archive";
pub const SERIES_DELETED: &str = "deleted";

/// Series colours in QtCharts Light-theme order (first four series pens):
/// blue, orange-red, green, violet.
pub const COLOR_ALL: &str = "#209fdf";
pub const COLOR_INBOX: &str = "#f26522";
pub const COLOR_ARCHIVE: &str = "#2ca02c";
pub const COLOR_DELETED: &str = "#9467bd";

/// Checkbox labels exactly as in the Python panel.
pub const LABEL_SHOW_ALL: &str = "show all";
pub const LABEL_SHOW_INBOX: &str = "show inbox";
pub const LABEL_SHOW_ARCHIVE: &str = "show archive";
pub const LABEL_SHOW_DELETED: &str = "show deleted";

/// Refit buttons exactly as in the Python panel.
pub const BUTTON_REFIT_Y: &str = "refit y axis";
pub const BUTTON_REFIT_X: &str = "refit x axis";
pub const TOOLTIP_REFIT_Y: &str =
    "Recalculate a y axis range based on the current data that is in view.";
pub const TOOLTIP_REFIT_X: &str =
    "Recalculate an x axis range based on the current data that is in view.";

/// Spinbox bounds exactly as in the Python panel (`BetterSpinBox 0..1000000000`).
pub const Y_SPIN_MIN: i64 = 0;
pub const Y_SPIN_MAX: i64 = 1_000_000_000;

/// Minimum chart size exactly as in the Python panel.
pub const CHART_MIN_WIDTH: i32 = 720;
pub const CHART_MIN_HEIGHT: i32 = 480;

/// X axis date format exactly as in Python (`setFormat('yyyy-MM-dd')`).
pub const DATE_FORMAT: &str = "%Y-%m-%d";

/// `FileHistory._AutoSetYRange`: minimum 0, maximum at least 1, over the
/// visible series only.
pub fn auto_y_max(
    current: &[(i64, i64)],
    inbox: &[(i64, i64)],
    archive: &[(i64, i64)],
    deleted: &[(i64, i64)],
    show_all: bool,
    show_inbox: bool,
    show_archive: bool,
    show_deleted: bool,
) -> i64 {
    let mut max_y = 1_i64;

    for (series, show) in [
        (current, show_all),
        (inbox, show_inbox),
        (archive, show_archive),
        (deleted, show_deleted),
    ] {
        if show {
            if let Some(peak) = series.iter().map(|(_, n)| *n).max() {
                max_y = max_y.max(peak);
            }
        }
    }

    max_y
}

/// `FileHistory.AutoSetXRange`: first timestamp of the earliest visible
/// series to last timestamp of the latest visible series.
pub fn auto_x_range(
    current: &[(i64, i64)],
    inbox: &[(i64, i64)],
    archive: &[(i64, i64)],
    deleted: &[(i64, i64)],
    show_all: bool,
    show_inbox: bool,
    show_archive: bool,
    show_deleted: bool,
) -> Option<(i64, i64)> {
    let mut start: Option<i64> = None;
    let mut end: Option<i64> = None;

    for (series, show) in [
        (current, show_all),
        (inbox, show_inbox),
        (archive, show_archive),
        (deleted, show_deleted),
    ] {
        if show && !series.is_empty() {
            let first = series.first().map(|(t, _)| *t).unwrap_or(0);
            let last = series.last().map(|(t, _)| *t).unwrap_or(0);
            start = Some(start.map_or(first, |s: i64| s.min(first)));
            end = Some(end.map_or(last, |e: i64| e.max(last)));
        }
    }

    match (start, end) {
        (Some(start), Some(end)) => Some((start, end)),
        _ => None,
    }
}

/// Approximation of `QValueAxis.applyNiceNumbers`: rounds the ceiling up to a
/// 1/2/2.5/5/10 step so the axis top lands on a round tick.
pub fn nice_ceiling(value: i64) -> i64 {
    if value <= 0 {
        return 1;
    }

    let mut magnitude = 1_i64;
    while magnitude * 10 <= value {
        magnitude *= 10;
    }

    for step in [1, 2, 5, 10] {
        let candidate = step * magnitude;
        if candidate >= value {
            return candidate;
        }
    }

    magnitude * 10
}

/// `QValueAxis.setLabelFormat('%i')`: plain integer, no thousands separator.
pub fn y_tick_label(value: i64) -> String {
    format!("{value}")
}

/// X axis tick label in the Python `yyyy-MM-dd` format (UTC, as QtCharts does
/// with `fromSecsSinceEpoch` in the port's timezone-independent path).
pub fn date_label(timestamp_secs: i64) -> String {
    chrono::DateTime::from_timestamp(timestamp_secs, 0)
        .map(|dt| dt.format(DATE_FORMAT).to_string())
        .unwrap_or_default()
}

/// Status text while the history is being computed.
pub fn loading_status() -> String {
    format!("loading{}", '\u{2026}')
}

#[cfg(test)]
mod tests {
    use super::*;

    fn series(points: &[(i64, i64)]) -> Vec<(i64, i64)> {
        points.to_vec()
    }

    #[test]
    fn series_names_and_colours_match_python() {
        assert_eq!(SERIES_ALL, "all files");
        assert_eq!(SERIES_INBOX, "inbox");
        assert_eq!(SERIES_ARCHIVE, "archive");
        assert_eq!(SERIES_DELETED, "deleted");
        assert_eq!(COLOR_ALL, "#209fdf");
        assert_eq!(COLOR_INBOX, "#f26522");
        assert_eq!(COLOR_ARCHIVE, "#2ca02c");
        assert_eq!(COLOR_DELETED, "#9467bd");
    }

    #[test]
    fn controls_match_python_verbatim() {
        assert_eq!(LABEL_SHOW_ALL, "show all");
        assert_eq!(LABEL_SHOW_INBOX, "show inbox");
        assert_eq!(LABEL_SHOW_ARCHIVE, "show archive");
        assert_eq!(LABEL_SHOW_DELETED, "show deleted");
        assert_eq!(BUTTON_REFIT_Y, "refit y axis");
        assert_eq!(BUTTON_REFIT_X, "refit x axis");
        assert_eq!(
            TOOLTIP_REFIT_Y,
            "Recalculate a y axis range based on the current data that is in view."
        );
        assert_eq!(
            TOOLTIP_REFIT_X,
            "Recalculate an x axis range based on the current data that is in view."
        );
        assert_eq!((Y_SPIN_MIN, Y_SPIN_MAX), (0, 1_000_000_000));
        assert_eq!((CHART_MIN_WIDTH, CHART_MIN_HEIGHT), (720, 480));
    }

    #[test]
    fn auto_y_uses_visible_series_only() {
        let current = series(&[(1, 10), (2, 30)]);
        let inbox = series(&[(1, 5)]);
        let archive = series(&[(1, 7)]);
        let deleted = series(&[(1, 50)]);

        assert_eq!(auto_y_max(&current, &inbox, &archive, &deleted, true, true, true, true), 50);
        assert_eq!(
            auto_y_max(&current, &inbox, &archive, &deleted, true, false, false, false),
            30
        );
        // Nothing visible still yields the Python floor of 1.
        assert_eq!(
            auto_y_max(&current, &inbox, &archive, &deleted, false, false, false, false),
            1
        );
        assert_eq!(auto_y_max(&[], &[], &[], &[], true, true, true, true), 1);
    }

    #[test]
    fn auto_x_spans_visible_series_only() {
        let current = series(&[(100, 1), (300, 3)]);
        let inbox = series(&[(200, 1)]);
        let empty: Vec<(i64, i64)> = Vec::new();

        assert_eq!(
            auto_x_range(&current, &inbox, &empty, &empty, true, true, false, false),
            Some((100, 300))
        );
        assert_eq!(
            auto_x_range(&current, &inbox, &empty, &empty, false, true, false, false),
            Some((200, 200))
        );
        assert_eq!(auto_x_range(&current, &inbox, &empty, &empty, false, false, true, true), None);
    }

    #[test]
    fn nice_numbers_round_up() {
        assert_eq!(nice_ceiling(0), 1);
        assert_eq!(nice_ceiling(1), 1);
        assert_eq!(nice_ceiling(3), 5);
        assert_eq!(nice_ceiling(30), 50);
        assert_eq!(nice_ceiling(100), 100);
        assert_eq!(nice_ceiling(101), 200);
    }

    #[test]
    fn axis_labels_match_python_formats() {
        assert_eq!(y_tick_label(1234567), "1234567");
        assert_eq!(date_label(1_700_000_000), "2023-11-14");
        assert_eq!(loading_status(), "loading\u{2026}");
    }
}
