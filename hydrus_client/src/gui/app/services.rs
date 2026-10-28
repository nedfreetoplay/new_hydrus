//! Service editor, remote server administration and login credentials.
//!
//! Local services live in the client database. Repository and admin services
//! keep connection details in their `dictionary_string` as a plain JSON object,
//! and every change is persisted immediately. Remote server calls are made by
//! [`crate::networking::hydrus_admin`] and only their summaries are kept in
//! memory.

use std::collections::BTreeMap;
use std::path::Path;

use hydrus_core::hydrus_constants::{ContentType, PermissionAction, ServiceType};
use hydrus_core::hydrus_exceptions::unknown;
use hydrus_core::hydrus_serialisable::SerialisableObject;
use hydrus_core::networking::hydrus_network::AccountType;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::networking::hydrus_admin::{
    read_service, HydrusServiceClient, RemoteAccount, RemoteAccountType, RemoteService,
};

use super::{render, AppState, UiCommand};

const LOGINS_FILENAME: &str = "logins.json";
const ADMIN_FILENAME: &str = "server admin.json";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LoginEntry {
    pub domain: String,
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub password: String,
    #[serde(default)]
    pub access_key: String,
    #[serde(default)]
    pub expires: Option<i64>,
}

impl LoginEntry {
    fn valid(&self) -> bool {
        !self.domain.trim().is_empty()
            && (self.access_key.trim().is_empty()
                || (self.access_key.len() % 2 == 0
                    && self.access_key.chars().all(|c| c.is_ascii_hexdigit())))
    }
}

/// Persisted connection state. Fetched services/accounts are intentionally not
/// saved: they are a cache for the open dialog.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AdminState {
    #[serde(default)]
    pub address: String,
    #[serde(default)]
    pub access_key: String,
    #[serde(default)]
    pub selected_service: String,
    #[serde(default)]
    pub service_keys: BTreeMap<String, String>,
    #[serde(default)]
    pub status: String,
    #[serde(skip)]
    pub services: Vec<RemoteService>,
    #[serde(skip)]
    pub accounts: Vec<RemoteAccount>,
    #[serde(skip)]
    pub account_types: Vec<RemoteAccountType>,
    #[serde(skip)]
    pub options: Value,
    #[serde(skip)]
    pub service_info: Value,
}

pub fn load_logins(path: &Path) -> Vec<LoginEntry> {
    let Ok(text) = std::fs::read_to_string(path) else {
        return Vec::new();
    };

    serde_json::from_str(&text).unwrap_or_default()
}

pub fn save_logins(path: &Path, logins: &[LoginEntry]) -> std::io::Result<()> {
    let dump = serde_json::to_string_pretty(logins).map_err(std::io::Error::other)?;
    std::fs::write(path, dump)
}

pub fn load_admin(path: &Path) -> AdminState {
    let Ok(text) = std::fs::read_to_string(path) else {
        return AdminState::default();
    };

    serde_json::from_str(&text).unwrap_or_default()
}

pub fn save_admin(path: &Path, admin: &AdminState) -> std::io::Result<()> {
    let dump = serde_json::to_string_pretty(admin).map_err(std::io::Error::other)?;
    std::fs::write(path, dump)
}

fn logins_of(state: &AppState) -> Vec<LoginEntry> {
    load_logins(&state.controller.db_dir().join(LOGINS_FILENAME))
}

fn persist_logins(state: &AppState, logins: &[LoginEntry]) {
    let _ = save_logins(&state.controller.db_dir().join(LOGINS_FILENAME), logins);
}

fn persist_admin(state: &AppState) {
    let _ = save_admin(
        &state.controller.db_dir().join(ADMIN_FILENAME),
        &state.admin,
    );
}

pub(super) fn service_dictionary(service: &crate::db::services::Service) -> Value {
    serde_json::from_str(&service.dictionary_string).unwrap_or_else(|_| json!({}))
}

pub(super) fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

pub fn services_json(state: &AppState) -> String {
    let manager = state.controller.services();

    let services: Vec<Value> = manager
        .get_services(None)
        .into_iter()
        .map(|service| {
            let dictionary = service_dictionary(service);

            json!({
                "service_id": service.service_id,
                "key": hex_encode(&service.service_key),
                "name": service.name,
                "type": service.service_type.as_str(),
                "type_short": service.service_type.short_str(),
                "type_int": service.service_type.as_int(),
                "builtin": matches!(service.name.as_str(), "my files" | "local tags" | "trash"),
                "dictionary": dictionary,
            })
        })
        .collect();

    let types: Vec<Value> = service_types()
        .into_iter()
        .map(|(service_type, family)| {
            json!({
                "label": service_type.as_str(),
                "short": service_type.short_str(),
                "value": service_type.as_int(),
                "family": family,
            })
        })
        .collect();

    let mut admin = state.admin.clone();
    admin
        .services
        .sort_by(|left, right| left.name.cmp(&right.name));

    let admin_services: Vec<Value> = admin
        .services
        .iter()
        .map(|service| {
            json!({
                "key": service.key,
                "name": service.name,
                "type": service.type_name,
                "type_int": service.type_int,
                "port": service.port,
                "access_key": admin.service_keys.get(&service.key).cloned().unwrap_or_default(),
                "selected": admin.selected_service == service.key,
            })
        })
        .collect();

    let accounts: Vec<Value> = admin
        .accounts
        .iter()
        .map(|account| {
            json!({
                "key": account.key,
                "type_key": account.type_key,
                "type": account.type_title,
                "created": account.created,
                "expires": account.expires,
                "banned": account.banned,
                "banned_reason": account.banned_reason,
                "message": account.message,
            })
        })
        .collect();

    let account_types: Vec<Value> = admin
        .account_types
        .iter()
        .map(|account_type| {
            json!({
                "key": account_type.key,
                "title": account_type.title,
            })
        })
        .collect();

    let logins: Vec<Value> = logins_of(state)
        .into_iter()
        .map(|login| {
            json!({
                "domain": login.domain,
                "username": login.username,
                "password": login.password,
                "access_key": login.access_key,
                "expires": login.expires,
            })
        })
        .collect();

    json!({
        "services": services,
        "types": types,
        "logins": logins,
        "admin": {
            "address": admin.address,
            "access_key": admin.access_key,
            "status": admin.status,
            "selected_service": admin.selected_service,
            "services": admin_services,
            "accounts": accounts,
            "account_types": account_types,
            "options": admin.options,
            "service_info": admin.service_info,
        },
    })
    .to_string()
}

