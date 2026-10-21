//! Rust port of `hydrus/core/HydrusData.py`.
//!
//! The Python module is a grab-bag of helpers: dict construction, human-readable
//! sizes, hashing/keys, exception formatting and the deferred-call wrapper.
//!
//! Two Python-only concepts do not survive the port:
//!
//! * `defaultdict` factories (`default_dict_list`, `default_dict_set`,
//!   `get_empty_data_dict`) are just empty [`HashMap`]s in Rust.
//! * `HydrusYAMLBase` only exists so `yaml.YAMLObject` can round-trip hydrus
//!   objects. Serde handles that in the Rust port, so there is no equivalent.

use std::collections::{HashMap, HashSet};
use std::hash::Hash;
use std::io::Write as _;

use rand::RngCore;

use crate::hydrus_constants::HYDRUS_KEY_LENGTH;
use crate::hydrus_exceptions::{ErrorKind, HydrusException, HydrusResult};
use crate::hydrus_numbers::to_human_int;

pub use crate::hydrus_constants::content_status_prefix as status_to_prefix;

/// `build_key_to_list_dict( pairs )`
pub fn build_key_to_list_dict<K, V, I>(pairs: I) -> HashMap<K, Vec<V>>
where
    K: Eq + Hash,
    I: IntoIterator<Item = (K, V)>,
{
    let mut result: HashMap<K, Vec<V>> = HashMap::new();

    for (key, value) in pairs {
        result.entry(key).or_default().push(value);
    }

    result
}

/// `build_key_to_set_dict( pairs )`
pub fn build_key_to_set_dict<K, V, I>(pairs: I) -> HashMap<K, HashSet<V>>
where
    K: Eq + Hash,
    V: Eq + Hash,
    I: IntoIterator<Item = (K, V)>,
{
    let mut result: HashMap<K, HashSet<V>> = HashMap::new();

    for (key, value) in pairs {
        result.entry(key).or_default().insert(value);
    }

    result
}

/// `merge_key_to_list_dicts( key_to_list_dicts )`
pub fn merge_key_to_list_dicts<K, V, I, D>(dicts: I) -> HashMap<K, Vec<V>>
where
    K: Eq + Hash,
    I: IntoIterator<Item = D>,
    D: IntoIterator<Item = (K, Vec<V>)>,
{
    let mut result: HashMap<K, Vec<V>> = HashMap::new();

    for dict in dicts {
        for (key, values) in dict {
            result.entry(key).or_default().extend(values);
        }
    }

    result
}

/// `bytes_to_none_or_hex( b )`
pub fn bytes_to_none_or_hex(bytes: Option<&[u8]>) -> Option<String> {
    bytes.map(hex_encode)
}

/// `hex_to_none_or_bytes( h )`
pub fn hex_to_none_or_bytes(hex: Option<&str>) -> Option<Vec<u8>> {
    hex.map(|h| hex_decode(h).unwrap_or_default())
}

pub(crate) fn hex_encode(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";

    let mut out = String::with_capacity(bytes.len() * 2);

    for byte in bytes {
        out.push(DIGITS[(byte >> 4) as usize] as char);
        out.push(DIGITS[(byte & 0x0F) as usize] as char);
    }

    out
}

pub(crate) fn hex_decode(hex: &str) -> Result<Vec<u8>, ()> {
    if hex.len() % 2 != 0 {
        return Err(());
    }

    let bytes = hex.as_bytes();

    let mut out = Vec::with_capacity(bytes.len() / 2);

    for pair in bytes.chunks_exact(2) {
        let high = (pair[0] as char).to_digit(16).ok_or(())?;
        let low = (pair[1] as char).to_digit(16).ok_or(())?;

        out.push(((high << 4) | low) as u8);
    }

    Ok(out)
}

/// `calculate_score_from_rating( count, rating )`
///
/// <https://www.evanmiller.org/how-not-to-sort-by-average-rating.html>
pub fn calculate_score_from_rating(count: f64, rating: f64) -> f64 {
    let positive = count * rating;
    let negative = count * (1.0 - rating);

    ((positive + 1.9208) / count
        - 1.96 * (((positive * negative) / count + 0.9604).sqrt()) / count)
        / (1.0 + 3.8416 / count)
}

/// `check_program_is_not_shutting_down()`
///
/// Python raises `ShutdownException`; the Rust port returns it.
pub fn check_program_is_not_shutting_down() -> HydrusResult<()> {
    if crate::hydrus_globals::model_shutdown() {
        Err(HydrusException::new(
            ErrorKind::Shutdown,
            "Application is shutting down!",
        ))
    } else {
        Ok(())
    }
}

