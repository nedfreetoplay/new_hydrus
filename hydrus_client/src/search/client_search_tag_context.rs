//! Tag search context from `hydrus/client/search/ClientSearchTagContext.py`.

use std::any::Any;

use hydrus_core::hydrus_exceptions::HydrusResult;
use hydrus_core::hydrus_serialisable::{self, SerialisableObject};
use serde_json::{Value, json};

use crate::client_constants;

pub const SERIALISABLE_VERSION: u16 = 2;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagContext {
    pub service_key: Vec<u8>,
    pub include_current_tags: bool,
    pub include_pending_tags: bool,
    pub display_service_key: Vec<u8>,
}

impl Default for TagContext {
    fn default() -> Self {
        Self::new(
            client_constants::COMBINED_TAG_SERVICE_KEY.to_vec(),
            true,
            true,
            None,
        )
    }
}

impl TagContext {
    pub fn new(
        service_key: Vec<u8>,
        include_current_tags: bool,
        include_pending_tags: bool,
        display_service_key: Option<Vec<u8>>,
    ) -> Self {
        let display_service_key = display_service_key.unwrap_or_else(|| service_key.clone());

        Self {
            service_key,
            include_current_tags,
            include_pending_tags,
            display_service_key,
        }
    }

    pub fn is_all_known_tags(&self) -> bool {
        self.service_key == client_constants::COMBINED_TAG_SERVICE_KEY
    }

    pub fn fix_missing_services(&mut self, exists: impl Fn(&[u8]) -> bool) {
        if !exists(&self.service_key) {
            self.service_key = client_constants::COMBINED_TAG_SERVICE_KEY.to_vec();
        }

        if !exists(&self.display_service_key) {
            self.display_service_key = client_constants::COMBINED_TAG_SERVICE_KEY.to_vec();
        }
    }

    pub fn to_string(&self, mut name_method: impl FnMut(&[u8]) -> String) -> String {
        name_method(&self.service_key)
    }

    pub fn to_dict_for_api(&self) -> Value {
        json!({
            "service_key": hex_encode(&self.service_key),
            "include_current_tags": self.include_current_tags,
            "include_pending_tags": self.include_pending_tags,
            "display_service_key": hex_encode(&self.display_service_key),
        })
    }
}

impl SerialisableObject for TagContext {
    fn serialisable_type(&self) -> u16 {
        hydrus_serialisable::SERIALISABLE_TYPE_TAG_CONTEXT
    }

    fn serialisable_version(&self) -> u16 {
        SERIALISABLE_VERSION
    }

    fn serialisable_name(&self) -> &'static str {
        "Tag Search Context"
    }

    fn get_serialisable_info(&self) -> Value {
        json!([
            hex_encode(&self.service_key),
            self.include_current_tags,
            self.include_pending_tags,
            hex_encode(&self.display_service_key),
        ])
    }

    fn initialise_from_serialisable_info(&mut self, info: &Value) -> HydrusResult<()> {
        let values = info.as_array().ok_or_else(|| {
            hydrus_core::hydrus_exceptions::serialisation("TagContext info must be a list")
        })?;

        self.service_key = values
            .first()
            .and_then(Value::as_str)
            .and_then(hex_decode)
            .unwrap_or_else(|| client_constants::COMBINED_TAG_SERVICE_KEY.to_vec());
        self.include_current_tags = values.get(1).and_then(Value::as_bool).unwrap_or(true);
        self.include_pending_tags = values.get(2).and_then(Value::as_bool).unwrap_or(true);
        self.display_service_key = values
            .get(3)
            .and_then(Value::as_str)
            .and_then(hex_decode)
            .unwrap_or_else(|| self.service_key.clone());

        Ok(())
    }

    fn update_serialisable_info(
        &self,
        version: u16,
        old_info: Value,
    ) -> HydrusResult<(u16, Value)> {
        if version != 1 {
            return Ok((version, old_info));
        }

        let values = old_info.as_array().cloned().unwrap_or_default();
        let service_key = values
            .first()
            .cloned()
            .unwrap_or_else(|| json!(hex_encode(client_constants::COMBINED_TAG_SERVICE_KEY)));

        Ok((
            2,
            json!([
                service_key,
                values.get(1).and_then(Value::as_bool).unwrap_or(true),
                values.get(2).and_then(Value::as_bool).unwrap_or(true),
                service_key,
            ]),
        ))
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
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
mod tests;
