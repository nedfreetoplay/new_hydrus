//! Rust port of `hydrus/core/networking/HydrusNetworkVariableHandling.py`.
//!
//! Python passes loosely typed dictionaries around and relies on
//! `isinstance` checks to validate them. This port uses [`ArgValue`], a small
//! typed value enum, and [`ParsedRequestArguments`], an insertion-ordered map
//! of those values. The public helper functions mirror the Python module:
//!
//! * HTTP GET arguments are parsed by [`parse_hydrus_network_get_args`].
//! * Network bodies are parsed by
//!   [`parse_network_bytes_to_parsed_hydrus_args`] and produced by
//!   [`dump_hydrus_args_to_network_bytes`].
//! * [`ParsedRequestArguments::get_value`] and friends implement the usual
//!   server-side "fetch this parameter and check its type" pattern.
//!
//! CBOR support is reported as unavailable, exactly as in the Python module
//! when `cbor2` is not installed.

use std::any::Any;
use std::fmt;
use std::sync::Arc;

use indexmap::IndexMap;
use serde_json::{Map, Value};

use crate::files::hydrus_file_handling;
use crate::files::images::hydrus_image_handling::{self, ThumbnailScale};
use crate::hydrus_constants::{
    ContentType, DECOMPRESSION_BOMB_IMAGES, SERVER_THUMBNAIL_DIMENSIONS, mimes_with_thumbnails,
};
use crate::hydrus_data::{hex_decode, hex_encode};
use crate::hydrus_exceptions::{ErrorKind, HydrusException, HydrusResult};
use crate::hydrus_serialisable::{
    SerialisableDictionary, SerialisableList, SerialisableObject, SerialisableValue,
    create_from_network_bytes,
};
use crate::networking::hydrus_network::{
    Account, AccountIdentifier, Content, ContentData, ServerService,
    generate_service_from_serialisable_tuple,
};

/// Python tries to import `cbor2`; this port does not.
pub const CBOR_AVAILABLE: bool = false;

pub const INT_PARAMS: &[&str] = &[
    "expires",
    "num",
    "since",
    "content_type",
    "action",
    "status",
];

pub const BYTE_PARAMS: &[&str] = &[
    "access_key",
    "account_type_key",
    "subject_account_key",
    "registration_key",
    "hash",
    "subject_hash",
    "update_hash",
];

pub const STRING_PARAMS: &[&str] = &["subject_tag", "reason", "message"];

pub const JSON_PARAMS: &[&str] = &[];

pub const JSON_BYTE_LIST_PARAMS: &[&str] = &["registration_keys"];

pub const HASH_BYTE_PARAMS: &[&str] = &["hash", "subject_hash", "update_hash"];

/// The Python types accepted by `test_variable_type`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ExpectedType {
    Int,
    Float,
    String,
    Bytes,
    Bool,
    List,
    Dict,
}

impl ExpectedType {
    /// The description used by `variable_type_to_text_lookup`.
    pub const fn text(self) -> &'static str {
        match self {
            ExpectedType::Int => "integer",
            ExpectedType::Float => "float",
            ExpectedType::String => "string",
            ExpectedType::Bytes => "hex-encoded bytestring",
            ExpectedType::Bool => "boolean",
            ExpectedType::List => "list",
            ExpectedType::Dict => "object/dict",
        }
    }
}

/// One value in a request/response argument dictionary.
///
/// JSON arrays and objects are expanded into [`ArgValue::List`] and
/// [`ArgValue::Dict`] so callers can walk them without touching `serde_json`.
/// Nested Hydrus serialisables are kept as objects.
#[derive(Clone)]
pub enum ArgValue {
    /// A JSON scalar: null, bool, number or string.
    Json(Value),
    Bytes(Vec<u8>),
    List(Vec<ArgValue>),
    Dict(IndexMap<String, ArgValue>),
    /// A Python `dict[bytes, bytes]`, used by `service_keys_to_access_keys`.
    BytesDict(IndexMap<Vec<u8>, Vec<u8>>),
    Object(ArgValueObject),
}

/// A Python object held in an argument dictionary.
///
/// Most network objects are `HydrusSerialisable` subclasses, but a few (for
/// example `Account` and `ServerService`) are plain Python objects. Keeping
/// both cases lets the network helpers carry them exactly like Python does.
#[derive(Clone)]
pub enum ArgValueObject {
    Serialisable(Arc<dyn SerialisableObject>),
    Other(Arc<dyn Any + Send + Sync>),
}

impl fmt::Debug for ArgValueObject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ArgValueObject::Serialisable(value) => {
                f.debug_tuple("Serialisable").field(value).finish()
            }
            ArgValueObject::Other(_) => f.write_str("Other(<object>)"),
        }
    }
}

impl PartialEq for ArgValueObject {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (ArgValueObject::Serialisable(a), ArgValueObject::Serialisable(b)) => {
                a.get_serialisable_tuple() == b.get_serialisable_tuple()
            }
            _ => false,
        }
    }
}

impl fmt::Debug for ArgValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ArgValue::Json(value) => f.debug_tuple("Json").field(value).finish(),
            ArgValue::Bytes(value) => f.debug_tuple("Bytes").field(value).finish(),
            ArgValue::List(value) => f.debug_tuple("List").field(value).finish(),
            ArgValue::Dict(value) => f.debug_tuple("Dict").field(value).finish(),
            ArgValue::BytesDict(value) => f.debug_tuple("BytesDict").field(value).finish(),
            ArgValue::Object(value) => f.debug_tuple("Object").field(value).finish(),
        }
    }
}

impl PartialEq for ArgValue {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (ArgValue::Json(a), ArgValue::Json(b)) => a == b,
            (ArgValue::Bytes(a), ArgValue::Bytes(b)) => a == b,
            (ArgValue::List(a), ArgValue::List(b)) => a == b,
            (ArgValue::Dict(a), ArgValue::Dict(b)) => a == b,
            (ArgValue::BytesDict(a), ArgValue::BytesDict(b)) => a == b,
            (ArgValue::Object(a), ArgValue::Object(b)) => a == b,
            _ => false,
        }
    }
}

impl fmt::Display for ArgValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ArgValue::Json(Value::Null) => f.write_str("None"),
            ArgValue::Json(Value::Bool(true)) => f.write_str("True"),
            ArgValue::Json(Value::Bool(false)) => f.write_str("False"),
            ArgValue::Json(Value::String(value)) => f.write_str(value),
            ArgValue::Json(value) => write!(f, "{value}"),
            ArgValue::Bytes(value) => write!(f, "b'{}'", hex_encode(value)),
            ArgValue::List(items) => {
                f.write_str("[")?;

                for (index, item) in items.iter().enumerate() {
                    if index > 0 {
                        f.write_str(", ")?;
                    }

                    write!(f, "{item}")?;
                }

                f.write_str("]")
            }
            ArgValue::Dict(entries) => {
                f.write_str("{")?;

                for (index, (key, value)) in entries.iter().enumerate() {
                    if index > 0 {
                        f.write_str(", ")?;
                    }

                    write!(f, "{key}: {value}")?;
                }

                f.write_str("}")
            }
            ArgValue::BytesDict(entries) => {
                f.write_str("{")?;

                for (index, (key, value)) in entries.iter().enumerate() {
                    if index > 0 {
                        f.write_str(", ")?;
                    }

                    write!(f, "{}: {}", hex_encode(key), hex_encode(value))?;
                }

                f.write_str("}")
            }
            ArgValue::Object(value) => match value {
                ArgValueObject::Serialisable(value) => {
                    f.write_str(&value.get_serialisable_description())
                }
                ArgValueObject::Other(_) => f.write_str("<object>"),
            },
        }
    }
}

impl ArgValue {
    pub fn null() -> Self {
        ArgValue::Json(Value::Null)
    }

    pub fn is_null(&self) -> bool {
        matches!(self, ArgValue::Json(Value::Null))
    }

    pub fn as_json(&self) -> Option<&Value> {
        match self {
            ArgValue::Json(value) => Some(value),
            _ => None,
        }
    }

