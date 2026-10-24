//! `hydrus/client/ClientOptions.py`
//!
//! Options are kept in a nested dictionary exactly like the Python object, so
//! unknown keys survive a load/save cycle. Typed accessors mirror the
//! `get_boolean`/`get_integer`/... API.

use std::any::Any;

use hydrus_core::hydrus_exceptions::HydrusResult;
use hydrus_core::hydrus_serialisable::SerialisableObject;
use serde_json::{Map, Value, json};

mod defaults;
mod migrations;

pub const SERIALISABLE_TYPE: u16 = 22;
pub const SERIALISABLE_VERSION: u16 = 8;

#[derive(Debug, Clone, PartialEq)]
pub struct ClientOptions {
    dictionary: Map<String, Value>,
}

impl Default for ClientOptions {
    fn default() -> Self {
        Self::new()
    }
}

impl ClientOptions {
    pub fn new() -> Self {
        Self {
            dictionary: defaults::default_options_dictionary(),
        }
    }

    pub fn from_dictionary(dictionary: Map<String, Value>) -> Self {
        Self { dictionary }
    }

    pub fn get_dictionary(&self) -> &Map<String, Value> {
        &self.dictionary
    }

    fn group(&self, group: &str) -> Option<&Map<String, Value>> {
        self.dictionary.get(group).and_then(Value::as_object)
    }

    fn group_mut(&mut self, group: &str) -> &mut Map<String, Value> {
        self.dictionary
            .entry(group.to_string())
            .or_insert_with(|| json!({}))
            .as_object_mut()
            .expect("options groups are objects")
    }

    pub fn get_boolean(&self, name: &str) -> bool {
        self.group("booleans")
            .and_then(|group| group.get(name))
            .and_then(Value::as_bool)
            .unwrap_or(false)
    }

    pub fn set_boolean(&mut self, name: &str, value: bool) {
        self.group_mut("booleans")
            .insert(name.to_string(), json!(value));
    }

    pub fn flip_boolean(&mut self, name: &str) -> bool {
        let value = !self.get_boolean(name);
        self.set_boolean(name, value);
        value
    }

    pub fn get_integer(&self, name: &str) -> i64 {
        self.group("integers")
            .and_then(|group| group.get(name))
            .and_then(Value::as_i64)
            .unwrap_or(0)
    }

    pub fn set_integer(&mut self, name: &str, value: i64) {
        self.group_mut("integers")
            .insert(name.to_string(), json!(value));
    }

    pub fn get_noneable_integer(&self, name: &str) -> Option<i64> {
        self.group("noneable_integers")
            .and_then(|group| group.get(name))
            .and_then(Value::as_i64)
    }

    pub fn set_noneable_integer(&mut self, name: &str, value: Option<i64>) {
        self.group_mut("noneable_integers")
            .insert(name.to_string(), json!(value));
    }

    pub fn get_float(&self, name: &str) -> f64 {
        self.group("floats")
            .and_then(|group| group.get(name))
            .and_then(Value::as_f64)
            .unwrap_or(0.0)
    }

    pub fn set_float(&mut self, name: &str, value: f64) {
        self.group_mut("floats")
            .insert(name.to_string(), json!(value));
    }

    pub fn get_string(&self, name: &str) -> String {
        self.group("strings")
            .and_then(|group| group.get(name))
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string()
    }

    pub fn set_string(&mut self, name: &str, value: impl Into<String>) {
        self.group_mut("strings")
            .insert(name.to_string(), json!(value.into()));
    }

    pub fn get_noneable_string(&self, name: &str) -> Option<String> {
        self.group("noneable_strings")
            .and_then(|group| group.get(name))
            .and_then(Value::as_str)
            .map(str::to_string)
    }

    pub fn set_noneable_string(&mut self, name: &str, value: Option<&str>) {
        self.group_mut("noneable_strings")
            .insert(name.to_string(), json!(value));
    }

    pub fn get_string_list(&self, name: &str) -> Vec<String> {
        self.group("string_list")
            .and_then(|group| group.get(name))
            .and_then(Value::as_array)
            .map(|values| {
                values
                    .iter()
                    .filter_map(Value::as_str)
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn get_integer_list(&self, name: &str) -> Vec<i64> {
        self.group("integer_list")
            .and_then(|group| group.get(name))
            .and_then(Value::as_array)
            .map(|values| values.iter().filter_map(Value::as_i64).collect())
            .unwrap_or_default()
    }

    pub fn get_key(&self, name: &str) -> Option<Vec<u8>> {
        self.group("keys")
            .and_then(|group| group.get(name))
            .and_then(Value::as_str)
            .and_then(hex_decode)
    }

    pub fn get_key_list(&self, name: &str) -> Vec<Vec<u8>> {
        self.group("key_list")
            .and_then(|group| group.get(name))
            .and_then(Value::as_array)
            .map(|values| {
                values
                    .iter()
                    .filter_map(Value::as_str)
                    .filter_map(hex_decode)
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn get_current_colourset(&self) -> String {
        self.get_string("current_colourset")
    }

    pub fn get_colourset(&self, name: &str) -> Map<String, Value> {
        self.group("colours")
            .and_then(|colours| colours.get(name))
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default()
    }

    pub fn get_colour(&self, colour_id: i32) -> Option<(u8, u8, u8)> {
        let colourset = self.get_colourset(&self.get_current_colourset());
        let value = colourset.get(&colour_id.to_string())?.as_array()?;

        if value.len() != 3 {
            return None;
        }

        Some((
            value[0].as_u64()? as u8,
            value[1].as_u64()? as u8,
            value[2].as_u64()? as u8,
        ))
    }

    pub fn get_media_viewer_tags_scrolling_behaviour(&self) -> i32 {
        self.get_integer("media_viewer_tags_scrolling_behaviour") as i32
    }

    pub fn set_media_viewer_tags_scrolling_behaviour(&mut self, value: i32) {
        self.set_integer("media_viewer_tags_scrolling_behaviour", value as i64);
    }
}

mod serialisation;
use serialisation::hex_decode;
pub use serialisation::*;

#[cfg(test)]
mod tests;
