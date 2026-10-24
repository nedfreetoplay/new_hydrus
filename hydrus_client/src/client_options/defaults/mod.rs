//! The default `ClientOptions` dictionary.
//!
//! The Python object defaults to several hundred keys. The port starts with the
//! keys used by the model, database and GUI shell; new keys are added next to
//! the feature that needs them.

use serde_json::{Map, Value};

mod basic;
mod lists;
mod noneable;

use basic::{booleans, floats, integers, strings};
use lists::{colours, integer_list, keys};
use noneable::{noneable_integers, noneable_strings, string_list};

pub fn default_options_dictionary() -> Map<String, Value> {
    let mut dictionary = Map::new();

    dictionary.insert("booleans".to_string(), Value::Object(booleans()));
    dictionary.insert("integers".to_string(), Value::Object(integers()));
    dictionary.insert("floats".to_string(), Value::Object(floats()));
    dictionary.insert("strings".to_string(), Value::Object(strings()));
    dictionary.insert(
        "noneable_integers".to_string(),
        Value::Object(noneable_integers()),
    );
    dictionary.insert(
        "noneable_strings".to_string(),
        Value::Object(noneable_strings()),
    );
    dictionary.insert("string_list".to_string(), Value::Object(string_list()));
    dictionary.insert("integer_list".to_string(), Value::Object(integer_list()));
    dictionary.insert("keys".to_string(), Value::Object(keys()));
    dictionary.insert("key_list".to_string(), Value::Object(Map::new()));
    dictionary.insert("colours".to_string(), colours());

    dictionary
}
