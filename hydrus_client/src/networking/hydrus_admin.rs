//! Client for the Hydrus server administration service.
//!
//! The admin service speaks the same network format as the repositories:
//! compressed Hydrus network bytes in the body and a `Hydrus-Key` header for
//! credentials. The server is usually on a self-signed certificate, so this
//! client accepts invalid certificates for Hydrus connections only.
//!
//! The server object model is shared with the server crate through
//! `hydrus_core::networking::hydrus_network`.

use std::sync::Arc;
use std::time::Duration;

use hydrus_core::hydrus_constants::{Filter, ServiceType, NETWORK_VERSION};
use hydrus_core::hydrus_serialisable::{create_from_serialisable_tuple, SerialisableObject};
use hydrus_core::hydrus_tags::TagFilter;
use hydrus_core::networking::hydrus_network::{
    generate_service, generate_service_from_serialisable_tuple, Account, AccountType, ServerService,
};
use hydrus_core::networking::hydrus_network_variable_handling::{
    dump_hydrus_args_to_network_bytes, parse_network_bytes_to_parsed_hydrus_args, ArgValue,
    ArgValueObject, ParsedRequestArguments,
};
use serde_json::{json, Value};

/// One service as reported by `/services`.
#[derive(Debug, Clone)]
pub struct RemoteService {
    pub key: String,
    pub name: String,
    pub type_int: u16,
    pub type_name: String,
    pub port: u16,
    pub tuple: Value,
}

/// One account as reported by `/all_accounts`.
#[derive(Debug, Clone)]
pub struct RemoteAccount {
    pub key: String,
    pub type_key: String,
    pub type_title: String,
    pub created: i64,
    pub expires: Option<i64>,
    pub banned: bool,
    pub banned_reason: String,
    pub message: String,
}

/// One account type as reported by `/account_types`.
#[derive(Debug, Clone)]
pub struct RemoteAccountType {
    pub key: String,
    pub title: String,
    pub permissions: Vec<(u16, Option<u8>)>,
    pub auto_create: (i64, i64),
    pub is_null: bool,
    pub tuple: Value,
}

#[derive(Debug, Clone)]
pub struct ServiceSnapshot {
    pub address: String,
    pub services: Vec<RemoteService>,
    pub accounts: Vec<RemoteAccount>,
    pub account_types: Vec<RemoteAccountType>,
    pub options: Value,
    pub service_info: Value,
}

/// A connection to one Hydrus service endpoint.
#[derive(Debug, Clone)]
pub struct HydrusServiceClient {
    address: String,
    access_key: String,
}

impl HydrusServiceClient {
    /// `address` is `host[:port]` or a full URL. A missing scheme defaults to
    /// https and a bare host uses port 443; the UI always supplies the admin
    /// service port from the service list for follow-up requests.
    pub fn new(address: &str, access_key: &str) -> Result<Self, String> {
        hydrus_core::networking::hydrus_network::register_serialisable_types();
        hydrus_core::networking::hydrus_networking::register_serialisable_types();
        hydrus_core::hydrus_tags::register_serialisable_types();

        let access_key = access_key.trim().to_ascii_lowercase();

        if access_key.len() % 2 != 0 || !access_key.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err("the access key must be an even number of hex characters".to_string());
        }

        if access_key.is_empty() {
            return Err("enter the service access key".to_string());
        }

        let mut address = address.trim().trim_end_matches('/').to_string();

        if address.is_empty() {
            return Err("enter the service address".to_string());
        }

        if !address.starts_with("http://") && !address.starts_with("https://") {
            address = format!("https://{address}");
        }

