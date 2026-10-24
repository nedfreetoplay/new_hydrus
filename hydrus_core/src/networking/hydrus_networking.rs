//! Rust port of `hydrus/core/networking/HydrusNetworking.py`.
//!
//! Time accounting is done in UTC seconds, exactly as Python does: a new
//! "day"/"month" is a UTC boundary, so a server can hand a tracker to a client
//! and both agree on when the counters roll over.
//!
//! The Python `collections.Counter` buckets become `HashMap<i64, i64>` and the
//! `threading.Lock` guarding them becomes a `Mutex`. `local_port_in_use` uses a
//! TCP connect probe on every platform; Python only does that on non-Windows
//! because it prefers psutil there, which this port does not have.

use std::collections::{HashMap, HashSet};
use std::net::{Ipv4Addr, SocketAddrV4, TcpStream};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};
use std::time::Duration;

use chrono::{Datelike, NaiveDate, TimeZone, Utc};
use serde_json::{Value, json};

use crate::hydrus_constants::BandwidthType;
use crate::hydrus_data::{convert_value_range_to_bytes, to_human_bytes};
use crate::hydrus_date_time::from_timestamp_utc;
use crate::hydrus_exceptions::{ErrorKind, HydrusException, HydrusResult};
use crate::hydrus_numbers::{base_to_human_int, value_range_to_pretty_string};
use crate::hydrus_serialisable::{
    SERIALISABLE_TYPE_BANDWIDTH_RULES, SERIALISABLE_TYPE_BANDWIDTH_TRACKER, SerialisableObject,
    register_serialisable_type,
};
use crate::hydrus_time::{
    get_now, get_timedelta_until_time, time_delta_to_pretty_timedelta, time_has_passed,
};

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|error| error.into_inner())
}

/// One `( bandwidth_type, time_delta, max_allowed )` tuple.
///
/// `time_delta == None` means "per calendar month".
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct BandwidthRule {
    pub bandwidth_type: BandwidthType,
    pub time_delta: Option<i64>,
    pub max_allowed: i64,
}

impl BandwidthRule {
    pub fn new(bandwidth_type: BandwidthType, time_delta: Option<i64>, max_allowed: i64) -> Self {
        Self {
            bandwidth_type,
            time_delta,
            max_allowed,
        }
    }

    fn from_serialisable_info(info: &Value) -> HydrusResult<Self> {
        let values = info.as_array().ok_or_else(|| {
            HydrusException::new(
                ErrorKind::Serialisation,
                "A bandwidth rule was not an array!",
            )
        })?;

        let (Some(bandwidth_type), Some(time_delta), Some(max_allowed)) =
            (values.first(), values.get(1), values.get(2))
        else {
            return Err(HydrusException::new(
                ErrorKind::Serialisation,
                "A bandwidth rule was not length 3!",
            ));
        };

        let bandwidth_type = bandwidth_type
            .as_u64()
            .and_then(|value| u8::try_from(value).ok())
            .and_then(BandwidthType::try_from_int)
            .ok_or_else(|| {
                HydrusException::new(
                    ErrorKind::Serialisation,
                    "A bandwidth rule had an unknown bandwidth type!",
                )
            })?;

        let time_delta = match time_delta {
            Value::Null => None,
            value => Some(value.as_i64().ok_or_else(|| {
                HydrusException::new(
                    ErrorKind::Serialisation,
                    "A bandwidth rule's time delta was not an integer!",
                )
            })?),
        };

        let max_allowed = max_allowed.as_i64().ok_or_else(|| {
            HydrusException::new(
                ErrorKind::Serialisation,
                "A bandwidth rule's maximum was not an integer!",
            )
        })?;

        Ok(Self::new(bandwidth_type, time_delta, max_allowed))
    }

    fn to_serialisable_info(self) -> Value {
        json!([
            self.bandwidth_type.as_int(),
            self.time_delta,
            self.max_allowed
        ])
    }
}

/// `convert_bandwidth_rule_to_string( rule )`
pub fn convert_bandwidth_rule_to_string(rule: BandwidthRule) -> String {
    if rule.max_allowed == 0 {
        return "No requests currently permitted.".to_string();
    }

    let mut text = match rule.bandwidth_type {
        BandwidthType::Data => to_human_bytes(rule.max_allowed.max(0) as u64),
        BandwidthType::Requests => format!("{} rqs", base_to_human_int(rule.max_allowed)),
    };

    text.push_str(" per ");

    text.push_str(&time_delta_to_pretty_timedelta(
        rule.time_delta.map(|delta| delta as f64),
        true,
        false,
    ));

    text
}

