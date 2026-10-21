//! Rust port of `hydrus/core/HydrusTime.py` plus `HydrusDateTime.py`.
//!
//! Hydrus mostly deals in "unix timestamps as whole seconds", with milliseconds
//! where sub-second precision matters. The Python module uses naive
//! `datetime.datetime` objects and paper over the pre-1970 and DST problems;
//! `chrono` handles those cases directly, so the port uses
//! [`chrono::NaiveDateTime`] and converts through [`chrono::Local`] only when a
//! timestamp is involved.
//!
//! `time.perf_counter()` maps to [`std::time::Instant`], which is the Rust
//! idiom for monotonic measurements.

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use chrono::{Datelike, Duration as ChronoDuration, Local, LocalResult, NaiveDate, NaiveDateTime, TimeZone};

use crate::hydrus_constants::{TIMEZONE_LOCAL, TIMEZONE_UTC};
use crate::hydrus_numbers::to_human_int;

/// `ALWAYS_SHOW_ISO_TIME_ON_DELTA_CALL`
pub static ALWAYS_SHOW_ISO_TIME_ON_DELTA_CALL: AtomicBool = AtomicBool::new(false);

const SECONDS_IN_MINUTE: i64 = 60;
const SECONDS_IN_HOUR: i64 = 60 * SECONDS_IN_MINUTE;
const SECONDS_IN_DAY: i64 = 24 * SECONDS_IN_HOUR;
const SECONDS_IN_MONTH: i64 = 2_629_800; // 365.25 / 12 * day
const SECONDS_IN_YEAR: i64 = 31_557_600; // 365.25 * day

fn epoch_naive() -> NaiveDateTime {
    NaiveDate::from_ymd_opt(1970, 1, 1)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap()
}

fn local_offset_seconds_now() -> i64 {
    Local::now().offset().local_minus_utc() as i64
}

fn naive_local_to_timestamp(dt: &NaiveDateTime) -> i64 {
    match Local.from_local_datetime(dt) {
        LocalResult::Single(local) => local.timestamp(),
        LocalResult::Ambiguous(earliest, _) => earliest.timestamp(),
        LocalResult::None => {
            // A gap in local time (spring-forward). Python's `timestamp()`
            // mangles this too; treating it as UTC is the least surprising
            // fallback.
            dt.and_utc().timestamp()
        }
    }
}

/// `datetime_to_pretty_time( dt, include_24h_time = True, include_milliseconds = False )`
pub fn datetime_to_pretty_time(
    dt: NaiveDateTime,
    include_24h_time: bool,
    include_milliseconds: bool,
) -> String {
    let phrase = if include_24h_time {
        "%Y-%m-%d %H:%M:%S"
    } else {
        "%Y-%m-%d"
    };

    let mut result = dt.format(phrase).to_string();

    if include_milliseconds {
        let milliseconds = dt.and_utc().timestamp_subsec_micros() / 1000;

        result = format!("{result}.{milliseconds:03}");
    }

    result
}

/// `datetime_to_timestamp( dt )`
pub fn datetime_to_timestamp(dt: NaiveDateTime) -> i64 {
    naive_local_to_timestamp(&dt)
}

/// `datetime_to_timestamp_ms( dt )`
pub fn datetime_to_timestamp_ms(dt: NaiveDateTime) -> i64 {
    millisecondise_s(Some(datetime_to_timestamp(dt) as f64)).unwrap()
        + (dt.and_utc().timestamp_subsec_micros() / 1000) as i64
}

/// `get_datetime( year, month, day, hour, minute )`
///
/// Returns `None` for invalid dates, where Python raises `ValueError`.
pub fn get_datetime(year: i32, month: u32, day: u32, hour: u32, minute: u32) -> Option<NaiveDateTime> {
    NaiveDate::from_ymd_opt(year, month, day)?.and_hms_opt(hour, minute, 0)
}

/// `get_now()`
pub fn get_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0)
}

/// `get_now_float()`
pub fn get_now_float() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs_f64())
        .unwrap_or(0.0)
}

/// `get_now_ms()`
pub fn get_now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or(0)
}

/// `get_now_precise()`
pub fn get_now_precise() -> Instant {
    Instant::now()
}

/// `get_timedelta_since_time( timestamp )`
pub fn get_timedelta_since_time(timestamp: i64) -> i64 {
    (get_now() - timestamp).max(0)
}

