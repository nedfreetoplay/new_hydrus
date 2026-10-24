use std::collections::{BTreeMap, BTreeSet};

use hydrus_core::hydrus_constants::{Mime, ServiceType};

use super::operator::{LogicalOperator, Operator, RelationalOperator, Unit};
use super::predicate::Predicate;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DateOrTimeInterval {
    DateTime(chrono::NaiveDateTime),
    Interval {
        years: u32,
        months: u32,
        days: u32,
        hours: u32,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub enum ValueData {
    Natural(u64),
    Integer(i64),
    HashListWithDistance {
        hashes: BTreeSet<String>,
        distance: u64,
    },
    SimilarToHashListWithDistance {
        pixel_hashes: BTreeSet<String>,
        perceptual_hashes: BTreeSet<String>,
        distance: u64,
    },
    HashListWithAlgorithm {
        hashes: BTreeSet<String>,
        algorithm: String,
    },
    FiletypeList(BTreeSet<u16>),
    DateOrTimeInterval(DateOrTimeInterval),
    TimeToMsec(i64),
    AnyString(String),
    TimeInterval {
        days: u32,
        hours: u32,
        minutes: u32,
        seconds: u32,
    },
    Ratio {
        first: u32,
        second: u32,
    },
    RatingServiceNameAndNumericalValue {
        value: u32,
        service_name: String,
    },
    RatingServiceNameAndLikeDislike {
        value: f64,
        service_name: String,
    },
    RatingServiceNameAndIncDec {
        value: i64,
        service_name: String,
    },
    NamespaceAndNumTags {
        namespace: String,
        operator: RelationalOperator,
        num: u32,
    },
    TagAdvancedTag(String),
    RatingAdvanced {
        logical_operator: LogicalOperator,
        primary: ServiceSpecifier,
        secondary: ServiceSpecifier,
        rated: bool,
    },
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ServiceSpecifier {
    pub service_types: BTreeSet<ServiceType>,
    pub service_names: Vec<String>,
}

/// The result of `parse_system_predicate`.
#[derive(Clone, Debug, PartialEq)]
pub struct SystemPredicate {
    pub predicate: Predicate,
    pub operator: Option<Operator>,
    pub value: Option<ValueData>,
    pub unit: Option<Unit>,
}

/// Maps filetype names to the enum values Hydrus uses.
#[derive(Clone, Debug, Default)]
pub struct FiletypeRegistry {
    values: BTreeMap<String, Vec<u16>>,
}

impl FiletypeRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Initialise from Hydrus' own mime names and mimetypes.
    pub fn hydrus_defaults() -> Self {
        let mut registry = Self::new();

        for mime in Mime::ALL {
            let value = vec![mime.as_int()];
            registry.values.insert(mime.as_str().to_string(), value.clone());
            registry.values.insert(mime.mimetype_str().to_string(), value);
        }

        registry.insert("gif", vec![Mime::StaticGif.as_int(), Mime::AnimationGif.as_int()]);
        registry.insert("image/gif", vec![Mime::StaticGif.as_int(), Mime::AnimationGif.as_int()]);

        registry
    }

    pub fn insert(&mut self, name: impl Into<String>, values: Vec<u16>) {
        self.values.insert(name.into(), values);
    }

    pub fn get(&self, name: &str) -> Option<&[u16]> {
        self.values.get(name).map(Vec::as_slice)
    }

    pub fn contains(&self, name: &str) -> bool {
        self.values.contains_key(name)
    }

    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.values.keys().map(String::as_str)
    }
}
