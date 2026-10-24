//! Rust port of `hydrus/core/HydrusTags.py`.
//!
//! Tag strings are plain `String`s. Python's `_lock` around `TagFilter` is not
//! needed: the Rust type has no interior mutability, so shared reads are safe
//! by construction and mutations require `&mut self`.
//!
//! Rule ordering is preserved with `indexmap::IndexMap`, matching Python's
//! insertion-ordered dicts (and therefore on-disk tuple order).

use std::any::Any;
use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

use indexmap::IndexMap;
use serde_json::{Value, json};

use crate::hydrus_constants::Filter;
use crate::hydrus_exceptions::{ErrorKind, HydrusException, HydrusResult};
use crate::hydrus_numbers;
use crate::hydrus_serialisable::{
    self, SERIALISABLE_TYPE_TAG_FILTER, SerialisableObject, register_serialisable_type,
};
use crate::hydrus_text;

pub const ALL_UNNAMESPACED_TAG_SLICE: &str = "";
pub const ALL_NAMESPACED_TAG_SLICE: &str = ":";

/// `CORE_TAG_SLICES`
pub fn core_tag_slices() -> HashSet<&'static str> {
    HashSet::from([ALL_UNNAMESPACED_TAG_SLICE, ALL_NAMESPACED_TAG_SLICE])
}

// ============================================================================
// Simple tag helpers
// ============================================================================

/// `collapse_multiple_sorted_numeric_tags_to_min_max`
///
/// The input is assumed sorted. A list of three or more decimal tags becomes
/// its first and last entries; anything with a non-decimal tag is unchanged.
pub fn collapse_multiple_sorted_numeric_tags_to_min_max(tags: &[String]) -> Vec<String> {
    if tags.len() <= 2 {
        return tags.to_vec();
    }

    // Python's `str.isdecimal()` is Unicode `Nd`. Rust has no direct Unicode
    // category test in std; `is_numeric()` is the closest available and only
    // differs for a handful of non-decimal numeric characters.
    if tags.iter().any(|tag| !tag.chars().all(|c| c.is_numeric())) {
        return tags.to_vec();
    }

    vec![tags[0].clone(), tags[tags.len() - 1].clone()]
}

/// `filter_namespaces`
///
/// `None` in `namespaces` means the unnamed namespace, exactly like Python.
pub fn filter_namespaces(tags: &HashSet<String>, namespaces: &[Option<&str>]) -> HashSet<String> {
    let mut processed_tags: HashMap<&str, HashSet<&str>> = HashMap::new();

    for tag in tags {
        let (namespace, _subtag) = split_tag(tag);

        processed_tags.entry(namespace).or_default().insert(tag);
    }

    let mut result = HashSet::new();

    for namespace in namespaces {
        let key = namespace.unwrap_or("");

        if let Some(tags) = processed_tags.get(key) {
            result.extend(tags.iter().map(|tag| (*tag).to_string()));
        }
    }

    result
}

/// `sort_numeric_tags`
pub fn sort_numeric_tags<I, S>(tags: I) -> Vec<String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut tags: Vec<String> = tags
        .into_iter()
        .map(|tag| tag.as_ref().to_string())
        .collect();

    tags.sort_by_cached_key(|tag| hydrus_text::human_text_sort_key(tag));

    tags
}

/// `check_tag_not_empty`
pub fn check_tag_not_empty(tag: &str) -> HydrusResult<()> {
    let (_namespace, subtag) = split_tag(tag);

    if subtag.is_empty() {
        Err(HydrusException::new(
            ErrorKind::TagSize,
            "Received a zero-length tag!",
        ))
    } else {
        Ok(())
    }
}

/// `clean_tag`
///
/// Lower-cases, truncates to 1024 characters, strips gumpf and normalises the
/// namespace/subtag split.
pub fn clean_tag(tag: &str) -> HydrusResult<String> {
    clean_tag_inner(tag).map_err(|e| {
        HydrusException::new(
            ErrorKind::Parse,
            format!("Was unable to parse the tag: {tag}\n\n{e}"),
        )
    })
}

fn clean_tag_inner(tag: &str) -> HydrusResult<String> {
    let mut tag: String = tag.chars().take(1024).collect();

    tag = tag.to_lowercase();

    if hydrus_text::RE_LEADING_SINGLE_COLON_AND_NO_MORE_COLONS.is_match(&tag) {
        // ":D" -> "::D", but leave ":weird:stuff" alone (it is a forced
        // unnamespaced tag whose subtag contains a colon).
        tag = format!(":{tag}");
    }

    if tag.contains(':') {
        tag = strip_tag_text_of_gumpf(&tag); // catches "system:" too

        let (namespace, subtag) = split_tag(&tag);

        let namespace = strip_tag_text_of_gumpf(namespace);
        let subtag = strip_tag_text_of_gumpf(subtag);

        Ok(combine_tag(&namespace, &subtag))
    } else {
        Ok(strip_tag_text_of_gumpf(&tag))
    }
}