/// `get_timedelta_until_time( timestamp )`
pub fn get_timedelta_until_time(timestamp: i64) -> i64 {
    (timestamp - get_now()).max(0)
}

/// `get_timedelta_until_time_float( timestamp )`
pub fn get_timedelta_until_time_float(timestamp: f64) -> f64 {
    (timestamp - get_now_float()).max(0.0)
}

/// `get_timedelta_until_time_precise( t )`
pub fn get_timedelta_until_time_precise(t: Instant) -> f64 {
    t.saturating_duration_since(Instant::now()).as_secs_f64()
}

/// `millisecondise_s( time_delta_s )`
pub fn millisecondise_s(time_delta_s: Option<f64>) -> Option<i64> {
    time_delta_s.map(|seconds| (seconds * 1000.0) as i64)
}

/// `secondise_ms( time_delta_ms )`
pub fn secondise_ms(time_delta_ms: Option<f64>) -> Option<i64> {
    time_delta_ms.map(|milliseconds| (milliseconds / 1000.0).floor() as i64)
}

/// `secondise_ms_float( time_delta_ms )`
pub fn secondise_ms_float(time_delta_ms: Option<f64>) -> Option<f64> {
    time_delta_ms.map(|milliseconds| milliseconds / 1000.0)
}

/// `time_has_passed( timestamp )`
pub fn time_has_passed(timestamp: Option<i64>) -> bool {
    timestamp.is_some_and(|timestamp| get_now() > timestamp)
}

/// `time_has_passed_float( timestamp )`
pub fn time_has_passed_float(timestamp: f64) -> bool {
    get_now_float() > timestamp
}

/// `time_has_passed_ms( timestamp_ms )`
pub fn time_has_passed_ms(timestamp_ms: Option<i64>) -> bool {
    timestamp_ms.is_some_and(|timestamp_ms| get_now_ms() > timestamp_ms)
}

/// `time_has_passed_precise( precise_timestamp )`
pub fn time_has_passed_precise(precise_timestamp: Instant) -> bool {
    Instant::now() > precise_timestamp
}

/// `time_until( timestamp )`
pub fn time_until(timestamp: i64) -> i64 {
    timestamp - get_now()
}

fn days_in_month(year: i32, month: u32) -> u32 {
    let (next_year, next_month) = if month == 12 {
        (year + 1, 1)
    } else {
        (year, month + 1)
    };

    let first = match NaiveDate::from_ymd_opt(year, month, 1) {
        Some(date) => date,
        None => return 30,
    };

    let next = match NaiveDate::from_ymd_opt(next_year, next_month, 1) {
        Some(date) => date,
        None => return 30,
    };

    next.signed_duration_since(first).num_days() as u32
}

/// `calendar_delta_to_datetime( years, months, days, hours )`
///
/// Returns `None` if the resulting year/month are outside chrono's range.
pub fn calendar_delta_to_datetime(
    years: i32,
    months: i32,
    days: i64,
    hours: i64,
) -> Option<NaiveDateTime> {
    let now = Local::now().naive_local();

    let result = now - ChronoDuration::days(days) - ChronoDuration::hours(hours);

    let mut new_year = result.year() - years;
    let mut new_month = result.month() as i32 - months;

    while new_month < 1 {
        new_year -= 1;
        new_month += 12;
    }

    while new_month > 12 {
        new_year += 1;
        new_month -= 12;
    }

    let new_month = new_month as u32;

    let new_day = days_in_month(new_year, new_month).min(result.day());

    Some(NaiveDate::from_ymd_opt(new_year, new_month, new_day)?.and_time(result.time()))
}

/// `calendar_delta_to_rough_datetime_timedelta( years, months, days, hours )`
pub fn calendar_delta_to_rough_datetime_timedelta(
    years: i32,
    months: i32,
    days: i64,
    hours: i64,
) -> ChronoDuration {
    let total_seconds = days as f64 * 86_400.0
        + months as f64 * (365.25 / 12.0) * 86_400.0
        + years as f64 * 365.25 * 86_400.0
        + hours as f64 * 3_600.0;

    ChronoDuration::milliseconds((total_seconds * 1000.0) as i64)
}

