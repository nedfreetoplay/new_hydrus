//! Rust port of `hydrus/core/HydrusGlobals.py`.
//!
//! Python uses module-level mutable globals (and even rebinds `HG`'s module
//! namespace from the test harness). Rust's safe equivalent for the boolean
//! flags is atomics; the handful of numbers are atomics too. The client-only
//! "controller" reference lives in the client crate.

use std::sync::atomic::{AtomicBool, AtomicI64, AtomicU8, AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

macro_rules! flag {
    ($name:ident, $get:ident, $set:ident, $default:expr) => {
        static $name: AtomicBool = AtomicBool::new($default);

        pub fn $get() -> bool {
            $name.load(Ordering::Relaxed)
        }

        pub fn $set(value: bool) {
            $name.store(value, Ordering::Relaxed);
        }
    };
}

// Shutdown state
flag!(STARTED_SHUTDOWN, started_shutdown, set_started_shutdown, false);
flag!(VIEW_SHUTDOWN, view_shutdown, set_view_shutdown, false);
flag!(MODEL_SHUTDOWN, model_shutdown, set_model_shutdown, false);
flag!(SHUTDOWN_COMPLETE, shutdown_complete, set_shutdown_complete, false);
flag!(RESTART, restart, set_restart, false);

// Boot / running
flag!(NON_INTERACTIVE_UPDATE, non_interactive_update, set_non_interactive_update, false);
flag!(BOOT_DEBUG, boot_debug, set_boot_debug, false);
flag!(IMPORT_FOLDERS_RUNNING, import_folders_running, set_import_folders_running, false);
flag!(EXPORT_FOLDERS_RUNNING, export_folders_running, set_export_folders_running, false);
flag!(
    BOOT_WITH_NETWORK_TRAFFIC_PAUSED,
    boot_with_network_traffic_paused_command_line,
    set_boot_with_network_traffic_paused_command_line,
    false
);

// Reporting / debug modes
flag!(CALLTO_REPORT_MODE, callto_report_mode, set_callto_report_mode, false);
flag!(DB_REPORT_MODE, db_report_mode, set_db_report_mode, false);
flag!(FILE_REPORT_MODE, file_report_mode, set_file_report_mode, false);
flag!(FILE_SORT_REPORT_MODE, file_sort_report_mode, set_file_sort_report_mode, false);
flag!(MEDIA_LOAD_REPORT_MODE, media_load_report_mode, set_media_load_report_mode, false);
flag!(GUI_REPORT_MODE, gui_report_mode, set_gui_report_mode, false);
flag!(SHORTCUT_REPORT_MODE, shortcut_report_mode, set_shortcut_report_mode, false);
flag!(CACHE_REPORT_MODE, cache_report_mode, set_cache_report_mode, false);
flag!(IDLE_REPORT_MODE, idle_report_mode, set_idle_report_mode, false);
flag!(SUBPROCESS_REPORT_MODE, subprocess_report_mode, set_subprocess_report_mode, false);
flag!(
    SUBSCRIPTION_REPORT_MODE,
    subscription_report_mode,
    set_subscription_report_mode,
    false
);
flag!(HOVER_WINDOW_REPORT_MODE, hover_window_report_mode, set_hover_window_report_mode, false);
flag!(
    FILE_IMPORT_REPORT_MODE,
    file_import_report_mode,
    set_file_import_report_mode,
    false
);
flag!(
    PHASH_GENERATION_REPORT_MODE,
    phash_generation_report_mode,
    set_phash_generation_report_mode,
    false
);
flag!(NETWORK_REPORT_MODE, network_report_mode, set_network_report_mode, false);
flag!(
    NETWORK_REPORT_MODE_SILENT,
    network_report_mode_silent,
    set_network_report_mode_silent,
    false
);
flag!(PUBSUB_REPORT_MODE, pubsub_report_mode, set_pubsub_report_mode, false);
flag!(DAEMON_REPORT_MODE, daemon_report_mode, set_daemon_report_mode, false);
flag!(MPV_REPORT_MODE, mpv_report_mode, set_mpv_report_mode, false);
flag!(
    POTENTIAL_DUPLICATES_REPORT_MODE,
    potential_duplicates_report_mode,
    set_potential_duplicates_report_mode,
    false
);
flag!(
    THUMBNAIL_GRAPHICS_VIEW_REPORT_MODE,
    thumbnail_graphics_view_report_mode,
    set_thumbnail_graphics_view_report_mode,
    false
);
flag!(SHUTDOWN_REPORT_MODE, shutdown_report_mode, set_shutdown_report_mode, false);

// Misc behaviour toggles
flag!(MACOS_ANTIFLICKER_TEST, macos_antiflicker_test, set_macos_antiflicker_test, false);
flag!(CANVAS_TILE_OUTLINE_MODE, canvas_tile_outline_mode, set_canvas_tile_outline_mode, false);
flag!(MPV_ALLOW_CRASHY_FILES, mpv_allow_crashy_files, set_mpv_allow_crashy_files, false);
flag!(
    MPV_ALLOW_CRASHY_FILES_SILENTLY,
    mpv_allow_crashy_files_silently,
    set_mpv_allow_crashy_files_silently,
    false
);
flag!(FAKE_PETITION_MODE, fake_petition_mode, set_fake_petition_mode, false);
flag!(FORCE_IDLE_MODE, force_idle_mode, set_force_idle_mode, false);
flag!(THUMBNAIL_DEBUG_MODE, thumbnail_debug_mode, set_thumbnail_debug_mode, false);
flag!(AUTOCOMPLETE_DELAY_MODE, autocomplete_delay_mode, set_autocomplete_delay_mode, false);
flag!(BLURHASH_MODE, blurhash_mode, set_blurhash_mode, false);
flag!(DO_IDLE_SHUTDOWN_WORK, do_idle_shutdown_work, set_do_idle_shutdown_work, false);
flag!(TWISTED_IS_BROKE, twisted_is_broke, set_twisted_is_broke, false);
flag!(DOING_FAST_EXIT, doing_fast_exit, set_doing_fast_exit, false);

// Numbers
static DB_CACHE_SIZE: AtomicU64 = AtomicU64::new(256);
static DB_TRANSACTION_COMMIT_PERIOD: AtomicU64 = AtomicU64::new(30);
static DB_SYNCHRONOUS: AtomicU8 = AtomicU8::new(2);
static DB_PROFILE_MIN_JOB_TIME_MS: AtomicU64 = AtomicU64::new(16);
static CALLTO_PROFILE_MIN_JOB_TIME_MS: AtomicU64 = AtomicU64::new(10);
static SERVER_PROFILE_MIN_JOB_TIME_MS: AtomicU64 = AtomicU64::new(10);
static MENU_PROFILE_MIN_JOB_TIME_MS: AtomicU64 = AtomicU64::new(16);
static PUBSUB_PROFILE_MIN_JOB_TIME_MS: AtomicU64 = AtomicU64::new(5);
static UI_TIMER_PROFILE_MIN_JOB_TIME_MS: AtomicU64 = AtomicU64::new(5);

macro_rules! number {
    ($name:ident, $get:ident, $set:ident, $ty:ty) => {
        pub fn $get() -> $ty {
            $name.load(Ordering::Relaxed)
        }

        pub fn $set(value: $ty) {
            $name.store(value, Ordering::Relaxed);
        }
    };
}

number!(DB_CACHE_SIZE, db_cache_size, set_db_cache_size, u64);
number!(
    DB_TRANSACTION_COMMIT_PERIOD,
    db_transaction_commit_period,
    set_db_transaction_commit_period,
    u64
);
number!(DB_SYNCHRONOUS, db_synchronous, set_db_synchronous, u8);
number!(
    DB_PROFILE_MIN_JOB_TIME_MS,
    db_profile_min_job_time_ms,
    set_db_profile_min_job_time_ms,
    u64
);
number!(
    CALLTO_PROFILE_MIN_JOB_TIME_MS,
    callto_profile_min_job_time_ms,
    set_callto_profile_min_job_time_ms,
    u64
);
number!(
    SERVER_PROFILE_MIN_JOB_TIME_MS,
    server_profile_min_job_time_ms,
    set_server_profile_min_job_time_ms,
    u64
);
number!(
    MENU_PROFILE_MIN_JOB_TIME_MS,
    menu_profile_min_job_time_ms,
    set_menu_profile_min_job_time_ms,
    u64
);
number!(
    PUBSUB_PROFILE_MIN_JOB_TIME_MS,
    pubsub_profile_min_job_time_ms,
    set_pubsub_profile_min_job_time_ms,
    u64
);
number!(
    UI_TIMER_PROFILE_MIN_JOB_TIME_MS,
    ui_timer_profile_min_job_time_ms,
    set_ui_timer_profile_min_job_time_ms,
    u64
);

static SERVER_ACTION: OnceLock<Mutex<String>> = OnceLock::new();

fn server_action_cell() -> &'static Mutex<String> {
    SERVER_ACTION.get_or_init(|| Mutex::new("start".to_string()))
}

