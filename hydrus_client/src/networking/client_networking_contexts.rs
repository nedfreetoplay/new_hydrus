//! Network contexts from `hydrus/client/networking/ClientNetworkingContexts.py`.

use std::any::Any;

use hydrus_core::hydrus_exceptions::HydrusResult;
use hydrus_core::hydrus_serialisable::{self, SerialisableObject};
use serde_json::{Value, json};

use crate::client_constants;

pub const SERIALISABLE_VERSION: u16 = 2;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ContextData {
    Key(Vec<u8>),
    Text(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NetworkContext {
    pub context_type: i32,
    pub context_data: Option<ContextData>,
}

impl Default for NetworkContext {
    fn default() -> Self {
        Self::default_context()
    }
}

impl NetworkContext {
    pub fn new(context_type: i32, context_data: Option<ContextData>) -> Self {
        Self {
            context_type,
            context_data,
        }
    }

    pub fn default_context() -> Self {
        Self::new(client_constants::NETWORK_CONTEXT_GLOBAL, None)
    }

    pub fn hydrus() -> Self {
        Self::new(client_constants::NETWORK_CONTEXT_HYDRUS, None)
    }

    pub fn for_domain(domain: impl Into<String>) -> Self {
        Self::new(
            client_constants::NETWORK_CONTEXT_DOMAIN,
            Some(ContextData::Text(domain.into())),
        )
    }

    pub fn for_key(context_type: i32, key: Vec<u8>) -> Self {
        Self::new(context_type, Some(ContextData::Key(key)))
    }

    pub fn is_default(&self) -> bool {
        self.context_type == client_constants::NETWORK_CONTEXT_GLOBAL
    }

    pub fn is_hydrus(&self) -> bool {
        self.context_type == client_constants::NETWORK_CONTEXT_HYDRUS
    }

    pub fn is_ephemeral(&self) -> bool {
        matches!(
            self.context_type,
            client_constants::NETWORK_CONTEXT_DOWNLOADER_PAGE
                | client_constants::NETWORK_CONTEXT_WATCHER_PAGE
        )
    }

    pub fn get_default(&self) -> Self {
        Self::default_context()
    }

    pub fn get_sortable(&self) -> (i32, String) {
        (self.context_type, self.to_string())
    }

    pub fn get_summary(&self) -> String {
        self.to_string()
    }

    pub fn to_string(&self) -> String {
        let base = client_constants::network_context_type_string(self.context_type);

        match (&self.context_data, self.context_type) {
            (None, _) => base.to_string(),
            (Some(ContextData::Text(text)), _) => format!("{base} {text}"),
            (Some(ContextData::Key(key)), _) => format!("{base} {}", hex_encode(key)),
        }
    }

    pub fn to_human_string(&self) -> String {
        match (&self.context_data, self.context_type) {
            (Some(ContextData::Text(text)), client_constants::NETWORK_CONTEXT_DOMAIN) => {
                text.clone()
            }
            _ => self.to_string(),
        }
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

impl SerialisableObject for NetworkContext {
    fn serialisable_type(&self) -> u16 {
        hydrus_serialisable::SERIALISABLE_TYPE_NETWORK_CONTEXT
    }

    fn serialisable_version(&self) -> u16 {
        SERIALISABLE_VERSION
    }

    fn serialisable_name(&self) -> &'static str {
        "Network Context"
    }

    fn get_serialisable_info(&self) -> Value {
        let data = match &self.context_data {
            None => Value::Null,
            Some(ContextData::Text(text)) => json!(text),
            Some(ContextData::Key(key)) => json!(hex_encode(key)),
        };

        json!([self.context_type, data])
    }

    fn initialise_from_serialisable_info(&mut self, info: &Value) -> HydrusResult<()> {
        let values = info.as_array().ok_or_else(|| {
            hydrus_core::hydrus_exceptions::serialisation("NetworkContext info must be a list")
        })?;

        self.context_type = values.first().and_then(Value::as_i64).unwrap_or(0) as i32;
        self.context_data = values.get(1).and_then(Value::as_str).map(|text| {
            if matches!(
                self.context_type,
                client_constants::NETWORK_CONTEXT_DOMAIN
                    | client_constants::NETWORK_CONTEXT_SUBSCRIPTION
            ) {
                ContextData::Text(text.to_string())
            } else {
                ContextData::Key(hex_decode(text).unwrap_or_default())
            }
        });

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
        hydrus_serialisable::SERIALISABLE_TYPE_NETWORK_CONTEXT,
        || Box::new(NetworkContext::default_context()),
    )
}

#[cfg(test)]
mod tests;