    /// Python's `isinstance(value, int)`, which also accepts booleans.
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            ArgValue::Json(Value::Number(number)) => number
                .as_i64()
                .or_else(|| number.as_u64().and_then(|n| i64::try_from(n).ok())),
            ArgValue::Json(Value::Bool(value)) => Some(i64::from(*value)),
            _ => None,
        }
    }

    /// Python's `isinstance(value, float)`, which also accepts integers and
    /// booleans through the `float`/`int` special case.
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            ArgValue::Json(Value::Number(number)) => number.as_f64(),
            ArgValue::Json(Value::Bool(value)) => Some(if *value { 1.0 } else { 0.0 }),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            ArgValue::Json(Value::Bool(value)) => Some(*value),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            ArgValue::Json(Value::String(value)) => Some(value),
            _ => None,
        }
    }

    pub fn as_bytes(&self) -> Option<&[u8]> {
        match self {
            ArgValue::Bytes(value) => Some(value),
            _ => None,
        }
    }

    pub fn as_list(&self) -> Option<&[ArgValue]> {
        match self {
            ArgValue::List(value) => Some(value),
            _ => None,
        }
    }

    pub fn as_dict(&self) -> Option<&IndexMap<String, ArgValue>> {
        match self {
            ArgValue::Dict(value) => Some(value),
            _ => None,
        }
    }

    pub fn as_bytes_dict(&self) -> Option<&IndexMap<Vec<u8>, Vec<u8>>> {
        match self {
            ArgValue::BytesDict(value) => Some(value),
            _ => None,
        }
    }

    pub fn as_object(&self) -> Option<&dyn SerialisableObject> {
        match self {
            ArgValue::Object(ArgValueObject::Serialisable(value)) => Some(value.as_ref()),
            _ => None,
        }
    }

    pub fn as_object_arc(&self) -> Option<&Arc<dyn SerialisableObject>> {
        match self {
            ArgValue::Object(ArgValueObject::Serialisable(value)) => Some(value),
            _ => None,
        }
    }

    /// Build a value around a Hydrus serialisable object.
    pub fn object<T: SerialisableObject + 'static>(value: Arc<T>) -> Self {
        ArgValue::Object(ArgValueObject::Serialisable(value))
    }

    /// Build a value around a plain object, such as `Account` or
    /// `ServerService`.
    pub fn other_object<T: Any + Send + Sync + 'static>(value: Arc<T>) -> Self {
        ArgValue::Object(ArgValueObject::Other(value))
    }

    /// Downcast a nested object to a concrete type.
    pub fn downcast_object<T: 'static>(&self) -> Option<&T> {
        match self {
            ArgValue::Object(ArgValueObject::Serialisable(value)) => {
                value.as_any().downcast_ref::<T>()
            }
            ArgValue::Object(ArgValueObject::Other(value)) => value.downcast_ref::<T>(),
            _ => None,
        }
    }

    /// Convert back to the representation used by
    /// [`SerialisableDictionary`]. Lists and dictionaries are expanded into
    /// nested serialisable objects, matching Python's dictionary coercion.
    ///
    /// Plain objects cannot be represented; callers such as
    /// [`dump_hydrus_args_to_network_bytes`] convert them first.
    pub fn to_serialisable_value(&self) -> HydrusResult<SerialisableValue> {
        match self {
            ArgValue::Json(value) => Ok(SerialisableValue::Json(value.clone())),
            ArgValue::Bytes(value) => Ok(SerialisableValue::Bytes(value.clone())),
            ArgValue::List(items) => {
                let items = items
                    .iter()
                    .map(ArgValue::to_serialisable_value)
                    .collect::<HydrusResult<Vec<_>>>()?;

                Ok(SerialisableValue::Object(Arc::new(
                    SerialisableList::from_values(items),
                )))
            }
            ArgValue::Dict(entries) => {
                let mut dictionary = SerialisableDictionary::new();

                for (key, value) in entries {
                    dictionary.insert(
                        SerialisableValue::Json(Value::String(key.clone())),
                        value.to_serialisable_value()?,
                    );
                }

                Ok(SerialisableValue::Object(Arc::new(dictionary)))
            }
            ArgValue::BytesDict(entries) => {
                let mut dictionary = SerialisableDictionary::new();

                for (key, value) in entries {
                    dictionary.insert(
                        SerialisableValue::Bytes(key.clone()),
                        SerialisableValue::Bytes(value.clone()),
                    );
                }

                Ok(SerialisableValue::Object(Arc::new(dictionary)))
            }
            ArgValue::Object(ArgValueObject::Serialisable(value)) => {
                Ok(SerialisableValue::Object(value.clone()))
            }
            ArgValue::Object(ArgValueObject::Other(_)) => Err(bad_request(
                "A plain object cannot be serialised as a network argument!",
            )),
        }
    }

    /// Convert to plain JSON. Bytes cannot be represented in JSON, so they are
    /// rejected. Nested serialisable objects become their serialisable tuple.
    pub fn to_json_value(&self) -> HydrusResult<Value> {
        match self {
            ArgValue::Json(value) => Ok(value.clone()),
            ArgValue::List(items) => Ok(Value::Array(
                items
                    .iter()
                    .map(ArgValue::to_json_value)
                    .collect::<HydrusResult<Vec<_>>>()?,
            )),
            ArgValue::Dict(entries) => Ok(Value::Object(
                entries
                    .iter()
                    .map(|(key, value)| Ok((key.clone(), value.to_json_value()?)))
                    .collect::<HydrusResult<Map<String, Value>>>()?,
            )),
            ArgValue::Object(ArgValueObject::Serialisable(value)) => {
                Ok(value.get_serialisable_tuple())
            }
            ArgValue::Object(ArgValueObject::Other(_)) => Err(bad_request(
                "Could not convert a plain object argument into plain JSON!",
            )),
            ArgValue::Bytes(_) | ArgValue::BytesDict(_) => Err(bad_request(
                "Could not convert a bytestring argument into plain JSON!",
            )),
        }
    }

    fn coerce_from_serialisable(value: SerialisableValue) -> Self {
        match value {
            SerialisableValue::Json(value) => ArgValue::from(value),
            SerialisableValue::Bytes(value) => ArgValue::Bytes(value),
            SerialisableValue::Object(object) => {
                if let Some(list) = object.as_any().downcast_ref::<SerialisableList>() {
                    return ArgValue::List(
                        list.as_slice()
                            .iter()
                            .cloned()
                            .map(ArgValue::coerce_from_serialisable)
                            .collect(),
                    );
                }

                if let Some(dictionary) = object.as_any().downcast_ref::<SerialisableDictionary>() {
                    if let Some(entries) = string_keyed_dictionary(dictionary) {
                        return ArgValue::Dict(entries);
                    }

                    if let Some(entries) = bytes_keyed_dictionary(dictionary) {
                        return ArgValue::BytesDict(entries);
                    }
                }

                ArgValue::Object(ArgValueObject::Serialisable(object))
            }
        }
    }
}

fn string_keyed_dictionary(
    dictionary: &SerialisableDictionary,
) -> Option<IndexMap<String, ArgValue>> {
    let mut entries = IndexMap::with_capacity(dictionary.len());

    for (key, value) in dictionary.iter() {
        let key = key.as_json()?.as_str()?.to_string();

        entries.insert(key, ArgValue::coerce_from_serialisable(value.clone()));
    }

    Some(entries)
}

fn bytes_keyed_dictionary(
    dictionary: &SerialisableDictionary,
) -> Option<IndexMap<Vec<u8>, Vec<u8>>> {
    let mut entries = IndexMap::with_capacity(dictionary.len());

    for (key, value) in dictionary.iter() {
        entries.insert(key.as_bytes()?.to_vec(), value.as_bytes()?.to_vec());
    }

    Some(entries)
}

impl From<Value> for ArgValue {
    fn from(value: Value) -> Self {
        match value {
            Value::Array(items) => ArgValue::List(items.into_iter().map(ArgValue::from).collect()),
            Value::Object(entries) => ArgValue::Dict(
                entries
                    .into_iter()
                    .map(|(key, value)| (key, ArgValue::from(value)))
                    .collect(),
            ),
            other => ArgValue::Json(other),
        }
    }
}

impl From<Vec<u8>> for ArgValue {
    fn from(value: Vec<u8>) -> Self {
        ArgValue::Bytes(value)
    }
}

impl From<&[u8]> for ArgValue {
    fn from(value: &[u8]) -> Self {
        ArgValue::Bytes(value.to_vec())
    }
}