        Ok(Self {
            address,
            access_key,
        })
    }

    pub fn address(&self) -> &str {
        &self.address
    }

    pub fn with_port(&self, port: u16) -> Self {
        let without_port = self.address.trim_end_matches('/');
        let (scheme, rest) = without_port
            .split_once("://")
            .unwrap_or(("https", without_port));
        let host = match rest.rsplit_once(':') {
            Some((host, suffix))
                if suffix.chars().all(|c| c.is_ascii_digit()) && !suffix.is_empty() =>
            {
                host
            }
            _ => rest,
        };

        Self {
            address: format!("{scheme}://{}:{port}", host.trim_end_matches('/')),
            access_key: self.access_key.clone(),
        }
    }

    fn client(&self) -> Result<reqwest::blocking::Client, String> {
        reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(20))
            .danger_accept_invalid_certs(true)
            .user_agent(format!(
                "hydrus/{} (rust client {})",
                NETWORK_VERSION,
                env!("CARGO_PKG_VERSION")
            ))
            .build()
            .map_err(|error| format!("could not build the network client: {error}"))
    }

    fn request(
        &self,
        method: &str,
        path: &str,
        args: Option<ParsedRequestArguments>,
    ) -> Result<ParsedRequestArguments, String> {
        let client = self.client()?;
        let mut url = format!("{}/{}", self.address, path.trim_start_matches('/'));
        let mut request = client
            .request(
                reqwest::Method::from_bytes(method.as_bytes())
                    .map_err(|error| error.to_string())?,
                &url,
            )
            .header("Hydrus-Key", &self.access_key);

        if let Some(args) = args {
            if method == "GET" {
                let query =
                    hydrus_core::networking::hydrus_network_variable_handling::dump_to_get_query(
                        &args,
                    )
                    .map_err(|error| error.to_string())?;

                if !query.is_empty() {
                    url.push('?');
                    url.push_str(&query);
                    request = client
                        .request(reqwest::Method::GET, &url)
                        .header("Hydrus-Key", &self.access_key);
                }
            } else {
                let body =
                    dump_hydrus_args_to_network_bytes(&args).map_err(|error| error.to_string())?;
                request = request
                    .header("Content-Type", "application/json")
                    .body(body);
            }
        }

        let response = request
            .send()
            .map_err(|error| format!("could not reach {}: {error}", self.address))?;

        let status = response.status();
        let body = response
            .bytes()
            .map_err(|error| format!("could not read the response: {error}"))?
            .to_vec();

        if !status.is_success() {
            let message = serde_json::from_slice::<Value>(&body)
                .ok()
                .and_then(|value| {
                    value
                        .get("error")
                        .and_then(Value::as_str)
                        .map(str::to_string)
                })
                .unwrap_or_else(|| String::from_utf8_lossy(&body).to_string());

            return Err(format!("{status}: {message}"));
        }

        if body.is_empty() {
            return Ok(ParsedRequestArguments::new());
        }

        parse_network_bytes_to_parsed_hydrus_args(&body).map_err(|error| error.to_string())
    }

    /// `GET /services` returns every service the admin service knows about.
    pub fn services(&self) -> Result<Vec<RemoteService>, String> {
        hydrus_core::networking::hydrus_networking::register_serialisable_types();

        let args = self.request("GET", "services", None)?;
        let values = args
            .get_list("services")
            .map_err(|error| error.to_string())?;
        let mut services = Vec::with_capacity(values.len());

        for value in values {
            let Some(service) = value.downcast_object::<ServerService>() else {
                return Err("the server returned a service this client cannot read".to_string());
            };

            services.push(remote_service(service));
        }

        services.sort_by(|left, right| left.name.to_lowercase().cmp(&right.name.to_lowercase()));

        Ok(services)
    }

    /// Replaces the server's service list. `POST /services` returns a map of
    /// new service keys to their access keys.
    pub fn save_services(&self, services: &[Value]) -> Result<Vec<(String, String)>, String> {
        hydrus_core::networking::hydrus_networking::register_serialisable_types();

        let mut args = ParsedRequestArguments::new();
        let mut values = Vec::with_capacity(services.len());

        for tuple in services {
            let service = generate_service_from_serialisable_tuple(tuple)
                .map_err(|error| error.to_string())?;
            values.push(ArgValue::other_object(std::sync::Arc::new(service)));
        }

        args.insert("services", ArgValue::List(values));

        let response = self.request("POST", "services", Some(args))?;

        let Some(value) = response.get("service_keys_to_access_keys") else {
            return Ok(Vec::new());
        };

        if let Some(pairs) = value.as_bytes_dict() {
            return Ok(pairs
                .iter()
                .map(|(service_key, access_key)| (hex_encode(service_key), hex_encode(access_key)))
                .collect());
        }

        let Some(pairs) = value.as_list() else {
            return Ok(Vec::new());
        };

        let mut keys = Vec::new();

        for pair in pairs {
            if let Some(items) = pair.as_list() {
                let (Some(service_key), Some(access_key)) = (
                    items.first().and_then(hex_value),
                    items.get(1).and_then(hex_value),
                ) else {
                    continue;
                };

                keys.push((service_key, access_key));
                continue;
            }

            if let Some(items) = pair.as_json().and_then(Value::as_array) {
                let service_key = items.first().and_then(Value::as_str);
                let access_key = items.get(1).and_then(Value::as_str);

                if let (Some(service_key), Some(access_key)) = (service_key, access_key) {
                    keys.push((service_key.to_string(), access_key.to_string()));
                }
            }
        }

        Ok(keys)
    }

    pub fn accounts(&self) -> Result<Vec<RemoteAccount>, String> {
        let args = self.request("GET", "all_accounts", None)?;
        let values = args
            .get_list("accounts")
            .map_err(|error| error.to_string())?;
        let mut accounts = Vec::with_capacity(values.len());

        for value in values {
            if let Some(account) = value.downcast_object::<Account>() {
                accounts.push(remote_account(account));
                continue;
            }

            let Some(tuple) = value
                .as_json()
                .cloned()
                .or_else(|| value.to_json_value().ok())
            else {
                continue;
            };
            let account = Account::generate_account_from_serialisable_tuple(&tuple)
                .map_err(|error| error.to_string())?;
            accounts.push(remote_account(&account));
        }

        accounts.sort_by(|left, right| right.created.cmp(&left.created));

        Ok(accounts)
    }

    pub fn account_types(&self) -> Result<Vec<RemoteAccountType>, String> {
        let args = self.request("GET", "account_types", None)?;
        let values = args
            .get_list("account_types")
            .map_err(|error| error.to_string())?;
        let mut types = Vec::with_capacity(values.len());

        for value in values {
            let Some(account_type) = value.downcast_object::<AccountType>() else {
                continue;
            };

            types.push(RemoteAccountType {
                key: hex_encode(account_type.get_account_type_key()),
                title: account_type.get_title().to_string(),
                permissions: account_type
                    .get_permissions()
                    .iter()
                    .map(|(content_type, action)| {
                        (content_type.as_int(), action.map(|action| action.as_int()))
                    })
                    .collect(),
                auto_create: account_type.get_auto_create_account_velocity(),
                is_null: account_type.is_null_account(),
                tuple: account_type.get_serialisable_tuple(),
            });
        }

        types.sort_by(|left, right| left.title.cmp(&right.title));

        Ok(types)
    }

    /// Replaces the account types of the selected admin/repository service.
    pub fn save_account_types(
        &self,
        types: &[Value],
        replacements: &[(String, String)],
    ) -> Result<(), String> {
        let mut values = Vec::with_capacity(types.len());

        for tuple in types {
            let object = create_from_serialisable_tuple(tuple, false)
                .map_err(|error| format!("could not build an account type: {error}"))?;
            values.push(ArgValue::Object(ArgValueObject::Serialisable(
                std::sync::Arc::from(object),
            )));
        }

        let mut replacements_dictionary =
            hydrus_core::hydrus_serialisable::SerialisableBytesDictionary::new();

        for (old, new) in replacements {
            if let (Some(old), Some(new)) = (decode_hex(old), decode_hex(new)) {
                replacements_dictionary.insert(
                    hydrus_core::hydrus_serialisable::BytesDictionaryKey::Bytes(old),
                    hydrus_core::hydrus_serialisable::BytesDictionaryValue::Bytes(new),
                );
            }
        }

        let mut args = ParsedRequestArguments::new();
        args.insert("account_types", ArgValue::List(values));
        args.insert(
            "deletee_account_type_keys_to_new_account_type_keys",
            ArgValue::object(std::sync::Arc::new(replacements_dictionary)),
        );

        self.request("POST", "account_types", Some(args))?;

        Ok(())
    }

    /// Reads the tag filter rules of a tag repository.
    pub fn tag_filter(&self) -> Result<Vec<(String, String)>, String> {
        let args = self.request("GET", "tag_filter", None)?;
        let value = args
            .get("tag_filter")
            .ok_or_else(|| "the server returned no tag filter".to_string())?;
        let filter = value
            .downcast_object::<TagFilter>()
            .ok_or_else(|| "the server returned an unreadable tag filter".to_string())?;

        Ok(filter
            .get_tag_slices_to_rules()
            .iter()
            .map(|(tag_slice, rule)| (tag_slice.clone(), rule.as_str().to_string()))
            .collect())
    }

    /// Replaces the tag filter of a tag repository.
    pub fn set_tag_filter(&self, rules: &[(String, String)]) -> Result<(), String> {
        let mut filter = TagFilter::new();

        for (tag_slice, rule) in rules {
            let rule = match rule.as_str() {
                "whitelist" => Filter::Whitelist,
                "blacklist" => Filter::Blacklist,
                other => return Err(format!("'{other}' is not a tag filter rule")),
            };

            filter.set_rule(tag_slice, rule);
        }

        filter.clean_rules();

        let mut args = ParsedRequestArguments::new();
        args.insert("tag_filter", ArgValue::object(Arc::new(filter)));

        self.request("POST", "tag_filter", Some(args))?;

        Ok(())
    }

    pub fn options(&self) -> Result<Value, String> {
        let args = self.request("GET", "options", None)?;
        Ok(args
            .get("service_options")
            .and_then(ArgValue::as_json)
            .cloned()
            .unwrap_or(Value::Null))
    }

    pub fn service_info(&self) -> Result<Value, String> {
        let args = self.request("GET", "service_info", None)?;
        let values = args
            .get_list("service_info")
            .map_err(|error| error.to_string())?;

        let entries: Vec<Value> = values
            .iter()
            .filter_map(ArgValue::as_list)
            .map(|pair| {
                json!({
                    "info_type": pair.first().and_then(ArgValue::as_json).cloned().unwrap_or(Value::Null),
                    "value": pair.get(1).and_then(ArgValue::as_json).cloned().unwrap_or(Value::Null),
                })
            })
            .collect();

        Ok(Value::Array(entries))
    }

    pub fn set_period(&self, kind: &str, period: i64) -> Result<(), String> {
        let path = match kind {
            "update" => "options_update_period",
            "nullification" => "options_nullification_period",
            _ => return Err("unknown period kind".to_string()),
        };

        let mut args = ParsedRequestArguments::new();
        args.insert(format!("{kind}_period"), period.max(0));

        self.request("POST", path, Some(args))?;

        Ok(())
    }

    pub fn post_empty(&self, path: &str) -> Result<(), String> {
        self.request("POST", path, None)?;
        Ok(())
    }

    pub fn modify_account(&self, action: &str, payload: &Value) -> Result<(), String> {
        let path = match action {
            "ban" => "modify_account_ban",
            "unban" => "modify_account_unban",
            "expires" => "modify_account_expires",
            "message" => "modify_account_set_message",
            "account_type" => "modify_account_account_type",
            "delete_all_content" => "modify_account_delete_all_content",
            _ => return Err("unknown account action".to_string()),
        };

        let subject = payload
            .get("subject_account_key")
            .and_then(Value::as_str)
            .ok_or_else(|| "the account key is missing".to_string())?;

        let mut args = ParsedRequestArguments::new();
        args.insert(
            "subject_account_key",
            ArgValue::Bytes(decode_hex(subject).ok_or_else(|| "invalid account key".to_string())?),
        );

        match action {
            "ban" => {
                args.insert(
                    "reason",
                    payload
                        .get("reason")
                        .and_then(Value::as_str)
                        .unwrap_or("banned by admin")
                        .to_string(),
                );

                match payload.get("expires").and_then(Value::as_i64) {
                    Some(expires) => args.insert("expires", expires),
                    None => args.insert("expires", ArgValue::null()),
                };
            }
            "expires" => {
                let _ = match payload.get("expires").and_then(Value::as_i64) {
                    Some(expires) => args.insert("expires", expires),
                    None => args.insert("expires", ArgValue::null()),
                };
            }
            "message" => {
                args.insert(
                    "message",
                    payload
                        .get("message")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                );
            }
            "account_type" => {
                let key = payload
                    .get("account_type_key")
                    .and_then(Value::as_str)
                    .and_then(decode_hex)
                    .ok_or_else(|| "choose an account type".to_string())?;

                args.insert("account_type_key", ArgValue::Bytes(key));
            }
            _ => {}
        }

        self.request("POST", path, Some(args))?;

        Ok(())
    }

    /// Renames a service or moves it to another port.
    pub fn edit_service(
        &self,
        existing: &[RemoteService],
        service_key: &str,
        name: &str,
        port: u16,
    ) -> Result<(), String> {
        if name.trim().is_empty() {
            return Err("enter a service name".to_string());
        }

        if port == 0 {
            return Err("enter a service port".to_string());
        }

        if existing
            .iter()
            .any(|service| service.port == port && service.key != service_key)
        {
            return Err("another service already uses that port".to_string());
        }

        let mut found = false;
        let mut tuples = Vec::with_capacity(existing.len());

        for service in existing {
            if service.key == service_key {
                let object = generate_service_from_serialisable_tuple(&service.tuple)
                    .map_err(|error| error.to_string())?;
                object.set_name(name.trim());
                object.set_port(port);
                tuples.push(object.to_serialisable_tuple());
                found = true;
            } else {
                tuples.push(service.tuple.clone());
            }
        }

        if !found {
            return Err("the service is missing".to_string());
        }

        self.save_services(&tuples)?;

        Ok(())
    }

    /// Adds one service through the admin endpoint and returns its access key.
    pub fn add_service(
        &self,
        existing: &[RemoteService],
        name: &str,
        service_type: ServiceType,
        port: u16,
    ) -> Result<Option<(String, String)>, String> {
        if name.trim().is_empty() {
            return Err("enter a service name".to_string());
        }

        if port == 0 {
            return Err("enter a service port".to_string());
        }

        if existing.iter().any(|service| service.port == port) {
            return Err("another service already uses that port".to_string());
        }

        let key = random_key();
        let service = generate_service(key, service_type, name.trim(), port, None)
            .map_err(|error| error.to_string())?;

        let mut tuples: Vec<Value> = existing
            .iter()
            .map(|service| service.tuple.clone())
            .collect();
        tuples.push(service.to_serialisable_tuple());

        let keys = self.save_services(&tuples)?;

        Ok(keys.into_iter().next())
    }

    /// Deletes one service by sending the list without it.
    pub fn remove_service(
        &self,
        existing: &[RemoteService],
        service_key: &str,
    ) -> Result<(), String> {
        let tuples: Vec<Value> = existing
            .iter()
            .filter(|service| service.key != service_key)
            .map(|service| service.tuple.clone())
            .collect();

        if tuples.len() == existing.len() {
            return Err("the service is missing".to_string());
        }

        self.save_services(&tuples)?;

        Ok(())
    }

    /// Connects to an admin service and reads its services plus, when the
    /// admin key is reused for the repository endpoints, account data.
    pub fn snapshot(&self) -> Result<ServiceSnapshot, String> {
        let services = self.services()?;
        let accounts = self.accounts()?;
        let account_types = self.account_types()?;
        let options = self.options().unwrap_or(Value::Null);
        let service_info = self.service_info().unwrap_or(Value::Null);

        Ok(ServiceSnapshot {
            address: self.address.clone(),
            services,
            accounts,
            account_types,
            options,
            service_info,
        })
    }
}

