//! File-location contexts from `hydrus/client/ClientLocation.py`.

use std::any::Any;
use std::collections::BTreeSet;

use hydrus_core::hydrus_constants::{self, ContentStatus};
use hydrus_core::hydrus_exceptions::HydrusResult;
use hydrus_core::hydrus_serialisable::{self, SerialisableObject};
use serde_json::{Value, json};

use crate::client_constants;

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct LocationContext {
    current_service_keys: BTreeSet<Vec<u8>>,
    deleted_service_keys: BTreeSet<Vec<u8>>,
}

impl LocationContext {
    pub fn new(
        current_service_keys: impl IntoIterator<Item = Vec<u8>>,
        deleted_service_keys: impl IntoIterator<Item = Vec<u8>>,
    ) -> Self {
        Self {
            current_service_keys: current_service_keys.into_iter().collect(),
            deleted_service_keys: deleted_service_keys.into_iter().collect(),
        }
    }

    pub fn static_create_all_current(
        current_service_keys: impl IntoIterator<Item = Vec<u8>>,
    ) -> Self {
        Self::new(current_service_keys, std::iter::empty())
    }

    pub fn static_create_simple(file_service_key: impl Into<Vec<u8>>) -> Self {
        Self::new([file_service_key.into()], std::iter::empty())
    }

    pub fn current_service_keys(&self) -> &BTreeSet<Vec<u8>> {
        &self.current_service_keys
    }

    pub fn deleted_service_keys(&self) -> &BTreeSet<Vec<u8>> {
        &self.deleted_service_keys
    }

    pub fn includes_current(&self) -> bool {
        !self.current_service_keys.is_empty()
    }

    pub fn includes_deleted(&self) -> bool {
        !self.deleted_service_keys.is_empty()
    }

    pub fn is_empty(&self) -> bool {
        self.current_service_keys.is_empty() && self.deleted_service_keys.is_empty()
    }

    pub fn is_all_known_files(&self) -> bool {
        self.current_service_keys
            .contains(client_constants::COMBINED_FILE_SERVICE_KEY)
    }

    pub fn is_one_domain(&self) -> bool {
        self.current_service_keys.len() + self.deleted_service_keys.len() == 1
    }

    pub fn is_hydrus_local_file_storage(&self) -> bool {
        self.is_one_domain()
            && self
                .current_service_keys
                .contains(client_constants::HYDRUS_LOCAL_FILE_STORAGE_SERVICE_KEY)
    }

    pub fn is_combined_local_file_domains(&self) -> bool {
        self.is_one_domain()
            && self
                .current_service_keys
                .contains(client_constants::COMBINED_LOCAL_FILE_DOMAINS_SERVICE_KEY)
    }

    /// Returns `(service_keys, file_location_is_cross_referenced)`.
    pub fn get_covering_current_file_service_keys(&self) -> (Vec<Vec<u8>>, bool) {
        let file_location_is_cross_referenced =
            !(self.is_all_known_files() || self.includes_deleted());

        let mut keys: Vec<Vec<u8>> = self.current_service_keys.iter().cloned().collect();

        if self.includes_deleted() {
            keys.push(client_constants::COMBINED_DELETED_FILE_SERVICE_KEY.to_vec());
        }

        (keys, file_location_is_cross_referenced)
    }

    pub fn get_deleted_inverse(&self) -> Self {
        Self {
            current_service_keys: self.deleted_service_keys.clone(),
            deleted_service_keys: self.current_service_keys.clone(),
        }
    }

    pub fn get_statuses_and_service_keys_list(&self) -> Vec<(ContentStatus, Vec<u8>)> {
        let mut statuses =
            Vec::with_capacity(self.current_service_keys.len() + self.deleted_service_keys.len());

        for key in &self.current_service_keys {
            statuses.push((ContentStatus::Current, key.clone()));
        }

        for key in &self.deleted_service_keys {
            statuses.push((ContentStatus::Deleted, key.clone()));
        }

        statuses
    }

    pub fn fix_missing_services(
        &mut self,
        mut services_exist: impl FnMut(&[Vec<u8>]) -> Vec<Vec<u8>>,
    ) -> bool {
        let previous_len = self.current_service_keys.len() + self.deleted_service_keys.len();

        let current: Vec<Vec<u8>> = self.current_service_keys.iter().cloned().collect();
        let deleted: Vec<Vec<u8>> = self.deleted_service_keys.iter().cloned().collect();

        self.current_service_keys = services_exist(&current).into_iter().collect();
        self.deleted_service_keys = services_exist(&deleted).into_iter().collect();

        previous_len != self.current_service_keys.len() + self.deleted_service_keys.len()
    }

    pub fn limit_to_service_types(
        &mut self,
        mut service_type: impl FnMut(&[u8]) -> hydrus_constants::ServiceType,
        allowed: &[hydrus_constants::ServiceType],
    ) {
        self.current_service_keys
            .retain(|key| allowed.contains(&service_type(key)));
        self.deleted_service_keys
            .retain(|key| allowed.contains(&service_type(key)));
    }

