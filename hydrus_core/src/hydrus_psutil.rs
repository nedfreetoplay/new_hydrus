//! Rust port of `hydrus/core/HydrusPSUtil.py`.
//!
//! Python's module is a one-shot import guard around `psutil`, with the actual
//! library re-exported as `HydrusPSUtil.psutil`. The Rust port does not pull in
//! a process-info crate yet, so the guarded capability is reported as
//! unavailable and the small convenience wrappers return `None`.
//!
//! `get_free_space`/`get_total_space` in [`crate::hydrus_paths`] use direct
//! platform calls, so they keep working without this module. When a Rust
//! equivalent of psutil is added, flip [`PSUTIL_OK`] and fill in the wrappers.

/// `PSUTIL_OK`
pub const PSUTIL_OK: bool = false;

/// `PSUTIL_MODULE_NOT_FOUND`
pub const PSUTIL_MODULE_NOT_FOUND: bool = false;

/// `PSUTIL_IMPORT_ERROR`
pub const PSUTIL_IMPORT_ERROR: &str = "psutil is not available in the Rust port yet";

/// `psutil.virtual_memory().available`
pub fn available_memory() -> Option<u64> {
    None
}

/// `psutil.disk_usage( path ).free`
pub fn disk_free(path: &std::path::Path) -> Option<u64> {
    crate::hydrus_paths::get_free_space(path).map(|bytes| bytes as u64)
}

/// `psutil.disk_usage( path ).total`
pub fn disk_total(path: &std::path::Path) -> Option<u64> {
    crate::hydrus_paths::get_total_space(path).map(|bytes| bytes as u64)
}

/// `psutil.cpu_count()`
pub fn cpu_count() -> Option<usize> {
    std::thread::available_parallelism().ok().map(|n| n.get())
}

/// `psutil.pid_exists( pid )`
pub fn pid_exists(pid: u32) -> bool {
    // Only a hint on Unix (zombies count); good enough for the port's uses.
    #[cfg(unix)]
    {
        std::path::Path::new(&format!("/proc/{pid}")).exists()
    }

    #[cfg(windows)]
    {
        // `tasklist` is expensive, so use OpenProcess via a tiny FFI shim.
        use std::ffi::c_void;

        const PROCESS_QUERY_LIMITED_INFORMATION: u32 = 0x1000;

        #[link(name = "kernel32")]
        unsafe extern "system" {
            fn OpenProcess(
                desired_access: u32,
                inherit_handle: i32,
                process_id: u32,
            ) -> *mut c_void;
            fn CloseHandle(handle: *mut c_void) -> i32;
        }

        unsafe {
            let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);

            if handle.is_null() {
                false
            } else {
                CloseHandle(handle);
                true
            }
        }
    }

    #[cfg(not(any(unix, windows)))]
    {
        let _ = pid;
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capability_flags_are_consistent() {
        assert!(!PSUTIL_OK);
        assert!(!PSUTIL_MODULE_NOT_FOUND);
        assert!(!PSUTIL_IMPORT_ERROR.is_empty());
    }

    #[test]
    fn current_process_exists() {
        assert!(pid_exists(std::process::id()));
        assert!(!pid_exists(u32::MAX - 1));
    }

    #[test]
    fn cpu_count_is_sane() {
        assert!(cpu_count().unwrap_or(0) >= 1 || cpu_count().is_none());
    }

    #[test]
    fn disk_usage_delegates_to_paths() {
        let temp = std::env::temp_dir();

        // Free space can legitimately be unavailable on exotic filesystems.
        if let Some(free) = disk_free(&temp) {
            assert!(free > 0);
        }
    }
}
