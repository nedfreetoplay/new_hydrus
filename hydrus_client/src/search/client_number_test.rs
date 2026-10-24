//! Number tests from `hydrus/client/search/ClientNumberTest.py`.

use std::any::Any;

use hydrus_core::hydrus_exceptions::HydrusResult;
use hydrus_core::hydrus_serialisable::{self, SerialisableObject};
use serde_json::{Value, json};

pub const NUMBER_TEST_OPERATOR_LESS_THAN: i32 = 0;
pub const NUMBER_TEST_OPERATOR_GREATER_THAN: i32 = 1;
pub const NUMBER_TEST_OPERATOR_EQUAL: i32 = 2;
pub const NUMBER_TEST_OPERATOR_APPROXIMATE_PERCENT: i32 = 3;
pub const NUMBER_TEST_OPERATOR_NOT_EQUAL: i32 = 4;
pub const NUMBER_TEST_OPERATOR_APPROXIMATE_ABSOLUTE: i32 = 5;
pub const NUMBER_TEST_OPERATOR_LESS_THAN_OR_EQUAL_TO: i32 = 6;
pub const NUMBER_TEST_OPERATOR_GREATER_THAN_OR_EQUAL_TO: i32 = 7;

pub const SERIALISABLE_VERSION: u16 = 2;

#[derive(Debug, Clone, PartialEq)]
pub struct NumberTest {
    pub operator: i32,
    pub value: f64,
    pub extra_value: Option<f64>,
}

impl Default for NumberTest {
    fn default() -> Self {
        Self::new(NUMBER_TEST_OPERATOR_EQUAL, 1.0, None)
    }
}

impl NumberTest {
    pub fn new(operator: i32, value: f64, extra_value: Option<f64>) -> Self {
        Self {
            operator,
            value,
            extra_value,
        }
    }

    /// `Test( value, replacement_test_value = None )`
    pub fn test(&self, value: Option<f64>, replacement_test_value: Option<f64>) -> bool {
        let value_to_test = replacement_test_value.unwrap_or(self.value);

        match self.operator {
            NUMBER_TEST_OPERATOR_LESS_THAN => {
                if value_to_test > 0.0 {
                    value.is_none_or(|value| value < value_to_test)
                } else {
                    value.is_some_and(|value| value < value_to_test)
                }
            }
            NUMBER_TEST_OPERATOR_LESS_THAN_OR_EQUAL_TO => {
                if value_to_test >= 0.0 {
                    value.is_none_or(|value| value <= value_to_test)
                } else {
                    value.is_some_and(|value| value <= value_to_test)
                }
            }
            NUMBER_TEST_OPERATOR_GREATER_THAN => {
                if value_to_test < 0.0 {
                    value.is_none_or(|value| value > value_to_test)
                } else {
                    value.is_some_and(|value| value > value_to_test)
                }
            }
            NUMBER_TEST_OPERATOR_GREATER_THAN_OR_EQUAL_TO => {
                if value_to_test <= 0.0 {
                    value.is_none_or(|value| value >= value_to_test)
                } else {
                    value.is_some_and(|value| value >= value_to_test)
                }
            }
            NUMBER_TEST_OPERATOR_EQUAL => {
                if value_to_test == 0.0 {
                    value.is_none_or(|value| value == value_to_test)
                } else {
                    value == Some(value_to_test)
                }
            }
            NUMBER_TEST_OPERATOR_APPROXIMATE_PERCENT => {
                let extra = self.extra_value.unwrap_or(0.0);
                let lower = value_to_test * (1.0 - extra);
                let upper = value_to_test * (1.0 + extra);

                if lower <= 0.0 {
                    value.is_none_or(|value| value <= upper)
                } else {
                    value.is_some_and(|value| lower <= value && value <= upper)
                }
            }
            NUMBER_TEST_OPERATOR_APPROXIMATE_ABSOLUTE => {
                let extra = self.extra_value.unwrap_or(0.0);
                let lower = value_to_test - extra;
                let upper = value_to_test + extra;

                if lower <= 0.0 {
                    value.is_none_or(|value| value <= upper)
                } else {
                    value.is_some_and(|value| lower <= value && value <= upper)
                }
            }
            NUMBER_TEST_OPERATOR_NOT_EQUAL => {
                if value_to_test == 0.0 {
                    value.is_some_and(|value| value != value_to_test)
                } else {
                    value.is_none_or(|value| value != value_to_test)
                }
            }
            _ => false,
        }
    }

    pub fn is_zero(&self) -> bool {
        (self.operator == NUMBER_TEST_OPERATOR_EQUAL && self.value == 0.0)
            || (self.operator == NUMBER_TEST_OPERATOR_LESS_THAN && self.value == 1.0)
            || (self.operator == NUMBER_TEST_OPERATOR_LESS_THAN_OR_EQUAL_TO && self.value == 0.0)
    }

    pub fn is_anything_but_zero(&self) -> bool {
        (self.operator == NUMBER_TEST_OPERATOR_NOT_EQUAL
            || self.operator == NUMBER_TEST_OPERATOR_GREATER_THAN)
            && self.value == 0.0
    }

    pub fn wants_zero(&self) -> bool {
        self.is_zero()
    }

