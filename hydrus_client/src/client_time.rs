//! Date/time helpers from `hydrus/client/ClientTime.py`.
//!
//! The Python implementation relies on `dateparser`/`dateutil`. The Rust port
//! keeps a deliberately small set of unambiguous formats and leaves
//! locale-specific parsing to the caller.

use chrono::{Datelike, Duration, NaiveDate, NaiveDateTime, TimeZone, Utc};

/// `CalendarDelta( dt, month_delta = 0, day_delta = 0 )`
pub fn calendar_delta(dt: NaiveDateTime, month_delta: i32, day_delta: i64) -> NaiveDateTime {
    let mut year = dt.year();
    let mut month = dt.month() as i32 + month_delta;

    while month < 1 {
        year -= 1;
        month += 12;
    }

    while month > 12 {
        year += 1;
        month -= 12;
    }

    let month = month as u32;
    let day = dt.day().min(last_day_of_month(year, month));

    NaiveDate::from_ymd_opt(year, month, day)
        .unwrap_or_else(|| dt.date())
        .and_time(dt.time())
        .checked_add_signed(Duration::days(day_delta))
        .unwrap_or(dt)
}

fn last_day_of_month(year: i32, month: u32) -> u32 {
    let (next_year, next_month) = if month == 12 {
        (year + 1, 1)
    } else {
        (year, month + 1)
    };

    NaiveDate::from_ymd_opt(next_year, next_month, 1)
        .and_then(|date| date.pred_opt())
        .map(|date| date.day())
        .unwrap_or(28)
}

/// `ParseDate( date_string )`
///
/// Supports RFC 3339/ISO 8601 and the common Hydrus timestamp formats. The
/// return value is a Unix timestamp in seconds, as in Python.
pub fn parse_date(date_string: &str) -> Option<f64> {
    let date_string = date_string.trim();

    if let Ok(datetime) = chrono::DateTime::parse_from_rfc3339(date_string) {
        return Some(
            datetime.timestamp() as f64 + f64::from(datetime.timestamp_subsec_nanos()) / 1e9,
        );
    }

    for format in [
        "%Y-%m-%d %H:%M:%S",
        "%Y-%m-%d %H:%M",
        "%Y-%m-%d",
        "%Y/%m/%d %H:%M:%S",
        "%Y/%m/%d",
        "%d/%m/%Y %H:%M:%S",
        "%d/%m/%Y",
    ] {
        let parsed = if format.contains("%H") {
            NaiveDateTime::parse_from_str(date_string, format).ok()
        } else {
            NaiveDate::parse_from_str(date_string, format)
                .ok()
                .and_then(|date| date.and_hms_opt(0, 0, 0))
        };

        if let Some(datetime) = parsed {
            return Some(Utc.from_utc_datetime(&datetime).timestamp() as f64);
        }
    }

    None
}

/// `ShouldUpdateModifiedTime( existing_timestamp, new_timestamp )`
pub fn should_update_modified_time(
    existing_timestamp: Option<i64>,
    new_timestamp: Option<i64>,
) -> bool {
    let Some(new_timestamp) = new_timestamp else {
        return false;
    };

    let Some(existing_timestamp) = existing_timestamp else {
        return true;
    };

    // Only go backwards, in general.
    new_timestamp < existing_timestamp
}

/// `MergeModifiedTimes( existing_timestamp, new_timestamp )`
pub fn merge_modified_times(
    existing_timestamp: Option<i64>,
    new_timestamp: Option<i64>,
) -> Option<i64> {
    if should_update_modified_time(existing_timestamp, new_timestamp) {
        new_timestamp
    } else {
        existing_timestamp
    }
}

/// `TimestampIsSensible( timestamp )`
pub fn timestamp_is_sensible(timestamp: Option<i64>) -> bool {
    timestamp.is_some_and(|timestamp| timestamp > 86400 * 7)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dt(year: i32, month: u32, day: u32) -> NaiveDateTime {
        NaiveDate::from_ymd_opt(year, month, day)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap()
    }

    #[test]
    fn calendar_delta_clamps_month_end() {
        let january_31 = dt(2024, 1, 31);
        let february = calendar_delta(january_31, 1, 0);

        assert_eq!(
            february.date(),
            NaiveDate::from_ymd_opt(2024, 2, 29).unwrap()
        );
    }

    #[test]
    fn calendar_delta_handles_negative_months() {
        let march = dt(2024, 3, 15);
        let january = calendar_delta(march, -2, 5);

        assert_eq!(
            january.date(),
            NaiveDate::from_ymd_opt(2024, 1, 20).unwrap()
        );
    }

    #[test]
    fn parses_iso_and_simple_formats() {
        assert_eq!(parse_date("1970-01-02"), Some(86400.0));
        assert_eq!(parse_date("1970-01-01 00:00:00"), Some(0.0));
        assert_eq!(parse_date("2023-07-18T08:32:00Z"), Some(1689669120.0));
        assert_eq!(parse_date("not a date"), None);
    }

    #[test]
    fn modified_times_only_move_backwards() {
        assert_eq!(merge_modified_times(Some(100), Some(90)), Some(90));
        assert_eq!(merge_modified_times(Some(100), Some(110)), Some(100));
        assert_eq!(merge_modified_times(None, Some(5)), Some(5));
        assert_eq!(merge_modified_times(Some(5), None), Some(5));
    }

    #[test]
    fn early_timestamps_are_not_sensible() {
        assert!(!timestamp_is_sensible(None));
        assert!(!timestamp_is_sensible(Some(86400 * 7)));
        assert!(timestamp_is_sensible(Some(86400 * 7 + 1)));
    }
}
