//! Split out from the parent module.

use hydrus_core::hydrus_exceptions::HydrusResult;
use hydrus_core::hydrus_serialisable::{self, SerialisableObject};
use serde_json::{Value, json};
use std::any::Any;

use super::*;

impl SerialisableObject for NumberTest {
    fn serialisable_type(&self) -> u16 {
        hydrus_serialisable::SERIALISABLE_TYPE_NUMBER_TEST
    }

    fn serialisable_version(&self) -> u16 {
        SERIALISABLE_VERSION
    }

    fn serialisable_name(&self) -> &'static str {
        "Number Test"
    }

    fn get_serialisable_info(&self) -> Value {
        json!([self.operator, self.value, self.extra_value])
    }

    fn initialise_from_serialisable_info(&mut self, info: &Value) -> HydrusResult<()> {
        let values = info.as_array().ok_or_else(|| {
            hydrus_core::hydrus_exceptions::serialisation("NumberTest info must be a list")
        })?;

        self.operator = values.first().and_then(Value::as_i64).unwrap_or(2) as i32;
        self.value = values.get(1).and_then(Value::as_f64).unwrap_or(1.0);
        self.extra_value = values.get(2).and_then(Value::as_f64);

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
        let operator = values.first().and_then(Value::as_i64).unwrap_or(2);
        let value = values.get(1).cloned().unwrap_or(json!(1.0));
        let extra = if operator == NUMBER_TEST_OPERATOR_APPROXIMATE_PERCENT as i64 {
            json!(0.15)
        } else {
            Value::Null
        };

        Ok((2, json!([operator, value, extra])))
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