/// `clean_tags`
///
/// `None` entries are skipped by the caller; empty subtags are dropped, as in
/// Python's `except TagSizeException: continue`.
pub fn clean_tags<I, S>(tags: I) -> HydrusResult<HashSet<String>>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut clean_tags = HashSet::new();

    for tag in tags {
        let tag = clean_tag(tag.as_ref())?;

        if check_tag_not_empty(&tag).is_ok() {
            clean_tags.insert(tag);
        }
    }

    Ok(clean_tags)
}

/// `combine_tag`
pub fn combine_tag(namespace: &str, subtag: &str) -> String {
    if namespace.is_empty() {
        if subtag.contains(':') {
            format!(":{subtag}")
        } else {
            subtag.to_string()
        }
    } else {
        format!("{namespace}:{subtag}")
    }
}

/// `convert_tag_slice_to_pretty_string`
pub fn convert_tag_slice_to_pretty_string(tag_slice: &str) -> String {
    if tag_slice.is_empty() {
        "unnamespaced tags".to_string()
    } else if tag_slice == ":" {
        "namespaced tags".to_string()
    } else if is_namespace_tag_slice(tag_slice) {
        let namespace = &tag_slice[..tag_slice.len() - 1];

        format!("'{namespace}' tags")
    } else {
        tag_slice.to_string()
    }
}

/// `convert_ugly_namespace_to_pretty_string`
pub fn convert_ugly_namespace_to_pretty_string(namespace: Option<&str>) -> String {
    match namespace {
        None | Some("") => "no namespace".to_string(),
        Some(namespace) => namespace.to_string(),
    }
}

/// `convert_ugly_namespaces_to_pretty_strings`
pub fn convert_ugly_namespaces_to_pretty_strings(namespaces: &[Option<&str>]) -> Vec<String> {
    let mut namespaces: Vec<&str> = namespaces
        .iter()
        .map(|namespace| namespace.unwrap_or(""))
        .collect();

    namespaces.sort_unstable();

    namespaces
        .into_iter()
        .map(|namespace| {
            convert_ugly_namespace_to_pretty_string(if namespace.is_empty() {
                None
            } else {
                Some(namespace)
            })
        })
        .collect()
}

/// `is_namespace_tag_slice`
///
/// A slice like `creator:`. The empty and bare-colon slices are not namespace
/// slices.
pub fn is_namespace_tag_slice(tag_slice: &str) -> bool {
    !tag_slice.is_empty()
        && tag_slice != ALL_NAMESPACED_TAG_SLICE
        && tag_slice.ends_with(':')
        && tag_slice.matches(':').count() == 1
}

/// `is_unnamespaced`
pub fn is_unnamespaced(tag: &str) -> bool {
    split_tag(tag).0.is_empty()
}

/// `split_tag`
pub fn split_tag(tag: &str) -> (&str, &str) {
    match tag.split_once(':') {
        Some((namespace, subtag)) => (namespace, subtag),
        None => ("", tag),
    }
}

/// `strip_tag_text_of_gumpf`
pub fn strip_tag_text_of_gumpf(t: &str) -> String {
    let mut t = hydrus_text::RE_UNDESIRED_CONTROL_CHARACTERS
        .replace_all(t, "")
        .into_owned();

    t = hydrus_text::RE_ONE_OR_MORE_WHITESPACE
        .replace_all(&t, " ")
        .into_owned();
    t = t.trim().to_string();

    t = hydrus_text::RE_LEADING_GARBAGE
        .replace_all(&t, "")
        .into_owned();
    t = t.trim().to_string();

    if !hydrus_text::RE_LOOKS_LIKE_HANGUL.is_match(&t) {
        t = t.replace(hydrus_text::HANGUL_FILLER_CHARACTER, "");
    }

    if hydrus_text::RE_THIS_IS_ALL_LATIN_AND_ZERO_WIDTH.is_match(&t) {
        t = hydrus_text::RE_ZERO_WIDTH_JOINERS
            .replace_all(&t, "")
            .into_owned();
    }

    t = hydrus_text::RE_OOPS_ALL_ZERO_WIDTH_JOINERS
        .replace_all(&t, "")
        .into_owned();

    t = hydrus_text::RE_ONE_OR_MORE_WHITESPACE
        .replace_all(&t, " ")
        .into_owned();

    t.trim().to_string()
}

/// `tag_ok`
pub fn tag_ok(t: &str) -> bool {
    clean_tag(t)
        .map(|tag| check_tag_not_empty(&tag).is_ok())
        .unwrap_or(false)
}

// ============================================================================
// TagFilter
// ============================================================================

const WOAH_TOO_MANY_RULES_THRESHOLD: usize = 12;

static TAG_FILTER_REGISTERED: OnceLock<()> = OnceLock::new();

fn ensure_registered() {
    TAG_FILTER_REGISTERED.get_or_init(|| {
        register_serialisable_type(SERIALISABLE_TYPE_TAG_FILTER, || Box::new(TagFilter::new()));
    });
}

/// Register `TagFilter` with the serialisable registry. Idempotent; also
/// called by [`crate::initialise`].
pub fn register_serialisable_types() {
    ensure_registered();
}

/// `TagFilter`: an ordered map of tag slices (`''`, `':'`, `creator:`, or a
/// whole tag) to [`Filter`] rules, plus a derived rule cache.
#[derive(Clone, Debug)]
pub struct TagFilter {
    tag_slices_to_rules: IndexMap<String, Filter>,

