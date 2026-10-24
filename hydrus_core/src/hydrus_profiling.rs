//! Rust port of `hydrus/core/HydrusProfiling.py`.
//!
//! Python uses `cProfile`/`pstats` to dump call-level reports. There is no
//! stable equivalent in the Rust standard library, so the port keeps the mode
//! bookkeeping and the slow/fast job counters, times jobs with
//! [`std::time::Instant`], and logs the summary. `profile_text` stays `None`.

use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, AtomicI64, AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

use chrono::Local;

use crate::hydrus_data::safe_print as show_text;
use crate::hydrus_globals::db_dir;
use crate::hydrus_numbers::to_human_int;
use crate::hydrus_time::{get_now, time_delta_to_pretty_timedelta};

static PROFILE_MODE: AtomicBool = AtomicBool::new(false);
static PROFILE_MODE_NAME: OnceLock<Mutex<String>> = OnceLock::new();
static PROFILE_START_TIME: AtomicI64 = AtomicI64::new(0);
static PROFILE_SLOW_COUNT: AtomicU64 = AtomicU64::new(0);
static PROFILE_FAST_COUNT: AtomicU64 = AtomicU64::new(0);
static CURRENT_PROFILE_LOCK: Mutex<()> = Mutex::new(());

static QUERY_PLANNER_MODE: AtomicBool = AtomicBool::new(false);
static QUERY_PLANNER_START_TIME: AtomicI64 = AtomicI64::new(0);
static QUERY_PLANNER_QUERY_COUNT: AtomicU64 = AtomicU64::new(0);
static QUERIES_PLANNED: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();

fn profile_mode_name_cell() -> &'static Mutex<String> {
    PROFILE_MODE_NAME.get_or_init(|| Mutex::new(String::new()))
}

fn queries_planned() -> &'static Mutex<HashSet<String>> {
    QUERIES_PLANNED.get_or_init(|| Mutex::new(HashSet::new()))
}

fn timestamp_phrase(timestamp: i64, phrase: &str) -> String {
    use chrono::TimeZone as _;

    Local
        .timestamp_opt(timestamp, 0)
        .single()
        .map(|dt| dt.format(phrase).to_string())
        .unwrap_or_else(|| "unknown-time".to_string())
}

/// `profile_mode` as a readable flag.
pub fn profile_mode() -> bool {
    PROFILE_MODE.load(Ordering::Relaxed)
}

/// `profile_mode_name`
pub fn profile_mode_name() -> String {
    profile_mode_name_cell().lock().unwrap().clone()
}

/// `flip_profile_mode( name )`
pub fn flip_profile_mode(name: &str) {
    if profile_mode() {
        let start_new_mode = profile_mode_name() != name;

        stop_profile_mode();

        if start_new_mode {
            start_profile_mode(name);
        }
    } else {
        start_profile_mode(name);
    }
}

/// `flip_query_planner_mode()`
pub fn flip_query_planner_mode() {
    if !QUERY_PLANNER_MODE.load(Ordering::Relaxed) {
        QUERY_PLANNER_START_TIME.store(get_now(), Ordering::Relaxed);
        QUERY_PLANNER_QUERY_COUNT.store(0, Ordering::Relaxed);

        QUERY_PLANNER_MODE.store(true, Ordering::Relaxed);

        show_text("Query Planner mode on!");
    } else {
        QUERY_PLANNER_MODE.store(false, Ordering::Relaxed);

        queries_planned().lock().unwrap().clear();

        let count = QUERY_PLANNER_QUERY_COUNT.load(Ordering::Relaxed);

        show_text(format!(
            "Query Planning done: {} queries analyzed",
            to_human_int(count as i128)
        ));
    }
}

/// `is_profile_mode( name )`
pub fn is_profile_mode(name: &str) -> bool {
    profile_mode() && profile_mode_name() == name
}

/// `query_planner_mode`
pub fn query_planner_mode() -> bool {
    QUERY_PLANNER_MODE.load(Ordering::Relaxed)
}

/// `query in queries_planned`
pub fn query_was_planned(query: &str) -> bool {
    queries_planned().lock().unwrap().contains(query)
}

/// `print_profile( summary, profile_text = None )`
pub fn print_profile(summary: &str, profile_text: Option<&str>) {
    let Some(db_dir) = db_dir() else {
        show_text(summary);

        return;
    };

    let name = crate::hydrus_globals::program_name();

    let pretty_timestamp = timestamp_phrase(
        PROFILE_START_TIME.load(Ordering::Relaxed),
        "%Y-%m-%d %H-%M-%S",
    );

    let profile_log_filename = format!(
        "{name} profile ({}) - {pretty_timestamp}.log",
        profile_mode_name()
    );

    let profile_log_path = db_dir.join(profile_log_filename);

    let prefix = Local::now().format("%Y-%m-%d %H:%M:%S: ").to_string();

    let mut text = format!("{prefix}{summary}");

    if let Some(profile_text) = profile_text {
        text.push_str("\n\n");
        text.push_str(profile_text);
    }

    if let Err(error) = append_to_file(&profile_log_path, &text) {
        show_text(format!(
            "Could not write profile log to \"{}\": {error}",
            profile_log_path.display()
        ));
    }
}

fn append_to_file(path: &std::path::Path, text: &str) -> std::io::Result<()> {
    use std::io::Write as _;

    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;

    file.write_all(text.as_bytes())
}