/// `time_delta_to_pretty_timedelta( seconds, show_seconds = True, no_bigger_than_days = False )`
pub fn time_delta_to_pretty_timedelta(
    seconds: Option<f64>,
    show_seconds: bool,
    no_bigger_than_days: bool,
) -> String {
    let Some(seconds) = seconds else {
        return "per month".to_string();
    };

    if seconds == 0.0 {
        return "0 seconds".to_string();
    }

    let negative = seconds < 0.0;
    let seconds = seconds.abs();

    let mut result;

    if seconds >= 60.0 {
        let mut seconds = seconds as i64;

        let mut lines: Vec<(&str, i64)> = Vec::new();

        if !no_bigger_than_days {
            lines.push(("year", SECONDS_IN_YEAR));
            lines.push(("month", SECONDS_IN_MONTH));
        }

        lines.push(("day", SECONDS_IN_DAY));
        lines.push(("hour", SECONDS_IN_HOUR));
        lines.push(("minute", SECONDS_IN_MINUTE));

        if show_seconds {
            lines.push(("second", 1));
        }

        let mut result_components: Vec<String> = Vec::new();

        for (time_string, time_delta_s) in lines {
            let mut time_quantity = seconds / time_delta_s;

            seconds %= time_delta_s;

            // Little rounding thing if you get the 364th day with 30 day months.
            if time_string == "month" && time_quantity > 11 {
                time_quantity = 11;
            }

            if time_quantity > 0 {
                let mut s = format!("{} {}", to_human_int(time_quantity), time_string);

                if time_quantity > 1 {
                    s.push('s');
                }

                result_components.push(s);

                if result_components.len() == 2 {
                    break;
                }
            } else if !result_components.is_empty() {
                break;
            }
        }

        result = result_components.join(" ");
    } else if seconds > 1.0 {
        if seconds.trunc() == seconds {
            result = format!("{} seconds", to_human_int(seconds as i128));
        } else {
            result = format!("{seconds:.1} seconds");
        }
    } else if seconds == 1.0 {
        result = "1 second".to_string();
    } else {
        let milliseconds = seconds * 1000.0;

        if milliseconds > 100.0 || milliseconds.fract() == 0.0 {
            result = format!("{} milliseconds", milliseconds as i64);
        } else if milliseconds > 10.0 {
            result = format!("{milliseconds:.1} milliseconds");
        } else if milliseconds >= 1.0 {
            result = format!("{milliseconds:.2} milliseconds");
        } else {
            result = format!("{} microseconds", (milliseconds * 1000.0) as i64);
        }
    }

    if negative {
        result = format!("-{result}");
    }

    result
}

/// `Timestamp_ms_to_datetime( timestamp_ms, timezone = None )`
pub fn timestamp_ms_to_datetime(timestamp_ms: i64, timezone: u8) -> NaiveDateTime {
    let dt = epoch_naive() + ChronoDuration::milliseconds(timestamp_ms);

    if timezone == TIMEZONE_LOCAL {
        dt + ChronoDuration::seconds(local_offset_seconds_now())
    } else {
        dt
    }
}

/// `timestamp_to_datetime( timestamp, timezone = None )`
pub fn timestamp_to_datetime(timestamp: f64, timezone: u8) -> NaiveDateTime {
    let dt = epoch_naive() + ChronoDuration::milliseconds((timestamp * 1000.0) as i64);

    if timezone == TIMEZONE_LOCAL {
        dt + ChronoDuration::seconds(local_offset_seconds_now())
    } else {
        dt
    }
}

/// `timestamp_to_pretty_expires( timestamp )`
pub fn timestamp_to_pretty_expires(timestamp: Option<i64>) -> String {
    let Some(timestamp) = timestamp else {
        return "does not expire".to_string();
    };

    if timestamp == 0 {
        return "unknown expiration".to_string();
    }

    let time_delta_string = timestamp_to_pretty_time_delta(Some(timestamp), &PrettyTimeDeltaOptions {
        force_no_iso: true,
        ..Default::default()
    });

    if time_has_passed(Some(timestamp)) {
        format!("expired {time_delta_string}")
    } else {
        format!("expires {time_delta_string}")
    }
}