fn running_file_path(db_path: &std::path::Path, instance: &str) -> std::path::PathBuf {
    db_path.join(format!("{instance}_running"))
}

/// `clean_running_file( db_path, instance )`
pub fn clean_running_file(db_path: &std::path::Path, instance: &str) {
    // Just to be careful: a missing file (or permissions error) is not fatal.
    let _ = std::fs::remove_file(running_file_path(db_path, instance));
}

/// `last_shutdown_was_bad( db_path, instance )`
pub fn last_shutdown_was_bad(db_path: &std::path::Path, instance: &str) -> bool {
    running_file_path(db_path, instance).exists()
}

/// `convert_value_range_to_bytes( value, range )`
pub fn convert_value_range_to_bytes(value: u64, range: u64) -> String {
    format!("{}/{}", to_human_bytes(value), to_human_bytes(range))
}

/// `debug_print( debug_info )`
pub fn debug_print(debug_info: impl std::fmt::Display) {
    safe_print(debug_info);

    let _ = std::io::stdout().flush();
    let _ = std::io::stderr().flush();
}

/// `generate_key()`
pub fn generate_key() -> [u8; HYDRUS_KEY_LENGTH] {
    let mut key = [0_u8; HYDRUS_KEY_LENGTH];

    rand::thread_rng().fill_bytes(&mut key);

    key
}

/// `get_64_bit_hamming_distance( perceptual_hash1, perceptual_hash2 )`
///
/// The Python implementation unpacks both hashes as big-endian 64-bit integers.
/// `count_ones` is the port of `bin( n ).count( '1' )`.
pub fn get_64_bit_hamming_distance(perceptual_hash1: &[u8], perceptual_hash2: &[u8]) -> u32 {
    assert!(
        perceptual_hash1.len() >= 8 && perceptual_hash2.len() >= 8,
        "perceptual hashes must be at least 8 bytes"
    );

    let hash1 = u64::from_be_bytes(perceptual_hash1[..8].try_into().unwrap());
    let hash2 = u64::from_be_bytes(perceptual_hash2[..8].try_into().unwrap());

    (hash1 ^ hash2).count_ones()
}

/// Exact rational representation of a positive `f64`, like Python's
/// `fractions.Fraction( float )`.
fn rational_from_f64(value: f64) -> (i128, i128) {
    let bits = value.to_bits();

    let sign: i128 = if bits >> 63 == 1 { -1 } else { 1 };
    let exponent_bits = ((bits >> 52) & 0x7FF) as i32;
    let fraction = (bits & ((1 << 52) - 1)) as i128;

    let (mantissa, exponent) = if exponent_bits == 0 {
        (fraction, -1074)
    } else {
        (fraction | (1 << 52), exponent_bits - 1075)
    };

    let mantissa = sign * mantissa;

    if exponent >= 0 {
        (mantissa << exponent, 1)
    } else {
        let denominator = 1_i128 << (-exponent);

        let divisor = gcd(mantissa.unsigned_abs(), denominator as u128) as i128;

        (mantissa / divisor, denominator / divisor)
    }
}

fn gcd(mut a: u128, mut b: u128) -> u128 {
    while b != 0 {
        let remainder = a % b;
        a = b;
        b = remainder;
    }

    a
}

/// Python's `Fraction.limit_denominator( max_denominator )`.
fn limit_denominator(
    numerator: i128,
    denominator: i128,
    max_denominator: i128,
) -> (i128, i128) {
    let (mut p0, mut q0) = (0_i128, 1_i128);
    let (mut p1, mut q1) = (1_i128, 0_i128);

    let (mut n, mut d) = (numerator, denominator);

    loop {
        let a = n.div_euclid(d);
        let q2 = q0 + a * q1;

        if q2 > max_denominator {
            break;
        }

        let next_p0 = p1;
        let next_q0 = q1;
        let next_p1 = p0 + a * p1;
        let next_q1 = q2;

        p0 = next_p0;
        q0 = next_q0;
        p1 = next_p1;
        q1 = next_q1;

        let next_n = d;
        let next_d = n - a * d;

        n = next_n;
        d = next_d;
    }

    let k = (max_denominator - q0) / q1;

    let bound1 = (p0 + k * p1, q0 + k * q1);
    let bound2 = (p1, q1);

    let target = numerator as f64 / denominator as f64;

    let error1 = (bound1.0 as f64 / bound1.1 as f64 - target).abs();
    let error2 = (bound2.0 as f64 / bound2.1 as f64 - target).abs();

    if error2 <= error1 { bound2 } else { bound1 }
}