/// `print_query_plan( query, plan_lines )`
pub fn print_query_plan(query: &str, plan_lines: &[String]) {
    QUERY_PLANNER_QUERY_COUNT.fetch_add(1, Ordering::Relaxed);

    {
        let mut queries = queries_planned().lock().unwrap();

        if !queries.insert(query.to_string()) {
            return;
        }
    }

    let name = crate::hydrus_globals::program_name();

    let pretty_timestamp = timestamp_phrase(
        QUERY_PLANNER_START_TIME.load(Ordering::Relaxed),
        "%Y-%m-%d %H-%M-%S",
    );

    let query_planner_log_filename = format!("{name} query planner - {pretty_timestamp}.log");

    let Some(db_dir) = db_dir() else {
        show_text(format!("Query plan for: {query}"));

        return;
    };

    let query_planner_log_path = db_dir.join(query_planner_log_filename);

    let prefix = Local::now().format("%Y-%m-%d %H:%M:%S: ").to_string();

    let first_word = query.split(' ').next().unwrap_or("unknown");

    let mut text = format!("{prefix}{first_word}\n{query}");

    if !plan_lines.is_empty() {
        text.push('\n');
        text.push_str(&plan_lines.join("\n"));
    }

    text.push_str("\n\n");

    if let Err(error) = append_to_file(&query_planner_log_path, &text) {
        show_text(format!(
            "Could not write query planner log to \"{}\": {error}",
            query_planner_log_path.display()
        ));
    }
}

/// `profile( summary, func, min_duration_ms = 20, show_summary = False )`
pub fn profile(summary: &str, func: impl FnOnce(), min_duration_ms: u64, show_summary: bool) {
    let started = Instant::now();

    if let Ok(_guard) = CURRENT_PROFILE_LOCK.try_lock() {
        func();

        let time_took = started.elapsed().as_secs_f64();
        let time_took_ms = (time_took * 1000.0) as u64;

        if time_took_ms > min_duration_ms {
            PROFILE_SLOW_COUNT.fetch_add(1, Ordering::Relaxed);

            if show_summary {
                show_text(summary);
            }

            print_profile(summary, None);
        } else {
            PROFILE_FAST_COUNT.fetch_add(1, Ordering::Relaxed);

            if show_summary {
                print_profile(&format!("{summary}\n\n"), None);
            }
        }
    } else {
        func();

        let time_took = started.elapsed().as_secs_f64();
        let time_took_ms = (time_took * 1000.0) as u64;

        if time_took_ms > min_duration_ms {
            PROFILE_SLOW_COUNT.fetch_add(1, Ordering::Relaxed);
        } else {
            PROFILE_FAST_COUNT.fetch_add(1, Ordering::Relaxed);
        }

        print_profile(
            &format!(
                "EXCLUSIVE: {summary} took {}\n\n",
                time_delta_to_pretty_timedelta(Some(time_took), true, false)
            ),
            None,
        );
    }
}

/// Convenience variant used by the database layer to report a job that took a
/// known amount of time without re-running it.
pub fn report_job_time(summary: &str, time_took: f64, min_duration_ms: u64) {
    let time_took_ms = (time_took * 1000.0) as u64;

    if time_took_ms > min_duration_ms {
        PROFILE_SLOW_COUNT.fetch_add(1, Ordering::Relaxed);
    } else {
        PROFILE_FAST_COUNT.fetch_add(1, Ordering::Relaxed);
    }

    if profile_mode() {
        print_profile(
            &format!(
                "{summary} took {}",
                time_delta_to_pretty_timedelta(Some(time_took), true, false)
            ),
            None,
        );
    }
}

/// `start_profile_mode( name )`
pub fn start_profile_mode(name: &str) {
    if profile_mode() {
        stop_profile_mode();
    }

    PROFILE_MODE.store(true, Ordering::Relaxed);

    *profile_mode_name_cell().lock().unwrap() = name.to_string();

    PROFILE_START_TIME.store(get_now(), Ordering::Relaxed);
    PROFILE_SLOW_COUNT.store(0, Ordering::Relaxed);
    PROFILE_FAST_COUNT.store(0, Ordering::Relaxed);

    show_text(format!("Profile mode \"{name}\" on!"));
}

/// `stop_profile_mode()`
pub fn stop_profile_mode() {
    let old_name = profile_mode_name();

    PROFILE_MODE.store(false, Ordering::Relaxed);

    *profile_mode_name_cell().lock().unwrap() = String::new();

    let slow = PROFILE_SLOW_COUNT.load(Ordering::Relaxed);
    let fast = PROFILE_FAST_COUNT.load(Ordering::Relaxed);

    show_text(format!(
        "Profiling \"{old_name}\" done: {} slow jobs, {} fast jobs",
        to_human_int(slow as i128),
        to_human_int(fast as i128)
    ));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_mode_toggles_and_filters_by_name() {
        start_profile_mode("test");

        assert!(profile_mode());
        assert!(is_profile_mode("test"));
        assert!(!is_profile_mode("other"));

        stop_profile_mode();

        assert!(!profile_mode());
        assert!(!is_profile_mode("test"));
    }

    #[test]
    fn query_planner_mode_toggles() {
        flip_query_planner_mode();

        assert!(query_planner_mode());
        assert!(!query_was_planned("SELECT 1;"));

        print_query_plan("SELECT 1;", &["SCAN table".to_string()]);

        assert!(query_was_planned("SELECT 1;"));

        flip_query_planner_mode();

        assert!(!query_planner_mode());
        assert!(!query_was_planned("SELECT 1;"));
    }

    #[test]
    fn profile_runs_the_closure() {
        let called = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));

        let flag = called.clone();

        profile(
            "test job",
            move || {
                flag.store(true, Ordering::SeqCst);
            },
            0,
            false,
        );

        assert!(called.load(Ordering::SeqCst));
    }

    #[test]
    fn time_reporting_does_not_panic() {
        report_job_time("job", 0.0001, 20);
        report_job_time("job", 0.5, 20);
    }
}
