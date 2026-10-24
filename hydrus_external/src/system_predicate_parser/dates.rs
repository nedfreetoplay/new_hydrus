//! Date, duration and time interval parsing.

use fancy_regex::Regex;
use hydrus_core::hydrus_exceptions::HydrusResult;

use super::parse_error;
use super::state::ParseState;
use super::types::{DateOrTimeInterval, ValueData};

pub(crate) fn parse_date_or_time_interval(
    state: &mut ParseState,
    string: &str,
) -> HydrusResult<()> {
    if string == "now" {
        state.text_remainder = String::new();
        state.value = Some(ValueData::DateOrTimeInterval(
            DateOrTimeInterval::DateTime(chrono::Local::now().naive_local()),
        ));

        return Ok(());
    }

    let interval = Regex::new(
        r"^(?:(?P<year>0|([1-9][0-9]*))\s*(years|year))?\s*(?:(?P<month>0|([1-9][0-9]*))\s*(months|month))?\s*(?:(?P<day>0|([1-9][0-9]*))\s*(days|day))?\s*(?:(?P<hour>0|([1-9][0-9]*))\s*(hours|hour|h))?",
    )
    .expect("valid date interval regex");

    if let Some(captures) = interval
        .captures(string)
        .map_err(|error| parse_error(error.to_string()))?
    {
        let mut values = [0_u32; 4];
        let mut found = false;

        for (index, name) in ["year", "month", "day", "hour"].iter().enumerate() {
            if let Some(value) = captures.name(name) {
                values[index] = value.as_str().parse().unwrap_or(0);
                found = true;
            }
        }

        if found {
            let match_end = captures.get(0).expect("whole match").end();
            let mut remainder = string[match_end..].trim().to_string();

            if remainder == "ago" {
                remainder.clear();
            }

            state.text_remainder = remainder;
            state.value = Some(ValueData::DateOrTimeInterval(
                DateOrTimeInterval::Interval {
                    years: values[0],
                    months: values[1],
                    days: values[2],
                    hours: values[3],
                },
            ));

            return Ok(());
        }
    }

    let date = Regex::new(
        r"^(?P<year>[0-9][0-9][0-9][0-9])-(?P<month>[0-9][0-9]?)-(?P<day>[0-9][0-9]?)",
    )
    .expect("valid date regex");

    if let Some(captures) = date
        .captures(string)
        .map_err(|error| parse_error(error.to_string()))?
    {
        let year: i32 = captures["year"].parse().unwrap_or(0);
        let month: u32 = captures["month"].parse().unwrap_or(0);
        let day: u32 = captures["day"].parse().unwrap_or(0);

        let value = chrono::NaiveDate::from_ymd_opt(year, month, day)
            .and_then(|date| date.and_hms_opt(0, 0, 0))
            .ok_or_else(|| parse_error("Invalid value, expected a date or a time interval"))?;

        state.text_remainder =
            string[captures.get(0).expect("whole match").end()..].to_string();
        state.value = Some(ValueData::DateOrTimeInterval(
            DateOrTimeInterval::DateTime(value),
        ));

        return Ok(());
    }

    Err(parse_error(
        "Invalid value, expected a date or a time interval",
    ))
}