fn service_types() -> Vec<(ServiceType, &'static str)> {
    vec![
        (ServiceType::LocalFileDomain, "local"),
        (ServiceType::LocalTag, "local"),
        (ServiceType::LocalNotes, "local"),
        (ServiceType::LocalRatingLike, "local"),
        (ServiceType::LocalRatingNumerical, "local"),
        (ServiceType::LocalRatingIncDec, "local"),
        (ServiceType::LocalFileTrashDomain, "local"),
        (ServiceType::TagRepository, "remote"),
        (ServiceType::FileRepository, "remote"),
        (ServiceType::ServerAdmin, "remote"),
    ]
}

pub fn save_service(state: &mut AppState, payload: &str) -> Vec<UiCommand> {
    let Ok(value) = serde_json::from_str::<Value>(payload) else {
        return service_commands(state, "invalid service payload");
    };

    let name = value
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_string();
    let original = value
        .get("original")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_string();

    if name.is_empty() {
        return service_commands(state, "the service needs a name");
    }

    let Some(service_type) = value
        .get("type_int")
        .and_then(Value::as_u64)
        .and_then(|value| u16::try_from(value).ok())
        .and_then(ServiceType::try_from_int)
    else {
        return service_commands(state, "choose a service type");
    };

    if !service_types()
        .iter()
        .any(|(known, _)| *known == service_type)
    {
        return service_commands(state, "that service type is not supported yet");
    }

    let dictionary = value
        .get("dictionary")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let dictionary_string = dictionary.to_string();

    let result = if original.is_empty() {
        state
            .controller
            .add_service(&name, service_type)
            .and_then(|_| {
                let key = state
                    .controller
                    .services()
                    .get_service_key_from_name(&name)
                    .map(ToOwned::to_owned)
                    .ok_or_else(|| unknown("the new service disappeared"));

                key.and_then(|key| {
                    crate::db::services::update_service(
                        state.controller.db().db(),
                        &key,
                        &name,
                        &dictionary_string,
                    )
                })
            })
            .and_then(|_| state.controller.reload_services())
    } else {
        state
            .controller
            .update_service(&original, &name, &dictionary_string)
    };

    match result {
        Ok(()) => service_commands(state, format!("service '{name}' saved")),
        Err(error) => service_commands(state, format!("service error: {error}")),
    }
}

pub fn delete_service(state: &mut AppState, payload: &str) -> Vec<UiCommand> {
    let key = payload.trim();
    let result = state
        .controller
        .delete_service(key)
        .map(|_| "service deleted".to_string());

    match result {
        Ok(message) => service_commands(state, message),
        Err(error) => service_commands(state, format!("service error: {error}")),
    }
}

pub fn service_specifier_json(state: &AppState, types: &[ServiceType]) -> String {
    let manager = state.controller.services();

    let services: Vec<Value> = manager
        .get_services(if types.is_empty() { None } else { Some(types) })
        .into_iter()
        .map(|service| {
            json!({
                "name": service.name,
                "key": hex_encode(&service.service_key),
                "type": service.service_type.short_str(),
            })
        })
        .collect();

    json!({ "services": services }).to_string()
}

pub fn login_save(state: &mut AppState, payload: &str) -> Vec<UiCommand> {
    let Ok(value) = serde_json::from_str::<Value>(payload) else {
        return service_commands(state, "invalid login payload");
    };

    let entry = LoginEntry {
        domain: value
            .get("domain")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .trim()
            .to_lowercase(),
        username: value
            .get("username")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        password: value
            .get("password")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        access_key: value
            .get("access_key")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase(),
        expires: value.get("expires").and_then(Value::as_i64),
    };

    if !entry.valid() {
        return service_commands(
            state,
            "the domain is required and the access key must be hex",
        );
    }

    let mut logins = logins_of(state);
    logins.retain(|login| login.domain != entry.domain);
    logins.push(entry.clone());
    logins.sort_by(|left, right| left.domain.cmp(&right.domain));
    persist_logins(state, &logins);

    service_commands(state, format!("credentials for '{}' saved", entry.domain))
}

pub fn login_remove(state: &mut AppState, payload: &str) -> Vec<UiCommand> {
    let domain = payload.trim().to_lowercase();
    let mut logins = logins_of(state);
    let before = logins.len();
    logins.retain(|login| login.domain != domain);
    persist_logins(state, &logins);

    let message = if logins.len() == before {
        format!("no credentials for '{domain}'")
    } else {
        format!("credentials for '{domain}' removed")
    };

    service_commands(state, message)
}