/// Bootstraps the first administration access key from the server's built-in
/// `init` registration key. This is the equivalent of Hydrus' first-run
/// "access key" step and only works while the server has an unused init key.
pub fn bootstrap_admin(address: &str) -> Result<String, String> {
    let mut address = address.trim().trim_end_matches('/').to_string();

    if address.is_empty() {
        return Err("enter the service address".to_string());
    }

    if !address.starts_with("http://") && !address.starts_with("https://") {
        address = format!("https://{address}");
    }

    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(20))
        .danger_accept_invalid_certs(true)
        .user_agent(format!("hydrus/{NETWORK_VERSION}"))
        .build()
        .map_err(|error| format!("could not build the network client: {error}"))?;

    let mut args = ParsedRequestArguments::new();
    args.insert("registration_key", ArgValue::Bytes(b"init".to_vec()));
    let query = hydrus_core::networking::hydrus_network_variable_handling::dump_to_get_query(&args)
        .map_err(|error| error.to_string())?;

    let response = client
        .get(format!("{address}/access_key?{query}"))
        .send()
        .map_err(|error| format!("could not reach {address}: {error}"))?;

    let status = response.status();
    let body = response
        .bytes()
        .map_err(|error| format!("could not read the response: {error}"))?
        .to_vec();

    if !status.is_success() {
        let message = serde_json::from_slice::<Value>(&body)
            .ok()
            .and_then(|value| {
                value
                    .get("error")
                    .and_then(Value::as_str)
                    .map(str::to_string)
            })
            .unwrap_or_else(|| String::from_utf8_lossy(&body).to_string());

        return Err(format!("{status}: {message}"));
    }

    let parsed =
        parse_network_bytes_to_parsed_hydrus_args(&body).map_err(|error| error.to_string())?;
    let key = parsed
        .get_bytes("access_key")
        .map_err(|error| error.to_string())?;

    Ok(hex_encode(&key))
}