/// `local_port_in_use( port )`
pub fn local_port_in_use(port: u16) -> bool {
    let address = SocketAddrV4::new(Ipv4Addr::LOCALHOST, port);

    match TcpStream::connect_timeout(&address.into(), Duration::from_millis(200)) {
        Ok(stream) => {
            drop(stream);

            true
        }
        Err(_) => false,
    }
}

fn month_start_timestamp(timestamp: i64) -> i64 {
    let datetime = from_timestamp_utc(timestamp as f64);

    let date = NaiveDate::from_ymd_opt(datetime.year(), datetime.month(), 1)
        .expect("the first of a month always exists");

    Utc.from_utc_datetime(&date.and_hms_opt(0, 0, 0).expect("midnight always exists"))
        .timestamp()
}

fn truncate_to_hour(timestamp: i64) -> i64 {
    timestamp - timestamp.rem_euclid(3600)
}

fn truncate_to_minute(timestamp: i64) -> i64 {
    timestamp - timestamp.rem_euclid(60)
}

fn truncate_to_day(timestamp: i64) -> i64 {
    let datetime = from_timestamp_utc(timestamp as f64);

    Utc.from_utc_datetime(
        &NaiveDate::from_ymd_opt(datetime.year(), datetime.month(), datetime.day())
            .expect("the date always exists")
            .and_hms_opt(0, 0, 0)
            .expect("midnight always exists"),
    )
    .timestamp()
}

// ---------------------------------------------------------------------------
// BandwidthRules
// ---------------------------------------------------------------------------

fn rules_registry() -> &'static OnceLock<()> {
    static REGISTERED: OnceLock<()> = OnceLock::new();

    &REGISTERED
}

fn ensure_registered() {
    rules_registry().get_or_init(|| {
        register_serialisable_type(SERIALISABLE_TYPE_BANDWIDTH_RULES, || {
            Box::new(BandwidthRules::new())
        });
        register_serialisable_type(SERIALISABLE_TYPE_BANDWIDTH_TRACKER, || {
            Box::new(BandwidthTracker::new())
        });
    });
}

/// Register `BandwidthRules` and `BandwidthTracker`. Idempotent.
pub fn register_serialisable_types() {
    ensure_registered();
}

/// `BandwidthRules`
pub struct BandwidthRules {
    rules: Mutex<HashSet<BandwidthRule>>,
}

impl Default for BandwidthRules {
    fn default() -> Self {
        Self::new()
    }
}

impl BandwidthRules {
    pub fn new() -> Self {
        ensure_registered();

        Self {
            rules: Mutex::new(HashSet::new()),
        }
    }

    pub fn add_rule(
        &self,
        bandwidth_type: BandwidthType,
        time_delta: Option<i64>,
        max_allowed: i64,
    ) {
        lock(&self.rules).insert(BandwidthRule::new(bandwidth_type, time_delta, max_allowed));
    }

    /// `can_continue_download( bandwidth_tracker, threshold = 15 )`
    pub fn can_continue_download(
        &self,
        bandwidth_tracker: &BandwidthTracker,
        threshold: i64,
    ) -> bool {
        for rule in lock(&self.rules).iter().copied() {
            // Do not stop ongoing work just because starts are throttled.
            let requests_rule = rule.bandwidth_type == BandwidthType::Requests;

            // Do not block an ongoing jpg download because the current month is
            // 100.03% used.
            let wait_is_too_long = rule.time_delta.is_none_or(|delta| delta > threshold);

            if requests_rule || wait_is_too_long {
                continue;
            }

            if bandwidth_tracker.get_usage(rule.bandwidth_type, rule.time_delta, false)
                >= rule.max_allowed
            {
                return false;
            }
        }

        true
    }

    /// `can_do_work( bandwidth_tracker, expected_requests, expected_bytes, threshold = 30 )`
    pub fn can_do_work(
        &self,
        bandwidth_tracker: &BandwidthTracker,
        expected_requests: i64,
        expected_bytes: i64,
        threshold: i64,
    ) -> bool {
        for rule in lock(&self.rules).iter().copied() {
            // Do not prohibit a raft of work because one small rule is over at
            // this current second.
            if rule.time_delta.is_some_and(|delta| delta <= threshold) {
                continue;
            }

            let adjusted_max = match rule.bandwidth_type {
                BandwidthType::Requests => rule.max_allowed - expected_requests,
                BandwidthType::Data => rule.max_allowed - expected_bytes,
            };

            if bandwidth_tracker.get_usage(rule.bandwidth_type, rule.time_delta, false)
                >= adjusted_max
            {
                return false;
            }
        }

        true
    }