pub fn admin_connect(state: &mut AppState, payload: &str) -> Vec<UiCommand> {
    let Ok(value) = serde_json::from_str::<Value>(payload) else {
        return service_commands(state, "invalid server payload");
    };
    let address = value
        .get("address")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_string();
    let access_key = value
        .get("access_key")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase();

    let access_key = if access_key.is_empty() {
        match crate::networking::hydrus_admin::bootstrap_admin(&address) {
            Ok(key) => key,
            Err(error) => {
                return service_commands(
                    state,
                    format!("could not bootstrap the admin key: {error}"),
                );
            }
        }
    } else {
        access_key
    };

    let client = match HydrusServiceClient::new(&address, &access_key) {
        Ok(client) => client,
        Err(error) => return service_commands(state, error),
    };

    match client.snapshot() {
        Ok(snapshot) => {
            state.admin.address = address;
            state.admin.access_key = access_key;
            state.admin.services = snapshot.services;
            state.admin.accounts = snapshot.accounts;
            state.admin.account_types = snapshot.account_types;
            state.admin.options = snapshot.options;
            state.admin.service_info = snapshot.service_info;
            state.admin.selected_service.clear();
            state.admin.status = format!(
                "connected to {} | {} services",
                snapshot.address,
                state.admin.services.len()
            );
            persist_admin(state);
            service_commands(state, state.admin.status.clone())
        }
        Err(error) => {
            state.admin.status = error.clone();
            service_commands(state, format!("server error: {error}"))
        }
    }
}

fn selected_service(state: &AppState) -> Option<RemoteService> {
    state
        .admin
        .services
        .iter()
        .find(|service| service.key == state.admin.selected_service)
        .cloned()
}

pub fn admin_service_action(state: &mut AppState, payload: &str) -> Vec<UiCommand> {
    let Ok(value) = serde_json::from_str::<Value>(payload) else {
        return service_commands(state, "invalid server action");
    };
    let action = value
        .get("action")
        .and_then(Value::as_str)
        .unwrap_or_default();

    if action == "select" {
        let service_key = value
            .get("service_key")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        let access_key = value
            .get("access_key")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase();

        state.admin.selected_service = service_key.clone();

        if !access_key.is_empty() {
            state
                .admin
                .service_keys
                .insert(service_key.clone(), access_key.clone());
        }

        let Some(service) = selected_service(state) else {
            return service_commands(state, "the service is missing; refresh the server");
        };

        let access_key = state
            .admin
            .service_keys
            .get(&service_key)
            .cloned()
            .unwrap_or_default();

        if access_key.is_empty() {
            state.admin.status = format!(
                "select '{}' and enter its access key to review accounts",
                service.name
            );
            persist_admin(state);
            return service_commands(state, state.admin.status.clone());
        }

        match read_service(&state.admin.address, &service, &access_key) {
            Ok(snapshot) => {
                state.admin.accounts = snapshot.accounts;
                state.admin.account_types = snapshot.account_types;
                state.admin.options = snapshot.options;
                state.admin.service_info = snapshot.service_info;
                state.admin.status = format!("reviewing {}:{}", service.name, service.port);
            }
            Err(error) => {
                state.admin.status = format!("{}: {error}", service.name);
            }
        }

        persist_admin(state);
        return service_commands(state, state.admin.status.clone());
    }

    let Some(service) = selected_service(state) else {
        return service_commands(state, "choose a server service first");
    };

    let access_key = state
        .admin
        .service_keys
        .get(&service.key)
        .cloned()
        .unwrap_or_default();

    if access_key.is_empty() {
        return service_commands(state, "enter the access key for that service first");
    }

    let client = match HydrusServiceClient::new(&state.admin.address, &access_key) {
        Ok(client) => client.with_port(service.port),
        Err(error) => return service_commands(state, error),
    };

    let result = match action {
        "lock_on" => client
            .post_empty("lock_on")
            .map(|_| "service locked".to_string()),
        "lock_off" => client
            .post_empty("lock_off")
            .map(|_| "service unlocked".to_string()),
        "regen_info" => client
            .post_empty("maintenance_regen_service_info")
            .map(|_| "service info queued for regeneration".to_string()),
        "update_period" => {
            let period = value.get("period").and_then(Value::as_i64).unwrap_or(0);
            client
                .set_period("update", period)
                .map(|_| "update period saved".to_string())
        }
        "nullification_period" => {
            let period = value.get("period").and_then(Value::as_i64).unwrap_or(0);
            client
                .set_period("nullification", period)
                .map(|_| "nullification period saved".to_string())
        }
        "refresh" => read_service(&state.admin.address, &service, &access_key)
            .map(|snapshot| format!("{} accounts", snapshot.accounts.len())),
        _ => Err("unknown server action".to_string()),
    };

    match result {
        Ok(message) => {
            state.admin.status = message.clone();
            persist_admin(state);
            service_commands(state, message)
        }
        Err(error) => {
            state.admin.status = error.clone();
            service_commands(state, format!("server error: {error}"))
        }
    }
}

pub fn admin_add_service(state: &mut AppState, payload: &str) -> Vec<UiCommand> {
    let Ok(value) = serde_json::from_str::<Value>(payload) else {
        return service_commands(state, "invalid server payload");
    };
    let name = value
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_string();
    let port = value
        .get("port")
        .and_then(Value::as_u64)
        .and_then(|port| u16::try_from(port).ok())
        .unwrap_or(0);
    let Some(service_type) = value
        .get("type_int")
        .and_then(Value::as_u64)
        .and_then(|value| u16::try_from(value).ok())
        .and_then(ServiceType::try_from_int)
    else {
        return service_commands(state, "choose a server service type");
    };

    let client = match HydrusServiceClient::new(&state.admin.address, &state.admin.access_key) {
        Ok(client) => client,
        Err(error) => return service_commands(state, error),
    };

    match client.add_service(&state.admin.services, &name, service_type, port) {
        Ok(added) => {
            if let Some((service_key, access_key)) = added {
                state.admin.service_keys.insert(service_key, access_key);
            }

            state.admin.status = format!("service '{name}' added on port {port}");
            refresh_admin(state);
            persist_admin(state);
            service_commands(state, state.admin.status.clone())
        }
        Err(error) => service_commands(state, format!("server error: {error}")),
    }
}