pub fn server_action() -> String {
    server_action_cell().lock().unwrap().clone()
}

pub fn set_server_action(value: impl Into<String>) {
    *server_action_cell().lock().unwrap() = value.into();
}

static DB_JOURNAL_MODE: OnceLock<Mutex<String>> = OnceLock::new();

fn db_journal_mode_cell() -> &'static Mutex<String> {
    DB_JOURNAL_MODE.get_or_init(|| Mutex::new("WAL".to_string()))
}

pub fn db_journal_mode() -> String {
    db_journal_mode_cell().lock().unwrap().clone()
}

pub fn set_db_journal_mode(value: impl Into<String>) {
    *db_journal_mode_cell().lock().unwrap() = value.into();
}

static NO_DB_TEMP_FILES: AtomicBool = AtomicBool::new(false);

pub fn no_db_temp_files() -> bool {
    NO_DB_TEMP_FILES.load(Ordering::Relaxed)
}

pub fn set_no_db_temp_files(value: bool) {
    NO_DB_TEMP_FILES.store(value, Ordering::Relaxed);
}

static TWISTED_IS_BROKE_EXCEPTION: OnceLock<Mutex<Option<String>>> = OnceLock::new();

pub fn twisted_is_broke_exception() -> Option<String> {
    TWISTED_IS_BROKE_EXCEPTION
        .get_or_init(|| Mutex::new(None))
        .lock()
        .unwrap()
        .clone()
}

