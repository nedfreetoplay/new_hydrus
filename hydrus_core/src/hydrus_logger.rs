//! Rust port of `hydrus/core/HydrusLogger.py`.
//!
//! Python swaps `sys.stdout`/`sys.stderr` for an object that prefixes every
//! write and mirrors it to a monthly log file. Rust cannot safely replace the
//! global streams, so [`HydrusLogger`] exposes `write`/`flush` for the ported
//! call sites and mirrors writes to stdout. `faulthandler` crash reporting has
//! no std equivalent; the flag is still tracked so the UI can show it.

use std::fs::File;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use chrono::Local;

use crate::hydrus_constants::{SOFTWARE_VERSION, UNICODE_BYTE_ORDER_MARK};
use crate::hydrus_exceptions::{ErrorKind, HydrusException, HydrusResult};

const LOG_WARNING: &str =
    "THIS LOG FILE MAY CONTAIN PRIVATE INFORMATION! DO NOT SHARE IT CASUALLY!\n\n";

struct LogState {
    file: File,
    path: PathBuf,
}

/// `class HydrusLogger`
pub struct HydrusLogger {
    db_dir: PathBuf,
    prefix: String,
    state: Mutex<LogState>,
    currently_crash_reporting: AtomicBool,
    log_closed: AtomicBool,
}

impl HydrusLogger {
    pub fn new(db_dir: impl Into<PathBuf>, prefix: impl Into<String>) -> HydrusResult<Self> {
        let db_dir = db_dir.into();
        let prefix = prefix.into();

        let (file, path) = open_log(&db_dir, &prefix)
            .map_err(|e| HydrusException::new(ErrorKind::FileMissing, e.to_string()))?;

        Ok(Self {
            db_dir,
            prefix,
            state: Mutex::new(LogState { file, path }),
            currently_crash_reporting: AtomicBool::new(false),
            log_closed: AtomicBool::new(false),
        })
    }

    fn get_log_path(&self) -> PathBuf {
        let now = Local::now();

        let log_filename = format!(
            "{} - {}-{:02}.log",
            self.prefix,
            now.format("%Y"),
            now.format("%m")
        );

        self.db_dir.join(log_filename)
    }

    fn switch_to_a_new_log_file_if_due(&self, state: &mut LogState) {
        let correct_path = self.get_log_path();

        if state.path != correct_path {
            let _ = state.file.flush();

            if let Ok((file, path)) = open_log(&self.db_dir, &self.prefix) {
                state.file = file;
                state.path = path;
            }
        }
    }

    /// `flip_crash_reporting()`
    pub fn flip_crash_reporting(&self) {
        let was = self.currently_crash_reporting.load(Ordering::Relaxed);

        self.currently_crash_reporting
            .store(!was, Ordering::Relaxed);
    }

    /// `currently_crash_reporting()`
    pub fn currently_crash_reporting(&self) -> bool {
        self.currently_crash_reporting.load(Ordering::Relaxed)
    }

    /// `flush()`
    pub fn flush(&self) {
        if self.log_closed.load(Ordering::Relaxed) {
            return;
        }

        let mut state = self.state.lock().unwrap();

        let _ = state.file.flush();
        let _ = std::io::stdout().flush();

        self.switch_to_a_new_log_file_if_due(&mut state);
    }

    /// `isatty()`
    pub fn isatty(&self) -> bool {
        false
    }

    /// `write( value )`
    pub fn write(&self, value: &str) {
        if self.log_closed.load(Ordering::Relaxed) {
            return;
        }

        let prefix = if value == "\n" || value == "\r\n" {
            String::new()
        } else {
            format!(
                "v{}, {}: ",
                SOFTWARE_VERSION,
                Local::now().format("%Y-%m-%d %H:%M:%S")
            )
        };

        let message = format!("{prefix}{value}");

        {
            let mut state = self.state.lock().unwrap();

            let _ = state.file.write_all(message.as_bytes());
        }

        print!("{message}");
    }

    /// `_close_log()`
    pub fn close(&self) {
        let mut state = self.state.lock().unwrap();

        let _ = state.file.flush();

        self.log_closed.store(true, Ordering::Relaxed);
    }

    pub fn log_path(&self) -> PathBuf {
        self.state.lock().unwrap().path.clone()
    }
}

fn open_log(db_dir: &Path, prefix: &str) -> std::io::Result<(File, PathBuf)> {
    std::fs::create_dir_all(db_dir)?;

    let now = Local::now();

    let log_filename = format!(
        "{} - {}-{:02}.log",
        prefix,
        now.format("%Y"),
        now.format("%m")
    );

    let log_path = db_dir.join(log_filename);

    let is_new_file = !log_path.exists();

    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)?;

    if is_new_file {
        file.write_all(UNICODE_BYTE_ORDER_MARK.as_bytes())?;
        file.write_all(LOG_WARNING.as_bytes())?;
    }

    Ok((file, log_path))
}

impl Drop for HydrusLogger {
    fn drop(&mut self) {
        if let Ok(mut state) = self.state.lock() {
            let _ = state.file.flush();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unique_dir(tag: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "hydrus-logger-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));

        std::fs::create_dir_all(&path).unwrap();

        path
    }

    #[test]
    fn log_file_is_created_with_bom_and_warning() {
        let dir = unique_dir("create");

        let logger = HydrusLogger::new(&dir, "client").unwrap();

        logger.write("hello\n");
        logger.flush();

        let text = std::fs::read_to_string(logger.log_path()).unwrap();

        assert!(text.starts_with(UNICODE_BYTE_ORDER_MARK));
        assert!(text.contains(LOG_WARNING));
        assert!(text.contains("hello"));

        logger.close();

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn writes_are_prefixed_but_newlines_are_not() {
        let dir = unique_dir("prefix");

        let logger = HydrusLogger::new(&dir, "server").unwrap();

        logger.write("first");
        logger.write("\n");

        let text = std::fs::read_to_string(logger.log_path()).unwrap();

        assert!(text.contains(&format!("v{}, ", SOFTWARE_VERSION)));
        assert!(text.contains("first"));

        logger.close();

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn crash_flag_flips() {
        let dir = unique_dir("crash");

        let logger = HydrusLogger::new(&dir, "test").unwrap();

        assert!(!logger.currently_crash_reporting());

        logger.flip_crash_reporting();

        assert!(logger.currently_crash_reporting());

        logger.flip_crash_reporting();

        assert!(!logger.currently_crash_reporting());

        logger.close();

        let _ = std::fs::remove_dir_all(dir);
    }
}