pub fn admin_edit_service(state: &mut AppState, payload: &str) -> Vec<UiCommand> {
    let Ok(value) = serde_json::from_str::<Value>(payload) else {
        return service_commands(state, "invalid server payload");
    };
    let service_key = value
        .get("service_key")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let name = value
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_string();
    let port = value
        .get("port")
        .and_then(Value::as_u64)
        .and_then(|port| u16::try_from(port).ok())
        .unwrap_or(0);

    let client = match HydrusServiceClient::new(&state.admin.address, &state.admin.access_key) {
        Ok(client) => client,
        Err(error) => return service_commands(state, error),
    };

    match client.edit_service(&state.admin.services, service_key, &name, port) {
        Ok(()) => {
            state.admin.status = format!("service '{name}' saved on port {port}");
            refresh_admin(state);
            persist_admin(state);
            service_commands(state, state.admin.status.clone())
        }
        Err(error) => service_commands(state, format!("server error: {error}")),
    }
}

pub fn admin_remove_service(state: &mut AppState, payload: &str) -> Vec<UiCommand> {
    let service_key = payload.trim().to_string();
    let client = match HydrusServiceClient::new(&state.admin.address, &state.admin.access_key) {
        Ok(client) => client,
        Err(error) => return service_commands(state, error),
    };

    match client.remove_service(&state.admin.services, &service_key) {
        Ok(()) => {
            state.admin.status = "service deleted".to_string();
            state.admin.selected_service.clear();
            state.admin.accounts.clear();
            refresh_admin(state);
            persist_admin(state);
            service_commands(state, state.admin.status.clone())
        }
        Err(error) => service_commands(state, format!("server error: {error}")),
    }
}

pub fn admin_account_action(state: &mut AppState, payload: &str) -> Vec<UiCommand> {
    let Ok(value) = serde_json::from_str::<Value>(payload) else {
        return service_commands(state, "invalid account action");
    };
    let action = value
        .get("action")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let Some(service) = selected_service(state) else {
        return service_commands(state, "choose a server service first");
    };
    let Some(access_key) = state.admin.service_keys.get(&service.key).cloned() else {
        return service_commands(state, "enter the access key for that service first");
    };

    let client = match HydrusServiceClient::new(&state.admin.address, &access_key) {
        Ok(client) => client.with_port(service.port),
        Err(error) => return service_commands(state, error),
    };

    match client.modify_account(action, &value) {
        Ok(()) => {
            let message = match action {
                "ban" => "account banned",
                "unban" => "account unbanned",
                "expires" => "account expiry saved",
                "message" => "account message saved",
                "account_type" => "account type changed",
                "delete_all_content" => "account content deleted",
                _ => "account updated",
            };
            state.admin.status = message.to_string();
            if let Ok(snapshot) = read_service(&state.admin.address, &service, &access_key) {
                state.admin.accounts = snapshot.accounts;
                state.admin.account_types = snapshot.account_types;
            }
            service_commands(state, message)
        }
        Err(error) => service_commands(state, format!("server error: {error}")),
    }
}

fn selected_client(state: &AppState) -> Result<HydrusServiceClient, String> {
    let service =
        selected_service(state).ok_or_else(|| "choose a server service first".to_string())?;
    let access_key = state
        .admin
        .service_keys
        .get(&service.key)
        .cloned()
        .ok_or_else(|| "enter the access key for that service first".to_string())?;

    HydrusServiceClient::new(&state.admin.address, &access_key)
        .map(|client| client.with_port(service.port))
}