impl From<&str> for ArgValue {
    fn from(value: &str) -> Self {
        ArgValue::Json(Value::String(value.to_string()))
    }
}

impl From<String> for ArgValue {
    fn from(value: String) -> Self {
        ArgValue::Json(Value::String(value))
    }
}

impl From<bool> for ArgValue {
    fn from(value: bool) -> Self {
        ArgValue::Json(Value::Bool(value))
    }
}

impl From<f64> for ArgValue {
    fn from(value: f64) -> Self {
        ArgValue::Json(Value::from(value))
    }
}

impl From<f32> for ArgValue {
    fn from(value: f32) -> Self {
        ArgValue::Json(Value::from(f64::from(value)))
    }
}

macro_rules! arg_value_from_int {
    ($($type:ty),* $(,)?) => {
        $(
            impl From<$type> for ArgValue {
                fn from(value: $type) -> Self {
                    ArgValue::Json(Value::from(value))
                }
            }
        )*
    };
}

arg_value_from_int!(i8, i16, i32, i64, u8, u16, u32, u64, usize);

impl From<Vec<ArgValue>> for ArgValue {
    fn from(value: Vec<ArgValue>) -> Self {
        ArgValue::List(value)
    }
}

impl From<IndexMap<String, ArgValue>> for ArgValue {
    fn from(value: IndexMap<String, ArgValue>) -> Self {
        ArgValue::Dict(value)
    }
}

impl From<Arc<dyn SerialisableObject>> for ArgValue {
    fn from(value: Arc<dyn SerialisableObject>) -> Self {
        ArgValue::Object(ArgValueObject::Serialisable(value))
    }
}

impl From<ArgValueObject> for ArgValue {
    fn from(value: ArgValueObject) -> Self {
        ArgValue::Object(value)
    }
}

/// The ordered argument dictionary used throughout the server.
#[derive(Clone, Debug, Default)]
pub struct ParsedRequestArguments {
    values: IndexMap<String, ArgValue>,
}

impl ParsedRequestArguments {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_map(values: IndexMap<String, ArgValue>) -> Self {
        Self { values }
    }

    pub fn insert(
        &mut self,
        key: impl Into<String>,
        value: impl Into<ArgValue>,
    ) -> Option<ArgValue> {
        self.values.insert(key.into(), value.into())
    }

    /// Insert a Hydrus serialisable object.
    pub fn insert_object(
        &mut self,
        key: impl Into<String>,
        value: Arc<dyn SerialisableObject>,
    ) -> Option<ArgValue> {
        self.values.insert(
            key.into(),
            ArgValue::Object(ArgValueObject::Serialisable(value)),
        )
    }

    /// Insert a plain object, such as `Account` or `ServerService`.
    pub fn insert_other_object<T: Any + Send + Sync + 'static>(
        &mut self,
        key: impl Into<String>,
        value: Arc<T>,
    ) -> Option<ArgValue> {
        self.values
            .insert(key.into(), ArgValue::Object(ArgValueObject::Other(value)))
    }

    pub fn remove(&mut self, key: &str) -> Option<ArgValue> {
        self.values.shift_remove(key)
    }

    pub fn get(&self, key: &str) -> Option<&ArgValue> {
        self.values.get(key)
    }

    pub fn get_mut(&mut self, key: &str) -> Option<&mut ArgValue> {
        self.values.get_mut(key)
    }

    /// Python's `__missing__`: a missing parameter is a bad request.
    pub fn get_required(&self, key: &str) -> HydrusResult<&ArgValue> {
        self.values.get(key).ok_or_else(|| {
            bad_request(format!(
                "It looks like the parameter \"{key}\" was missing!"
            ))
        })
    }

    pub fn contains_key(&self, key: &str) -> bool {
        self.values.contains_key(key)
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    pub fn iter(&self) -> indexmap::map::Iter<'_, String, ArgValue> {
        self.values.iter()
    }

    pub fn keys(&self) -> indexmap::map::Keys<'_, String, ArgValue> {
        self.values.keys()
    }

    pub fn values(&self) -> indexmap::map::Values<'_, String, ArgValue> {
        self.values.values()
    }

    pub fn into_inner(self) -> IndexMap<String, ArgValue> {
        self.values
    }

    pub fn as_map(&self) -> &IndexMap<String, ArgValue> {
        &self.values
    }

    /// Convert to a `SerialisableDictionary` without applying the network
    /// parameter conventions.
    pub fn to_serialisable_dictionary(&self) -> HydrusResult<SerialisableDictionary> {
        let mut dictionary = SerialisableDictionary::new();

        for (key, value) in &self.values {
            dictionary.insert(
                SerialisableValue::Json(Value::String(key.clone())),
                value.to_serialisable_value()?,
            );
        }

        Ok(dictionary)
    }

    pub fn from_serialisable_dictionary(dictionary: &SerialisableDictionary) -> Self {
        let mut values = IndexMap::with_capacity(dictionary.len());

        for (key, value) in dictionary.iter() {
            let Some(key) = key.as_json().and_then(Value::as_str) else {
                // Python dictionaries may hold non-string keys, but request
                // parameters are always strings. Ignore anything else rather
                // than inventing a lossy encoding.
                continue;
            };

            values.insert(
                key.to_string(),
                ArgValue::coerce_from_serialisable(value.clone()),
            );
        }

        Self { values }
    }

    /// `dict.get_value`.
    pub fn get_value(&self, key: &str, expected_type: ExpectedType) -> HydrusResult<ArgValue> {
        get_value_from_dict(self, key, expected_type, None, None, None, false)
    }

    /// `dict.get_value` with the optional list/dict/default arguments.
    pub fn get_value_with(
        &self,
        key: &str,
        expected_type: ExpectedType,
        expected_list_type: Option<ExpectedType>,
        expected_dict_types: Option<(ExpectedType, ExpectedType)>,
        default_value: Option<ArgValue>,
    ) -> HydrusResult<ArgValue> {
        get_value_from_dict(
            self,
            key,
            expected_type,
            expected_list_type,
            expected_dict_types,
            default_value,
            false,
        )
    }

    /// `dict.get_value_or_none`.
    pub fn get_value_or_none(
        &self,
        key: &str,
        expected_type: ExpectedType,
    ) -> HydrusResult<Option<ArgValue>> {
        let value = get_value_from_dict(self, key, expected_type, None, None, None, true)?;

        if value.is_null() {
            Ok(None)
        } else {
            Ok(Some(value))
        }
    }

    pub fn get_i64(&self, key: &str) -> HydrusResult<i64> {
        let value = self.get_value(key, ExpectedType::Int)?;

        value
            .as_i64()
            .ok_or_else(|| bad_request(format!("\"{key}\" was not an integer!")))
    }

    pub fn get_i64_or_none(&self, key: &str) -> HydrusResult<Option<i64>> {
        match self.get_value_or_none(key, ExpectedType::Int)? {
            Some(value) => value
                .as_i64()
                .map(Some)
                .ok_or_else(|| bad_request(format!("\"{key}\" was not an integer!"))),
            None => Ok(None),
        }
    }

    pub fn get_i64_default(&self, key: &str, default_value: i64) -> HydrusResult<i64> {
        match self.get_i64_or_none(key)? {
            Some(value) => Ok(value),
            None => Ok(default_value),
        }
    }

    pub fn get_f64(&self, key: &str) -> HydrusResult<f64> {
        let value = self.get_value(key, ExpectedType::Float)?;

        value
            .as_f64()
            .ok_or_else(|| bad_request(format!("\"{key}\" was not a float!")))
    }

    pub fn get_f64_or_none(&self, key: &str) -> HydrusResult<Option<f64>> {
        match self.get_value_or_none(key, ExpectedType::Float)? {
            Some(value) => value
                .as_f64()
                .map(Some)
                .ok_or_else(|| bad_request(format!("\"{key}\" was not a float!"))),
            None => Ok(None),
        }
    }

    pub fn get_bool(&self, key: &str) -> HydrusResult<bool> {
        let value = self.get_value(key, ExpectedType::Bool)?;

        value
            .as_bool()
            .ok_or_else(|| bad_request(format!("\"{key}\" was not a boolean!")))
    }

    pub fn get_bool_default(&self, key: &str, default_value: bool) -> HydrusResult<bool> {
        match self.get_value_or_none(key, ExpectedType::Bool)? {
            Some(value) => value
                .as_bool()
                .ok_or_else(|| bad_request(format!("\"{key}\" was not a boolean!"))),
            None => Ok(default_value),
        }
    }

    pub fn get_string(&self, key: &str) -> HydrusResult<String> {
        let value = self.get_value(key, ExpectedType::String)?;

        value
            .as_str()
            .map(str::to_string)
            .ok_or_else(|| bad_request(format!("\"{key}\" was not a string!")))
    }

    pub fn get_string_or_none(&self, key: &str) -> HydrusResult<Option<String>> {
        match self.get_value_or_none(key, ExpectedType::String)? {
            Some(value) => value
                .as_str()
                .map(|value| Some(value.to_string()))
                .ok_or_else(|| bad_request(format!("\"{key}\" was not a string!"))),
            None => Ok(None),
        }
    }

    pub fn get_bytes(&self, key: &str) -> HydrusResult<Vec<u8>> {
        let value = self.get_value(key, ExpectedType::Bytes)?;

        value
            .as_bytes()
            .map(<[u8]>::to_vec)
            .ok_or_else(|| bad_request(format!("\"{key}\" was not a bytestring!")))
    }

    pub fn get_bytes_or_none(&self, key: &str) -> HydrusResult<Option<Vec<u8>>> {
        match self.get_value_or_none(key, ExpectedType::Bytes)? {
            Some(value) => value
                .as_bytes()
                .map(|value| Some(value.to_vec()))
                .ok_or_else(|| bad_request(format!("\"{key}\" was not a bytestring!"))),
            None => Ok(None),
        }
    }

    pub fn get_list(&self, key: &str) -> HydrusResult<Vec<ArgValue>> {
        let value = self.get_value(key, ExpectedType::List)?;

        value
            .as_list()
            .map(<[ArgValue]>::to_vec)
            .ok_or_else(|| bad_request(format!("\"{key}\" was not a list!")))
    }
}