/// Reads a service's accounts/options through its own port and access key.
pub fn read_service(
    admin_address: &str,
    service: &RemoteService,
    access_key: &str,
) -> Result<ServiceSnapshot, String> {
    let admin = HydrusServiceClient::new(admin_address, access_key)?;
    let client = admin.with_port(service.port);
    let accounts = client.accounts()?;
    let account_types = client.account_types()?;
    let options = client.options().unwrap_or(Value::Null);
    let service_info = client.service_info().unwrap_or(Value::Null);

    Ok(ServiceSnapshot {
        address: client.address.clone(),
        services: vec![service.clone()],
        accounts,
        account_types,
        options,
        service_info,
    })
}

fn remote_service(service: &ServerService) -> RemoteService {
    RemoteService {
        key: hex_encode(service.get_service_key()),
        name: service.get_name(),
        type_int: service.get_service_type().as_int(),
        type_name: service.get_service_type().as_str().to_string(),
        port: service.get_port(),
        tuple: service.to_serialisable_tuple(),
    }
}

fn remote_account(account: &Account) -> RemoteAccount {
    let account_type = account.get_account_type();
    let banned = account.get_banned_info();
    let (message, _) = account.get_message_and_timestamp();

    RemoteAccount {
        key: hex_encode(&account.get_account_key()),
        type_key: hex_encode(account_type.get_account_type_key()),
        type_title: account_type.get_title().to_string(),
        created: account.get_created(),
        expires: account.get_expires(),
        banned: banned.is_some(),
        banned_reason: banned.map(|info| info.reason).unwrap_or_default(),
        message,
    }
}