/// `milliseconds_duration_to_pretty_time( duration_ms, force_numbers = False )`
pub fn milliseconds_duration_to_pretty_time(
    duration_ms: Option<i64>,
    force_numbers: bool,
) -> String {
    let mut duration_ms = duration_ms.unwrap_or(0);

    if duration_ms == 0 && !force_numbers {
        return "no duration".to_string();
    }

    let hours = duration_ms / 3_600_000;
    duration_ms %= 3_600_000;

    let minutes = duration_ms / 60_000;
    duration_ms %= 60_000;

    let seconds = duration_ms / 1000;
    duration_ms %= 1000;

    let minutes_result = if minutes == 1 {
        "1 minute".to_string()
    } else {
        format!("{minutes} minutes")
    };

    if hours > 0 {
        let hours_result = if hours == 1 {
            "1 hour".to_string()
        } else {
            format!("{hours} hours")
        };

        return format!("{hours_result} {minutes_result}");
    }

    if minutes > 0 {
        let seconds_result = if seconds == 1 {
            "1 second".to_string()
        } else {
            format!("{seconds} seconds")
        };

        return format!("{minutes_result} {seconds_result}");
    }

    if seconds > 0 {
        let detailed_seconds = seconds as f64 + secondise_ms_float(Some(duration_ms as f64)).unwrap();

        if detailed_seconds.trunc() == detailed_seconds {
            return format!("{} seconds", to_human_int(detailed_seconds as i128));
        }

        return format!("{detailed_seconds:.1} seconds");
    }

    if duration_ms == 1 {
        "1 millisecond".to_string()
    } else {
        format!("{duration_ms} milliseconds")
    }
}

/// `timestamp_ms_to_pretty_time( timestamp_ms, in_utc = False, include_24h_time = True, include_milliseconds = True )`
pub fn timestamp_ms_to_pretty_time(
    timestamp_ms: Option<i64>,
    in_utc: bool,
    include_24h_time: bool,
    include_milliseconds: bool,
) -> String {
    let Some(timestamp_ms) = timestamp_ms else {
        return "unknown time".to_string();
    };

    let timezone = if in_utc { TIMEZONE_UTC } else { TIMEZONE_LOCAL };

    let dt = timestamp_ms_to_datetime(timestamp_ms, timezone);

    datetime_to_pretty_time(dt, include_24h_time, include_milliseconds)
}

/// `timestamp_to_pretty_time( timestamp, in_utc = False, include_24h_time = True )`
pub fn timestamp_to_pretty_time(
    timestamp: Option<f64>,
    in_utc: bool,
    include_24h_time: bool,
) -> String {
    let Some(timestamp) = timestamp else {
        return "unknown time".to_string();
    };

    let timezone = if in_utc { TIMEZONE_UTC } else { TIMEZONE_LOCAL };

    let dt = timestamp_to_datetime(timestamp, timezone);

    datetime_to_pretty_time(dt, include_24h_time, false)
}

/// The keyword arguments of `timestamp_to_pretty_time_delta`.
#[derive(Debug, Clone)]
pub struct PrettyTimeDeltaOptions {
    pub just_now_string: String,
    pub just_now_threshold: i64,
    pub history_suffix: String,
    pub show_seconds: bool,
    pub no_prefix: bool,
    pub reverse_iso_delta_setting: bool,
    pub force_no_iso: bool,
}

impl Default for PrettyTimeDeltaOptions {
    fn default() -> Self {
        Self {
            just_now_string: "now".to_string(),
            just_now_threshold: 3,
            history_suffix: " ago".to_string(),
            show_seconds: true,
            no_prefix: false,
            reverse_iso_delta_setting: false,
            force_no_iso: false,
        }
    }
}

/// `timestamp_to_pretty_time_delta( timestamp, ... )`
pub fn timestamp_to_pretty_time_delta(
    timestamp: Option<i64>,
    options: &PrettyTimeDeltaOptions,
) -> String {
    let iso_on = ALWAYS_SHOW_ISO_TIME_ON_DELTA_CALL.load(Ordering::Relaxed);

    if !options.force_no_iso && (iso_on ^ options.reverse_iso_delta_setting) {
        return timestamp_to_pretty_time(timestamp.map(|timestamp| timestamp as f64), false, true);
    }

    let Some(timestamp) = timestamp else {
        return "at an unknown time".to_string();
    };

    let just_now_threshold = if options.show_seconds {
        options.just_now_threshold
    } else {
        options.just_now_threshold.max(60)
    };

    let time_delta = (timestamp - get_now()).abs();

    if time_delta <= just_now_threshold {
        return options.just_now_string.clone();
    }

    let time_delta_string =
        time_delta_to_pretty_timedelta(Some(time_delta as f64), options.show_seconds, false);

    if time_has_passed(Some(timestamp)) {
        format!("{time_delta_string}{}", options.history_suffix)
    } else if options.no_prefix {
        time_delta_string
    } else {
        format!("in {time_delta_string}")
    }
}

