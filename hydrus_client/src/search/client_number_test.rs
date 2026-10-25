//! Number tests from `hydrus/client/search/ClientNumberTest.py`.

use hydrus_core::hydrus_serialisable::SerialisableObject;
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

mod serialisation;

#[cfg(test)]
mod tests;