impl std::ops::Deref for ParsedRequestArguments {
    type Target = IndexMap<String, ArgValue>;

    fn deref(&self) -> &Self::Target {
        &self.values
    }
}

impl std::ops::DerefMut for ParsedRequestArguments {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.values
    }
}

impl IntoIterator for ParsedRequestArguments {
    type Item = (String, ArgValue);
    type IntoIter = indexmap::map::IntoIter<String, ArgValue>;

    fn into_iter(self) -> Self::IntoIter {
        self.values.into_iter()
    }
}

impl<'a> IntoIterator for &'a ParsedRequestArguments {
    type Item = (&'a String, &'a ArgValue);
    type IntoIter = indexmap::map::Iter<'a, String, ArgValue>;

    fn into_iter(self) -> Self::IntoIter {
        self.values.iter()
    }
}

impl From<IndexMap<String, ArgValue>> for ParsedRequestArguments {
    fn from(values: IndexMap<String, ArgValue>) -> Self {
        Self { values }
    }
}

impl From<Map<String, Value>> for ParsedRequestArguments {
    fn from(values: Map<String, Value>) -> Self {
        Self {
            values: values
                .into_iter()
                .map(|(key, value)| (key, ArgValue::from(value)))
                .collect(),
        }
    }
}

impl FromIterator<(String, ArgValue)> for ParsedRequestArguments {
    fn from_iter<T: IntoIterator<Item = (String, ArgValue)>>(iter: T) -> Self {
        Self {
            values: iter.into_iter().collect(),
        }
    }
}

impl FromIterator<(&'static str, ArgValue)> for ParsedRequestArguments {
    fn from_iter<T: IntoIterator<Item = (&'static str, ArgValue)>>(iter: T) -> Self {
        Self {
            values: iter
                .into_iter()
                .map(|(key, value)| (key.to_string(), value))
                .collect(),
        }
    }
}

impl Extend<(String, ArgValue)> for ParsedRequestArguments {
    fn extend<T: IntoIterator<Item = (String, ArgValue)>>(&mut self, iter: T) {
        self.values.extend(iter);
    }
}

impl Extend<(&'static str, ArgValue)> for ParsedRequestArguments {
    fn extend<T: IntoIterator<Item = (&'static str, ArgValue)>>(&mut self, iter: T) {
        self.values.extend(
            iter.into_iter()
                .map(|(key, value)| (key.to_string(), value)),
        );
    }
}

fn bad_request(message: impl Into<String>) -> HydrusException {
    HydrusException::new(ErrorKind::BadRequest, message)
}

// ---------------------------------------------------------------------------
// Percent encoding helpers (`urllib.parse.quote` / `unquote`)
// ---------------------------------------------------------------------------

/// Python's `urllib.parse.quote` with the default `safe='/'`.
pub fn percent_encode(value: &str) -> String {
    let mut out = String::with_capacity(value.len());

    for byte in value.as_bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~' | b'/') {
            out.push(*byte as char);
        } else {
            out.push('%');
            out.push(
                char::from_digit(u32::from(byte >> 4), 16)
                    .unwrap_or('0')
                    .to_ascii_uppercase(),
            );
            out.push(
                char::from_digit(u32::from(byte & 0x0F), 16)
                    .unwrap_or('0')
                    .to_ascii_uppercase(),
            );
        }
    }

    out
}

/// Python's `urllib.parse.unquote` with UTF-8 and replacement characters.
pub fn percent_decode(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;

    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let (Some(high), Some(low)) = (hex_digit(bytes[i + 1]), hex_digit(bytes[i + 2])) {
                out.push((high << 4) | low);
                i += 3;
                continue;
            }
        }

        out.push(bytes[i]);
        i += 1;
    }

    String::from_utf8_lossy(&out).into_owned()
}

fn hex_digit(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

/// `bytes.fromhex`, which ignores ASCII whitespace.
fn bytes_from_hex(value: &str) -> Option<Vec<u8>> {
    let cleaned: String = value
        .chars()
        .filter(|character| !character.is_ascii_whitespace())
        .collect();

    hex_decode(&cleaned).ok()
}

fn strip_hash_prefix<'a>(value: &'a str, param_name: &str) -> &'a str {
    if HASH_BYTE_PARAMS.contains(&param_name) {
        if let Some((_prefix, hex)) = value.split_once(':') {
            return hex;
        }
    }

    value
}

fn value_as_query_string(value: &ArgValue, key: &str) -> HydrusResult<String> {
    match value {
        ArgValue::Json(Value::String(value)) => Ok(value.clone()),
        ArgValue::Json(Value::Null) => Ok("None".to_string()),
        ArgValue::Json(Value::Bool(true)) => Ok("True".to_string()),
        ArgValue::Json(Value::Bool(false)) => Ok("False".to_string()),
        ArgValue::Json(Value::Number(value)) => Ok(value.to_string()),
        _ => Err(bad_request(format!(
            "Could not encode the parameter \"{key}\" as a query string value!"
        ))),
    }
}

// ---------------------------------------------------------------------------
// Dumping
// ---------------------------------------------------------------------------