/// `get_nicely_divisible_number_for_zoom( zoom, no_bigger_than )`
///
/// Returns `None` where Python returns `-1`.
pub fn get_nicely_divisible_number_for_zoom(zoom: f64, no_bigger_than: i128) -> Option<i128> {
    let base_frac = rational_from_f64(zoom);

    let mut denominator_limit: i128 = 10_000;

    let mut frac = base_frac;

    while frac.0 > no_bigger_than {
        let (numerator, denominator) =
            limit_denominator(base_frac.0, base_frac.1, denominator_limit);

        frac = (numerator, denominator);

        denominator_limit /= 2;

        if denominator_limit < 10 {
            return None;
        }
    }

    if frac.0 == 0 { None } else { Some(frac.0) }
}

/// `get_empty_data_dict()`
///
/// Python returns `defaultdict( default_dict_list )`; this is the same empty
/// structure keyed by content type, then content status, holding file hashes.
pub type DataDict = HashMap<i64, HashMap<i64, HashSet<Vec<u8>>>>;

pub fn get_empty_data_dict() -> DataDict {
    HashMap::new()
}

/// `get_non_dupe_name( original_name, disallowed_names, do_casefold = False )`
pub fn get_non_dupe_name(
    original_name: &str,
    disallowed_names: &HashSet<String>,
    do_casefold: bool,
) -> String {
    let fold = |name: &str| {
        if do_casefold {
            name.to_lowercase()
        } else {
            name.to_string()
        }
    };

    let disallowed: HashSet<String> = disallowed_names.iter().map(|name| fold(name)).collect();

    let mut non_dupe_name = original_name.to_string();
    let mut i = 1;

    while disallowed.contains(&fold(&non_dupe_name)) {
        non_dupe_name = format!("{original_name} ({i})");

        i += 1;
    }

    non_dupe_name
}

/// `get_type_name( obj_type )`
pub fn get_type_name<T: ?Sized>() -> &'static str {
    std::any::type_name::<T>()
}

/// `safe_print( text )`
pub fn safe_print(text: impl std::fmt::Display) {
    println!("{text}");
}

/// Python's `show_text` alias, kept for the ported call sites.
pub use safe_print as show_text;

/// `convert_exception_tuple_to_nice_trace( etype, value, tb )`
///
/// `trace` and `stack` are the pre-formatted Python traceback and stack text;
/// Rust has no equivalent to `traceback.format_exception`, so callers provide
/// whatever context they captured. Returns `None` for shutdown, exactly like
/// Python.
pub fn convert_exception_tuple_to_nice_trace(
    exception: Option<&HydrusException>,
    trace: Option<&str>,
    stack: Option<&str>,
) -> Option<String> {
    let kind = exception.map(HydrusException::kind).unwrap_or(ErrorKind::Unknown);

    if kind == ErrorKind::Shutdown {
        return None;
    }

    let value = exception
        .map(ToString::to_string)
        .unwrap_or_else(|| "Unknown error".to_string());

    let trace = trace.unwrap_or("No error trace!").trim_end();
    let stack = stack.unwrap_or("No error trace!").trim_end();

    Some(format!(
        "================ Exception ================\n\
         {kind}: {value}\n\
         ================ Traceback ================\n\
         {trace}\n\
         ================== Stack ==================\n\
         {stack}\n\
         =================== End ==================="
    ))
}

/// `convert_exception_tuple_to_summary( etype, value, tb )`
pub fn convert_exception_tuple_to_summary(exception: Option<&HydrusException>) -> String {
    let kind = exception.map(HydrusException::kind).unwrap_or(ErrorKind::Unknown);

    let value = exception
        .map(ToString::to_string)
        .unwrap_or_else(|| "Unknown error".to_string());

    format!("{kind}: {value}")
}

/// `print_exception_tuple( etype, value, tb, do_wait = True )`
pub fn print_exception_tuple(
    exception: Option<&HydrusException>,
    trace: Option<&str>,
    stack: Option<&str>,
    do_wait: bool,
) -> String {
    if exception.map(HydrusException::kind) == Some(ErrorKind::Shutdown) {
        return "shutting down".to_string();
    }

    let message = convert_exception_tuple_to_nice_trace(exception, trace, stack)
        .unwrap_or_else(|| "shutting down".to_string());

    debug_print(format!("\n{message}"));

    if do_wait {
        std::thread::sleep(std::time::Duration::from_millis(200));
    }

    message
}