    all_unnamespaced_whitelisted: bool,
    all_namespaced_whitelisted: bool,
    namespaces_whitelist: HashSet<String>,
    tags_whitelist: HashSet<String>,

    all_unnamespaced_blacklisted: bool,
    all_namespaced_blacklisted: bool,
    namespaces_blacklist: HashSet<String>,
    tags_blacklist: HashSet<String>,

    namespaces_interesting: bool,
    tags_interesting: bool,
}

impl Default for TagFilter {
    fn default() -> Self {
        Self::new()
    }
}

impl PartialEq for TagFilter {
    fn eq(&self, other: &Self) -> bool {
        self.tag_slices_to_rules == other.tag_slices_to_rules
    }
}

impl Eq for TagFilter {}

impl TagFilter {
    pub fn new() -> Self {
        ensure_registered();

        Self {
            tag_slices_to_rules: IndexMap::new(),
            all_unnamespaced_whitelisted: false,
            all_namespaced_whitelisted: false,
            namespaces_whitelist: HashSet::new(),
            tags_whitelist: HashSet::new(),
            all_unnamespaced_blacklisted: false,
            all_namespaced_blacklisted: false,
            namespaces_blacklist: HashSet::new(),
            tags_blacklist: HashSet::new(),
            namespaces_interesting: false,
            tags_interesting: false,
        }
    }

    fn tag_ok_inner(&self, tag: &str, apply_unnamespaced_rules_to_namespaced_tags: bool) -> bool {
        // Exceptions resolve tags before namespaces, and namespaces before the
        // all-namespaces rules.
        if self.tags_interesting {
            if self.tags_whitelist.contains(tag) {
                return true;
            }

            if self.tags_blacklist.contains(tag) {
                return false;
            }

            if apply_unnamespaced_rules_to_namespaced_tags {
                let (namespace, subtag) = split_tag(tag);

                if !namespace.is_empty() {
                    if self.tags_whitelist.contains(subtag) {
                        return true;
                    }

                    if self.tags_blacklist.contains(subtag) {
                        return false;
                    }
                }
            }
        }

        if self.namespaces_interesting {
            let (namespace, _subtag) = split_tag(tag);

            if namespace.is_empty() {
                if self.all_unnamespaced_whitelisted {
                    return true;
                }

                if self.all_unnamespaced_blacklisted {
                    return false;
                }
            } else {
                if self.namespaces_whitelist.contains(namespace) {
                    return true;
                }

                if self.namespaces_blacklist.contains(namespace) {
                    return false;
                }

                if self.all_namespaced_whitelisted {
                    return true;
                }

                if self.all_namespaced_blacklisted {
                    return false;
                }
            }
        }

        true
    }

    fn update_rule_cache(&mut self) {
        self.all_unnamespaced_whitelisted = false;
        self.all_namespaced_whitelisted = false;
        self.namespaces_whitelist.clear();
        self.tags_whitelist.clear();

        self.all_unnamespaced_blacklisted = false;
        self.all_namespaced_blacklisted = false;
        self.namespaces_blacklist.clear();
        self.tags_blacklist.clear();

        self.namespaces_interesting = false;
        self.tags_interesting = false;

        for (tag_slice, rule) in &self.tag_slices_to_rules {
            let whitelisting = *rule == Filter::Whitelist;

            if tag_slice.is_empty() {
                if whitelisting {
                    self.all_unnamespaced_whitelisted = true;
                } else {
                    self.all_unnamespaced_blacklisted = true;
                }

                self.namespaces_interesting = true;
            } else if tag_slice == ":" {
                if whitelisting {
                    self.all_namespaced_whitelisted = true;
                } else {
                    self.all_namespaced_blacklisted = true;
                }

                self.namespaces_interesting = true;
            } else if is_namespace_tag_slice(tag_slice) {
                let namespace = &tag_slice[..tag_slice.len() - 1];

                if whitelisting {
                    self.namespaces_whitelist.insert(namespace.to_string());
                } else {
                    self.namespaces_blacklist.insert(namespace.to_string());
                }

                self.namespaces_interesting = true;
            } else {
                if whitelisting {
                    self.tags_whitelist.insert(tag_slice.clone());
                } else {
                    self.tags_blacklist.insert(tag_slice.clone());
                }

                self.tags_interesting = true;
            }
        }
    }

    /// `allows_everything`
    pub fn allows_everything(&self) -> bool {
        !self
            .tag_slices_to_rules
            .values()
            .any(|rule| *rule == Filter::Blacklist)
    }

    /// `clean_rules`
    pub fn clean_rules(&mut self) {
        let mut new_tag_slices_to_rules = IndexMap::new();

        for (tag_slice, rule) in &self.tag_slices_to_rules {
            let tag_slice = if tag_slice.is_empty() || tag_slice == ":" {
                tag_slice.clone()
            } else if is_namespace_tag_slice(tag_slice) {
                let example_tag = format!("{tag_slice}example");

                let Ok(clean_example_tag) = clean_tag(&example_tag) else {
                    continue;
                };

                clean_example_tag[..clean_example_tag.len() - "example".len()].to_string()
            } else {
                let Ok(clean_tag) = clean_tag(tag_slice) else {
                    continue;
                };

                clean_tag
            };

            new_tag_slices_to_rules.insert(tag_slice, *rule);
        }

        self.tag_slices_to_rules = new_tag_slices_to_rules;

        self.update_rule_cache();
    }