/// `dump_hydrus_args_to_network_bytes`.
///
/// Unlike the Python function this does not mutate the caller's dictionary;
/// it returns the compressed network body directly.
pub fn dump_hydrus_args_to_network_bytes(args: &ParsedRequestArguments) -> HydrusResult<Vec<u8>> {
    let mut args = args.clone();

    for param_name in BYTE_PARAMS {
        if let Some(value) = args.values.get_mut(*param_name) {
            let ArgValue::Bytes(bytes) = value else {
                return Err(bad_request(format!(
                    "The parameter \"{param_name}\" was not a bytestring!"
                )));
            };

            *value = ArgValue::Json(Value::String(hex_encode(bytes)));
        }
    }

    for param_name in JSON_BYTE_LIST_PARAMS {
        if let Some(value) = args.values.get_mut(*param_name) {
            let ArgValue::List(items) = value else {
                return Err(bad_request(format!(
                    "The parameter \"{param_name}\" was not a list of bytestrings!"
                )));
            };

            let mut encoded = Vec::with_capacity(items.len());

            for item in items.iter_mut() {
                let ArgValue::Bytes(bytes) = item else {
                    return Err(bad_request(format!(
                        "The parameter \"{param_name}\" held an item that was not a bytestring!"
                    )));
                };

                encoded.push(ArgValue::Json(Value::String(hex_encode(bytes))));
            }

            *items = encoded;
        }
    }

    if let Some(value) = args.values.get_mut("account") {
        if let Some(account) = value.downcast_object::<Account>() {
            let account = Account::generate_serialisable_tuple_from_account(account)?;

            *value = ArgValue::Json(account);
        }
    }

    if let Some(value) = args.values.get_mut("accounts") {
        if let ArgValue::List(items) = value {
            for item in items.iter_mut() {
                if let Some(account) = item.downcast_object::<Account>() {
                    let account = Account::generate_serialisable_tuple_from_account(account)?;

                    *item = ArgValue::Json(account);
                }
            }
        }
    }

    if let Some(value) = args.values.get_mut("service_keys_to_access_keys") {
        if let ArgValue::BytesDict(entries) = value {
            *value = ArgValue::List(
                entries
                    .iter()
                    .map(|(service_key, access_key)| {
                        ArgValue::List(vec![
                            ArgValue::Json(Value::String(hex_encode(service_key))),
                            ArgValue::Json(Value::String(hex_encode(access_key))),
                        ])
                    })
                    .collect(),
            );
        }
    }

    if let Some(value) = args.values.get_mut("services") {
        if let ArgValue::List(items) = value {
            for item in items.iter_mut() {
                if let Some(service) = item.downcast_object::<ServerService>() {
                    let service = service.to_serialisable_tuple();

                    *item = ArgValue::Json(service);
                }
            }
        }
    }

    args.to_serialisable_dictionary()?.dump_to_network_bytes()
}

/// `dump_to_get_query`.
pub fn dump_to_get_query(args: &ParsedRequestArguments) -> HydrusResult<String> {
    let mut args = args.clone();

    if let Some(subject_identifier) = args.values.shift_remove("subject_identifier") {
        let Some(subject_identifier) = subject_identifier.downcast_object::<AccountIdentifier>()
        else {
            return Err(bad_request(
                "The subject identifier was not an AccountIdentifier!",
            ));
        };

        if subject_identifier.has_account_key() {
            args.insert(
                "subject_account_key",
                subject_identifier.get_account_key()?.to_vec(),
            );
        } else if subject_identifier.has_content() {
            let content = subject_identifier.get_content()?;

            match content.get_content_type() {
                ContentType::Files => {
                    let ContentData::Files(hashes) = content.get_content_data() else {
                        return Err(bad_request(
                            "The subject identifier did not hold file hashes!",
                        ));
                    };

                    let hash = hashes.first().ok_or_else(|| {
                        bad_request("The subject identifier held no file hashes!")
                    })?;

                    args.insert("subject_hash", hash.clone());
                }
                ContentType::Mapping => {
                    let ContentData::Mapping(tag, hash) = content.get_content_data() else {
                        return Err(bad_request(
                            "The subject identifier did not hold a mapping!",
                        ));
                    };

                    args.insert("subject_hash", hash.clone());
                    args.insert("subject_tag", tag.clone());
                }
                _ => {}
            }
        }
    }

    for name in INT_PARAMS {
        if let Some(value) = args.values.get(*name) {
            let value = value_as_query_string(value, name)?;

            args.values
                .insert((*name).to_string(), ArgValue::Json(Value::String(value)));
        }
    }

    for name in BYTE_PARAMS {
        if let Some(value) = args.values.get_mut(*name) {
            let ArgValue::Bytes(bytes) = value else {
                return Err(bad_request(format!(
                    "The parameter \"{name}\" was not a bytestring!"
                )));
            };

            *value = ArgValue::Json(Value::String(hex_encode(bytes)));
        }
    }

    for name in STRING_PARAMS {
        if let Some(value) = args.values.get_mut(*name) {
            let ArgValue::Json(Value::String(string)) = value else {
                return Err(bad_request(format!(
                    "The parameter \"{name}\" was not a string!"
                )));
            };

            *value = ArgValue::Json(Value::String(percent_encode(string)));
        }
    }

    let mut query = Vec::with_capacity(args.values.len());

    for (key, value) in &args.values {
        let value = value_as_query_string(value, key)?;

        query.push(format!("{key}={value}"));
    }

    Ok(query.join("&"))
}

// ---------------------------------------------------------------------------
// Parsing
// ---------------------------------------------------------------------------

/// `parse_hydrus_network_get_args`.
pub fn parse_hydrus_network_get_args(
    requests_args: &IndexMap<Vec<u8>, Vec<Vec<u8>>>,
) -> HydrusResult<ParsedRequestArguments> {
    let mut args = parse_twisted_request_get_args(
        requests_args,
        INT_PARAMS,
        BYTE_PARAMS,
        STRING_PARAMS,
        JSON_PARAMS,
        JSON_BYTE_LIST_PARAMS,
    )?;

    if let Some(hash) = args.get("subject_hash").and_then(ArgValue::as_bytes) {
        let hash = hash.to_vec();

        let content = match args.get("subject_tag").and_then(ArgValue::as_str) {
            Some(tag) => Content::mapping(tag, hash),
            None => Content::files(vec![hash]),
        };

        args.insert(
            "subject_identifier",
            ArgValue::object(Arc::new(AccountIdentifier::new_content(content))),
        );
    }

    Ok(args)
}

/// `parse_network_bytes_to_parsed_hydrus_args`.
pub fn parse_network_bytes_to_parsed_hydrus_args(
    network_bytes: &[u8],
) -> HydrusResult<ParsedRequestArguments> {
    if network_bytes.is_empty() {
        return Ok(ParsedRequestArguments::new());
    }

    let object = create_from_network_bytes(network_bytes, false)?;

    let Some(dictionary) = object.as_any().downcast_ref::<SerialisableDictionary>() else {
        return Err(bad_request(
            "The given parameter did not seem to be a JSON Object!",
        ));
    };

    let mut args = ParsedRequestArguments::from_serialisable_dictionary(dictionary);

    for param_name in BYTE_PARAMS {
        let Some(value) = args.values.get(*param_name) else {
            continue;
        };

        let ArgValue::Json(Value::String(encoded)) = value else {
            return Err(bad_request(format!(
                "The parameter \"{param_name}\" was not a hex string!"
            )));
        };

        let encoded = strip_hash_prefix(encoded, param_name);

        let bytes = bytes_from_hex(encoded).ok_or_else(|| {
            bad_request(format!(
                "The parameter \"{param_name}\" was not a hex string!"
            ))
        })?;

        args.values
            .insert((*param_name).to_string(), ArgValue::Bytes(bytes));
    }

    for param_name in JSON_BYTE_LIST_PARAMS {
        let Some(value) = args.values.get(*param_name) else {
            continue;
        };

        let ArgValue::List(items) = value else {
            return Err(bad_request(format!(
                "The parameter \"{param_name}\" was not a list of hex strings!"
            )));
        };

        let mut decoded = Vec::with_capacity(items.len());

        for item in items {
            let ArgValue::Json(Value::String(hex)) = item else {
                return Err(bad_request(format!(
                    "The parameter \"{param_name}\" held an item that was not a hex string!"
                )));
            };

            decoded.push(ArgValue::Bytes(bytes_from_hex(hex).ok_or_else(|| {
                bad_request(format!(
                    "The parameter \"{param_name}\" held an item that was not a hex string!"
                ))
            })?));
        }

        args.values
            .insert((*param_name).to_string(), ArgValue::List(decoded));
    }

    if let Some(value) = args.values.shift_remove("account") {
        let account = Account::generate_account_from_serialisable_tuple(&value.to_json_value()?)?;

        args.values.insert(
            "account".to_string(),
            ArgValue::other_object(Arc::new(account)),
        );
    }

    if let Some(value) = args.values.shift_remove("accounts") {
        let ArgValue::List(items) = value else {
            return Err(bad_request("The accounts parameter was not a list!"));
        };

        let mut accounts = Vec::with_capacity(items.len());

        for item in items {
            let account =
                Account::generate_account_from_serialisable_tuple(&item.to_json_value()?)?;

            accounts.push(ArgValue::other_object(Arc::new(account)));
        }

        args.values
            .insert("accounts".to_string(), ArgValue::List(accounts));
    }

    if let Some(value) = args.values.shift_remove("service_keys_to_access_keys") {
        let ArgValue::List(pairs) = value else {
            return Err(bad_request(
                "The service_keys_to_access_keys parameter was not a list!",
            ));
        };

        let mut entries = IndexMap::with_capacity(pairs.len());

        for pair in pairs {
            let ArgValue::List(pair) = pair else {
                return Err(bad_request(
                    "A service_keys_to_access_keys pair was not a list!",
                ));
            };

            let [service_key, access_key] = pair.as_slice() else {
                return Err(bad_request(
                    "A service_keys_to_access_keys pair did not have two items!",
                ));
            };

            let key = service_key
                .as_str()
                .and_then(bytes_from_hex)
                .ok_or_else(|| bad_request("A service key was not a hex string!"))?;

            let value = access_key
                .as_str()
                .and_then(bytes_from_hex)
                .ok_or_else(|| bad_request("An access key was not a hex string!"))?;

            entries.insert(key, value);
        }

        args.values.insert(
            "service_keys_to_access_keys".to_string(),
            ArgValue::BytesDict(entries),
        );
    }

    if let Some(value) = args.values.shift_remove("services") {
        let ArgValue::List(items) = value else {
            return Err(bad_request("The services parameter was not a list!"));
        };

        let mut services = Vec::with_capacity(items.len());

        for item in items {
            let service = generate_service_from_serialisable_tuple(&item.to_json_value()?)?;

            services.push(ArgValue::other_object(Arc::new(service)));
        }

        args.values
            .insert("services".to_string(), ArgValue::List(services));
    }

    Ok(args)
}