    /// `can_start_request( bandwidth_tracker, threshold = 5 )`
    pub fn can_start_request(&self, bandwidth_tracker: &BandwidthTracker, threshold: i64) -> bool {
        for rule in lock(&self.rules).iter().copied() {
            // Do not prohibit a new job just because the current download speed
            // is 210/200KB/s.
            let ignore_rule = rule.bandwidth_type == BandwidthType::Data
                && rule.time_delta.is_some_and(|delta| delta <= threshold);

            if ignore_rule {
                continue;
            }

            if bandwidth_tracker.get_usage(rule.bandwidth_type, rule.time_delta, false)
                >= rule.max_allowed
            {
                return false;
            }
        }

        true
    }

    /// `get_waiting_estimate( bandwidth_tracker )`
    pub fn get_waiting_estimate(&self, bandwidth_tracker: &BandwidthTracker) -> i64 {
        let mut estimates = Vec::new();

        for rule in lock(&self.rules).iter().copied() {
            if bandwidth_tracker.get_usage(rule.bandwidth_type, rule.time_delta, false)
                >= rule.max_allowed
            {
                estimates.push(bandwidth_tracker.get_waiting_estimate(
                    rule.bandwidth_type,
                    rule.time_delta,
                    rule.max_allowed,
                ));
            }
        }

        estimates.into_iter().max().unwrap_or(0)
    }

    /// `get_bandwidth_strings_and_gauge_tuples( bandwidth_tracker, threshold = 600 )`
    pub fn get_bandwidth_strings_and_gauge_tuples(
        &self,
        bandwidth_tracker: &BandwidthTracker,
        threshold: i64,
    ) -> Vec<(String, (i64, i64))> {
        let mut rules: Vec<BandwidthRule> = lock(&self.rules).iter().copied().collect();

        rules.sort_by_key(|rule| rule.time_delta.unwrap_or(-1));

        let mut rows = Vec::new();

        for rule in rules {
            let time_is_less_than_threshold =
                rule.time_delta.is_some_and(|delta| delta <= threshold);

            if time_is_less_than_threshold || rule.max_allowed == 0 {
                continue;
            }

            let usage = bandwidth_tracker.get_usage(rule.bandwidth_type, rule.time_delta, false);

            let mut text = "used ".to_string();

            match rule.bandwidth_type {
                BandwidthType::Data => text.push_str(&convert_value_range_to_bytes(
                    usage.max(0) as u64,
                    rule.max_allowed.max(0) as u64,
                )),
                BandwidthType::Requests => text.push_str(&format!(
                    "{} requests",
                    value_range_to_pretty_string(
                        Some(usage as i128),
                        Some(rule.max_allowed as i128)
                    )
                )),
            }

            match rule.time_delta {
                None => text.push_str(" this month"),
                Some(delta) => {
                    text.push_str(" in the past ");
                    text.push_str(&time_delta_to_pretty_timedelta(
                        Some(delta as f64),
                        true,
                        false,
                    ));
                }
            }

            rows.push((text, (usage, rule.max_allowed)));
        }

        rows
    }

    pub fn get_rules(&self) -> Vec<BandwidthRule> {
        lock(&self.rules).iter().copied().collect()
    }
}

impl SerialisableObject for BandwidthRules {
    fn serialisable_type(&self) -> u16 {
        SERIALISABLE_TYPE_BANDWIDTH_RULES
    }

    fn serialisable_version(&self) -> u16 {
        1
    }