    pub fn to_string(&self, mut name_method: impl FnMut(&[u8]) -> String) -> String {
        if self.is_empty() {
            return "nothing".to_string();
        }

        let prefix = if self.includes_current() && self.includes_deleted() {
            if self.current_service_keys == self.deleted_service_keys {
                "current and deleted files of "
            } else {
                "a mix of current and deleted files of "
            }
        } else if self.includes_deleted() {
            "deleted files of "
        } else {
            ""
        };

        let keys: BTreeSet<&Vec<u8>> = if self.current_service_keys == self.deleted_service_keys {
            self.current_service_keys.iter().collect()
        } else {
            self.current_service_keys
                .iter()
                .chain(self.deleted_service_keys.iter())
                .collect()
        };

        let service_string = if keys.len() <= 2 {
            let mut names: Vec<String> = keys.into_iter().map(|key| name_method(key)).collect();
            names.sort();
            names.join(", ")
        } else {
            format!(
                "{} services",
                hydrus_core::hydrus_numbers::base_to_human_int(keys.len() as i128)
            )
        };

        format!("{prefix}{service_string}")
    }

    pub fn to_dict_for_api(&self) -> serde_json::Value {
        serde_json::json!({
            "current_service_keys": self.current_service_keys.iter().map(hex_encode).collect::<Vec<_>>(),
            "deleted_service_keys": self.deleted_service_keys.iter().map(hex_encode).collect::<Vec<_>>(),
        })
    }
}

fn hex_encode(bytes: &Vec<u8>) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

impl SerialisableObject for LocationContext {
    fn serialisable_type(&self) -> u16 {
        hydrus_serialisable::SERIALISABLE_TYPE_LOCATION_CONTEXT
    }

    fn serialisable_version(&self) -> u16 {
        1
    }

    fn serialisable_name(&self) -> &'static str {
        "Location Context"
    }

    fn get_serialisable_info(&self) -> Value {
        json!([
            self.current_service_keys
                .iter()
                .map(hex_encode)
                .collect::<Vec<_>>(),
            self.deleted_service_keys
                .iter()
                .map(hex_encode)
                .collect::<Vec<_>>(),
        ])
    }

    fn initialise_from_serialisable_info(&mut self, info: &Value) -> HydrusResult<()> {
        let values = info.as_array().ok_or_else(|| {
            hydrus_core::hydrus_exceptions::serialisation("LocationContext info must be a list")
        })?;

        let decode = |value: Option<&Value>| -> BTreeSet<Vec<u8>> {
            value
                .and_then(Value::as_array)
                .map(|values| {
                    values
                        .iter()
                        .filter_map(Value::as_str)
                        .filter_map(hex_decode)
                        .collect()
                })
                .unwrap_or_default()
        };

        self.current_service_keys = decode(values.first());
        self.deleted_service_keys = decode(values.get(1));

        Ok(())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

pub fn register_serialisable_type() -> bool {
    hydrus_core::hydrus_serialisable::register_serialisable_type(
        hydrus_serialisable::SERIALISABLE_TYPE_LOCATION_CONTEXT,
        || Box::new(LocationContext::default()),
    )
}

fn hex_decode(value: &str) -> Option<Vec<u8>> {
    if value.len() % 2 != 0 {
        return None;
    }

    (0..value.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(&value[index..index + 2], 16).ok())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(value: &str) -> Vec<u8> {
        value.as_bytes().to_vec()
    }

    #[test]
    fn simple_context_is_one_domain() {
        let context = LocationContext::static_create_simple(key("my files"));

        assert!(context.includes_current());
        assert!(!context.includes_deleted());
        assert!(context.is_one_domain());
        assert!(!context.is_all_known_files());
    }

    #[test]
    fn deleted_inverse_swaps_sets() {
        let context = LocationContext::new([key("a")], [key("b")]);
        let inverse = context.get_deleted_inverse();

        assert_eq!(inverse.current_service_keys(), &BTreeSet::from([key("b")]));
        assert_eq!(inverse.deleted_service_keys(), &BTreeSet::from([key("a")]));
    }

    #[test]
    fn statuses_match_python_order() {
        let context = LocationContext::new([key("a")], [key("b")]);
        let statuses = context.get_statuses_and_service_keys_list();

        assert_eq!(
            statuses,
            vec![
                (ContentStatus::Current, key("a")),
                (ContentStatus::Deleted, key("b"))
            ]
        );
    }

    #[test]
    fn string_summary_uses_service_names() {
        let context = LocationContext::new([key("a")], [key("b")]);
        let text = context.to_string(|_| "test".to_string());

        assert_eq!(text, "a mix of current and deleted files of test, test");
    }

    #[test]
    fn covering_keys_add_deleted_service() {
        let context = LocationContext::new([key("a")], [key("b")]);
        let (keys, cross_referenced) = context.get_covering_current_file_service_keys();

        assert!(keys.contains(&key("a")));
        assert!(keys.contains(&client_constants::COMBINED_DELETED_FILE_SERVICE_KEY.to_vec()));
        assert!(!cross_referenced);
    }
}
