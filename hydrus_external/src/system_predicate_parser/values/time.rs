//! Duration, time interval and ratio value parsing.

use fancy_regex::Regex;
use hydrus_core::hydrus_exceptions::HydrusResult;

use crate::system_predicate_parser::parse_error;
use crate::system_predicate_parser::state::ParseState;
use crate::system_predicate_parser::types::ValueData;

pub(super) fn parse_time_to_msec(state: &mut ParseState, string: &str) -> HydrusResult<()> {
    if string.starts_with("has") || string.starts_with("no") {
        state.text_remainder = String::new();
        state.value = Some(ValueData::TimeToMsec(0));
        return Ok(());
    }

    let pattern = concat!(
        r"^(?:(?P<hour>0|([1-9][0-9]*))\s*(hours|hour|hr|h))?\s*",
        r"(?:(?P<min>0|([1-9][0-9]*))\s*(minutes|minute|mins|m$|m\s|m(?=\d)))?\s*",
        r"(?:(?P<sec>0|([1-9][0-9]*))\s*(seconds|second|secs|sec|s))?\s*",
        r"(?:(?P<msec>0|([1-9][0-9]*))\s*(milliseconds|millisecond|msecs|msec|ms))?"
    );
    let regex = Regex::new(pattern).expect("valid duration regex");
    let captures = regex
        .captures(string)
        .map_err(|error| parse_error(error.to_string()))?
        .ok_or_else(|| parse_error("Invalid value, expected a duration"))?;

    let mut total = 0_i64;
    let mut found_any = false;

    for (name, multiplier) in [
        ("hour", 3600_i64 * 1000),
        ("min", 60 * 1000),
        ("sec", 1000),
        ("msec", 1),
    ] {
        if let Some(value) = captures.name(name) {
            if let Ok(value) = value.as_str().parse::<i64>() {
                total += value * multiplier;
                found_any = true;
            }
        }
    }

    if !found_any {
        return Err(parse_error("Invalid value, expected a duration"));
    }

    state.value = Some(ValueData::TimeToMsec(total));
    state.text_remainder = string[captures.get(0).expect("whole match").end()..].to_string();

    Ok(())
}

pub(super) fn parse_time_interval(state: &mut ParseState, string: &str) -> HydrusResult<()> {
    let regex = Regex::new(
        r"^(?:(?P<day>0|([1-9][0-9]*))\s*(days|day))?\s*(?:(?P<hour>0|([1-9][0-9]*))\s*(hours|hour|h))?\s*(?:(?P<minute>0|([1-9][0-9]*))\s*(minutes|minute|mins|min))?\s*(?:(?P<second>0|([1-9][0-9]*))\s*(seconds|second|secs|sec|s))?",
    )
    .expect("valid time interval regex");

    let captures = regex
        .captures(string)
        .map_err(|error| parse_error(error.to_string()))?
        .ok_or_else(|| parse_error("Invalid value, expected a time interval"))?;

    let mut days: u32 = captures.name("day").and_then(|v| v.as_str().parse().ok()).unwrap_or(0);
    let mut hours: u32 = captures.name("hour").and_then(|v| v.as_str().parse().ok()).unwrap_or(0);
    let mut minutes: u32 = captures.name("minute").and_then(|v| v.as_str().parse().ok()).unwrap_or(0);
    let mut seconds: u32 = captures.name("second").and_then(|v| v.as_str().parse().ok()).unwrap_or(0);

    if days + hours + minutes + seconds == 0 {
        return Err(parse_error("Invalid value, expected a time interval"));
    }

    minutes += seconds / 60;
    seconds %= 60;
    hours += minutes / 60;
    minutes %= 60;
    days += hours / 24;
    hours %= 24;

    state.value = Some(ValueData::TimeInterval {
        days,
        hours,
        minutes,
        seconds,
    });
    state.text_remainder = string[captures.get(0).expect("whole match").end()..].to_string();

    Ok(())
}

pub(super) fn parse_ratio(state: &mut ParseState, string: &str) -> HydrusResult<()> {
    let regex = Regex::new(r"^(?P<first>0|([1-9][0-9]*)):(?P<second>0|([1-9][0-9]*))")
        .expect("valid ratio regex");
    let captures = regex
        .captures(string)
        .map_err(|error| parse_error(error.to_string()))?
        .ok_or_else(|| parse_error("Invalid value, expected a ratio"))?;

    state.value = Some(ValueData::Ratio {
        first: captures["first"].parse().unwrap_or(0),
        second: captures["second"].parse().unwrap_or(0),
    });
    state.text_remainder = string[captures.get(0).expect("whole match").end()..].to_string();

    Ok(())
}
