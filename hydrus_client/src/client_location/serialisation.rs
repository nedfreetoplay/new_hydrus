//! Split out from the parent module.

use hydrus_core::hydrus_exceptions::HydrusResult;
use hydrus_core::hydrus_serialisable::{self, SerialisableObject};
use serde_json::{Value, json};
use std::any::Any;
use std::collections::BTreeSet;

use super::*;

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