/// `value_range_to_scanbar_timestamps_ms( value_ms, range_ms )`
pub fn value_range_to_scanbar_timestamps_ms(value_ms: i64, range_ms: i64) -> String {
    let range_hours = range_ms / 3_600_000;
    let value_hours = value_ms / 3_600_000;
    let range_minutes = (range_ms % 3_600_000) / 60_000;
    let value_minutes = (value_ms % 3_600_000) / 60_000;
    let range_seconds = (range_ms % 60_000) / 1000;
    let value_seconds = (value_ms % 60_000) / 1000;
    let range_ms = range_ms % 1000;
    let value_ms = value_ms % 1000;

    if range_hours > 0 {
        // 0:01:23.033/1:12:57.067
        format!(
            "{value_hours}:{value_minutes:02}:{value_seconds:02}.{value_ms:03}/\
             {range_hours}:{range_minutes:02}:{range_seconds:02}.{range_ms:03}"
        )
    } else if range_minutes > 0 {
        // 01:23.033/12:57.067 or 0:23.033/1:57.067
        if range_minutes > 9 {
            format!(
                "{value_minutes:02}:{value_seconds:02}.{value_ms:03}/\
                 {range_minutes:02}:{range_seconds:02}.{range_ms:03}"
            )
        } else {
            format!(
                "{value_minutes}:{value_seconds:02}.{value_ms:03}/\
                 {range_minutes}:{range_seconds:02}.{range_ms:03}"
            )
        }
    } else if range_seconds > 9 {
        // 23.033/57.067
        format!(
            "{value_seconds:02}.{value_ms:03}/{range_seconds:02}.{range_ms:03}"
        )
    } else {
        // 3.033/7.067 or 0.033/0.067
        format!("{value_seconds}.{value_ms:03}/{range_seconds}.{range_ms:03}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn naive(y: i32, mo: u32, d: u32, h: u32, mi: u32, s: u32) -> NaiveDateTime {
        NaiveDate::from_ymd_opt(y, mo, d)
            .unwrap()
            .and_hms_opt(h, mi, s)
            .unwrap()
    }

    #[test]
    fn pretty_time_formatting() {
        let dt = naive(2020, 1, 2, 3, 4, 5);

        assert_eq!(datetime_to_pretty_time(dt, true, false), "2020-01-02 03:04:05");
        assert_eq!(datetime_to_pretty_time(dt, false, false), "2020-01-02");

        let dt = dt + ChronoDuration::milliseconds(678);

        assert_eq!(
            datetime_to_pretty_time(dt, true, true),
            "2020-01-02 03:04:05.678"
        );
    }

    #[test]
    fn epoch_conversions() {
        let dt = timestamp_to_datetime(0.0, TIMEZONE_UTC);

        assert_eq!(dt, epoch_naive());
        assert_eq!(timestamp_ms_to_datetime(0, TIMEZONE_UTC), epoch_naive());

        let dt = naive(2020, 1, 2, 3, 4, 5);

        let timestamp = datetime_to_timestamp(dt);

        // Only assert the round-trip, since it depends on the local timezone.
        assert_eq!(timestamp_to_datetime(timestamp as f64, TIMEZONE_LOCAL), dt);
    }

    #[test]
    fn millisecond_helpers() {
        assert_eq!(millisecondise_s(Some(1.2345)), Some(1234));
        assert_eq!(millisecondise_s(None), None);

        assert_eq!(secondise_ms(Some(1234.0)), Some(1));
        assert_eq!(secondise_ms(Some(-1.0)), Some(-1));
        assert_eq!(secondise_ms(None), None);

        assert_eq!(secondise_ms_float(Some(1500.0)), Some(1.5));
        assert_eq!(secondise_ms_float(None), None);
    }

    #[test]
    fn timedelta_formatting() {
        assert_eq!(time_delta_to_pretty_timedelta(None, true, false), "per month");
        assert_eq!(time_delta_to_pretty_timedelta(Some(0.0), true, false), "0 seconds");
        assert_eq!(time_delta_to_pretty_timedelta(Some(1.0), true, false), "1 second");
        assert_eq!(time_delta_to_pretty_timedelta(Some(1.5), true, false), "1.5 seconds");
        assert_eq!(time_delta_to_pretty_timedelta(Some(45.67), true, false), "45.7 seconds");

        assert_eq!(
            time_delta_to_pretty_timedelta(Some(90.0), true, false),
            "1 minute 30 seconds"
        );
        assert_eq!(
            time_delta_to_pretty_timedelta(Some(3600.0), true, false),
            "1 hour"
        );
        assert_eq!(
            time_delta_to_pretty_timedelta(Some(SECONDS_IN_YEAR as f64), true, false),
            "1 year"
        );
        assert_eq!(
            time_delta_to_pretty_timedelta(Some(SECONDS_IN_YEAR as f64 * 1.5), true, false),
            "1 year 6 months"
        );

        assert_eq!(
            time_delta_to_pretty_timedelta(Some(0.0015), true, false),
            "1.50 milliseconds"
        );
        assert_eq!(
            time_delta_to_pretty_timedelta(Some(0.0005), true, false),
            "500 microseconds"
        );
        assert_eq!(
            time_delta_to_pretty_timedelta(Some(-90.0), true, false),
            "-1 minute 30 seconds"
        );
    }

    #[test]
    fn durations_formatting() {
        assert_eq!(
            milliseconds_duration_to_pretty_time(None, false),
            "no duration"
        );
        assert_eq!(
            milliseconds_duration_to_pretty_time(Some(0), false),
            "no duration"
        );
        assert_eq!(
            milliseconds_duration_to_pretty_time(Some(3_600_000), false),
            "1 hour 0 minutes"
        );
        assert_eq!(
            milliseconds_duration_to_pretty_time(Some(90_000), false),
            "1 minute 30 seconds"
        );
        assert_eq!(
            milliseconds_duration_to_pretty_time(Some(1500), false),
            "1.5 seconds"
        );
        assert_eq!(
            milliseconds_duration_to_pretty_time(Some(1), true),
            "1 millisecond"
        );
        assert_eq!(
            milliseconds_duration_to_pretty_time(Some(42), true),
            "42 milliseconds"
        );
    }

    #[test]
    fn scanbar_timestamps() {
        assert_eq!(
            value_range_to_scanbar_timestamps_ms(83_033, 4_327_067),
            "0:01:23.033/1:12:07.067"
        );
        assert_eq!(
            value_range_to_scanbar_timestamps_ms(83_033, 777_067),
            "01:23.033/12:57.067"
        );
        assert_eq!(
            value_range_to_scanbar_timestamps_ms(33_033, 57_067),
            "33.033/57.067"
        );
    }

    #[test]
    fn pretty_time_delta_respects_options() {
        let now = get_now();

        let options = PrettyTimeDeltaOptions {
            just_now_threshold: 0,
            force_no_iso: true,
            ..Default::default()
        };

        assert_eq!(
            timestamp_to_pretty_time_delta(Some(now - 90), &options),
            "1 minute 30 seconds ago"
        );

        let future = timestamp_to_pretty_time_delta(Some(now + 90), &options);

        assert!(future.starts_with("in 1 minute 30 seconds"));

        assert_eq!(
            timestamp_to_pretty_time_delta(None, &options),
            "at an unknown time"
        );

        let options = PrettyTimeDeltaOptions {
            just_now_threshold: 300,
            force_no_iso: true,
            ..Default::default()
        };

        assert_eq!(
            timestamp_to_pretty_time_delta(Some(now - 90), &options),
            "now"
        );
    }

    #[test]
    fn expiry_formatting() {
        assert_eq!(timestamp_to_pretty_expires(None), "does not expire");
        assert_eq!(timestamp_to_pretty_expires(Some(0)), "unknown expiration");

        let expired = timestamp_to_pretty_expires(Some(get_now() - 90));

        assert!(expired.starts_with("expired "));

        let unexpired = timestamp_to_pretty_expires(Some(get_now() + 90));

        assert!(unexpired.starts_with("expires "));
    }

    #[test]
    fn calendar_maths() {
        let rough = calendar_delta_to_rough_datetime_timedelta(1, 0, 0, 0);

        assert_eq!(rough.num_days(), 365);

        assert!(calendar_delta_to_datetime(0, 0, 0, 0).is_some());
        assert!(calendar_delta_to_datetime(300_000, 0, 0, 0).is_none());
    }
}