    fn serialisable_name(&self) -> &'static str {
        "Bandwidth Rules"
    }

    fn get_serialisable_info(&self) -> Value {
        let mut rules: Vec<BandwidthRule> = lock(&self.rules).iter().copied().collect();

        rules.sort();

        Value::Array(
            rules
                .into_iter()
                .map(BandwidthRule::to_serialisable_info)
                .collect(),
        )
    }

    fn initialise_from_serialisable_info(&mut self, serialisable_info: &Value) -> HydrusResult<()> {
        let rule_values = serialisable_info.as_array().ok_or_else(|| {
            HydrusException::new(
                ErrorKind::Serialisation,
                "Bandwidth rules serialisable info was not an array!",
            )
        })?;

        let mut rules = HashSet::new();

        for rule_value in rule_values {
            rules.insert(BandwidthRule::from_serialisable_info(rule_value)?);
        }

        *lock(&self.rules) = rules;

        Ok(())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

// ---------------------------------------------------------------------------
// BandwidthTracker
// ---------------------------------------------------------------------------

const MAX_SECONDS_TIME_DELTA: i64 = 240;
const MAX_MINUTES_TIME_DELTA: i64 = 180 * 60;
const MAX_HOURS_TIME_DELTA: i64 = 72 * 3600;
const MAX_DAYS_TIME_DELTA: i64 = 31 * 86400;

const CACHE_MAINTENANCE_TIME_DELTA: i64 = 120;

const MIN_TIME_DELTA_FOR_USER: i64 = 10;

#[derive(Clone, Copy)]
enum CounterWindow {
    Seconds,
    Minutes,
    Hours,
    Days,
}

struct TrackerState {
    next_cache_maintenance_timestamp: i64,

    months_bytes: HashMap<i64, i64>,
    days_bytes: HashMap<i64, i64>,
    hours_bytes: HashMap<i64, i64>,
    minutes_bytes: HashMap<i64, i64>,
    seconds_bytes: HashMap<i64, i64>,

    months_requests: HashMap<i64, i64>,
    days_requests: HashMap<i64, i64>,
    hours_requests: HashMap<i64, i64>,
    minutes_requests: HashMap<i64, i64>,
    seconds_requests: HashMap<i64, i64>,
}

impl TrackerState {
    fn new() -> Self {
        Self {
            next_cache_maintenance_timestamp: get_now() + CACHE_MAINTENANCE_TIME_DELTA,
            months_bytes: HashMap::new(),
            days_bytes: HashMap::new(),
            hours_bytes: HashMap::new(),
            minutes_bytes: HashMap::new(),
            seconds_bytes: HashMap::new(),
            months_requests: HashMap::new(),
            days_requests: HashMap::new(),
            hours_requests: HashMap::new(),
            minutes_requests: HashMap::new(),
            seconds_requests: HashMap::new(),
        }
    }

    fn window(&self, time_delta: i64) -> (i64, CounterWindow) {
        let (window, kind) = if time_delta < MAX_SECONDS_TIME_DELTA {
            (0, CounterWindow::Seconds)
        } else if time_delta < MAX_MINUTES_TIME_DELTA {
            (59, CounterWindow::Minutes)
        } else if time_delta < MAX_HOURS_TIME_DELTA {
            (3599, CounterWindow::Hours)
        } else {
            (86399, CounterWindow::Days)
        };

        (window, kind)
    }

    fn counter(&self, bandwidth_type: BandwidthType, kind: CounterWindow) -> &HashMap<i64, i64> {
        match (bandwidth_type, kind) {
            (BandwidthType::Data, CounterWindow::Seconds) => &self.seconds_bytes,
            (BandwidthType::Data, CounterWindow::Minutes) => &self.minutes_bytes,
            (BandwidthType::Data, CounterWindow::Hours) => &self.hours_bytes,
            (BandwidthType::Data, CounterWindow::Days) => &self.days_bytes,
            (BandwidthType::Requests, CounterWindow::Seconds) => &self.seconds_requests,
            (BandwidthType::Requests, CounterWindow::Minutes) => &self.minutes_requests,
            (BandwidthType::Requests, CounterWindow::Hours) => &self.hours_requests,
            (BandwidthType::Requests, CounterWindow::Days) => &self.days_requests,
        }
    }

    fn raw_usage(&self, bandwidth_type: BandwidthType, time_delta: Option<i64>) -> i64 {
        let Some(time_delta) = time_delta else {
            let month_time = month_start_timestamp(get_now());

            return match bandwidth_type {
                BandwidthType::Data => *self.months_bytes.get(&month_time).unwrap_or(&0),
                BandwidthType::Requests => *self.months_requests.get(&month_time).unwrap_or(&0),
            };
        };

        let (window, kind) = self.window(time_delta);

        let counter = self.counter(bandwidth_type, kind);

        if time_delta == 1 {
            // Checking the current second is good enough; including the previous
            // one makes consumption lag by up to a second.
            return *counter.get(&get_now()).unwrap_or(&0);
        }

        let now = get_now();
        let since = now - (time_delta + window);

        counter
            .iter()
            .filter(|(timestamp, _)| since <= **timestamp && **timestamp <= now)
            .map(|(_, value)| *value)
            .sum()
    }

    fn all_usage(&self, bandwidth_type: BandwidthType) -> i64 {
        match bandwidth_type {
            BandwidthType::Data => self.months_bytes.values().sum(),
            BandwidthType::Requests => self.months_requests.values().sum(),
        }
    }

    fn weighted_approximate_usage(&self, time_delta: i64) -> i64 {
        let now = get_now();
        let since = now - MIN_TIME_DELTA_FOR_USER;

        let valid_timestamps: Vec<i64> = self
            .seconds_bytes
            .keys()
            .copied()
            .filter(|timestamp| since <= *timestamp && *timestamp <= now)
            .collect();

        if valid_timestamps.is_empty() {
            return 0;
        }

        // If we want the average speed over the past five seconds but nothing
        // happened in seconds 4 and 5, we do not want to count them.
        let earliest_timestamp = valid_timestamps.iter().copied().min().unwrap_or(now);

        let sample_delta = (now - earliest_timestamp).max(1);

        let total_bytes: i64 = valid_timestamps
            .iter()
            .map(|timestamp| *self.seconds_bytes.get(timestamp).unwrap_or(&0))
            .sum();

        let average_per_sec = total_bytes as f64 / sample_delta as f64;

        (average_per_sec * time_delta as f64) as i64
    }

    fn maintain_cache_if_due(&mut self) {
        if !time_has_passed(Some(self.next_cache_maintenance_timestamp)) {
            return;
        }

        let now = get_now();

        let oldest_second = now - MAX_SECONDS_TIME_DELTA;
        let oldest_minute = now - MAX_MINUTES_TIME_DELTA;
        let oldest_hour = now - MAX_HOURS_TIME_DELTA;
        let oldest_day = now - MAX_DAYS_TIME_DELTA;

        fn clear_counter(counter: &mut HashMap<i64, i64>, oldest_timestamp: i64) {
            counter.retain(|timestamp, _| *timestamp >= oldest_timestamp);
        }

        clear_counter(&mut self.days_bytes, oldest_day);
        clear_counter(&mut self.days_requests, oldest_day);
        clear_counter(&mut self.hours_bytes, oldest_hour);
        clear_counter(&mut self.hours_requests, oldest_hour);
        clear_counter(&mut self.minutes_bytes, oldest_minute);
        clear_counter(&mut self.minutes_requests, oldest_minute);
        clear_counter(&mut self.seconds_bytes, oldest_second);
        clear_counter(&mut self.seconds_requests, oldest_second);

        self.next_cache_maintenance_timestamp = get_now() + CACHE_MAINTENANCE_TIME_DELTA;
    }

    fn usage(
        &mut self,
        bandwidth_type: BandwidthType,
        time_delta: Option<i64>,
        for_user: bool,
    ) -> i64 {
        let usage = if for_user && time_delta.is_some_and(|delta| delta <= MIN_TIME_DELTA_FOR_USER)
        {
            self.weighted_approximate_usage(time_delta.unwrap_or(0))
        } else {
            self.raw_usage(bandwidth_type, time_delta)
        };

        self.maintain_cache_if_due();

        usage
    }

    fn times(&self, timestamp: i64) -> (i64, i64, i64, i64, i64) {
        (
            month_start_timestamp(timestamp),
            truncate_to_day(timestamp),
            truncate_to_hour(timestamp),
            truncate_to_minute(timestamp),
            timestamp,
        )
    }

    fn report(&mut self, bandwidth_type: BandwidthType, amount: i64) {
        let (month_time, day_time, hour_time, minute_time, second_time) = self.times(get_now());

        let (months, days, hours, minutes, seconds) = match bandwidth_type {
            BandwidthType::Data => (
                &mut self.months_bytes,
                &mut self.days_bytes,
                &mut self.hours_bytes,
                &mut self.minutes_bytes,
                &mut self.seconds_bytes,
            ),
            BandwidthType::Requests => (
                &mut self.months_requests,
                &mut self.days_requests,
                &mut self.hours_requests,
                &mut self.minutes_requests,
                &mut self.seconds_requests,
            ),
        };

        *months.entry(month_time).or_insert(0) += amount;
        *days.entry(day_time).or_insert(0) += amount;
        *hours.entry(hour_time).or_insert(0) += amount;
        *minutes.entry(minute_time).or_insert(0) += amount;
        *seconds.entry(second_time).or_insert(0) += amount;

        self.maintain_cache_if_due();
    }
}

/// `BandwidthTracker`
pub struct BandwidthTracker {
    state: Mutex<TrackerState>,
}

impl Default for BandwidthTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl BandwidthTracker {
    pub fn new() -> Self {
        ensure_registered();

        Self {
            state: Mutex::new(TrackerState::new()),
        }
    }

    /// `get_current_month_summary()`
    pub fn get_current_month_summary(&self) -> String {
        let mut state = lock(&self.state);

        let num_bytes = state.usage(BandwidthType::Data, None, true);
        let num_requests = state.usage(BandwidthType::Requests, None, true);

        format!(
            "used {} in {} requests this month",
            to_human_bytes(num_bytes.max(0) as u64),
            base_to_human_int(num_requests)
        )
    }

    /// `get_monthly_data_usage()`
    pub fn get_monthly_data_usage(&self) -> Vec<(String, i64)> {
        let state = lock(&self.state);

        let mut result: Vec<(String, i64)> = state
            .months_bytes
            .iter()
            .map(|(month_time, usage)| {
                (
                    from_timestamp_utc(*month_time as f64)
                        .format("%Y-%m")
                        .to_string(),
                    *usage,
                )
            })
            .collect();

        result.sort();

        result
    }

    /// `get_all_usage( bandwidth_type )`
    pub fn get_all_usage(&self, bandwidth_type: BandwidthType) -> i64 {
        lock(&self.state).all_usage(bandwidth_type)
    }

    /// `get_usage( bandwidth_type, time_delta, for_user = False )`
    pub fn get_usage(
        &self,
        bandwidth_type: BandwidthType,
        time_delta: Option<i64>,
        for_user: bool,
    ) -> i64 {
        if time_delta == Some(0) {
            return 0;
        }

        lock(&self.state).usage(bandwidth_type, time_delta, for_user)
    }

    /// `get_waiting_estimate( bandwidth_type, time_delta, max_allowed )`
    pub fn get_waiting_estimate(
        &self,
        bandwidth_type: BandwidthType,
        time_delta: Option<i64>,
        max_allowed: i64,
    ) -> i64 {
        let state = lock(&self.state);

        let Some(time_delta) = time_delta else {
            // Monthly: wait until the start of the next month.
            let datetime = from_timestamp_utc(get_now() as f64);

            let (next_month_year, next_month) = if datetime.month() == 12 {
                (datetime.year() + 1, 1)
            } else {
                (datetime.year(), datetime.month() + 1)
            };

            let next_month_date = NaiveDate::from_ymd_opt(next_month_year, next_month, 1)
                .expect("the first of a month always exists");

            let next_month_time = Utc
                .from_utc_datetime(
                    &next_month_date
                        .and_hms_opt(0, 0, 0)
                        .expect("midnight always exists"),
                )
                .timestamp();

            return get_timedelta_until_time(next_month_time);
        };

        // We want the highest time delta at which usage is still >= max_allowed.
        let (window, kind) = state.window(time_delta);

        let time_delta_in_which_bandwidth_counts = time_delta + window;

        let counter = state.counter(bandwidth_type, kind);

        let mut time_and_values: Vec<(i64, i64)> = counter
            .iter()
            .map(|(time, value)| (*time, *value))
            .collect();

        time_and_values.sort_by(|a, b| b.cmp(a));

        let now = get_now();
        let mut usage = 0;

        for (timestamp, value) in time_and_values {
            let current_search_time_delta = now - timestamp;

            if current_search_time_delta > time_delta_in_which_bandwidth_counts {
                break;
            }

            usage += value;

            if usage >= max_allowed {
                return time_delta_in_which_bandwidth_counts - current_search_time_delta;
            }
        }

        0
    }

    /// `report_data_used( num_bytes )`
    pub fn report_data_used(&self, num_bytes: i64) {
        lock(&self.state).report(BandwidthType::Data, num_bytes);
    }

    /// `report_request_used( num_requests = 1 )`
    pub fn report_request_used(&self, num_requests: i64) {
        lock(&self.state).report(BandwidthType::Requests, num_requests);
    }
}

impl SerialisableObject for BandwidthTracker {
    fn serialisable_type(&self) -> u16 {
        SERIALISABLE_TYPE_BANDWIDTH_TRACKER
    }

    fn serialisable_version(&self) -> u16 {
        1
    }

    fn serialisable_name(&self) -> &'static str {
        "Bandwidth Tracker"
    }

    fn get_serialisable_info(&self) -> Value {
        let state = lock(&self.state);

        fn flat(counter: &HashMap<i64, i64>) -> Value {
            let mut entries: Vec<(i64, i64)> = counter
                .iter()
                .map(|(time, value)| (*time, *value))
                .collect();

            entries.sort();

            Value::Array(
                entries
                    .into_iter()
                    .map(|(time, value)| json!([time, value]))
                    .collect(),
            )
        }

        json!([
            flat(&state.months_bytes),
            flat(&state.days_bytes),
            flat(&state.hours_bytes),
            flat(&state.minutes_bytes),
            flat(&state.seconds_bytes),
            flat(&state.months_requests),
            flat(&state.days_requests),
            flat(&state.hours_requests),
            flat(&state.minutes_requests),
            flat(&state.seconds_requests),
        ])
    }

    fn initialise_from_serialisable_info(&mut self, serialisable_info: &Value) -> HydrusResult<()> {
        let counters = serialisable_info.as_array().ok_or_else(|| {
            HydrusException::new(
                ErrorKind::Serialisation,
                "Bandwidth tracker serialisable info was not an array!",
            )
        })?;

        // An unusual error someone reported by email: it came back empty.
        if counters.len() != 10 {
            return Ok(());
        }

        fn parse_counter(value: &Value) -> HydrusResult<HashMap<i64, i64>> {
            let entries = value.as_array().ok_or_else(|| {
                HydrusException::new(
                    ErrorKind::Serialisation,
                    "A bandwidth tracker counter was not an array!",
                )
            })?;

            let mut counter = HashMap::new();

            for entry in entries {
                let pair = entry.as_array().ok_or_else(|| {
                    HydrusException::new(
                        ErrorKind::Serialisation,
                        "A bandwidth tracker counter entry was not an array!",
                    )
                })?;

                let (Some(timestamp), Some(amount)) = (pair.first(), pair.get(1)) else {
                    return Err(HydrusException::new(
                        ErrorKind::Serialisation,
                        "A bandwidth tracker counter entry was not length 2!",
                    ));
                };

                let timestamp = timestamp.as_i64().ok_or_else(|| {
                    HydrusException::new(
                        ErrorKind::Serialisation,
                        "A bandwidth tracker timestamp was not an integer!",
                    )
                })?;

                let amount = amount.as_i64().ok_or_else(|| {
                    HydrusException::new(
                        ErrorKind::Serialisation,
                        "A bandwidth tracker amount was not an integer!",
                    )
                })?;

                counter.insert(timestamp, amount);
            }

            Ok(counter)
        }

        let mut parsed = Vec::with_capacity(10);

        for counter in counters {
            parsed.push(parse_counter(counter)?);
        }

        let mut state = lock(&self.state);

        state.months_bytes = parsed.remove(0);
        state.days_bytes = parsed.remove(0);
        state.hours_bytes = parsed.remove(0);
        state.minutes_bytes = parsed.remove(0);
        state.seconds_bytes = parsed.remove(0);
        state.months_requests = parsed.remove(0);
        state.days_requests = parsed.remove(0);
        state.hours_requests = parsed.remove(0);
        state.minutes_requests = parsed.remove(0);
        state.seconds_requests = parsed.remove(0);

        Ok(())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

/// Conveniences matching Python's default arguments.
impl BandwidthRules {
    pub fn can_continue_download_default(&self, bandwidth_tracker: &BandwidthTracker) -> bool {
        self.can_continue_download(bandwidth_tracker, 15)
    }

    pub fn can_start_request_default(&self, bandwidth_tracker: &BandwidthTracker) -> bool {
        self.can_start_request(bandwidth_tracker, 5)
    }
}

/// Shared handles, the Rust stand-in for the mutable objects Python stores in
/// service dictionaries.
pub type SharedBandwidthRules = Arc<BandwidthRules>;
pub type SharedBandwidthTracker = Arc<BandwidthTracker>;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hydrus_serialisable::{create_from_string, python_json_dumps};

    #[test]
    fn rule_strings_match_python_style() {
        assert_eq!(
            convert_bandwidth_rule_to_string(BandwidthRule::new(BandwidthType::Data, None, 0)),
            "No requests currently permitted."
        );

        let rule = BandwidthRule::new(BandwidthType::Data, None, 1024 * 1024);

        let text = convert_bandwidth_rule_to_string(rule);

        assert!(text.ends_with(" per month"), "got {text}");

        let rule = BandwidthRule::new(BandwidthType::Requests, Some(3600), 50);

        let text = convert_bandwidth_rule_to_string(rule);

        assert!(text.starts_with("50 rqs per "), "got {text}");
    }

    #[test]
    fn tracker_reports_and_queries_usage() {
        let tracker = BandwidthTracker::new();

        assert_eq!(tracker.get_usage(BandwidthType::Data, Some(0), false), 0);
        assert_eq!(tracker.get_all_usage(BandwidthType::Data), 0);

        tracker.report_data_used(1000);
        tracker.report_request_used(3);

        assert_eq!(tracker.get_usage(BandwidthType::Data, Some(1), false), 1000);
        assert_eq!(
            tracker.get_usage(BandwidthType::Data, Some(60), false),
            1000
        );
        assert_eq!(tracker.get_usage(BandwidthType::Data, None, false), 1000);
        assert_eq!(tracker.get_all_usage(BandwidthType::Data), 1000);

        assert_eq!(
            tracker.get_usage(BandwidthType::Requests, Some(60), false),
            3
        );
        assert_eq!(tracker.get_all_usage(BandwidthType::Requests), 3);

        // The user-facing short window is an approximation, but it should be
        // close to the actual amount.
        let user_usage = tracker.get_usage(BandwidthType::Data, Some(10), true);

        // Python extrapolates the per-second average over the requested window,
        // so a single 1000-byte second becomes 10,000 bytes over ten seconds.
        assert!((1000..=10000).contains(&user_usage), "got {user_usage}");
    }

    #[test]
    fn monthly_summary_and_history() {
        let tracker = BandwidthTracker::new();

        tracker.report_data_used(2048);
        tracker.report_request_used(2);

        let summary = tracker.get_current_month_summary();

        assert!(summary.contains("this month"), "got {summary}");

        let history = tracker.get_monthly_data_usage();

        assert_eq!(history.len(), 1);
        assert_eq!(history[0].1, 2048);
    }

    #[test]
    fn rules_gate_work_as_python_does() {
        let tracker = BandwidthTracker::new();
        let rules = BandwidthRules::new();

        // Monthly data rule: ignored by can_continue_download (wait too long),
        // honoured by can_start_request.
        rules.add_rule(BandwidthType::Data, None, 1000);

        assert!(rules.can_continue_download(&tracker, 15));
        assert!(rules.can_start_request(&tracker, 5));

        tracker.report_data_used(1000);

        assert!(rules.can_continue_download(&tracker, 15));
        assert!(!rules.can_start_request(&tracker, 5));

        // A short data rule throttles starts but not ongoing downloads.
        let rules = BandwidthRules::new();

        rules.add_rule(BandwidthType::Data, Some(60), 1000);

        tracker.report_data_used(500);

        assert!(!rules.can_start_request(&tracker, 5));
        assert!(rules.can_continue_download(&tracker, 15));

        // can_do_work only considers rules longer than its threshold.
        assert!(!rules.can_do_work(&tracker, 0, 0, 30));
        assert!(rules.can_do_work(&tracker, 0, 0, 60));
    }

    #[test]
    fn waiting_estimates_are_non_negative() {
        let tracker = BandwidthTracker::new();
        let rules = BandwidthRules::new();

        rules.add_rule(BandwidthType::Requests, Some(60), 1);
        tracker.report_request_used(1);

        assert!(!rules.can_start_request(&tracker, 5));

        let estimate = rules.get_waiting_estimate(&tracker);

        assert!(estimate >= 0, "got {estimate}");

        let monthly_estimate = tracker.get_waiting_estimate(BandwidthType::Data, None, 1);

        assert!(monthly_estimate >= 0);
    }

    #[test]
    fn bandwidth_rows_skip_small_rules() {
        let tracker = BandwidthTracker::new();
        let rules = BandwidthRules::new();

        rules.add_rule(BandwidthType::Data, Some(60), 500);
        rules.add_rule(BandwidthType::Data, Some(3600), 5000);
        rules.add_rule(BandwidthType::Requests, None, 100);

        tracker.report_data_used(100);
        tracker.report_request_used(2);

        let rows = rules.get_bandwidth_strings_and_gauge_tuples(&tracker, 600);

        assert_eq!(rows.len(), 2);

        for (text, (usage, max)) in rows {
            assert!(text.starts_with("used "), "got {text}");
            assert!(usage <= max);
        }
    }

    #[test]
    fn rules_serialise_and_load() {
        let rules = BandwidthRules::new();

        rules.add_rule(BandwidthType::Data, None, 1024);
        rules.add_rule(BandwidthType::Data, Some(60), 256);
        rules.add_rule(BandwidthType::Requests, Some(3600), 10);

        let dumped = rules.dump_to_string();

        let loaded = create_from_string(&dumped, false).unwrap();

        let loaded_rules = loaded
            .as_any()
            .downcast_ref::<BandwidthRules>()
            .expect("loaded object should be BandwidthRules");

        let mut original: Vec<BandwidthRule> = rules.get_rules();
        let mut reloaded: Vec<BandwidthRule> = loaded_rules.get_rules();

        original.sort();
        reloaded.sort();

        assert_eq!(original, reloaded);
    }

    #[test]
    fn tracker_serialises_and_loads() {
        let tracker = BandwidthTracker::new();

        tracker.report_data_used(123);
        tracker.report_request_used(4);

        let dumped = tracker.dump_to_string();

        let loaded = create_from_string(&dumped, false).unwrap();

        let loaded_tracker = loaded
            .as_any()
            .downcast_ref::<BandwidthTracker>()
            .expect("loaded object should be BandwidthTracker");

        assert_eq!(
            tracker.get_all_usage(BandwidthType::Data),
            loaded_tracker.get_all_usage(BandwidthType::Data)
        );
        assert_eq!(
            tracker.get_all_usage(BandwidthType::Requests),
            loaded_tracker.get_all_usage(BandwidthType::Requests)
        );

        // A broken ten-counter list is tolerated, as in Python.
        let mut broken = BandwidthTracker::new();
        broken
            .initialise_from_serialisable_info(&json!([]))
            .unwrap();
    }

    #[test]
    fn plain_json_dump_has_the_python_shape() {
        let rules = BandwidthRules::new();
        rules.add_rule(BandwidthType::Requests, Some(60), 5);

        let dump = python_json_dumps(&rules.get_serialisable_tuple());

        assert!(dump.starts_with("[38, 1, [["), "got {dump}");
    }
}