fn hex_value(value: &ArgValue) -> Option<String> {
    value
        .as_json()
        .and_then(Value::as_str)
        .map(str::to_string)
        .or_else(|| value.as_bytes().map(hex_encode))
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn decode_hex(text: &str) -> Option<Vec<u8>> {
    if text.len() % 2 != 0 {
        return None;
    }

    (0..text.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(&text[index..index + 2], 16).ok())
        .collect()
}

fn random_key() -> Vec<u8> {
    use rand::RngCore;

    let mut key = vec![0u8; 32];
    rand::thread_rng().fill_bytes(&mut key);
    key
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn access_keys_must_be_hex() {
        assert!(HydrusServiceClient::new("example.com:45870", "abcd").is_ok());
        assert!(HydrusServiceClient::new("example.com:45870", "abc").is_err());
        assert!(HydrusServiceClient::new("example.com:45870", "zz").is_err());
        assert!(HydrusServiceClient::new("example.com:45870", "").is_err());
        assert!(HydrusServiceClient::new("", "abcd").is_err());
    }

    #[test]
    fn addresses_get_a_scheme_and_service_port() {
        let client = HydrusServiceClient::new("127.0.0.1:45870", "abcd").unwrap();
        assert_eq!(client.address(), "https://127.0.0.1:45870");

        let repository = client.with_port(45871);
        assert_eq!(repository.address(), "https://127.0.0.1:45871");
    }

    #[test]
    fn account_tuples_are_readable() {
        let account = Account::new(
            vec![0xab; 32],
            std::sync::Arc::new(AccountType::default()),
            1_000,
            Some(2_000),
        );
        let tuple = Account::generate_serialisable_tuple_from_account(&account).unwrap();
        let parsed = Account::generate_account_from_serialisable_tuple(&tuple).unwrap();
        let remote = remote_account(&parsed);

        assert_eq!(remote.type_title, "standard user");
        assert_eq!(remote.created, 1_000);
        assert_eq!(remote.expires, Some(2_000));
        assert!(!remote.banned);
    }

    fn read_http_request(stream: &mut std::net::TcpStream) -> (String, Vec<u8>) {
        use std::io::Read;

        let mut data = Vec::new();
        let mut buffer = [0u8; 8192];

        loop {
            let read = stream.read(&mut buffer).unwrap();

            if read == 0 {
                break;
            }

            data.extend_from_slice(&buffer[..read]);

            let Some(position) = data.windows(4).position(|window| window == b"\r\n\r\n") else {
                continue;
            };

            let head = String::from_utf8_lossy(&data[..position]).to_string();
            let content_length = head
                .lines()
                .find_map(|line| {
                    let (name, value) = line.split_once(':')?;
                    (name.eq_ignore_ascii_case("content-length"))
                        .then(|| value.trim().parse::<usize>().ok())
                        .flatten()
                })
                .unwrap_or(0);

            if data.len() >= position + 4 + content_length {
                return (
                    head,
                    data[position + 4..position + 4 + content_length].to_vec(),
                );
            }
        }

        (String::new(), Vec::new())
    }

    fn http_reply(stream: &mut std::net::TcpStream, body: &[u8]) {
        use std::io::Write;

        let header = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        );

        stream.write_all(header.as_bytes()).unwrap();
        stream.write_all(body).unwrap();
        stream.flush().unwrap();
    }

    #[test]
    fn tag_filters_are_read_and_written_over_the_network() {
        use std::sync::{Arc, Mutex};

        let listener = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let address = format!("http://{}", listener.local_addr().unwrap());
        let stored: Arc<Mutex<Vec<(String, String)>>> = Arc::new(Mutex::new(Vec::new()));
        let server_stored = Arc::clone(&stored);

        std::thread::spawn(move || {
            for _ in 0..2 {
                let Ok((mut stream, _)) = listener.accept() else {
                    return;
                };
                let (head, body) = read_http_request(&mut stream);
                let path = head.split_whitespace().nth(1).unwrap_or("/");

                if path.starts_with("/tag_filter") && head.starts_with("POST") {
                    let parsed = parse_network_bytes_to_parsed_hydrus_args(&body).unwrap();
                    let value = parsed.get_required("tag_filter").unwrap();
                    let filter = value.downcast_object::<TagFilter>().unwrap();
                    *server_stored.lock().unwrap() = filter
                        .get_tag_slices_to_rules()
                        .iter()
                        .map(|(slice, rule)| (slice.clone(), rule.as_str().to_string()))
                        .collect();
                    http_reply(&mut stream, &[]);
                } else {
                    let mut filter = TagFilter::new();
                    filter.set_rule(":", Filter::Blacklist);
                    filter.set_rule("character:", Filter::Whitelist);
                    let mut args = ParsedRequestArguments::new();
                    args.insert("tag_filter", ArgValue::object(Arc::new(filter)));
                    let body = dump_hydrus_args_to_network_bytes(&args).unwrap();
                    http_reply(&mut stream, &body);
                }
            }
        });

        let client = HydrusServiceClient::new(&address, "abcd").unwrap();

        let filter = client.tag_filter().unwrap();
        assert!(filter
            .iter()
            .any(|(slice, rule)| slice == "character:" && rule == "whitelist"));
        assert!(filter
            .iter()
            .any(|(slice, rule)| slice == ":" && rule == "blacklist"));

        client
            .set_tag_filter(&[
                ("*".to_string(), "blacklist".to_string()),
                ("creator:".to_string(), "whitelist".to_string()),
            ])
            .unwrap();

        let read = stored.lock().unwrap().clone();
        assert!(read
            .iter()
            .any(|(slice, rule)| slice == "creator:" && rule == "whitelist"));
    }
}