    /// `filter`
    pub fn filter(
        &self,
        tags: &HashSet<String>,
        apply_unnamespaced_rules_to_namespaced_tags: bool,
    ) -> HashSet<String> {
        tags.iter()
            .filter(|tag| self.tag_ok_inner(tag, apply_unnamespaced_rules_to_namespaced_tags))
            .cloned()
            .collect()
    }

    /// `get_changes`
    pub fn get_changes(
        &self,
        old_tag_filter: &TagFilter,
    ) -> (
        Vec<(String, Filter)>,
        Vec<(String, Filter)>,
        Vec<(String, Filter)>,
    ) {
        let old_slices_to_rules = old_tag_filter.get_tag_slices_to_rules();

        let new_rules: Vec<(String, Filter)> = self
            .tag_slices_to_rules
            .iter()
            .filter(|(tag_slice, _)| !old_slices_to_rules.contains_key(*tag_slice))
            .map(|(tag_slice, rule)| (tag_slice.clone(), *rule))
            .collect();

        let changed_rules: Vec<(String, Filter)> = self
            .tag_slices_to_rules
            .iter()
            .filter(|(tag_slice, rule)| {
                old_slices_to_rules
                    .get(*tag_slice)
                    .is_some_and(|old_rule| old_rule != *rule)
            })
            .map(|(tag_slice, rule)| (tag_slice.clone(), *rule))
            .collect();

        let deleted_rules: Vec<(String, Filter)> = old_slices_to_rules
            .iter()
            .filter(|(tag_slice, _)| !self.tag_slices_to_rules.contains_key(*tag_slice))
            .map(|(tag_slice, rule)| (tag_slice.clone(), *rule))
            .collect();

        (new_rules, changed_rules, deleted_rules)
    }

    /// `get_changes_summary_text`
    ///
    /// This intentionally mirrors the Python implementation, including using
    /// the count of *new* rules for the changed/deleted branches.
    pub fn get_changes_summary_text(&self, old_tag_filter: &TagFilter) -> String {
        let (new_rules, changed_rules, deleted_rules) = self.get_changes(old_tag_filter);

        let mut summary_components = Vec::new();

        if !new_rules.is_empty() {
            if new_rules.len() > WOAH_TOO_MANY_RULES_THRESHOLD {
                summary_components.push(format!(
                    "Added {} rules",
                    hydrus_numbers::base_to_human_int(new_rules.len() as i128)
                ));
            } else {
                let rows: Vec<String> = new_rules
                    .iter()
                    .map(|(tag_slice, rule)| {
                        format!(
                            "Added rule: {} - {}",
                            rule.as_str(),
                            convert_tag_slice_to_pretty_string(tag_slice)
                        )
                    })
                    .collect();

                summary_components.push(rows.join("\n"));
            }
        }

        if !changed_rules.is_empty() {
            if new_rules.len() > WOAH_TOO_MANY_RULES_THRESHOLD {
                summary_components.push(format!(
                    "Changed {} rules",
                    hydrus_numbers::base_to_human_int(new_rules.len() as i128)
                ));
            } else {
                let rows: Vec<String> = changed_rules
                    .iter()
                    .map(|(tag_slice, rule)| {
                        format!(
                            "Flipped rule: to {} - {}",
                            rule.as_str(),
                            convert_tag_slice_to_pretty_string(tag_slice)
                        )
                    })
                    .collect();

                summary_components.push(rows.join("\n"));
            }
        }

        if !deleted_rules.is_empty() {
            if new_rules.len() > WOAH_TOO_MANY_RULES_THRESHOLD {
                summary_components.push(format!(
                    "Deleted {} rules",
                    hydrus_numbers::base_to_human_int(new_rules.len() as i128)
                ));
            } else {
                let rows: Vec<String> = deleted_rules
                    .iter()
                    .map(|(tag_slice, rule)| {
                        format!(
                            "Deleted rule: {} - {}",
                            rule.as_str(),
                            convert_tag_slice_to_pretty_string(tag_slice)
                        )
                    })
                    .collect();

                summary_components.push(rows.join("\n"));
            }
        }

        summary_components.join("\n")
    }

    /// `get_inverted_filter`
    pub fn get_inverted_filter(&self) -> TagFilter {
        let mut inverted_tag_filter = TagFilter::new();

        if !self.tag_slices_to_rules.contains_key("") {
            inverted_tag_filter.set_rule("", Filter::Blacklist);
        }

        if !self.tag_slices_to_rules.contains_key(":") {
            inverted_tag_filter.set_rule(":", Filter::Blacklist);
        }

        let blacklisted: Vec<String> = self
            .tag_slices_to_rules
            .iter()
            .filter(|(_, rule)| **rule == Filter::Blacklist)
            .map(|(tag_slice, _)| tag_slice.clone())
            .collect();

        inverted_tag_filter.set_rules(blacklisted, Filter::Whitelist);

        let whitelisted: Vec<String> = self
            .tag_slices_to_rules
            .iter()
            .filter(|(_, rule)| **rule == Filter::Whitelist)
            .map(|(tag_slice, _)| tag_slice.clone())
            .collect();

        inverted_tag_filter.set_rules(whitelisted, Filter::Blacklist);

        inverted_tag_filter
    }