pub fn account_types_json(state: &AppState) -> String {
    let mut types: Vec<&RemoteAccountType> = state.admin.account_types.iter().collect();
    types.sort_by(|left, right| left.title.to_lowercase().cmp(&right.title.to_lowercase()));

    let entries: Vec<Value> = types
        .iter()
        .map(|account_type| {
            json!({
                "key": account_type.key,
                "title": account_type.title,
                "permissions": account_type.permissions,
                "auto_create_accounts": account_type.auto_create.0,
                "auto_create_time_delta": account_type.auto_create.1,
                "is_null": account_type.is_null,
                "summary": permission_summary(account_type),
            })
        })
        .collect();

    let service_type = selected_service(state)
        .and_then(|service| u16::try_from(service.type_int).ok())
        .and_then(ServiceType::try_from_int);

    let possible: Vec<Value> = service_type
        .map(|service_type| {
            hydrus_core::networking::hydrus_network::get_possible_permissions(service_type)
                .into_iter()
                .map(|(content_type, actions)| {
                    let actions: Vec<Value> = actions
                        .into_iter()
                        .map(|action| {
                            json!({
                                "value": action.map(|action| action.as_int()),
                                "label": action
                                    .and_then(|action| {
                                        hydrus_core::hydrus_constants::permission_pair_description(
                                            content_type,
                                            Some(action),
                                        )
                                    })
                                    .unwrap_or("no permission"),
                            })
                        })
                        .collect();

                    json!({
                        "content_type": content_type.as_int(),
                        "label": content_type.as_str(),
                        "actions": actions,
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    json!({
        "service_key": state.admin.selected_service,
        "types": entries,
        "possible": possible,
    })
    .to_string()
}

fn permission_summary(account_type: &RemoteAccountType) -> String {
    if account_type.is_null {
        return "null account".to_string();
    }

    let labels: Vec<&str> = account_type
        .permissions
        .iter()
        .filter_map(|(content_type, action)| {
            let content_type = u16::try_from(*content_type)
                .ok()
                .and_then(ContentType::try_from_int)?;

            hydrus_core::hydrus_constants::permission_pair_description(
                content_type,
                action.and_then(|action| PermissionAction::try_from_int(action)),
            )
        })
        .collect();

    if labels.is_empty() {
        "no permissions".to_string()
    } else {
        labels.join(", ")
    }
}

pub fn open_account_types(state: &mut AppState, _payload: &str) -> Vec<UiCommand> {
    let client = match selected_client(state) {
        Ok(client) => client,
        Err(error) => return service_commands(state, error),
    };

    match client.account_types() {
        Ok(types) => {
            state.admin.account_types = types;
            state.admin.status = format!("{} account type(s)", state.admin.account_types.len());
            service_commands(state, state.admin.status.clone())
                .into_iter()
                .chain([UiCommand::AccountTypesDialog(account_types_json(state))])
                .collect()
        }
        Err(error) => service_commands(state, format!("server error: {error}")),
    }
}

pub fn save_account_type(state: &mut AppState, payload: &str) -> Vec<UiCommand> {
    let Ok(value) = serde_json::from_str::<Value>(payload) else {
        return service_commands(state, "invalid account type payload");
    };
    let key = value
        .get("account_type_key")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase();
    let title = value
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_string();

    if title.is_empty() {
        return service_commands(state, "enter an account type title");
    }

    let permissions = match parse_account_type_permissions(&value) {
        Ok(permissions) => permissions,
        Err(error) => return service_commands(state, error),
    };

    let auto_create = (
        value
            .get("auto_create_accounts")
            .and_then(Value::as_i64)
            .unwrap_or(0)
            .max(0),
        value
            .get("auto_create_time_delta")
            .and_then(Value::as_i64)
            .unwrap_or(86_400)
            .max(60),
    );

    let mut tuples: Vec<Value> = state
        .admin
        .account_types
        .iter()
        .filter(|account_type| account_type.key != key)
        .map(|account_type| account_type.tuple.clone())
        .collect();

    let tuple = if key.is_empty() {
        let account_type = AccountType::new(
            random_key(),
            &title,
            permissions,
            std::sync::Arc::new(hydrus_core::networking::hydrus_networking::BandwidthRules::new()),
            auto_create,
            std::sync::Arc::new(
                hydrus_core::networking::hydrus_networking::BandwidthTracker::new(),
            ),
        );

        account_type.get_serialisable_tuple()
    } else {
        let Some(existing) = state
            .admin
            .account_types
            .iter()
            .find(|account_type| account_type.key == key)
        else {
            return service_commands(state, "that account type is missing; refresh the service");
        };

        if existing.is_null {
            return service_commands(state, "the null account type cannot be edited");
        }

        match edit_account_type_tuple(&existing.tuple, &title, permissions, auto_create) {
            Ok(tuple) => tuple,
            Err(error) => return service_commands(state, error),
        }
    };

    tuples.push(tuple);

    let client = match selected_client(state) {
        Ok(client) => client,
        Err(error) => return service_commands(state, error),
    };

    match client.save_account_types(&tuples, &[]) {
        Ok(()) => {
            state.admin.status = format!("account type '{title}' saved");
            refresh_account_types(state);
            service_commands(state, state.admin.status.clone())
                .into_iter()
                .chain([UiCommand::UpdateAccountTypes(account_types_json(state))])
                .collect()
        }
        Err(error) => service_commands(state, format!("server error: {error}")),
    }
}

pub fn delete_account_type(state: &mut AppState, payload: &str) -> Vec<UiCommand> {
    let Ok(value) = serde_json::from_str::<Value>(payload) else {
        return service_commands(state, "invalid account type payload");
    };
    let key = value
        .get("account_type_key")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase();
    let replacement = value
        .get("replacement_account_type_key")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase();

    let Some(existing) = state
        .admin
        .account_types
        .iter()
        .find(|account_type| account_type.key == key)
        .cloned()
    else {
        return service_commands(state, "that account type is missing");
    };

    if existing.is_null {
        return service_commands(state, "the null account type cannot be deleted");
    }

    if replacement.is_empty() || replacement == key {
        return service_commands(state, "choose a replacement account type");
    }

    let tuples: Vec<Value> = state
        .admin
        .account_types
        .iter()
        .filter(|account_type| account_type.key != key)
        .map(|account_type| account_type.tuple.clone())
        .collect();

    let client = match selected_client(state) {
        Ok(client) => client,
        Err(error) => return service_commands(state, error),
    };

    match client.save_account_types(&tuples, &[(key.clone(), replacement)]) {
        Ok(()) => {
            state.admin.status = format!("account type '{}' deleted", existing.title);
            refresh_account_types(state);
            service_commands(state, state.admin.status.clone())
                .into_iter()
                .chain([UiCommand::UpdateAccountTypes(account_types_json(state))])
                .collect()
        }
        Err(error) => service_commands(state, format!("server error: {error}")),
    }
}

fn parse_account_type_permissions(
    value: &Value,
) -> Result<indexmap::IndexMap<ContentType, Option<PermissionAction>>, String> {
    let Some(entries) = value.get("permissions").and_then(Value::as_array) else {
        return Ok(indexmap::IndexMap::new());
    };

    let mut permissions = indexmap::IndexMap::new();

    for entry in entries {
        let Some(pair) = entry.as_array() else {
            return Err("a permission was not a pair".to_string());
        };
        let (Some(content_type), Some(action)) = (pair.first(), pair.get(1)) else {
            return Err("a permission was not a pair".to_string());
        };
        let Some(content_type) = content_type
            .as_u64()
            .and_then(|value| u16::try_from(value).ok())
            .and_then(ContentType::try_from_int)
        else {
            return Err("unknown permission content type".to_string());
        };

        let action = if action.is_null() {
            None
        } else {
            let Some(action) = action
                .as_u64()
                .and_then(|value| u8::try_from(value).ok())
                .and_then(PermissionAction::try_from_int)
            else {
                return Err("unknown permission action".to_string());
            };

            Some(action)
        };

        permissions.insert(content_type, action);
    }

    Ok(permissions)
}

fn edit_account_type_tuple(
    tuple: &Value,
    title: &str,
    permissions: indexmap::IndexMap<ContentType, Option<PermissionAction>>,
    auto_create: (i64, i64),
) -> Result<Value, String> {
    let mut tuple = tuple.clone();
    let Some(items) = tuple.as_array_mut() else {
        return Err("the account type tuple is malformed".to_string());
    };
    let Some(info) = items.get_mut(2).and_then(Value::as_array_mut) else {
        return Err("the account type info is malformed".to_string());
    };

    if info.len() < 5 {
        return Err("the account type info is malformed".to_string());
    }

    info[1] = json!(title);

    let permission_info: Vec<Value> = permissions
        .iter()
        .map(|(content_type, action)| {
            json!([content_type.as_int(), action.map(|action| action.as_int())])
        })
        .collect();
    info[2] = Value::Array(permission_info);
    info[4] = json!([auto_create.0, auto_create.1]);

    Ok(tuple)
}

fn refresh_account_types(state: &mut AppState) {
    if let Ok(client) = selected_client(state) {
        if let Ok(types) = client.account_types() {
            state.admin.account_types = types;
        }
    }
}

fn random_key() -> Vec<u8> {
    use rand::RngCore;

    let mut key = vec![0u8; 32];
    rand::thread_rng().fill_bytes(&mut key);
    key
}

fn refresh_admin(state: &mut AppState) {
    let Ok(client) = HydrusServiceClient::new(&state.admin.address, &state.admin.access_key) else {
        return;
    };

    if let Ok(services) = client.services() {
        state.admin.services = services;
    }
}

fn service_commands(state: &AppState, message: impl Into<String>) -> Vec<UiCommand> {
    vec![
        UiCommand::UpdateServices(services_json(state)),
        UiCommand::Menus(render::menus_json(state)),
        UiCommand::Status(render::status_json(state)),
        UiCommand::NotifyMessage(message.into()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::Arc;

    use hydrus_core::networking::hydrus_network::{generate_service, Account, AccountType};
    use hydrus_core::networking::hydrus_network_variable_handling::{
        dump_hydrus_args_to_network_bytes, ArgValue, ParsedRequestArguments,
    };

    fn reply(stream: &mut std::net::TcpStream, status: &str, body: &[u8]) {
        let header = format!(
            "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        );

        let _ = stream.write_all(header.as_bytes());
        let _ = stream.write_all(body);
        let _ = stream.flush();
    }

    fn network_body(args: ParsedRequestArguments) -> Vec<u8> {
        dump_hydrus_args_to_network_bytes(&args).unwrap()
    }

    /// Serves enough of the admin/repository endpoints for the client tests.
    fn start_stub(service_key: &[u8], access_key: &[u8]) -> (String, u16) {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let address = format!("http://{}", listener.local_addr().unwrap());
        let port = listener.local_addr().unwrap().port();
        let service_key = service_key.to_vec();
        let access_key = access_key.to_vec();

        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else {
                    continue;
                };

                let mut buffer = [0u8; 8192];
                let size = stream.read(&mut buffer).unwrap_or(0);
                let request = String::from_utf8_lossy(&buffer[..size]).to_string();
                let path = request.split_whitespace().nth(1).unwrap_or("/").to_string();

                if request.is_empty() {
                    continue;
                }

                if path.starts_with("/access_key") {
                    let mut args = ParsedRequestArguments::new();
                    args.insert("access_key", ArgValue::Bytes(access_key.clone()));
                    reply(&mut stream, "200 OK", &network_body(args));
                    continue;
                }

                if path.starts_with("/services") {
                    let service = generate_service(
                        service_key.clone(),
                        ServiceType::ServerAdmin,
                        "server admin",
                        port,
                        None,
                    )
                    .unwrap();
                    let mut args = ParsedRequestArguments::new();
                    args.insert(
                        "services",
                        ArgValue::List(vec![ArgValue::other_object(Arc::new(service))]),
                    );
                    reply(&mut stream, "200 OK", &network_body(args));
                    continue;
                }

                if path.starts_with("/all_accounts") {
                    let account = Account::new(
                        vec![0xab; 32],
                        Arc::new(AccountType::default()),
                        1_700_000_000,
                        None,
                    );
                    let tuple =
                        Account::generate_serialisable_tuple_from_account(&account).unwrap();
                    let mut args = ParsedRequestArguments::new();
                    args.insert("accounts", ArgValue::List(vec![ArgValue::Json(tuple)]));
                    reply(&mut stream, "200 OK", &network_body(args));
                    continue;
                }

                if path.starts_with("/account_types") {
                    let account_type = AccountType::default();
                    let mut args = ParsedRequestArguments::new();
                    args.insert(
                        "account_types",
                        ArgValue::List(vec![ArgValue::object(Arc::new(account_type))]),
                    );
                    reply(&mut stream, "200 OK", &network_body(args));
                    continue;
                }

                if path.starts_with("/options") {
                    let mut options = serde_json::Map::new();
                    options.insert("update_period".to_string(), json!(86400));
                    let mut args = ParsedRequestArguments::new();
                    args.insert("service_options", ArgValue::from(Value::Object(options)));
                    reply(&mut stream, "200 OK", &network_body(args));
                    continue;
                }

                if path.starts_with("/service_info") {
                    let mut args = ParsedRequestArguments::new();
                    args.insert("service_info", ArgValue::List(Vec::new()));
                    reply(&mut stream, "200 OK", &network_body(args));
                    continue;
                }

                reply(&mut stream, "200 OK", b"");
            }
        });

        (address, port)
    }

    fn test_state(directory: &Path) -> AppState {
        let _ = crate::client_options::register_serialisable_type();
        let controller = crate::controller::Controller::new(directory).unwrap();
        controller.init_model().unwrap();

        AppState::new(Arc::new(controller))
    }

    #[test]
    #[ignore = "needs a live hydrus_server; set HYDRUS_ADMIN_TEST_ADDRESS and HYDRUS_ADMIN_TEST_KEY"]
    fn live_admin_adds_and_removes_a_repository() {
        let address = std::env::var("HYDRUS_ADMIN_TEST_ADDRESS").unwrap();
        let key = crate::networking::hydrus_admin::bootstrap_admin(&address)
            .or_else(|_| std::env::var("HYDRUS_ADMIN_TEST_KEY"))
            .unwrap();
        let client = HydrusServiceClient::new(&address, &key).unwrap();
        let before = client.services().unwrap();
        let added = client
            .add_service(
                &before,
                "rust test repository",
                ServiceType::TagRepository,
                45997,
            )
            .unwrap()
            .expect("the server returns an access key for a new service");
        let after = client.services().unwrap();
        assert!(after.iter().any(|service| service.key == added.0));
        client
            .edit_service(&after, &added.0, "renamed rust test repository", 45998)
            .unwrap();
        let edited = client.services().unwrap();
        let edited_service = edited
            .iter()
            .find(|service| service.key == added.0)
            .unwrap();
        assert_eq!(edited_service.name, "renamed rust test repository");
        assert_eq!(edited_service.port, 45998);
        client.remove_service(&edited, &added.0).unwrap();
        let final_services = client.services().unwrap();
        assert!(!final_services.iter().any(|service| service.key == added.0));

        let types = client.account_types().unwrap();
        let mut tuples: Vec<Value> = types.iter().map(|kind| kind.tuple.clone()).collect();
        let new_type = AccountType::new(
            vec![0x5a; 32],
            "rust test account type",
            indexmap::IndexMap::new(),
            Arc::new(hydrus_core::networking::hydrus_networking::BandwidthRules::new()),
            (0, 86_400),
            Arc::new(hydrus_core::networking::hydrus_networking::BandwidthTracker::new()),
        );
        tuples.push(new_type.get_serialisable_tuple());
        client.save_account_types(&tuples, &[]).unwrap();

        let with_type = client.account_types().unwrap();
        assert!(with_type
            .iter()
            .any(|kind| kind.title == "rust test account type"));

        let replacement = with_type
            .iter()
            .find(|kind| !kind.is_null && kind.title != "rust test account type")
            .unwrap()
            .key
            .clone();
        let remaining: Vec<Value> = with_type
            .iter()
            .filter(|kind| kind.title != "rust test account type")
            .map(|kind| kind.tuple.clone())
            .collect();
        client
            .save_account_types(&remaining, &[("5a".repeat(32), replacement)])
            .unwrap();

        let final_types = client.account_types().unwrap();
        assert!(!final_types
            .iter()
            .any(|kind| kind.title == "rust test account type"));
    }

    #[test]
    fn account_lists_survive_the_network_encoding() {
        use hydrus_core::networking::hydrus_network_variable_handling::parse_network_bytes_to_parsed_hydrus_args;

        let account = Account::new(
            vec![0xab; 32],
            Arc::new(AccountType::default()),
            1_700_000_000,
            None,
        );
        let tuple = Account::generate_serialisable_tuple_from_account(&account).unwrap();

        let mut args = ParsedRequestArguments::new();
        args.insert("accounts", ArgValue::List(vec![ArgValue::Json(tuple)]));
        let bytes = dump_hydrus_args_to_network_bytes(&args).unwrap();
        let parsed = parse_network_bytes_to_parsed_hydrus_args(&bytes).unwrap();
        let values = parsed.get_list("accounts").unwrap();

        assert_eq!(values.len(), 1, "{parsed:?}");

        let value = &values[0];
        let created = if let Some(account) = value.downcast_object::<Account>() {
            account.get_created()
        } else {
            let tuple = value
                .as_json()
                .cloned()
                .or_else(|| value.to_json_value().ok())
                .unwrap_or_else(|| panic!("the account tuple is readable: {value:?}"));
            Account::generate_account_from_serialisable_tuple(&tuple)
                .unwrap()
                .get_created()
        };

        assert_eq!(created, 1_700_000_000);
    }

    #[test]
    fn admin_connect_and_account_review_talk_to_a_server() {
        let service_key = b"server admin".to_vec();
        let access_key = [0x4du8; 32];
        let (address, port) = start_stub(&service_key, &access_key);

        let directory = std::env::temp_dir().join(format!(
            "hydrus_services_admin_{}_{}",
            std::process::id(),
            rand::random::<u64>()
        ));
        std::fs::create_dir_all(&directory).unwrap();
        let mut state = test_state(&directory);

        let connect = json!({
            "address": address,
            "access_key": access_key.iter().map(|byte| format!("{byte:02x}")).collect::<String>(),
        })
        .to_string();
        let commands = admin_connect(&mut state, &connect);
        assert!(matches!(
            commands.first(),
            Some(UiCommand::UpdateServices(_))
        ));
        assert_eq!(state.admin.services.len(), 1, "{}", state.admin.status);
        assert_eq!(state.admin.services[0].name, "server admin");
        assert_eq!(state.admin.services[0].port, port);

        let direct = HydrusServiceClient::new(
            &address,
            &access_key
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>(),
        )
        .unwrap()
        .accounts();
        assert!(direct.is_ok(), "{:?}", direct.err());
        assert_eq!(direct.unwrap().len(), 1);

        let key_hex = state.admin.services[0].key.clone();
        let select = json!({
            "action": "select",
            "service_key": key_hex,
            "access_key": access_key.iter().map(|byte| format!("{byte:02x}")).collect::<String>(),
        })
        .to_string();
        admin_service_action(&mut state, &select);
        assert_eq!(state.admin.accounts.len(), 1, "{}", state.admin.status);
        assert_eq!(state.admin.account_types.len(), 1);
        assert_eq!(state.admin.accounts[0].type_title, "standard user");

        let message = json!({
            "action": "message",
            "subject_account_key": hex_encode(&[0xab; 32]),
            "message": "hello",
        })
        .to_string();
        admin_account_action(&mut state, &message);
        assert_eq!(state.admin.status, "account message saved");

        drop(state);
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn empty_admin_key_is_bootstrapped_from_the_init_registration_key() {
        let (address, _) = start_stub(b"server admin", &[0x11; 32]);
        let key = crate::networking::hydrus_admin::bootstrap_admin(&address).unwrap();

        assert_eq!(
            key,
            "1111111111111111111111111111111111111111111111111111111111111111"
        );
    }

    #[test]
    fn account_type_tuples_preserve_bandwidth_when_edited() {
        let account_type = AccountType::default();
        let tuple = account_type.get_serialisable_tuple();

        let mut permissions = indexmap::IndexMap::new();
        permissions.insert(ContentType::Files, Some(PermissionAction::Create));

        let edited = edit_account_type_tuple(&tuple, "renamed", permissions, (5, 3_600)).unwrap();
        let info = edited[2].as_array().unwrap();
        let old_info = tuple[2].as_array().unwrap();

        assert_eq!(info[1], json!("renamed"));
        assert_eq!(info[3], old_info[3]);
        assert_eq!(info[5], old_info[5]);
        assert_eq!(info[4], json!([5, 3_600]));
        assert!(info[2]
            .as_array()
            .unwrap()
            .iter()
            .any(|entry| entry[0] == json!(3) && entry[1] == json!(1)));
    }

    #[test]
    fn account_type_permissions_round_trip() {
        let payload = json!({
            "permissions": [[3, 1], [9, 2], [10, null]],
        });

        let permissions = parse_account_type_permissions(&payload).unwrap();

        assert_eq!(
            permissions.get(&ContentType::Files),
            Some(&Some(PermissionAction::Create))
        );
        assert_eq!(
            permissions.get(&ContentType::Accounts),
            Some(&Some(PermissionAction::Moderate))
        );
        assert_eq!(permissions.get(&ContentType::Options), Some(&None));
    }

    #[test]
    fn invalid_account_type_permissions_are_rejected() {
        assert!(parse_account_type_permissions(&json!({"permissions": [[3, 99]]})).is_err());
        assert!(parse_account_type_permissions(&json!({"permissions": [[999, 1]]})).is_err());
    }

    #[test]
    fn login_entries_follow_the_hydrus_fields() {
        let valid = LoginEntry {
            domain: "example.com".to_string(),
            username: "user".to_string(),
            password: "pass".to_string(),
            access_key: "abcd".to_string(),
            expires: None,
        };

        assert!(valid.valid());

        let mut invalid = valid.clone();
        invalid.domain.clear();
        assert!(!invalid.valid());

        let mut bad_key = valid;
        bad_key.access_key = "xyz".to_string();
        assert!(!bad_key.valid());
    }

    #[test]
    fn service_types_cover_local_and_remote_families() {
        let types = service_types();

        assert!(types
            .iter()
            .any(|(service_type, _)| *service_type == ServiceType::LocalTag));
        assert!(types
            .iter()
            .any(|(service_type, _)| *service_type == ServiceType::TagRepository));
        assert!(types
            .iter()
            .any(|(service_type, _)| *service_type == ServiceType::ServerAdmin));
    }

    #[test]
    fn admin_state_round_trips_credentials() {
        let directory = std::env::temp_dir().join(format!(
            "hydrus_admin_state_{}_{}",
            std::process::id(),
            rand::random::<u64>()
        ));
        std::fs::create_dir_all(&directory).unwrap();

        let mut admin = AdminState {
            address: "https://127.0.0.1:45870".to_string(),
            access_key: "abcd".to_string(),
            selected_service: "cd".to_string(),
            ..AdminState::default()
        };
        admin
            .service_keys
            .insert("cd".to_string(), "ef".to_string());

        save_admin(&directory.join(ADMIN_FILENAME), &admin).unwrap();
        let loaded = load_admin(&directory.join(ADMIN_FILENAME));

        assert_eq!(loaded.address, admin.address);
        assert_eq!(loaded.access_key, admin.access_key);
        assert_eq!(loaded.service_keys.get("cd"), Some(&"ef".to_string()));

        let _ = std::fs::remove_dir_all(directory);
    }
}