/// `print_exception( e, do_wait = True )`
pub fn print_exception(exception: &HydrusException, do_wait: bool) -> String {
    print_exception_tuple(Some(exception), None, None, do_wait)
}

/// Python's `show_exception` alias.
pub use print_exception as show_exception;
/// Python's `show_exception_tuple` alias.
pub use print_exception_tuple as show_exception_tuple;

/// `base_to_human_bytes( size, sig_figs = 3 )`
///
/// Python uses `decimal` to round to `sig_figs` significant figures with
/// `ROUND_HALF_EVEN` and trims trailing zeros. We do the same arithmetic on
/// floats: the values here are small powers of 1024, so the result is exact
/// enough for display.
pub fn base_to_human_bytes(size: Option<u64>, sig_figs: usize) -> String {
    let Some(size) = size else {
        return "unknown size".to_string();
    };

    if size < 1024 {
        return format!("{}B", to_human_int(size as i128));
    }

    const SUFFIXES: [&str; 6] = ["", "K", "M", "G", "T", "P"];

    let mut suffix_index = 0;
    let mut d = size as f64;

    while d >= 1024.0 {
        d /= 1024.0;
        suffix_index += 1;
    }

    let suffix = SUFFIXES[suffix_index];

    // If we have 237KB, we still want all 237, even if the user said 2 sf.
    let mut sig_figs = sig_figs;

    while d.log10() >= sig_figs as f64 {
        sig_figs += 1;
    }

    let rounded = round_to_significant(d, sig_figs);

    format!("{} {}B", format_trimmed(rounded), suffix)
}

/// `to_human_bytes` is an alias of `base_to_human_bytes` in Python.
pub fn to_human_bytes(size: u64) -> String {
    base_to_human_bytes(Some(size), 3)
}

fn round_to_significant(value: f64, sig_figs: usize) -> f64 {
    if value == 0.0 || !value.is_finite() {
        return value;
    }

    let exponent = value.abs().log10().floor() as i32;
    let shift = sig_figs as i32 - 1 - exponent;

    let factor = 10_f64.powi(shift);

    (value * factor).round_ties_even() / factor
}

/// Render a float without exponent notation or trailing zeros.
fn format_trimmed(value: f64) -> String {
    let text = format!("{value:.10}");

    let text = text.trim_end_matches('0').trim_end_matches('.');

    if text.is_empty() {
        "0".to_string()
    } else {
        text.to_string()
    }
}

/// `class Call`
///
/// A deferred function call. Python also forwards `*args` and `**kwargs`; the
/// Rust port stores a closure, which covers the same uses (and is what the
/// ported call sites need).
pub struct Call<T> {
    func: Box<dyn Fn() -> T>,
    label: Option<String>,
}

impl<T> Call<T> {
    pub fn new(func: impl Fn() -> T + 'static) -> Self {
        Self {
            func: Box::new(func),
            label: None,
        }
    }

    pub fn with_label(func: impl Fn() -> T + 'static, label: impl Into<String>) -> Self {
        Self {
            func: Box::new(func),
            label: Some(label.into()),
        }
    }

    pub fn call(&self) -> T {
        (self.func)()
    }

    pub fn get_label(&self) -> Option<&str> {
        self.label.as_deref()
    }

    pub fn set_label(&mut self, label: impl Into<String>) {
        self.label = Some(label.into());
    }
}

