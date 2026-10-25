//! Split out from the parent module.

use hydrus_core::hydrus_exceptions::HydrusResult;
use hydrus_core::hydrus_serialisable::SerialisableObject;
use serde_json::Value;
use std::any::Any;

use super::*;

impl SerialisableObject for ClientOptions {
    fn serialisable_type(&self) -> u16 {
        SERIALISABLE_TYPE
    }

    fn serialisable_version(&self) -> u16 {
        SERIALISABLE_VERSION
    }

    fn serialisable_name(&self) -> &'static str {
        "Client Options"
    }

    fn get_serialisable_info(&self) -> Value {
        Value::Object(self.dictionary.clone())
    }

    fn initialise_from_serialisable_info(&mut self, serialisable_info: &Value) -> HydrusResult<()> {
        self.dictionary = serialisable_info.as_object().cloned().unwrap_or_default();

        Ok(())
    }

    fn update_serialisable_info(
        &self,
        version: u16,
        old_serialisable_info: Value,
    ) -> HydrusResult<(u16, Value)> {
        migrations::migrate(version, old_serialisable_info)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

pub fn register_serialisable_type() -> bool {
    hydrus_core::hydrus_serialisable::register_serialisable_type(SERIALISABLE_TYPE, || {
        Box::new(ClientOptions::new())
    })
}

pub fn is_registered() -> bool {
    hydrus_core::hydrus_serialisable::serialisable_type_is_registered(SERIALISABLE_TYPE)
}

pub(super) fn hex_decode(value: &str) -> Option<Vec<u8>> {
    if value.len() % 2 != 0 {
        return None;
    }

    (0..value.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(&value[index..index + 2], 16).ok())
        .collect()
}