pub fn set_twisted_is_broke_exception(value: Option<String>) {
    *TWISTED_IS_BROKE_EXCEPTION
        .get_or_init(|| Mutex::new(None))
        .lock()
        .unwrap() = value;
}

// Python reaches these through `HG.controller`. The core crate does not own
// the controller, so the client/server sets them once at boot.

static DB_DIR: OnceLock<Mutex<Option<std::path::PathBuf>>> = OnceLock::new();

pub fn db_dir() -> Option<std::path::PathBuf> {
    DB_DIR
        .get_or_init(|| Mutex::new(None))
        .lock()
        .unwrap()
        .clone()
}

pub fn set_db_dir(path: impl Into<std::path::PathBuf>) {
    *DB_DIR.get_or_init(|| Mutex::new(None)).lock().unwrap() = Some(path.into());
}

static PROGRAM_NAME: OnceLock<Mutex<String>> = OnceLock::new();

pub fn program_name() -> String {
    PROGRAM_NAME
        .get_or_init(|| Mutex::new("hydrus".to_string()))
        .lock()
        .unwrap()
        .clone()
}

pub fn set_program_name(name: impl Into<String>) {
    *PROGRAM_NAME.get_or_init(|| Mutex::new(String::new())).lock().unwrap() = name.into();
}

static DB_PROFILE_MIN_JOB_TIME_MS_LEGACY: AtomicI64 = AtomicI64::new(16);

pub fn db_profile_min_job_time_ms_legacy() -> i64 {
    DB_PROFILE_MIN_JOB_TIME_MS_LEGACY.load(Ordering::Relaxed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flags_round_trip() {
        assert!(!blurhash_mode());
        set_blurhash_mode(true);
        assert!(blurhash_mode());
        set_blurhash_mode(false);

        set_db_cache_size(512);
        assert_eq!(db_cache_size(), 512);
        set_db_cache_size(256);

        set_server_action("stop");
        assert_eq!(server_action(), "stop");
        set_server_action("start");
    }
}
