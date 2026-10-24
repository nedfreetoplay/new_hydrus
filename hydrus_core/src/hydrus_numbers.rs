//! Rust port of `hydrus/core/HydrusNumbers.py`.

use std::fmt::Write as _;

/// `float_to_percentage( f, sf = 1 )`
pub fn float_to_percentage(f: f64, sf: usize) -> String {
    let percent = f * 100.0;

    if percent == percent.trunc() {
        format!("{}%", percent as i64)
    } else {
        format!("{percent:.sf$}%")
    }
}

/// `index_to_pretty_ordinal_string( index )`
pub fn index_to_pretty_ordinal_string(index: i64) -> String {
    if index >= 0 {
        int_to_pretty_ordinal_string(index + 1)
    } else {
        int_to_pretty_ordinal_string(index)
    }
}

/// `int_to_pixels( i )`
pub fn int_to_pixels(i: u64) -> &'static str {
    match i {
        1 => "pixels",
        1_000 => "kilopixels",
        1_000_000 => "megapixels",
        _ => "megapixels",
    }
}

/// `int_to_unit( unit )`. Python returns `None` for anything else.
pub fn int_to_unit(unit: u64) -> Option<&'static str> {
    match unit {
        1 => Some("B"),
        1024 => Some("KB"),
        1_048_576 => Some("MB"),
        1_073_741_824 => Some("GB"),
        _ => None,
    }
}

/// `int_to_pretty_ordinal_string( num )`
pub fn int_to_pretty_ordinal_string(num: i64) -> String {
    if num == 0 {
        return "unknown position".to_string();
    }

    let abs = num.unsigned_abs();

    let tens = (abs % 100) / 10;

    let ordinal = if tens == 1 {
        "th"
    } else {
        match abs % 10 {
            1 => "st",
            2 => "nd",
            3 => "rd",
            _ => "th",
        }
    };

    let mut s = format!("{}{}", base_to_human_int(abs as i128), ordinal);

    if num < 0 {
        if num == -1 {
            s = "last".to_string();
        } else {
            s = format!("{s} from last");
        }
    }

    s
}

/// `pixels_to_int( unit )`
pub fn pixels_to_int(unit: &str) -> Option<u64> {
    match unit {
        "pixels" => Some(1),
        "kilopixels" => Some(1_000),
        "megapixels" => Some(1_000_000),
        _ => None,
    }
}

/// `base_to_human_int( num )` -- commas as thousands separators.
pub fn base_to_human_int(num: impl Into<i128>) -> String {
    let num = num.into();

    let negative = num < 0;
    let digits = num.unsigned_abs().to_string();

    let mut out = String::with_capacity(digits.len() + digits.len() / 3 + 1);

    if negative {
        out.push('-');
    }

    for (index, ch) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index) % 3 == 0 {
            out.push(',');
        }

        out.push(ch);
    }

    out
}

pub use base_to_human_int as to_human_int;

/// `unit_to_int( unit )`
pub fn unit_to_int(unit: &str) -> Option<u64> {
    match unit {
        "B" => Some(1),
        "KB" => Some(1024),
        "MB" => Some(1024 * 1024),
        "GB" => Some(1024 * 1024 * 1024),
        "TB" => Some(1024_u64.pow(4)),
        _ => None,
    }
}

/// `value_range_to_pretty_string( value, range )`
pub fn value_range_to_pretty_string(value: Option<i128>, range: Option<i128>) -> String {
    let value = match (value, range) {
        (Some(v), Some(r)) => Some(v.min(r)),
        (v, _) => v,
    };

    let value_text = value
        .map(to_human_int)
        .unwrap_or_else(|| "unknown".to_string());
    let range_text = range
        .map(to_human_int)
        .unwrap_or_else(|| "unknown".to_string());

    let mut out = String::new();
    let _ = write!(out, "{value_text}/{range_text}");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn human_int_groups_digits() {
        assert_eq!(to_human_int(0_i128), "0");
        assert_eq!(to_human_int(999_i128), "999");
        assert_eq!(to_human_int(1000_i128), "1,000");
        assert_eq!(to_human_int(-1234567_i128), "-1,234,567");
    }

    #[test]
    fn ordinals_match_python() {
        assert_eq!(int_to_pretty_ordinal_string(0), "unknown position");
        assert_eq!(int_to_pretty_ordinal_string(1), "1st");
        assert_eq!(int_to_pretty_ordinal_string(2), "2nd");
        assert_eq!(int_to_pretty_ordinal_string(3), "3rd");
        assert_eq!(int_to_pretty_ordinal_string(4), "4th");
        assert_eq!(int_to_pretty_ordinal_string(11), "11th");
        assert_eq!(int_to_pretty_ordinal_string(21), "21st");
        assert_eq!(int_to_pretty_ordinal_string(-1), "last");
        assert_eq!(int_to_pretty_ordinal_string(-2), "2nd from last");
        assert_eq!(index_to_pretty_ordinal_string(0), "1st");
        assert_eq!(index_to_pretty_ordinal_string(-2), "2nd from last");
    }

    #[test]
    fn percentages() {
        assert_eq!(float_to_percentage(0.25, 1), "25%");
        assert_eq!(float_to_percentage(0.1234, 1), "12.3%");
        assert_eq!(float_to_percentage(0.1234, 2), "12.34%");
    }

    #[test]
    fn unit_round_trips() {
        assert_eq!(unit_to_int("GB"), Some(1024 * 1024 * 1024));
        assert_eq!(int_to_unit(1024), Some("KB"));
        assert_eq!(unit_to_int("nope"), None);
        assert_eq!(value_range_to_pretty_string(Some(5), Some(3)), "3/3");
    }
}
