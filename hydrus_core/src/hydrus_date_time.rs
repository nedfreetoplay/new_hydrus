//! Rust port of `hydrus/core/HydrusDateTime.py`.
//!
//! Python's module is a compatibility shim for pre-3.11 `utcnow`. Chrono has
//! always been explicit about the timezone, so these are thin wrappers.

use chrono::{DateTime, TimeZone, Utc};

/// `now_utc()`
pub fn now_utc() -> DateTime<Utc> {
    Utc::now()
}

/// `from_timestamp_utc( timestamp )`
pub fn from_timestamp_utc(timestamp: f64) -> DateTime<Utc> {
    let mut seconds = timestamp.floor() as i64;
    let mut nanoseconds = ((timestamp - seconds as f64) * 1_000_000_000.0).round() as i64;

    if nanoseconds >= 1_000_000_000 {
        seconds += 1;
        nanoseconds -= 1_000_000_000;
    }

    Utc.timestamp_opt(seconds, nanoseconds as u32)
        .single()
        .unwrap_or_else(Utc::now)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn epoch_round_trip() {
        assert_eq!(from_timestamp_utc(0.0).timestamp(), 0);
        assert_eq!(from_timestamp_utc(1234567890.0).timestamp(), 1234567890);
    }

    #[test]
    fn now_is_utc() {
        assert_eq!(now_utc().offset().to_string(), "UTC");
    }
}
