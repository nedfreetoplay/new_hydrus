//! Search predicates from `hydrus/client/search/ClientSearchPredicate.py`.

use crate::search::client_number_test::NumberTest;
use crate::search::predicate_constants::PredicateType;

mod deserialise;
mod hex;
mod methods;
mod serialisation;

pub const SERIALISABLE_VERSION: u16 = 8;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct PredicateCount {
    pub min_current_count: i64,
    pub min_pending_count: i64,
    pub max_current_count: Option<i64>,
    pub max_pending_count: Option<i64>,
}

impl PredicateCount {
    pub fn get_min_count(&self) -> i64 {
        self.min_current_count + self.min_pending_count
    }

    pub fn get_suffix_string(&self) -> String {
        if self.get_min_count() > 0 {
            format!("({})", self.get_min_count())
        } else {
            String::new()
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum PredicateValue {
    None,
    Text(String),
    Number(NumberTest),
    Numbers(Vec<i32>),
    Hashes(Vec<Vec<u8>>),
    Service {
        operator: i32,
        value: Option<f64>,
        service_key: Vec<u8>,
    },
    FileService {
        is_in: bool,
        status: i32,
        service_key: Vec<u8>,
    },
    Date {
        operator: String,
        date_type: String,
        values: Vec<i32>,
    },
    KnownUrl {
        rule_type: String,
        rule: String,
    },
    Similar {
        hashes: Vec<Vec<u8>>,
        distance: u32,
        /// True for `similar to files` (values are file hashes); false for
        /// `similar to data` (values are perceptual/pixel hashes).
        files: bool,
    },
    Relationships {
        number: NumberTest,
        kind: String,
    },
    ViewingStats {
        canvas_types: Vec<String>,
        operator: i32,
        value: f64,
        viewtime: bool,
    },
    Or(Vec<Predicate>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Predicate {
    pub predicate_type: PredicateType,
    pub value: PredicateValue,
    pub inclusive: bool,
    pub count: PredicateCount,
}

#[cfg(test)]
mod tests;