/// `parse_twisted_request_get_args`.
///
/// `requests_args` is a mapping of raw query-string bytes to value lists, as
/// exposed by Twisted's `Request.args`.
pub fn parse_twisted_request_get_args(
    requests_args: &IndexMap<Vec<u8>, Vec<Vec<u8>>>,
    int_params: &[&str],
    byte_params: &[&str],
    string_params: &[&str],
    json_params: &[&str],
    json_byte_list_params: &[&str],
) -> HydrusResult<ParsedRequestArguments> {
    let cbor_requested = requests_args.keys().any(|key| key == b"cbor");

    if cbor_requested && !CBOR_AVAILABLE {
        return Err(HydrusException::new(
            ErrorKind::NotAcceptable,
            "Sorry, this service does not support CBOR!",
        ));
    }

    let mut args = ParsedRequestArguments::new();

    for (name_bytes, values_bytes) in requests_args {
        let Ok(name) = std::str::from_utf8(name_bytes) else {
            continue;
        };

        let Some(value_bytes) = values_bytes.first() else {
            continue;
        };

        let Ok(value) = std::str::from_utf8(value_bytes) else {
            continue;
        };

        if int_params.contains(&name) {
            let parsed = value.trim().parse::<i64>().map_err(|_| {
                bad_request(format!(
                    "I was expecting to parse '{name}' as an integer, but it failed."
                ))
            })?;

            args.insert(name, parsed);
        } else if byte_params.contains(&name) {
            let encoded = strip_hash_prefix(value, name);

            let bytes = bytes_from_hex(encoded).ok_or_else(|| {
                bad_request(format!(
                    "I was expecting to parse '{name}' as a hex string, but it failed."
                ))
            })?;

            args.insert(name, ArgValue::Bytes(bytes));
        } else if string_params.contains(&name) {
            args.insert(name, percent_decode(value));
        } else if json_params.contains(&name) {
            let value: Value = serde_json::from_str(&percent_decode(value)).map_err(|_| {
                bad_request(format!(
                    "I was expecting to parse '{name}' as a json-encoded string, but it failed."
                ))
            })?;

            args.insert(name, value);
        } else if json_byte_list_params.contains(&name) {
            let value: Value = serde_json::from_str(&percent_decode(value)).map_err(|_| {
                bad_request(format!(
                    "I was expecting to parse '{name}' as a json-encoded hex strings, but it failed."
                ))
            })?;

            let encoded = value.as_array().ok_or_else(|| {
                bad_request(format!(
                    "I was expecting to parse '{name}' as a json-encoded hex strings, but it failed."
                ))
            })?;

            let mut decoded = Vec::with_capacity(encoded.len());

            for item in encoded {
                let hex = item.as_str().ok_or_else(|| {
                    bad_request(format!(
                        "I was expecting to parse '{name}' as a json-encoded hex strings, but it failed."
                    ))
                })?;

                decoded.push(ArgValue::Bytes(bytes_from_hex(hex).ok_or_else(|| {
                    bad_request(format!(
                        "I was expecting to parse '{name}' as a json-encoded hex strings, but it failed."
                    ))
                })?));
            }

            args.insert(name, ArgValue::List(decoded));
        }
    }

    Ok(args)
}

// ---------------------------------------------------------------------------
// Type checks
// ---------------------------------------------------------------------------

fn value_matches_type(value: &ArgValue, expected_type: ExpectedType) -> bool {
    match expected_type {
        ExpectedType::Int => value.as_i64().is_some(),
        ExpectedType::Float => value.as_f64().is_some(),
        ExpectedType::String => value.as_str().is_some(),
        ExpectedType::Bytes => value.as_bytes().is_some(),
        ExpectedType::Bool => value.as_bool().is_some(),
        ExpectedType::List => value.as_list().is_some(),
        ExpectedType::Dict => value.as_dict().is_some() || value.as_bytes_dict().is_some(),
    }
}

fn type_mismatch(name: &str, value: &ArgValue, expected_type: ExpectedType) -> HydrusException {
    bad_request(format!(
        "The parameter \"{}\", with value \"{}\", was not the expected type: {}!",
        name,
        value,
        expected_type.text()
    ))
}

/// `test_variable_type`.
pub fn test_variable_type(
    name: &str,
    value: &ArgValue,
    expected_type: ExpectedType,
    expected_list_type: Option<ExpectedType>,
    expected_dict_types: Option<(ExpectedType, ExpectedType)>,
    allowed_values: Option<&[ArgValue]>,
) -> HydrusResult<()> {
    // Python accepts ints (including bools) wherever a float is requested.
    if expected_type == ExpectedType::Float && value.as_i64().is_some() {
        return Ok(());
    }

    if !value_matches_type(value, expected_type) {
        return Err(type_mismatch(name, value, expected_type));
    }

    if let Some(allowed_values) = allowed_values {
        if !allowed_values.contains(value) {
            let allowed = allowed_values
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(", ");

            return Err(bad_request(format!(
                "The parameter \"{}\", with value \"{}\", was not in the allowed values: ({})!",
                name, value, allowed
            )));
        }
    }

    if expected_type == ExpectedType::List {
        if let (Some(expected_list_type), Some(items)) = (expected_list_type, value.as_list()) {
            for item in items {
                if !value_matches_type(item, expected_list_type) {
                    return Err(bad_request(format!(
                        "The list parameter \"{}\" held an item, \"{}\" that was {} and not the expected type: {}!",
                        name,
                        item,
                        item_type_text(item),
                        expected_list_type.text()
                    )));
                }
            }
        }
    }

    if expected_type == ExpectedType::Dict {
        if let Some((expected_key_type, expected_value_type)) = expected_dict_types {
            match value {
                ArgValue::Dict(entries) => {
                    for (key, value) in entries {
                        let key = ArgValue::Json(Value::String(key.clone()));

                        if !value_matches_type(&key, expected_key_type) {
                            return Err(bad_request(format!(
                                "The Object parameter \"{}\" held a key, \"{}\" that was {} and not the expected type: {}!",
                                name,
                                key,
                                item_type_text(&key),
                                expected_key_type.text()
                            )));
                        }

                        if !value_matches_type(value, expected_value_type) {
                            return Err(bad_request(format!(
                                "The Object parameter \"{}\" held a value, \"{}\" that was {} and not the expected type: {}!",
                                name,
                                value,
                                item_type_text(value),
                                expected_value_type.text()
                            )));
                        }
                    }
                }
                ArgValue::BytesDict(entries) => {
                    for (key, value) in entries {
                        let key = ArgValue::Bytes(key.clone());
                        let value = ArgValue::Bytes(value.clone());

                        if !value_matches_type(&key, expected_key_type) {
                            return Err(bad_request(format!(
                                "The Object parameter \"{}\" held a key, \"{}\" that was {} and not the expected type: {}!",
                                name,
                                key,
                                item_type_text(&key),
                                expected_key_type.text()
                            )));
                        }

                        if !value_matches_type(&value, expected_value_type) {
                            return Err(bad_request(format!(
                                "The Object parameter \"{}\" held a value, \"{}\" that was {} and not the expected type: {}!",
                                name,
                                value,
                                item_type_text(&value),
                                expected_value_type.text()
                            )));
                        }
                    }
                }
                _ => {}
            }
        }
    }

    Ok(())
}