    /// `get_tag_slices_to_rules` (a defensive copy, like Python's `dict(...)`).
    pub fn get_tag_slices_to_rules(&self) -> IndexMap<String, Filter> {
        self.tag_slices_to_rules.clone()
    }

    /// `set_rule`
    pub fn set_rule(&mut self, tag_slice: &str, rule: Filter) {
        self.set_rules([tag_slice], rule);
    }

    /// `set_rules`
    pub fn set_rules<I, S>(&mut self, tag_slices: I, rule: Filter)
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        for tag_slice in tag_slices {
            self.tag_slices_to_rules
                .insert(tag_slice.as_ref().to_string(), rule);
        }

        self.update_rule_cache();
    }

    /// `tag_ok`
    pub fn tag_ok(&self, tag: &str, apply_unnamespaced_rules_to_namespaced_tags: bool) -> bool {
        self.tag_ok_inner(tag, apply_unnamespaced_rules_to_namespaced_tags)
    }

    /// `to_blacklist_string`
    pub fn to_blacklist_string(&self) -> String {
        let (blacklist, whitelist) = self.sorted_black_white_lists();

        if blacklist.is_empty() {
            return "no blacklist set".to_string();
        }

        let blacklist_set: HashSet<&str> = blacklist.iter().map(String::as_str).collect();

        let mut text = if blacklist_set == HashSet::from(["", ":"]) {
            "blacklisting on any tags".to_string()
        } else if blacklist.len() > WOAH_TOO_MANY_RULES_THRESHOLD {
            format!(
                "blacklisting on {} rules",
                hydrus_numbers::base_to_human_int(blacklist.len() as i128)
            )
        } else {
            format!(
                "blacklisting on {}",
                blacklist
                    .iter()
                    .map(|tag_slice| convert_tag_slice_to_pretty_string(tag_slice))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        };

        if !whitelist.is_empty() {
            if whitelist.len() > WOAH_TOO_MANY_RULES_THRESHOLD {
                text.push_str(&format!(
                    " except {} other rules",
                    hydrus_numbers::base_to_human_int(whitelist.len() as i128)
                ));
            } else {
                text.push_str(&format!(
                    " except {}",
                    whitelist
                        .iter()
                        .map(|tag_slice| convert_tag_slice_to_pretty_string(tag_slice))
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
            }
        }

        text
    }

    /// `to_filter_string`
    pub fn to_filter_string(&self) -> String {
        let (blacklist, whitelist) = self.sorted_black_white_lists();

        let blacklist_set: HashSet<&str> = blacklist.iter().map(String::as_str).collect();
        let functional_blacklist: Vec<&String> = blacklist
            .iter()
            .filter(|tag_slice| !tag_slice.is_empty() && tag_slice.as_str() != ":")
            .collect();

        let functional_whitelist: Vec<&String> = whitelist
            .iter()
            .filter(|tag_slice| !tag_slice.is_empty() && tag_slice.as_str() != ":")
            .collect();

        if blacklist.is_empty() {
            return "all tags".to_string();
        }

        if blacklist_set.contains("") && blacklist_set.contains(":") {
            if whitelist.is_empty() {
                return "no tags!".to_string();
            }

            let mut text = if whitelist.len() > WOAH_TOO_MANY_RULES_THRESHOLD {
                format!(
                    "only adding on {} rules",
                    hydrus_numbers::base_to_human_int(whitelist.len() as i128)
                )
            } else {
                format!("only adding {}", sorted_pretty(&whitelist))
            };

            if !functional_blacklist.is_empty() {
                if functional_blacklist.len() > WOAH_TOO_MANY_RULES_THRESHOLD {
                    text.push_str(&format!(
                        " while still filtering on {} rules",
                        hydrus_numbers::base_to_human_int(functional_blacklist.len() as i128)
                    ));
                } else {
                    text.push_str(&format!(
                        " while still filtering out {}",
                        sorted_pretty_refs(&functional_blacklist)
                    ));
                }
            }

            return text;
        }

        if blacklist_set.contains("") || blacklist_set.contains(":") {
            let mut text = if blacklist_set.contains("") {
                "all namespaced tags".to_string()
            } else {
                "all unnamespaced tags".to_string()
            };

            if whitelist.len() > WOAH_TOO_MANY_RULES_THRESHOLD {
                text.push_str(&format!(
                    " and {} other rules",
                    hydrus_numbers::base_to_human_int(functional_whitelist.len() as i128)
                ));
            } else if !whitelist.is_empty() {
                text.push_str(&format!(
                    " and {}",
                    sorted_pretty_refs(&functional_whitelist)
                ));
            }

            if !functional_blacklist.is_empty() {
                if functional_blacklist.len() > WOAH_TOO_MANY_RULES_THRESHOLD {
                    text.push_str(&format!(
                        " while still filtering on {} rules",
                        hydrus_numbers::base_to_human_int(functional_blacklist.len() as i128)
                    ));
                } else {
                    text.push_str(&format!(
                        " while still filtering out {}",
                        sorted_pretty_refs(&functional_blacklist)
                    ));
                }
            }

            return text;
        }

        let mut text = if blacklist.len() > WOAH_TOO_MANY_RULES_THRESHOLD {
            format!(
                "all tags except on {} rules",
                hydrus_numbers::base_to_human_int(blacklist.len() as i128)
            )
        } else {
            format!("all tags except {}", sorted_pretty(&blacklist))
        };

        if functional_whitelist.len() > WOAH_TOO_MANY_RULES_THRESHOLD {
            text.push_str(&format!(
                " while still allowing on {} rules",
                hydrus_numbers::base_to_human_int(functional_whitelist.len() as i128)
            ));
        } else if !functional_whitelist.is_empty() {
            text.push_str(&format!(
                " while still allowing {}",
                sorted_pretty_refs(&functional_whitelist)
            ));
        }

        text
    }

    /// `to_permitted_string`
    pub fn to_permitted_string(&self) -> String {
        let (blacklist, whitelist) = self.sorted_black_white_lists();

        let blacklist_set: HashSet<&str> = blacklist.iter().map(String::as_str).collect();
        let functional_blacklist: Vec<&String> = blacklist
            .iter()
            .filter(|tag_slice| !tag_slice.is_empty() && tag_slice.as_str() != ":")
            .collect();

        let functional_whitelist: Vec<&String> = whitelist
            .iter()
            .filter(|tag_slice| !tag_slice.is_empty() && tag_slice.as_str() != ":")
            .collect();

        if blacklist.is_empty() {
            return "allowing all tags".to_string();
        }

        if blacklist_set.contains("") && blacklist_set.contains(":") {
            if whitelist.is_empty() {
                return "not allowing any tags".to_string();
            }

            let mut text = if whitelist.len() > WOAH_TOO_MANY_RULES_THRESHOLD {
                format!(
                    "only allowing on {} rules",
                    hydrus_numbers::base_to_human_int(whitelist.len() as i128)
                )
            } else {
                format!("only allowing {}", sorted_pretty(&whitelist))
            };

            if !functional_blacklist.is_empty() {
                if functional_blacklist.len() > WOAH_TOO_MANY_RULES_THRESHOLD {
                    text.push_str(&format!(
                        " while still disllowing {} rules",
                        hydrus_numbers::base_to_human_int(functional_blacklist.len() as i128)
                    ));
                } else {
                    text.push_str(&format!(
                        " while still disallowing {}",
                        sorted_pretty_refs(&functional_blacklist)
                    ));
                }
            }

            return text;
        }

        if blacklist_set.contains("") || blacklist_set.contains(":") {
            let mut text = if blacklist_set.contains("") {
                "allowing all namespaced tags".to_string()
            } else {
                "allowing all unnamespaced tags".to_string()
            };

            if whitelist.len() > WOAH_TOO_MANY_RULES_THRESHOLD {
                text.push_str(&format!(
                    " and {} other rules",
                    hydrus_numbers::base_to_human_int(functional_whitelist.len() as i128)
                ));
            } else if !whitelist.is_empty() {
                text.push_str(&format!(
                    " and {}",
                    sorted_pretty_refs(&functional_whitelist)
                ));
            }

            if !functional_blacklist.is_empty() {
                if functional_blacklist.len() > WOAH_TOO_MANY_RULES_THRESHOLD {
                    text.push_str(&format!(
                        " while still disllowing {} rules",
                        hydrus_numbers::base_to_human_int(functional_blacklist.len() as i128)
                    ));
                } else {
                    text.push_str(&format!(
                        " while still disallowing {}",
                        sorted_pretty_refs(&functional_blacklist)
                    ));
                }
            }

            return text;
        }

        let mut text = if blacklist.len() > WOAH_TOO_MANY_RULES_THRESHOLD {
            format!(
                "allowing all tags except on {} rules",
                hydrus_numbers::base_to_human_int(blacklist.len() as i128)
            )
        } else {
            format!("allowing all tags except {}", sorted_pretty(&blacklist))
        };

        if functional_whitelist.len() > WOAH_TOO_MANY_RULES_THRESHOLD {
            text.push_str(&format!(
                " while still allowing {} rules",
                hydrus_numbers::base_to_human_int(functional_whitelist.len() as i128)
            ));
        } else if !functional_whitelist.is_empty() {
            text.push_str(&format!(
                " while still allowing {}",
                sorted_pretty_refs(&functional_whitelist)
            ));
        }

        text
    }

    fn sorted_black_white_lists(&self) -> (Vec<String>, Vec<String>) {
        let mut blacklist = Vec::new();
        let mut whitelist = Vec::new();

        for (tag_slice, rule) in &self.tag_slices_to_rules {
            match rule {
                Filter::Blacklist => blacklist.push(tag_slice.clone()),
                Filter::Whitelist => whitelist.push(tag_slice.clone()),
            }
        }

        blacklist.sort();
        whitelist.sort();

        (blacklist, whitelist)
    }
}

fn sorted_pretty(tag_slices: &[String]) -> String {
    let mut sorted: Vec<&String> = tag_slices.iter().collect();
    sorted.sort();

    sorted_pretty_refs(&sorted)
}

fn sorted_pretty_refs(tag_slices: &[&String]) -> String {
    let mut sorted: Vec<&&String> = tag_slices.iter().collect();
    sorted.sort();

    sorted
        .into_iter()
        .map(|tag_slice| convert_tag_slice_to_pretty_string(tag_slice))
        .collect::<Vec<_>>()
        .join(", ")
}

impl SerialisableObject for TagFilter {
    fn serialisable_type(&self) -> u16 {
        SERIALISABLE_TYPE_TAG_FILTER
    }

    fn serialisable_version(&self) -> u16 {
        1
    }

    fn serialisable_name(&self) -> &'static str {
        "Tag Filter Rules"
    }

    fn get_serialisable_info(&self) -> Value {
        Value::Array(
            self.tag_slices_to_rules
                .iter()
                .map(|(tag_slice, rule)| json!([tag_slice, rule.as_int()]))
                .collect(),
        )
    }

    fn initialise_from_serialisable_info(&mut self, serialisable_info: &Value) -> HydrusResult<()> {
        let pairs = serialisable_info.as_array().ok_or_else(|| {
            HydrusException::new(
                ErrorKind::Serialisation,
                "Tag filter serialisable info was not an array!",
            )
        })?;

        let mut tag_slices_to_rules = IndexMap::new();

        for pair in pairs {
            let pair = pair.as_array().ok_or_else(|| {
                HydrusException::new(
                    ErrorKind::Serialisation,
                    "Tag filter rule was not an array!",
                )
            })?;

            let (Some(tag_slice), Some(rule)) = (pair.first(), pair.get(1)) else {
                return Err(HydrusException::new(
                    ErrorKind::Serialisation,
                    "Tag filter rule was not length 2!",
                ));
            };

            let tag_slice = tag_slice.as_str().ok_or_else(|| {
                HydrusException::new(
                    ErrorKind::Serialisation,
                    "Tag filter slice was not a string!",
                )
            })?;

            let rule_int = rule
                .as_u64()
                .and_then(|value| u8::try_from(value).ok())
                .and_then(Filter::try_from_int)
                .ok_or_else(|| {
                    HydrusException::new(
                        ErrorKind::Serialisation,
                        "Tag filter rule was not a valid filter value!",
                    )
                })?;

            tag_slices_to_rules.insert(tag_slice.to_string(), rule_int);
        }

        self.tag_slices_to_rules = tag_slices_to_rules;

        self.update_rule_cache();

        Ok(())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tags(values: &[&str]) -> HashSet<String> {
        values.iter().map(|value| value.to_string()).collect()
    }

    #[test]
    fn split_and_combine_round_trip() {
        assert_eq!(split_tag("creator:blah"), ("creator", "blah"));
        assert_eq!(split_tag("blah"), ("", "blah"));
        assert_eq!(split_tag(":blah:stuff"), ("", "blah:stuff"));

        assert_eq!(combine_tag("creator", "blah"), "creator:blah");
        assert_eq!(combine_tag("", "blah"), "blah");
        assert_eq!(combine_tag("", "blah:stuff"), ":blah:stuff");

        assert!(is_unnamespaced("blah"));
        assert!(!is_unnamespaced("creator:blah"));
        assert!(is_namespace_tag_slice("creator:"));
        assert!(!is_namespace_tag_slice(":"));
        assert!(!is_namespace_tag_slice(""));
        assert!(!is_namespace_tag_slice("creator:blah"));
    }

    #[test]
    fn clean_tag_normalises() {
        assert_eq!(clean_tag("  Blue   Eyes  ").unwrap(), "blue eyes");
        assert_eq!(clean_tag(":D").unwrap(), "::d");
        assert_eq!(clean_tag(":weird:stuff").unwrap(), ":weird:stuff");
        assert_eq!(
            clean_tag("CREATOR:  Some   Guy ").unwrap(),
            "creator:some guy"
        );
        assert_eq!(clean_tag("series:\u{200b}name").unwrap(), "series:name");
    }

    #[test]
    fn clean_tags_drops_empty_and_duplicates() {
        let cleaned = clean_tags(["blue eyes", "Blue  Eyes", "creator:", ""]).unwrap();

        assert_eq!(cleaned, tags(&["blue eyes"]));
    }

    #[test]
    fn empty_tag_is_a_tag_size_error() {
        let error = check_tag_not_empty("creator:").unwrap_err();

        assert_eq!(error.kind, ErrorKind::TagSize);
        assert!(!tag_ok("creator:"));
        assert!(tag_ok("creator:blah"));
    }

    #[test]
    fn filter_namespaces_handles_none_as_unnamespaced() {
        let all = tags(&["blue eyes", "creator:blah", "series:one"]);

        assert_eq!(filter_namespaces(&all, &[None]), tags(&["blue eyes"]));
        assert_eq!(
            filter_namespaces(&all, &[Some("creator")]),
            tags(&["creator:blah"])
        );
        assert_eq!(
            filter_namespaces(&all, &[None, Some("series")]),
            tags(&["blue eyes", "series:one"])
        );
    }

    #[test]
    fn numeric_tags_collapse_to_min_max() {
        let three = vec!["1".to_string(), "2".to_string(), "3".to_string()];

        assert_eq!(
            collapse_multiple_sorted_numeric_tags_to_min_max(&three),
            vec!["1".to_string(), "3".to_string()]
        );

        let mixed = vec!["1".to_string(), "two".to_string(), "3".to_string()];

        assert_eq!(
            collapse_multiple_sorted_numeric_tags_to_min_max(&mixed),
            mixed
        );
    }

    #[test]
    fn tag_filter_blacklists_namespaces_with_tag_exceptions() {
        let mut filter = TagFilter::new();

        filter.set_rule("creator:", Filter::Blacklist);
        filter.set_rule("creator:good", Filter::Whitelist);

        assert!(filter.tag_ok("creator:good", false));
        assert!(!filter.tag_ok("creator:bad", false));
        assert!(filter.tag_ok("blue eyes", false));
        assert!(!filter.allows_everything());

        let filtered = filter.filter(&tags(&["creator:good", "creator:bad", "blue eyes"]), false);

        assert_eq!(filtered, tags(&["creator:good", "blue eyes"]));
    }

    #[test]
    fn unnamespaced_rules_can_apply_to_namespaced_tags() {
        let mut filter = TagFilter::new();

        filter.set_rule("good", Filter::Blacklist);

        assert!(filter.tag_ok("creator:good", false));
        assert!(!filter.tag_ok("creator:good", true));
    }

    #[test]
    fn inverted_filter_flips_rules() {
        let mut filter = TagFilter::new();

        filter.set_rule("creator:", Filter::Blacklist);
        filter.set_rule("series:good", Filter::Whitelist);

        let inverted = filter.get_inverted_filter();
        let rules = inverted.get_tag_slices_to_rules();

        assert_eq!(rules.get("creator:"), Some(&Filter::Whitelist));
        assert_eq!(rules.get("series:good"), Some(&Filter::Blacklist));
        assert_eq!(rules.get(""), Some(&Filter::Blacklist));
        assert_eq!(rules.get(":"), Some(&Filter::Blacklist));
    }

    #[test]
    fn changes_and_summaries() {
        let mut old_filter = TagFilter::new();
        old_filter.set_rule("creator:", Filter::Blacklist);
        old_filter.set_rule("series:", Filter::Whitelist);

        let mut filter = TagFilter::new();
        filter.set_rule("creator:", Filter::Whitelist);
        filter.set_rule("blue eyes", Filter::Blacklist);

        let (new_rules, changed_rules, deleted_rules) = filter.get_changes(&old_filter);

        assert_eq!(
            new_rules,
            vec![("blue eyes".to_string(), Filter::Blacklist)]
        );
        assert_eq!(
            changed_rules,
            vec![("creator:".to_string(), Filter::Whitelist)]
        );
        assert_eq!(
            deleted_rules,
            vec![("series:".to_string(), Filter::Whitelist)]
        );

        let summary = filter.get_changes_summary_text(&old_filter);

        assert!(summary.contains("Added rule: blacklist - blue eyes"));
        assert!(summary.contains("Flipped rule: to whitelist - 'creator' tags"));
        assert!(summary.contains("Deleted rule: whitelist - 'series' tags"));
    }

    #[test]
    fn strings_are_human_readable() {
        let mut filter = TagFilter::new();

        assert_eq!(filter.to_filter_string(), "all tags");
        assert_eq!(filter.to_permitted_string(), "allowing all tags");
        assert_eq!(filter.to_blacklist_string(), "no blacklist set");

        filter.set_rule("creator:", Filter::Blacklist);

        assert_eq!(filter.to_filter_string(), "all tags except 'creator' tags");
        assert_eq!(
            filter.to_permitted_string(),
            "allowing all tags except 'creator' tags"
        );
        assert_eq!(
            filter.to_blacklist_string(),
            "blacklisting on 'creator' tags"
        );
    }

    #[test]
    fn clean_rules_cleans_namespaces_and_tags() {
        let mut filter = TagFilter::new();

        filter.set_rule("CREATOR:", Filter::Blacklist);
        filter.set_rule("  Blue   Eyes ", Filter::Whitelist);
        filter.set_rule("", Filter::Whitelist);

        filter.clean_rules();

        let rules = filter.get_tag_slices_to_rules();

        assert_eq!(rules.get("creator:"), Some(&Filter::Blacklist));
        assert_eq!(rules.get("blue eyes"), Some(&Filter::Whitelist));
        assert_eq!(rules.get(""), Some(&Filter::Whitelist));
    }

    #[test]
    fn serialisable_round_trip() {
        crate::initialise();

        let mut filter = TagFilter::new();

        filter.set_rule("creator:", Filter::Blacklist);
        filter.set_rule("blue eyes", Filter::Whitelist);

        let dumped = filter.dump_to_string();
        let loaded = hydrus_serialisable::create_from_string(&dumped, false).unwrap();

        let loaded = loaded.as_any().downcast_ref::<TagFilter>().unwrap();

        assert_eq!(loaded, &filter);
        assert_eq!(loaded.tag_ok("creator:bad", false), false);
        assert_eq!(loaded.tag_ok("blue eyes", false), true);
    }
}