impl<T> std::fmt::Display for Call<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.label {
            Some(label) => write!(f, "Call: {label}"),
            None => f.write_str("Call: <deferred>"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_and_set_dicts() {
        let lists = build_key_to_list_dict([(1, "a"), (1, "b"), (2, "c")]);

        assert_eq!(lists[&1], vec!["a", "b"]);
        assert_eq!(lists[&2], vec!["c"]);

        let merged = merge_key_to_list_dicts([
            lists.clone(),
            build_key_to_list_dict([(1, "d")]),
        ]);

        assert_eq!(merged[&1], vec!["a", "b", "d"]);

        let sets = build_key_to_set_dict([(1, "a"), (1, "a"), (2, "b")]);

        assert_eq!(sets[&1].len(), 1);
    }

    #[test]
    fn hex_round_trip() {
        let bytes = vec![0x00, 0x1F, 0xAB, 0xFF];

        let hex = bytes_to_none_or_hex(Some(&bytes)).unwrap();

        assert_eq!(hex, "001fabff");
        assert_eq!(hex_to_none_or_bytes(Some(&hex)), Some(bytes));

        assert_eq!(bytes_to_none_or_hex(None), None);
        assert_eq!(hex_to_none_or_bytes(None), None);
    }

    #[test]
    fn rating_score_matches_formula() {
        // Calculated with the Python formula.
        let score = calculate_score_from_rating(10.0, 0.8);

        assert!((score - 0.4902).abs() < 0.001);
    }

    #[test]
    fn hamming_distance_counts_bits() {
        let a = [0_u8; 8];
        let mut b = [0_u8; 8];

        b[7] = 0b0000_1011;

        assert_eq!(get_64_bit_hamming_distance(&a, &b), 3);
        assert_eq!(get_64_bit_hamming_distance(&b, &b), 0);
    }

    #[test]
    fn key_is_the_right_size() {
        let key = generate_key();

        assert_eq!(key.len(), HYDRUS_KEY_LENGTH);
    }

    #[test]
    fn human_bytes_match_python_behaviour() {
        assert_eq!(base_to_human_bytes(None, 3), "unknown size");
        assert_eq!(base_to_human_bytes(Some(0), 3), "0B");
        assert_eq!(base_to_human_bytes(Some(999), 3), "999B");
        assert_eq!(base_to_human_bytes(Some(1000), 3), "1,000B");
        assert_eq!(base_to_human_bytes(Some(1024), 3), "1 KB");
        assert_eq!(base_to_human_bytes(Some(1536), 3), "1.5 KB");
        assert_eq!(base_to_human_bytes(Some(237 * 1024), 2), "237 KB");
        assert_eq!(base_to_human_bytes(Some(237 * 1024), 3), "237 KB");
        assert_eq!(
            base_to_human_bytes(Some(2 * 1024 * 1024), 3),
            "2 MB"
        );
        assert_eq!(
            base_to_human_bytes(Some(1024_u64.pow(5)), 3),
            "1 PB"
        );
    }

    #[test]
    fn non_dupe_names() {
        let disallowed: HashSet<String> = ["cat".to_string(), "cat (1)".to_string()]
            .into_iter()
            .collect();

        assert_eq!(get_non_dupe_name("cat", &disallowed, false), "cat (2)");
        assert_eq!(get_non_dupe_name("dog", &disallowed, false), "dog");

        let disallowed: HashSet<String> = ["Cat".to_string()].into_iter().collect();

        assert_eq!(get_non_dupe_name("cat", &disallowed, true), "cat (1)");
        assert_eq!(get_non_dupe_name("cat", &disallowed, false), "cat");
    }

    #[test]
    fn nicely_divisible_zoom_number() {
        assert_eq!(get_nicely_divisible_number_for_zoom(1.0, 1024), Some(1));
        assert_eq!(get_nicely_divisible_number_for_zoom(1.4, 1024), Some(7));
        assert_eq!(get_nicely_divisible_number_for_zoom(2.0, 1024), Some(2));
        assert_eq!(get_nicely_divisible_number_for_zoom(0.5, 1024), Some(1));
    }

    #[test]
    fn exception_trace_formatting() {
        let e = HydrusException::new(ErrorKind::Veto, "nope");

        let summary = convert_exception_tuple_to_summary(Some(&e));

        assert_eq!(summary, "VetoException: nope");

        let trace = convert_exception_tuple_to_nice_trace(Some(&e), None, None).unwrap();

        assert!(trace.starts_with("================ Exception ================\nVetoException: nope"));
        assert!(trace.ends_with("=================== End ==================="));

        let shutdown = HydrusException::new(ErrorKind::Shutdown, "bye");

        assert_eq!(
            convert_exception_tuple_to_nice_trace(Some(&shutdown), None, None),
            None
        );

        assert_eq!(
            print_exception_tuple(Some(&shutdown), None, None, false),
            "shutting down"
        );
    }

    #[test]
    fn call_is_deferred() {
        let counter = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));

        let call = Call::new({
            let counter = counter.clone();

            move || {
                counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            }
        });

        call.call();
        call.call();

        assert_eq!(counter.load(std::sync::atomic::Ordering::SeqCst), 2);

        let call = Call::with_label(|| 42, "the answer");

        assert_eq!(call.call(), 42);
        assert_eq!(call.to_string(), "Call: the answer");
    }

    #[test]
    fn type_name_is_available() {
        assert!(get_type_name::<Vec<u8>>().contains("Vec"));
    }
}