fn item_type_text(value: &ArgValue) -> &'static str {
    match value {
        ArgValue::Json(Value::Bool(_)) => "boolean",
        ArgValue::Json(Value::Number(number)) if number.is_f64() => "float",
        ArgValue::Json(Value::Number(_)) => "integer",
        ArgValue::Json(Value::String(_)) => "string",
        ArgValue::Json(Value::Null) => "null",
        ArgValue::Json(_) => "unknown!",
        ArgValue::Bytes(_) => "hex-encoded bytestring",
        ArgValue::List(_) => "list",
        ArgValue::Dict(_) | ArgValue::BytesDict(_) => "object/dict",
        ArgValue::Object(_) => "serialisable object",
    }
}

/// `get_value_from_dict`.
pub fn get_value_from_dict(
    dictionary: &ParsedRequestArguments,
    key: &str,
    expected_type: ExpectedType,
    expected_list_type: Option<ExpectedType>,
    expected_dict_types: Option<(ExpectedType, ExpectedType)>,
    default_value: Option<ArgValue>,
    none_on_missing: bool,
) -> HydrusResult<ArgValue> {
    if let Some(value) = dictionary.get(key) {
        if !value.is_null() {
            test_variable_type(
                key,
                value,
                expected_type,
                expected_list_type,
                expected_dict_types,
                None,
            )?;

            return Ok(value.clone());
        }
    }

    if default_value.is_none() && !none_on_missing {
        Err(bad_request(format!(
            "The required parameter \"{key}\" was missing!"
        )))
    } else {
        Ok(default_value.unwrap_or_else(ArgValue::null))
    }
}

/// `parse_file_arguments( path, decompression_bombs_ok = False )`
pub fn parse_file_arguments(
    path: impl AsRef<std::path::Path>,
    decompression_bombs_ok: bool,
) -> HydrusResult<ParsedRequestArguments> {
    let path = path.as_ref();

    let hash = hydrus_file_handling::get_hash_from_path(path)?;
    let hash_hex = hex_encode(&hash);

    let parsed = (|| -> HydrusResult<(hydrus_file_handling::FileInfo, Option<Vec<u8>>)> {
        let mime = hydrus_file_handling::get_mime(path, false)?;

        if DECOMPRESSION_BOMB_IMAGES.contains(&mime)
            && !decompression_bombs_ok
            && hydrus_image_handling::is_decompression_bomb(path, None)
        {
            return Err(HydrusException::new(
                ErrorKind::InsufficientCredentials,
                "File seemed to be a Decompression Bomb, which you cannot upload!",
            ));
        }

        let file_info = hydrus_file_handling::get_file_info(path, Some(mime), false)?;

        let thumbnail = if mimes_with_thumbnails().contains(&mime) {
            let image_resolution = (file_info.width.unwrap_or(1), file_info.height.unwrap_or(1));

            let target_resolution = hydrus_image_handling::get_thumbnail_resolution(
                image_resolution,
                SERVER_THUMBNAIL_DIMENSIONS,
                ThumbnailScale::DownOnly,
                100,
            );

            Some(hydrus_file_handling::generate_thumbnail_bytes(
                path,
                target_resolution,
                mime,
                file_info.duration_ms,
                file_info.num_frames,
                35.0,
            )?)
        } else {
            None
        };

        Ok((file_info, thumbnail))
    })();

    let (file_info, thumbnail) = match parsed {
        Ok(parsed) => parsed,
        Err(error) => {
            return Err(HydrusException::new(
                ErrorKind::BadRequest,
                format!("File {hash_hex} could not parse: {error}"),
            ));
        }
    };

    let mut args = ParsedRequestArguments::new();

    args.insert("path", path.to_string_lossy().into_owned());
    args.insert("hash", hash);
    args.insert("size", file_info.size);
    args.insert("mime", u64::from(file_info.mime.as_int()));

    if let Some(width) = file_info.width {
        args.insert("width", u64::from(width));
    }

    if let Some(height) = file_info.height {
        args.insert("height", u64::from(height));
    }

    if let Some(duration_ms) = file_info.duration_ms {
        args.insert("duration", duration_ms);
    }

    if let Some(num_frames) = file_info.num_frames {
        args.insert("num_frames", u64::from(num_frames));
    }

    args.insert("has_audio", file_info.has_audio);

    if let Some(num_words) = file_info.num_words {
        args.insert("num_words", num_words);
    }

    if let Some(thumbnail) = thumbnail {
        args.insert("thumbnail", thumbnail);
    }

    Ok(args)
}

#[cfg(test)]
mod tests {
    use super::*;

    use serde_json::json;

    fn request_args(pairs: &[(&[u8], &[&[u8]])]) -> IndexMap<Vec<u8>, Vec<Vec<u8>>> {
        let mut args = IndexMap::new();

        for (key, values) in pairs {
            args.insert(
                key.to_vec(),
                values.iter().map(|value| value.to_vec()).collect(),
            );
        }

        args
    }

    #[test]
    fn parses_int_byte_and_string_get_args() {
        let hash = vec![0xAB, 0xCD, 0xEF];

        let args = request_args(&[
            (b"num", &[b"5"]),
            (b"access_key", &[b"abcdef"]),
            (b"reason", &[b"hello%20world"]),
            (b"hash", &[b"sha256:0011ff"]),
        ]);

        let parsed = parse_hydrus_network_get_args(&args).unwrap();

        assert_eq!(parsed.get_i64("num").unwrap(), 5);
        assert_eq!(
            parsed.get_bytes("access_key").unwrap(),
            vec![0xAB, 0xCD, 0xEF]
        );
        assert_eq!(parsed.get_string("reason").unwrap(), "hello world");
        assert_eq!(parsed.get_bytes("hash").unwrap(), vec![0x00, 0x11, 0xFF]);

        assert!(hash.len() == 3);
    }

    #[test]
    fn rejects_bad_int_and_bad_hex() {
        let args = request_args(&[(b"num", &[b"not a number"])]);
        let error = parse_hydrus_network_get_args(&args).unwrap_err();

        assert_eq!(error.kind(), ErrorKind::BadRequest);
        assert!(error.to_string().contains("as an integer"));

        let args = request_args(&[(b"access_key", &[b"not hex"])]);
        let error = parse_hydrus_network_get_args(&args).unwrap_err();

        assert_eq!(error.kind(), ErrorKind::BadRequest);
        assert!(error.to_string().contains("as a hex string"));
    }

    #[test]
    fn rejects_cbor() {
        let args = request_args(&[(b"cbor", &[b"1"])]);
        let error = parse_hydrus_network_get_args(&args).unwrap_err();

        assert_eq!(error.kind(), ErrorKind::NotAcceptable);
    }

