//! `FileSystemPredicates` from `hydrus/client/search/ClientSearchFileSearchContext.py`.

use std::collections::BTreeMap;

use hydrus_core::hydrus_constants::Mime;

use crate::search::client_number_test::NumberTest;
use crate::search::client_search_predicate::{Predicate, PredicateValue};
use crate::search::predicate_constants::*;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct FileSystemPredicates {
    pub inbox: bool,
    pub archive: bool,
    pub local: bool,
    pub not_local: bool,
    pub system_everything: bool,
    pub limit: Option<i64>,
    pub mimes: Vec<Mime>,
    pub hashes: Vec<Vec<u8>>,
    pub num_tags: BTreeMap<String, Vec<NumberTest>>,
    pub tags_include: Vec<String>,
    pub tags_exclude: Vec<String>,
    pub namespaces_include: Vec<String>,
    pub namespaces_exclude: Vec<String>,
    pub wildcards_include: Vec<String>,
    pub wildcards_exclude: Vec<String>,
    pub or_predicates: Vec<Predicate>,
    pub other_system: Vec<Predicate>,
}

impl FileSystemPredicates {
    pub fn new(predicates: &[Predicate]) -> Self {
        let mut system = Self::default();

        for predicate in predicates {
            match predicate.predicate_type {
                PredicateType::SystemInbox => system.inbox = true,
                PredicateType::SystemArchive => system.archive = true,
                PredicateType::SystemLocal => system.local = true,
                PredicateType::SystemNotLocal => system.not_local = true,
                PredicateType::SystemEverything => system.system_everything = true,
                PredicateType::SystemLimit => {
                    if let PredicateValue::Number(number) = &predicate.value {
                        system.limit = Some(number.value as i64);
                    }
                }
                PredicateType::SystemMime => {
                    if let PredicateValue::Numbers(mimes) = &predicate.value {
                        system.mimes = mimes
                            .iter()
                            .filter_map(|mime| Mime::try_from_int(*mime as u16))
                            .collect();
                    }
                }
                PredicateType::SystemHash => {
                    if let PredicateValue::Hashes(hashes) = &predicate.value {
                        system.hashes.extend(hashes.iter().cloned());
                    }
                }
                PredicateType::SystemNumTags => {
                    if let PredicateValue::Number(number) = &predicate.value {
                        let namespace = predicate.get_namespace().unwrap_or("").to_string();
                        system
                            .num_tags
                            .entry(namespace)
                            .or_default()
                            .push(number.clone());
                    }
                }
                PredicateType::SystemSize => system.other_system.push(predicate.clone()),
                PredicateType::Tag | PredicateType::Namespace | PredicateType::Wildcard => {
                    classify_tag_predicate(&mut system, predicate)
                }
                PredicateType::OrContainer => system.or_predicates.push(predicate.clone()),
                _ => system.other_system.push(predicate.clone()),
            }
        }

        system
    }

    pub fn must_be_inbox(&self) -> bool {
        self.inbox
    }

    pub fn must_be_archive(&self) -> bool {
        self.archive
    }

    pub fn must_be_local(&self) -> bool {
        self.local
    }

    pub fn must_not_be_local(&self) -> bool {
        self.not_local
    }

    pub fn has_system_everything(&self) -> bool {
        self.system_everything
    }

    pub fn has_system_limit(&self) -> bool {
        self.limit.is_some()
    }

    pub fn get_limit(&self, apply_implicit_limit: bool) -> Option<i64> {
        if self.limit.is_some() || !apply_implicit_limit {
            self.limit
        } else {
            None
        }
    }

    pub fn get_allowed_filetypes(&self) -> &[Mime] {
        &self.mimes
    }

    pub fn has_allowed_filetypes(&self) -> bool {
        !self.mimes.is_empty()
    }

    pub fn get_num_tags_number_tests(&self) -> &BTreeMap<String, Vec<NumberTest>> {
        &self.num_tags
    }

    pub fn get_similar_to_files(&self) -> bool {
        false
    }

    pub fn get_simple_info(&self) -> String {
        let mut parts = Vec::new();

        if self.inbox {
            parts.push("inbox".to_string());
        }
        if self.archive {
            parts.push("archive".to_string());
        }
        if self.local {
            parts.push("local".to_string());
        }
        if self.not_local {
            parts.push("not local".to_string());
        }
        if !self.mimes.is_empty() {
            parts.push(format!(
                "mime: {}",
                self.mimes
                    .iter()
                    .map(|mime| mime.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        if let Some(limit) = self.limit {
            parts.push(format!("limit {limit}"));
        }

        parts.join(", ")
    }
}

fn classify_tag_predicate(system: &mut FileSystemPredicates, predicate: &Predicate) {
    let PredicateValue::Text(text) = &predicate.value else {
        return;
    };

    let target = match predicate.predicate_type {
        PredicateType::Tag => {
            if predicate.inclusive {
                &mut system.tags_include
            } else {
                &mut system.tags_exclude
            }
        }
        PredicateType::Namespace => {
            if predicate.inclusive {
                &mut system.namespaces_include
            } else {
                &mut system.namespaces_exclude
            }
        }
        PredicateType::Wildcard => {
            if predicate.inclusive {
                &mut system.wildcards_include
            } else {
                &mut system.wildcards_exclude
            }
        }
        _ => return,
    };

    target.push(text.clone());
}

#[cfg(test)]
mod tests;