    /// `STATICCreateFromCharacters( operator_str, value )`
    pub fn static_create_from_characters(operator_str: &str, value: f64) -> Option<Self> {
        let operator = match operator_str {
            "<" => NUMBER_TEST_OPERATOR_LESS_THAN,
            ">" => NUMBER_TEST_OPERATOR_GREATER_THAN,
            "=" => NUMBER_TEST_OPERATOR_EQUAL,
            "≠" => NUMBER_TEST_OPERATOR_NOT_EQUAL,
            "≤" => NUMBER_TEST_OPERATOR_LESS_THAN_OR_EQUAL_TO,
            "≥" => NUMBER_TEST_OPERATOR_GREATER_THAN_OR_EQUAL_TO,
            "≈" => NUMBER_TEST_OPERATOR_APPROXIMATE_PERCENT,
            _ => return None,
        };

        let extra_value = (operator == NUMBER_TEST_OPERATOR_APPROXIMATE_PERCENT).then_some(0.15);

        Some(Self::new(operator, value, extra_value))
    }

    pub fn to_string(&self) -> String {
        let operator = operator_string(self.operator);

        let mut result = format!("{operator} {}", self.value);

        if self.operator == NUMBER_TEST_OPERATOR_APPROXIMATE_PERCENT {
            if let Some(extra) = self.extra_value {
                result.push_str(&format!(" ({:.1}%)", extra * 100.0));
            }
        } else if self.operator == NUMBER_TEST_OPERATOR_APPROXIMATE_ABSOLUTE {
            if let Some(extra) = self.extra_value {
                result.push_str(&format!(" (±{extra})"));
            }
        }

        result
    }

    pub fn to_json_value(&self) -> Value {
        json!([
            self.serialisable_type(),
            SERIALISABLE_VERSION,
            self.get_serialisable_info()
        ])
    }
}

pub fn operator_string(operator: i32) -> &'static str {
    match operator {
        NUMBER_TEST_OPERATOR_LESS_THAN => "<",
        NUMBER_TEST_OPERATOR_GREATER_THAN => ">",
        NUMBER_TEST_OPERATOR_EQUAL => "=",
        NUMBER_TEST_OPERATOR_APPROXIMATE_PERCENT => "≈",
        NUMBER_TEST_OPERATOR_NOT_EQUAL => "≠",
        NUMBER_TEST_OPERATOR_APPROXIMATE_ABSOLUTE => "≈",
        NUMBER_TEST_OPERATOR_LESS_THAN_OR_EQUAL_TO => "≤",
        NUMBER_TEST_OPERATOR_GREATER_THAN_OR_EQUAL_TO => "≥",
        _ => "?",
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn comparison_operators_match_python() {
        let less = NumberTest::new(NUMBER_TEST_OPERATOR_LESS_THAN, 10.0, None);

        assert!(less.test(Some(5.0), None));
        assert!(!less.test(Some(10.0), None));

        let greater = NumberTest::new(NUMBER_TEST_OPERATOR_GREATER_THAN, 10.0, None);

        assert!(greater.test(Some(11.0), None));
        assert!(!greater.test(None, None));
    }

    #[test]
    fn none_handling_depends_on_threshold_sign() {
        let positive = NumberTest::new(NUMBER_TEST_OPERATOR_LESS_THAN, 5.0, None);
        let negative = NumberTest::new(NUMBER_TEST_OPERATOR_LESS_THAN, -5.0, None);

        assert!(positive.test(None, None));
        assert!(!negative.test(None, None));
    }

    #[test]
    fn approximate_tests_use_extra_value() {
        let percent = NumberTest::new(NUMBER_TEST_OPERATOR_APPROXIMATE_PERCENT, 100.0, Some(0.15));

        assert!(percent.test(Some(90.0), None));
        assert!(!percent.test(Some(80.0), None));

        let absolute =
            NumberTest::new(NUMBER_TEST_OPERATOR_APPROXIMATE_ABSOLUTE, 100.0, Some(10.0));

        assert!(absolute.test(Some(105.0), None));
        assert!(!absolute.test(Some(111.0), None));
    }

    #[test]
    fn zero_helpers_match_python() {
        assert!(NumberTest::new(NUMBER_TEST_OPERATOR_EQUAL, 0.0, None).is_zero());
        assert!(NumberTest::new(NUMBER_TEST_OPERATOR_LESS_THAN, 1.0, None).is_zero());
        assert!(
            NumberTest::new(NUMBER_TEST_OPERATOR_GREATER_THAN, 0.0, None).is_anything_but_zero()
        );
    }

    #[test]
    fn serialisation_round_trips_and_migrates_v1() {
        let test = NumberTest::new(NUMBER_TEST_OPERATOR_APPROXIMATE_PERCENT, 3.0, Some(0.15));
        let info = test.get_serialisable_info();
        let mut loaded = NumberTest::default();

        loaded.initialise_from_serialisable_info(&info).unwrap();
        assert_eq!(loaded, test);

        let (version, migrated) = test
            .update_serialisable_info(1, json!([NUMBER_TEST_OPERATOR_APPROXIMATE_PERCENT, 3.0]))
            .unwrap();

        assert_eq!(version, 2);
        assert_eq!(migrated[2], json!(0.15));
    }
}
