//! Rust port of `hydrus/core/HydrusMemory.py`.
//!
//! Python uses the third-party `pympler` package to walk the interpreter's
//! object graph. Rust has no reflective heap walker, so the same facilities do
//! not exist in std. The port keeps the Python surface and the "pympler is not
//! available" behaviour: every diagnostic raises an error instead of silently
//! doing nothing.
//!
//! A native implementation would belong in the client crate (for example,
//! using a global allocator that records per-type totals). Keeping this module
//! honest avoids pretending the Python output exists.

use crate::hydrus_exceptions::{ErrorKind, HydrusException, HydrusResult};

/// Python's `PYMPLER_OK`.
pub const PYMPLER_OK: bool = false;

fn check_pympler_ok() -> HydrusResult<()> {
    if PYMPLER_OK {
        Ok(())
    } else {
        Err(HydrusException::new(
            ErrorKind::Unknown,
            "Pympler is not available! (Rust has no reflective heap walker; use a profiling allocator in the client.)",
        ))
    }
}

/// `print_current_memory_use( classes_to_track = None )`
pub fn print_current_memory_use() -> HydrusResult<()> {
    check_pympler_ok()
}

/// `print_snapshot_diff()`
pub fn print_snapshot_diff() -> HydrusResult<()> {
    check_pympler_ok()
}

/// `take_memory_use_snapshot()`
pub fn take_memory_use_snapshot() -> HydrusResult<()> {
    check_pympler_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diagnostics_report_unavailable() {
        assert!(!PYMPLER_OK);
        assert!(print_current_memory_use().is_err());
        assert!(print_snapshot_diff().is_err());
        assert!(take_memory_use_snapshot().is_err());
    }
}