    #[test]
    fn parses_json_byte_lists() {
        let args = request_args(&[(b"registration_keys", &[br#"["abcd", "00ff"]"#])]);

        let parsed = parse_hydrus_network_get_args(&args).unwrap();

        assert_eq!(
            parsed.get("registration_keys"),
            Some(&ArgValue::List(vec![
                ArgValue::Bytes(vec![0xAB, 0xCD]),
                ArgValue::Bytes(vec![0x00, 0xFF]),
            ]))
        );
    }

    #[test]
    fn builds_subject_identifier_from_hash_and_tag() {
        let hash = vec![0x01, 0x02];
        let args = request_args(&[
            (b"subject_hash", &[b"0102"]),
            (b"subject_tag", &[b"blue%20sky"]),
        ]);

        let parsed = parse_hydrus_network_get_args(&args).unwrap();
        let identifier = parsed
            .get("subject_identifier")
            .unwrap()
            .downcast_object::<AccountIdentifier>()
            .unwrap();

        assert!(identifier.has_content());

        let content = identifier.get_content().unwrap();

        assert_eq!(content.get_content_type(), ContentType::Mapping);
        assert_eq!(
            content.get_content_data(),
            &ContentData::Mapping("blue sky".to_string(), hash)
        );
    }

    #[test]
    fn dump_to_get_query_encodes_values() {
        let mut args = ParsedRequestArguments::new();

        args.insert("num", 7_i64);
        args.insert("access_key", vec![0xAB, 0xCD]);
        args.insert("reason", "hello world/blue");
        args.insert(
            "subject_identifier",
            ArgValue::object(Arc::new(AccountIdentifier::new_content(Content::files(
                vec![vec![0x00, 0x11]],
            )))),
        );

        let query = dump_to_get_query(&args).unwrap();

        assert!(query.contains("num=7"));
        assert!(query.contains("access_key=abcd"));
        assert!(query.contains("reason=hello%20world/blue"));
        assert!(query.contains("subject_hash=0011"));
        assert!(!query.contains("subject_identifier"));
    }

    #[test]
    fn dump_to_get_query_uses_subject_account_key() {
        let mut args = ParsedRequestArguments::new();

        args.insert(
            "subject_identifier",
            ArgValue::object(Arc::new(AccountIdentifier::new_account_key(vec![
                0xDE, 0xAD,
            ]))),
        );

        let query = dump_to_get_query(&args).unwrap();

        assert_eq!(query, "subject_account_key=dead");
    }

    #[test]
    fn network_bytes_round_trip_byte_params_and_byte_lists() {
        let mut args = ParsedRequestArguments::new();

        args.insert("access_key", vec![0xAB, 0xCD]);
        args.insert(
            "registration_keys",
            ArgValue::List(vec![
                ArgValue::Bytes(vec![0x01]),
                ArgValue::Bytes(vec![0x02, 0x03]),
            ]),
        );
        args.insert("num", 42_i64);

        let network_bytes = dump_hydrus_args_to_network_bytes(&args).unwrap();
        let parsed = parse_network_bytes_to_parsed_hydrus_args(&network_bytes).unwrap();

        assert_eq!(parsed.get_bytes("access_key").unwrap(), vec![0xAB, 0xCD]);
        assert_eq!(parsed.get_i64("num").unwrap(), 42);
        assert_eq!(
            parsed.get("registration_keys"),
            Some(&ArgValue::List(vec![
                ArgValue::Bytes(vec![0x01]),
                ArgValue::Bytes(vec![0x02, 0x03]),
            ]))
        );
    }

    #[test]
    fn network_bytes_round_trip_service_keys_to_access_keys() {
        let mut entries = IndexMap::new();
        entries.insert(vec![0x01, 0x02], vec![0xAA, 0xBB]);

        let mut args = ParsedRequestArguments::new();
        args.insert(
            "service_keys_to_access_keys",
            ArgValue::BytesDict(entries.clone()),
        );

        let network_bytes = dump_hydrus_args_to_network_bytes(&args).unwrap();
        let parsed = parse_network_bytes_to_parsed_hydrus_args(&network_bytes).unwrap();

        assert_eq!(
            parsed.get("service_keys_to_access_keys"),
            Some(&ArgValue::BytesDict(entries))
        );
    }

    #[test]
    fn network_bytes_round_trip_account() {
        crate::initialise();

        let account = Arc::new(Account::generate_unknown_account(vec![0x01, 0x02, 0x03]));

        let mut args = ParsedRequestArguments::new();
        args.insert("account", ArgValue::other_object(account.clone()));

        let network_bytes = dump_hydrus_args_to_network_bytes(&args).unwrap();
        let parsed = parse_network_bytes_to_parsed_hydrus_args(&network_bytes).unwrap();

        let parsed_account = parsed
            .get("account")
            .unwrap()
            .downcast_object::<Account>()
            .unwrap();

        assert_eq!(parsed_account.get_account_key(), vec![0x01, 0x02, 0x03]);
    }

    #[test]
    fn network_bytes_round_trip_services() {
        crate::initialise();

        let service = Arc::new(
            crate::networking::hydrus_network::generate_service(
                vec![0x10, 0x20],
                crate::hydrus_constants::ServiceType::ServerAdmin,
                "admin service",
                45870,
                None,
            )
            .unwrap(),
        );

        let mut args = ParsedRequestArguments::new();
        args.insert(
            "services",
            ArgValue::List(vec![ArgValue::other_object(service)]),
        );

        let network_bytes = dump_hydrus_args_to_network_bytes(&args).unwrap();
        let parsed = parse_network_bytes_to_parsed_hydrus_args(&network_bytes).unwrap();

        let ArgValue::List(services) = parsed.get("services").unwrap() else {
            panic!("services was not a list");
        };

        let service = services[0].downcast_object::<ServerService>().unwrap();

        assert_eq!(service.get_name(), "admin service");
        assert_eq!(service.get_port(), 45870);
        assert_eq!(service.get_service_key(), &[0x10, 0x20]);
    }

    #[test]
    fn empty_network_bytes_is_an_empty_dictionary() {
        let parsed = parse_network_bytes_to_parsed_hydrus_args(&[]).unwrap();

        assert!(parsed.is_empty());
    }

    #[test]
    fn network_bytes_must_be_an_object() {
        let mut list = SerialisableList::new();
        list.push(json!(1_i64));

        let bytes = SerialisableObject::dump_to_network_bytes(&list).unwrap();
        let error = parse_network_bytes_to_parsed_hydrus_args(&bytes).unwrap_err();

        assert_eq!(error.kind(), ErrorKind::BadRequest);
    }

    #[test]
    fn test_variable_type_accepts_python_number_rules() {
        let integer = ArgValue::from(5_i64);

        test_variable_type("value", &integer, ExpectedType::Int, None, None, None).unwrap();
        test_variable_type(
            "value",
            &ArgValue::from(true),
            ExpectedType::Int,
            None,
            None,
            None,
        )
        .unwrap();
        test_variable_type("value", &integer, ExpectedType::Float, None, None, None).unwrap();

        let string = ArgValue::from("nope");
        let error =
            test_variable_type("value", &string, ExpectedType::Int, None, None, None).unwrap_err();

        assert_eq!(error.kind(), ErrorKind::BadRequest);
        assert!(error.to_string().contains("integer"));
    }

    #[test]
    fn test_variable_type_checks_list_items() {
        let value = ArgValue::List(vec![ArgValue::from("a"), ArgValue::from(2_i64)]);

        let error = test_variable_type(
            "tags",
            &value,
            ExpectedType::List,
            Some(ExpectedType::String),
            None,
            None,
        )
        .unwrap_err();

        assert_eq!(error.kind(), ErrorKind::BadRequest);
        assert!(error.to_string().contains("expected type: string"));
    }

    #[test]
    fn test_variable_type_checks_dict_types_and_allowed_values() {
        let mut entries = IndexMap::new();
        entries.insert("a".to_string(), ArgValue::from("b"));

        let dict = ArgValue::Dict(entries);

        test_variable_type(
            "notes",
            &dict,
            ExpectedType::Dict,
            None,
            Some((ExpectedType::String, ExpectedType::String)),
            None,
        )
        .unwrap();

        let error = test_variable_type(
            "count",
            &ArgValue::from(3_i64),
            ExpectedType::Int,
            None,
            None,
            Some(&[ArgValue::from(1_i64), ArgValue::from(2_i64)]),
        )
        .unwrap_err();

        assert_eq!(error.kind(), ErrorKind::BadRequest);
        assert!(error.to_string().contains("allowed values"));
    }

    #[test]
    fn get_value_handles_missing_and_defaults() {
        let mut args = ParsedRequestArguments::new();
        args.insert("present", 1_i64);
        args.insert("null", ArgValue::null());

        assert_eq!(args.get_i64("present").unwrap(), 1);

        let error = args.get_i64("missing").unwrap_err();
        assert_eq!(error.kind(), ErrorKind::BadRequest);
        assert!(error.to_string().contains("required parameter"));

        assert!(args.get_i64_or_none("missing").unwrap().is_none());
        assert!(args.get_i64_or_none("null").unwrap().is_none());
        assert_eq!(args.get_i64_default("missing", 9).unwrap(), 9);
    }

    #[test]
    fn percent_helpers_match_python_defaults() {
        assert_eq!(percent_encode("hello world/\u{e9}"), "hello%20world/%C3%A9");
        assert_eq!(percent_decode("hello%20world/%C3%A9"), "hello world/\u{e9}");
        assert_eq!(percent_decode("100%"), "100%");
        assert_eq!(percent_decode("%zz"), "%zz");
    }
}
