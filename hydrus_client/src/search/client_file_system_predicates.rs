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
                PREDICATE_TYPE_SYSTEM_INBOX => system.inbox = true,
                PREDICATE_TYPE_SYSTEM_ARCHIVE => system.archive = true,
                PREDICATE_TYPE_SYSTEM_LOCAL => system.local = true,
                PREDICATE_TYPE_SYSTEM_NOT_LOCAL => system.not_local = true,
                PREDICATE_TYPE_SYSTEM_EVERYTHING => system.system_everything = true,
                PREDICATE_TYPE_SYSTEM_LIMIT => {
                    if let PredicateValue::Number(number) = &predicate.value {
                        system.limit = Some(number.value as i64);
                    }
                }
                PREDICATE_TYPE_SYSTEM_MIME => {
                    if let PredicateValue::Numbers(mimes) = &predicate.value {
                        system.mimes = mimes
                            .iter()
                            .filter_map(|mime| Mime::try_from_int(*mime as u16))
                            .collect();
                    }
                }
                PREDICATE_TYPE_SYSTEM_HASH => {
                    if let PredicateValue::Hashes(hashes) = &predicate.value {
                        system.hashes.extend(hashes.iter().cloned());
                    }
                }
                PREDICATE_TYPE_SYSTEM_NUM_TAGS => {
                    if let PredicateValue::Number(number) = &predicate.value {
                        let namespace = predicate.get_namespace().unwrap_or("").to_string();
                        system.num_tags.entry(namespace).or_default().push(number.clone());
                    }
                }
                PREDICATE_TYPE_SYSTEM_SIZE => system.other_system.push(predicate.clone()),
                PREDICATE_TYPE_TAG | PREDICATE_TYPE_NAMESPACE | PREDICATE_TYPE_WILDCARD => {
                    classify_tag_predicate(&mut system, predicate)
                }
                PREDICATE_TYPE_OR_CONTAINER => system.or_predicates.push(predicate.clone()),
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
        PREDICATE_TYPE_TAG => {
            if predicate.inclusive {
                &mut system.tags_include
            } else {
                &mut system.tags_exclude
            }
        }
        PREDICATE_TYPE_NAMESPACE => {
            if predicate.inclusive {
                &mut system.namespaces_include
            } else {
                &mut system.namespaces_exclude
            }
        }
        PREDICATE_TYPE_WILDCARD => {
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
mod tests {
    use super::*;
    use crate::search::client_number_test::NUMBER_TEST_OPERATOR_EQUAL;
    use crate::search::client_search_predicate::Predicate;

    fn number(predicate_type: i32, value: f64) -> Predicate {
        Predicate::system_number(
            predicate_type,
            NumberTest::new(NUMBER_TEST_OPERATOR_EQUAL, value, None),
        )
    }

    #[test]
    fn simple_booleans_are_extracted() {
        let predicates = [
            Predicate::new(PREDICATE_TYPE_SYSTEM_INBOX, PredicateValue::None),
            Predicate::new(PREDICATE_TYPE_SYSTEM_LOCAL, PredicateValue::None),
            Predicate::tag("blue eyes"),
            {
                let mut predicate = Predicate::tag("bad tag");
                predicate.set_inclusive(false);
                predicate
            },
        ];

        let system = FileSystemPredicates::new(&predicates);

        assert!(system.must_be_inbox());
        assert!(system.must_be_local());
        assert_eq!(system.tags_include, vec!["blue eyes"]);
        assert_eq!(system.tags_exclude, vec!["bad tag"]);
    }

    #[test]
    fn mime_and_limit_are_extracted() {
        let predicates = [
            Predicate::new(
                PREDICATE_TYPE_SYSTEM_MIME,
                PredicateValue::Numbers(vec![Mime::Png.as_int() as i32, Mime::Jpeg.as_int() as i32]),
            ),
            number(PREDICATE_TYPE_SYSTEM_LIMIT, 100.0),
        ];

        let system = FileSystemPredicates::new(&predicates);

        assert_eq!(system.get_allowed_filetypes(), &[Mime::Png, Mime::Jpeg]);
        assert_eq!(system.get_limit(true), Some(100));
    }

    #[test]
    fn number_tags_are_grouped_by_namespace() {
        let predicate = number(PREDICATE_TYPE_SYSTEM_NUM_TAGS, 2.0);
        let system = FileSystemPredicates::new(&[predicate]);

        assert!(system.get_num_tags_number_tests().contains_key(""));
    }

    #[test]
    fn size_predicates_are_kept_as_system() {
        let system = FileSystemPredicates::new(&[number(PREDICATE_TYPE_SYSTEM_SIZE, 1024.0)]);

        assert_eq!(system.other_system.len(), 1);
    }
}
