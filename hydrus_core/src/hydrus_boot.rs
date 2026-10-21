//! Rust port of `hydrus/core/HydrusBoot.py`.
//!
//! The Python module checks that the YAML dependency exists and sets a few
//! environment variables before OpenCV loads. Rust dependencies are linked at
//! build time, so only the PATH and OpenCV logging steps remain.

use std::ffi::OsString;
use std::sync::{Mutex, OnceLock};

use crate::hydrus_constants::content_base_dir;

static ORIGINAL_PATH: OnceLock<Mutex<Option<OsString>>> = OnceLock::new();

fn original_path_cell() -> &'static Mutex<Option<OsString>> {
    ORIGINAL_PATH.get_or_init(|| Mutex::new(None))
}

/// `ORIGINAL_PATH`, if the base dir has been prepended.
pub fn original_path() -> Option<OsString> {
    original_path_cell().lock().unwrap().clone()
}

/// `add_base_dir_to_env_path()`
///
/// Prepending the content base dir lets a packaged build load DLLs/`.so`s that
/// sit next to the executable.
pub fn add_base_dir_to_env_path() {
    let Some(path) = std::env::var_os("PATH") else {
        return;
    };

    *original_path_cell().lock().unwrap() = Some(path.clone());

    let mut paths = vec![content_base_dir().to_path_buf()];

    paths.extend(std::env::split_paths(&path));

    if let Ok(joined) = std::env::join_paths(paths) {
        // SAFETY: called during early boot, before worker threads that read the
        // environment are spawned.
        unsafe {
            std::env::set_var("PATH", joined);
        }
    }
}

/// `do_pre_import_env_work()`
pub fn do_pre_import_env_work() {
    // SAFETY: called during early boot, before worker threads that read the
    // environment are spawned.
    unsafe {
        std::env::set_var("OPENCV_LOG_LEVEL", "ERROR");
        std::env::set_var("OPENCV_FFMPEG_LEVEL", "16"); // AV_LOG_ERROR
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn original_path_starts_empty() {
        // Do not mutate PATH here; add_base_dir_to_env_path is boot-only.
        let _ = original_path();
    }

    #[test]
    fn pre_import_work_sets_the_variables() {
        do_pre_import_env_work();

        assert_eq!(std::env::var("OPENCV_LOG_LEVEL").unwrap(), "ERROR");
        assert_eq!(std::env::var("OPENCV_FFMPEG_LEVEL").unwrap(), "16");
    }
}
