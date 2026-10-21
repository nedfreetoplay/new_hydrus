//! Rust port of `hydrus/core/networking/HydrusNetwork.py`.
//!
//! The Python module is mostly data classes shared by the client and server.
//! Two Rust-specific choices are worth noting:
//!
//! * Python's loosely-typed "serialisable info" tuples are represented as
//!   `serde_json::Value`, matching the rest of this crate.
//! * Python's `HG.controller`/`HG.server_busy` globals are replaced by the
//!   [`ServerServiceController`] injection point, used only by repository
//!   synchronisation.

use std::any::Any;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock, RwLock};

use indexmap::IndexMap;
use serde_json::{json, Value};

use crate::hydrus_constants::{
    content_types_for_service, BandwidthType, ContentStatus, ContentType, ContentUpdate,
    DefinitionsType, PermissionAction, ServiceType, REPOSITORIES, RESTRICTED_SERVICES,
};
use crate::hydrus_data::{generate_key, hex_decode, hex_encode, safe_print};
use crate::hydrus_exceptions::{data_missing, ErrorKind, HydrusException, HydrusResult};
use crate::hydrus_globals;
use crate::hydrus_lists::split_list_into_chunks;
use crate::hydrus_networking::{BandwidthRules, BandwidthTracker};
use crate::hydrus_numbers::base_to_human_int;
use crate::hydrus_serialisable::{
    create_from_string, register_serialisable_type, SerialisableDictionary, SerialisableList,
    SerialisableObject, SerialisableValue, SERIALISABLE_TYPE_ACCOUNT_IDENTIFIER,
    SERIALISABLE_TYPE_ACCOUNT_TYPE, SERIALISABLE_TYPE_BANDWIDTH_RULES,
    SERIALISABLE_TYPE_BANDWIDTH_TRACKER, SERIALISABLE_TYPE_CLIENT_TO_SERVER_UPDATE,
    SERIALISABLE_TYPE_CONTENT, SERIALISABLE_TYPE_CONTENT_UPDATE, SERIALISABLE_TYPE_CREDENTIALS,
    SERIALISABLE_TYPE_DEFINITIONS_UPDATE, SERIALISABLE_TYPE_METADATA,
    SERIALISABLE_TYPE_PETITION, SERIALISABLE_TYPE_PETITION_HEADER,
};
use crate::hydrus_tags::TagFilter;
use crate::hydrus_time::{
    get_now, get_now_float, time_delta_to_pretty_timedelta, time_has_passed,
    timestamp_to_pretty_expires, timestamp_to_pretty_time_delta, PrettyTimeDeltaOptions,
};

pub const UPDATE_CHECKING_PERIOD: i64 = 240;
pub const MIN_UPDATE_PERIOD: i64 = 600;
pub const MAX_UPDATE_PERIOD: i64 = 100000 * 100;

pub const MIN_NULLIFICATION_PERIOD: i64 = 86400;
pub const MAX_NULLIFICATION_PERIOD: i64 = 86400 * 365 * 5;

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|error| error.into_inner())
}

fn serialisation_error(message: impl Into<String>) -> HydrusException {
    HydrusException::new(ErrorKind::Serialisation, message)
}

fn info_array<'a>(value: &'a Value, what: &str) -> HydrusResult<&'a Vec<Value>> {
    value
        .as_array()
        .ok_or_else(|| serialisation_error(format!("{what} was not an array!")))
}

fn element<'a>(items: &'a [Value], index: usize, what: &str) -> HydrusResult<&'a Value> {
    items
        .get(index)
        .ok_or_else(|| serialisation_error(format!("{what} did not have element {index}!")))
}

fn as_i64(value: &Value, what: &str) -> HydrusResult<i64> {
    value
        .as_i64()
        .ok_or_else(|| serialisation_error(format!("{what} was not an integer!")))
}

fn as_u16(value: &Value, what: &str) -> HydrusResult<u16> {
    value
        .as_u64()
        .and_then(|n| u16::try_from(n).ok())
        .ok_or_else(|| serialisation_error(format!("{what} was not a small integer!")))
}

fn as_u8(value: &Value, what: &str) -> HydrusResult<u8> {
    value
        .as_u64()
        .and_then(|n| u8::try_from(n).ok())
        .ok_or_else(|| serialisation_error(format!("{what} was not a small integer!")))
}

fn as_str<'a>(value: &'a Value, what: &str) -> HydrusResult<&'a str> {
    value
        .as_str()
        .ok_or_else(|| serialisation_error(format!("{what} was not a string!")))
}

fn opt_i64(value: &Value, what: &str) -> HydrusResult<Option<i64>> {
    if value.is_null() {
        Ok(None)
    } else {
        Ok(Some(as_i64(value, what)?))
    }
}

fn hex_bytes(value: &Value, what: &str) -> HydrusResult<Vec<u8>> {
    let hex = as_str(value, what)?;

    hex_decode(hex).map_err(|_| serialisation_error(format!("{what} was not valid hexadecimal!")))
}

fn decode_hashes(value: &Value, what: &str) -> HydrusResult<Vec<Vec<u8>>> {
    let encoded = info_array(value, what)?;

    encoded
        .iter()
        .map(|hash| hex_bytes(hash, what))
        .collect()
}

fn encode_hashes(hashes: &[Vec<u8>]) -> Vec<Value> {
    hashes.iter().map(|hash| json!(hex_encode(hash))).collect()
}

/// Parse a standard `( type, [name,] version, info )` tuple into a known type.
fn object_from_tuple<T>(value: &Value, raise_error_on_future_version: bool, what: &str) -> HydrusResult<T>
where
    T: SerialisableObject + Default,
{
    let items = info_array(value, what)?;

    let (version, info) = match items.len() {
        3 => (as_u16(&items[1], what)?, &items[2]),
        4 => (as_u16(&items[2], what)?, &items[3]),
        _ => {
            return Err(serialisation_error(format!(
                "{what} was not length 3 or 4!"
            )));
        }
    };

    let mut object = T::default();

    object.initialise_from_serialisable_tuple(version, info, raise_error_on_future_version)?;

    Ok(object)
}

fn downcast_clone<T>(object: Box<dyn SerialisableObject>, what: &str) -> HydrusResult<T>
where
    T: Clone + 'static,
{
    let borrowed = object
        .as_any()
        .downcast_ref::<T>()
        .ok_or_else(|| serialisation_error(format!("{what} had the wrong type!")))?;

    Ok(borrowed.clone())
}

fn dictionary_from_string(string: &str) -> HydrusResult<SerialisableDictionary> {
    let object = create_from_string(string, false)?;

    downcast_clone(object, "Serialisable dictionary")
}

fn duplicate_dictionary(dictionary: &SerialisableDictionary) -> HydrusResult<SerialisableDictionary> {
    dictionary_from_string(&dictionary.dump_to_string())
}

fn dict_key(key: &str) -> SerialisableValue {
    SerialisableValue::Json(Value::String(key.to_string()))
}

fn dict_insert(
    dictionary: &mut SerialisableDictionary,
    key: &str,
    value: impl Into<SerialisableValue>,
) {
    dictionary.insert(dict_key(key), value);
}

fn dict_get<'a>(dictionary: &'a SerialisableDictionary, key: &str) -> Option<&'a SerialisableValue> {
    dictionary.get(&dict_key(key))
}

fn dict_contains(dictionary: &SerialisableDictionary, key: &str) -> bool {
    dict_get(dictionary, key).is_some()
}

fn dict_get_i64(dictionary: &SerialisableDictionary, key: &str) -> Option<i64> {
    dict_get(dictionary, key)
        .and_then(SerialisableValue::as_json)
        .and_then(Value::as_i64)
}

fn dict_get_bool(dictionary: &SerialisableDictionary, key: &str) -> Option<bool> {
    dict_get(dictionary, key)
        .and_then(SerialisableValue::as_json)
        .and_then(Value::as_bool)
}

fn dict_get_string(dictionary: &SerialisableDictionary, key: &str) -> Option<String> {
    dict_get(dictionary, key)
        .and_then(SerialisableValue::as_json)
        .and_then(Value::as_str)
        .map(str::to_string)
}

/// Reconstruct the plain-JSON form of a dictionary slot.
///
/// Dictionary serialisation coerces JSON arrays/objects into
/// [`SerialisableList`]/[`SerialisableDictionary`] objects, so a slot that was
/// written as a raw JSON array may be read back as an object.
fn slot_to_json(value: &SerialisableValue) -> Option<Value> {
    if let Some(json) = value.as_json() {
        return Some(json.clone());
    }

    let object = value.as_object()?;

    if let Some(list) = object.as_any().downcast_ref::<SerialisableList>() {
        let mut items = Vec::with_capacity(list.as_slice().len());

        for item in list.as_slice() {
            items.push(slot_to_json(item)?);
        }

        return Some(Value::Array(items));
    }

    if let Some(dictionary) = object.as_any().downcast_ref::<SerialisableDictionary>() {
        let mut map = serde_json::Map::new();

        for (key, value) in dictionary.iter() {
            let key = slot_to_json(key)?.as_str()?.to_string();

            map.insert(key, slot_to_json(value)?);
        }

        return Some(Value::Object(map));
    }

    None
}

fn dictionary_from_slot(value: &SerialisableValue) -> HydrusResult<SerialisableDictionary> {    if let Some(object) = value.as_object() {
        if let Some(dictionary) = object.as_any().downcast_ref::<SerialisableDictionary>() {
            return Ok(dictionary.clone());
        }
    }

    if let Some(Value::Object(map)) = value.as_json() {
        return Ok(SerialisableDictionary::from_json_object(map.clone()));
    }

    Err(serialisation_error("A service dictionary was not a dictionary!"))
}

fn clone_object_from_slot<T>(
    value: &SerialisableValue,
    expected_type: u16,
    what: &str,
) -> HydrusResult<Arc<T>>
where
    T: SerialisableObject + Default,
{
    let object = value
        .as_object()
        .ok_or_else(|| serialisation_error(format!("{what} was not a serialisable object!")))?;

    if object.serialisable_type() != expected_type {
        return Err(serialisation_error(format!(
            "{what} had the wrong serialisable type!"
        )));
    }

    let mut cloned = T::default();

    cloned.initialise_from_serialisable_info(&object.get_serialisable_info())?;

    Ok(Arc::new(cloned))
}

// ---------------------------------------------------------------------------
// AccountType
// ---------------------------------------------------------------------------

/// `AccountType`.
#[derive(Clone)]
pub struct AccountType {
    account_type_key: Vec<u8>,
    title: String,
    permissions: IndexMap<ContentType, Option<PermissionAction>>,
    bandwidth_rules: Arc<BandwidthRules>,
    auto_creation_velocity: (i64, i64),
    auto_creation_history: Arc<BandwidthTracker>,
    is_null: bool,
}

impl Default for AccountType {
    fn default() -> Self {
        Self {
            account_type_key: generate_key().to_vec(),
            title: "standard user".to_string(),
            permissions: IndexMap::new(),
            bandwidth_rules: Arc::new(BandwidthRules::new()),
            auto_creation_velocity: (0, 86400),
            auto_creation_history: Arc::new(BandwidthTracker::new()),
            is_null: false,
        }
    }
}

impl std::fmt::Display for AccountType {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "AccountType: {}", self.title)
    }
}

impl AccountType {
    pub fn new(
        account_type_key: Vec<u8>,
        title: impl Into<String>,
        permissions: IndexMap<ContentType, Option<PermissionAction>>,
        bandwidth_rules: Arc<BandwidthRules>,
        auto_creation_velocity: (i64, i64),
        auto_creation_history: Arc<BandwidthTracker>,
    ) -> Self {
        Self {
            account_type_key,
            title: title.into(),
            permissions,
            bandwidth_rules,
            auto_creation_velocity,
            auto_creation_history,
            is_null: false,
        }
    }

    pub fn bandwidth_ok(&self, bandwidth_tracker: &BandwidthTracker) -> bool {
        self.bandwidth_rules.can_start_request_default(bandwidth_tracker)
    }

    pub fn can_auto_create_account_now(&self) -> bool {
        if !self.supports_auto_create_account() {
            return false;
        }

        let (num_accounts_per_time_delta, time_delta) = self.auto_creation_velocity;

        let num_created =
            self.auto_creation_history
                .get_usage(BandwidthType::Data, Some(time_delta), false);

        num_created < num_accounts_per_time_delta
    }

    pub fn get_auto_create_account_history(&self) -> Arc<BandwidthTracker> {
        self.auto_creation_history.clone()
    }

    pub fn get_auto_create_account_velocity(&self) -> (i64, i64) {
        self.auto_creation_velocity
    }

    pub fn get_bandwidth_rules(&self) -> Arc<BandwidthRules> {
        self.bandwidth_rules.clone()
    }

    pub fn get_bandwidth_strings_and_gauge_tuples(
        &self,
        bandwidth_tracker: &BandwidthTracker,
    ) -> Vec<(String, (i64, i64))> {
        self.bandwidth_rules
            .get_bandwidth_strings_and_gauge_tuples(bandwidth_tracker, 600)
    }

    pub fn get_account_type_key(&self) -> &[u8] {
        &self.account_type_key
    }

    pub fn get_permissions(&self) -> &IndexMap<ContentType, Option<PermissionAction>> {
        &self.permissions
    }

    pub fn get_permission_strings(&self) -> Vec<String> {
        if self.is_null_account() {
            return vec!["is null account, cannot do anything".to_string()];
        }

        self.permissions
            .iter()
            .filter_map(|(content_type, action)| {
                crate::hydrus_constants::permission_pair_description(*content_type, *action)
                    .map(str::to_string)
            })
            .collect()
    }

    pub fn get_title(&self) -> &str {
        &self.title
    }

    pub fn has_permission(&self, content_type: ContentType, permission: PermissionAction) -> bool {
        if self.is_null_account() {
            return false;
        }

        let Some(my_permission) = self.permissions.get(&content_type) else {
            return false;
        };

        match permission {
            PermissionAction::Moderate => *my_permission == Some(PermissionAction::Moderate),
            PermissionAction::Create => matches!(
                my_permission,
                Some(PermissionAction::Create) | Some(PermissionAction::Moderate)
            ),
            PermissionAction::Petition => matches!(
                my_permission,
                Some(PermissionAction::Petition)
                    | Some(PermissionAction::Create)
                    | Some(PermissionAction::Moderate)
            ),
        }
    }

    pub fn is_null_account(&self) -> bool {
        self.is_null
    }

    pub fn report_auto_create_account(&self) {
        self.auto_creation_history.report_request_used(1);
    }

    pub fn set_to_null_account(&mut self) {
        self.is_null = true;
    }

    pub fn supports_auto_create_account(&self) -> bool {
        if self.is_null_account() {
            return false;
        }

        let (num_accounts_per_time_delta, _time_delta) = self.auto_creation_velocity;

        num_accounts_per_time_delta > 0
    }

    pub fn generate_admin_account_type(service_type: ServiceType) -> HydrusResult<Self> {
        let bandwidth_rules = Arc::new(BandwidthRules::new());

        let mut permissions = IndexMap::new();

        permissions.insert(
            ContentType::Accounts,
            Some(PermissionAction::Moderate),
        );
        permissions.insert(
            ContentType::AccountTypes,
            Some(PermissionAction::Moderate),
        );
        permissions.insert(ContentType::Options, Some(PermissionAction::Moderate));

        if REPOSITORIES.contains(&service_type) {
            for content_type in content_types_for_service(service_type).unwrap_or(&[]) {
                permissions.insert(*content_type, Some(PermissionAction::Moderate));
            }
        } else if service_type == ServiceType::ServerAdmin {
            permissions.insert(ContentType::Services, Some(PermissionAction::Moderate));
        } else {
            return Err(HydrusException::new(
                ErrorKind::Unknown,
                "Do not have a default admin account type set up for this service yet!",
            ));
        }

        Ok(Self::generate_new_account_type(
            "administrator",
            permissions,
            bandwidth_rules,
        ))
    }

    pub fn generate_new_account_type(
        title: impl Into<String>,
        permissions: IndexMap<ContentType, Option<PermissionAction>>,
        bandwidth_rules: Arc<BandwidthRules>,
    ) -> Self {
        Self::new(
            generate_key().to_vec(),
            title,
            permissions,
            bandwidth_rules,
            (0, 86400),
            Arc::new(BandwidthTracker::new()),
        )
    }

    pub fn generate_null_account_type() -> Self {
        let mut account_type = Self::new(
            generate_key().to_vec(),
            "null account",
            IndexMap::new(),
            Arc::new(BandwidthRules::new()),
            (0, 86400),
            Arc::new(BandwidthTracker::new()),
        );

        account_type.set_to_null_account();

        account_type
    }

    pub fn generate_unknown_account_type() -> Self {
        let bandwidth_rules = Arc::new(BandwidthRules::new());

        bandwidth_rules.add_rule(BandwidthType::Requests, None, 0);

        Self::generate_new_account_type("unknown account", IndexMap::new(), bandwidth_rules)
    }

    fn permissions_to_info(&self) -> Vec<Value> {
        let mut info: Vec<Value> = self
            .permissions
            .iter()
            .map(|(content_type, action)| {
                json!([content_type.as_int(), action.map(PermissionAction::as_int)])
            })
            .collect();

        if self.is_null {
            info.push(json!(["null", true]));
        }

        info
    }

    fn permissions_from_info(
        info: &Value,
    ) -> HydrusResult<(IndexMap<ContentType, Option<PermissionAction>>, bool)> {
        let entries = info_array(info, "Account type permissions")?;

        let mut permissions = IndexMap::new();
        let mut is_null = false;

        for entry in entries {
            let pair = info_array(entry, "Account type permission")?;

            let key = element(pair, 0, "Account type permission")?;

            if key.as_str() == Some("null") {
                is_null = true;
                continue;
            }

            let content_type = ContentType::try_from_int(as_u16(key, "Permission content type")?)
                .ok_or_else(|| serialisation_error("Unknown permission content type!"))?;

            let action = if element(pair, 1, "Account type permission")?.is_null() {
                None
            } else {
                Some(
                    PermissionAction::try_from_int(as_u8(
                        element(pair, 1, "Account type permission")?,
                        "Permission action",
                    )?)
                    .ok_or_else(|| serialisation_error("Unknown permission action!"))?,
                )
            };

            permissions.insert(content_type, action);
        }

        Ok((permissions, is_null))
    }
}

impl SerialisableObject for AccountType {
    fn serialisable_type(&self) -> u16 {
        SERIALISABLE_TYPE_ACCOUNT_TYPE
    }

    fn serialisable_version(&self) -> u16 {
        2
    }

    fn serialisable_name(&self) -> &'static str {
        "Account Type"
    }

    fn get_serialisable_info(&self) -> Value {
        json!([
            hex_encode(&self.account_type_key),
            self.title,
            self.permissions_to_info(),
            self.bandwidth_rules.get_serialisable_tuple(),
            [self.auto_creation_velocity.0, self.auto_creation_velocity.1],
            self.auto_creation_history.get_serialisable_tuple(),
        ])
    }

    fn initialise_from_serialisable_info(&mut self, info: &Value) -> HydrusResult<()> {
        let items = info_array(info, "Account type info")?;

        self.account_type_key = hex_bytes(
            element(items, 0, "Account type info")?,
            "Account type key",
        )?;
        self.title = as_str(element(items, 1, "Account type info")?, "Account type title")?.to_string();

        let (permissions, is_null) =
            Self::permissions_from_info(element(items, 2, "Account type info")?)?;

        self.permissions = permissions;
        self.is_null = is_null;

        self.bandwidth_rules = Arc::new(object_from_tuple::<BandwidthRules>(
            element(items, 3, "Account type info")?,
            false,
            "Bandwidth rules",
        )?);

        let velocity = info_array(element(items, 4, "Account type info")?, "Auto creation velocity")?;

        self.auto_creation_velocity = (
            as_i64(element(velocity, 0, "Auto creation velocity")?, "Auto creation velocity")?,
            as_i64(element(velocity, 1, "Auto creation velocity")?, "Auto creation velocity")?,
        );

        self.auto_creation_history = Arc::new(object_from_tuple::<BandwidthTracker>(
            element(items, 5, "Account type info")?,
            false,
            "Auto creation history",
        )?);

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

        let items = info_array(&old_info, "Account type v1 info")?;

        let key = element(items, 0, "Account type v1 info")?.clone();
        let title = element(items, 1, "Account type v1 info")?.clone();
        let (mut permissions, _is_null) =
            Self::permissions_from_info(element(items, 2, "Account type v1 info")?)?;
        let rules = element(items, 3, "Account type v1 info")?.clone();
        let velocity = element(items, 4, "Account type v1 info")?.clone();
        let history = element(items, 5, "Account type v1 info")?.clone();

        // Admins can also manage service options.
        if permissions.get(&ContentType::AccountTypes) == Some(&Some(PermissionAction::Moderate)) {
            permissions.insert(ContentType::Options, Some(PermissionAction::Moderate));
        }

        let serialisable_permissions: Vec<Value> = permissions
            .iter()
            .map(|(content_type, action)| {
                json!([content_type.as_int(), action.map(PermissionAction::as_int)])
            })
            .collect();

        Ok((
            2,
            json!([key, title, serialisable_permissions, rules, velocity, history]),
        ))
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

// ---------------------------------------------------------------------------
// Account
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct BannedInfo {
    pub reason: String,
    pub created: i64,
    pub expires: Option<i64>,
}

struct AccountState {
    account_key: Vec<u8>,
    account_type: Arc<AccountType>,
    created: i64,
    expires: Option<i64>,
    message: String,
    message_created: i64,
    banned_info: Option<BannedInfo>,
    bandwidth_tracker: Arc<BandwidthTracker>,
    dirty: bool,
}

impl AccountState {
    fn is_admin(&self) -> bool {
        self.account_type
            .has_permission(ContentType::Services, PermissionAction::Moderate)
    }

    fn is_banned(&mut self) -> bool {
        let Some(info) = self.banned_info.clone() else {
            return false;
        };

        match info.expires {
            None => true,
            Some(expires) => {
                if time_has_passed(Some(expires)) {
                    self.banned_info = None;

                    false
                } else {
                    true
                }
            }
        }
    }

    fn is_expired(&self) -> bool {
        self.expires.is_some_and(|expires| time_has_passed(Some(expires)))
    }

    fn check_banned(&mut self) -> HydrusResult<()> {
        if self.is_banned() {
            return Err(HydrusException::new(
                ErrorKind::InsufficientCredentials,
                format!("This account is banned: {}", self.get_banned_string()),
            ));
        }

        Ok(())
    }

    fn check_expired(&self) -> HydrusResult<()> {
        if self.is_expired() {
            return Err(HydrusException::new(
                ErrorKind::InsufficientCredentials,
                format!("This account is expired: {}", self.get_expires_string()),
            ));
        }

        Ok(())
    }

    fn check_functional(&mut self) -> HydrusResult<()> {
        if self.created == 0 {
            return Err(HydrusException::new(ErrorKind::Conflict, "account is unsynced"));
        }

        if self.is_admin() {
            return Ok(());
        }

        self.check_banned()?;
        self.check_expired()?;

        if !self.account_type.bandwidth_ok(&self.bandwidth_tracker) {
            return Err(HydrusException::new(
                ErrorKind::Bandwidth,
                "account has exceeded bandwidth",
            ));
        }

        Ok(())
    }

    fn get_banned_string(&self) -> String {
        let Some(info) = &self.banned_info else {
            return "not banned".to_string();
        };

        format!(
            "banned {}, {} because: {}",
            timestamp_to_pretty_time_delta(Some(info.created), &PrettyTimeDeltaOptions::default()),
            timestamp_to_pretty_expires(info.expires),
            info.reason
        )
    }

    fn get_expires_string(&self) -> String {
        timestamp_to_pretty_expires(self.expires)
    }
}

/// `Account` (the server-side account object, not `HydrusSessions.Account`).
pub struct Account {
    state: Mutex<AccountState>,
}

impl std::fmt::Display for Account {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let title = lock(&self.state).account_type.get_title().to_string();

        write!(formatter, "Account: {title}")
    }
}

impl Account {
    pub fn new(
        account_key: Vec<u8>,
        account_type: Arc<AccountType>,
        created: i64,
        expires: Option<i64>,
    ) -> Self {
        Self {
            state: Mutex::new(AccountState {
                account_key,
                account_type,
                created,
                expires,
                message: String::new(),
                message_created: 0,
                banned_info: None,
                bandwidth_tracker: Arc::new(BandwidthTracker::new()),
                dirty: false,
            }),
        }
    }

    pub fn ban(&self, reason: impl Into<String>, created: i64, expires: Option<i64>) {
        let mut state = lock(&self.state);

        state.banned_info = Some(BannedInfo {
            reason: reason.into(),
            created,
            expires,
        });
        state.dirty = true;
    }

    pub fn check_at_least_one_permission(
        &self,
        content_types_and_actions: &[(ContentType, PermissionAction)],
    ) -> HydrusResult<()> {
        let state = lock(&self.state);

        if !content_types_and_actions
            .iter()
            .any(|(content_type, action)| state.account_type.has_permission(*content_type, *action))
        {
            return Err(HydrusException::new(
                ErrorKind::InsufficientCredentials,
                "You do not have permission to do that.",
            ));
        }

        Ok(())
    }

    pub fn check_functional(&self) -> HydrusResult<()> {
        lock(&self.state).check_functional()
    }

    pub fn check_permission(
        &self,
        content_type: ContentType,
        action: PermissionAction,
    ) -> HydrusResult<()> {
        let mut state = lock(&self.state);

        if state.is_admin() {
            return Ok(());
        }

        state.check_banned()?;
        state.check_expired()?;

        if !state.account_type.has_permission(content_type, action) {
            return Err(HydrusException::new(
                ErrorKind::InsufficientCredentials,
                "You do not have permission to do that.",
            ));
        }

        Ok(())
    }

    pub fn get_account_key(&self) -> Vec<u8> {
        lock(&self.state).account_key.clone()
    }

    pub fn get_account_type(&self) -> Arc<AccountType> {
        lock(&self.state).account_type.clone()
    }

    pub fn get_bandwidth_current_month_summary(&self) -> String {
        let state = lock(&self.state);

        state.bandwidth_tracker.get_current_month_summary()
    }

    pub fn get_bandwidth_strings_and_gauge_tuples(&self) -> Vec<(String, (i64, i64))> {
        let state = lock(&self.state);

        state
            .account_type
            .get_bandwidth_strings_and_gauge_tuples(&state.bandwidth_tracker)
    }

    pub fn get_bandwidth_tracker(&self) -> Arc<BandwidthTracker> {
        lock(&self.state).bandwidth_tracker.clone()
    }

    pub fn get_banned_info(&self) -> Option<BannedInfo> {
        lock(&self.state).banned_info.clone()
    }

    pub fn get_created(&self) -> i64 {
        lock(&self.state).created
    }

    pub fn get_expires(&self) -> Option<i64> {
        lock(&self.state).expires
    }

    pub fn get_expires_string(&self) -> String {
        let mut state = lock(&self.state);

        if state.is_banned() {
            state.get_banned_string()
        } else {
            state.get_expires_string()
        }
    }

    pub fn get_message_and_timestamp(&self) -> (String, i64) {
        let state = lock(&self.state);

        (state.message.clone(), state.message_created)
    }

    pub fn get_single_line_title(&self) -> String {
        let mut state = lock(&self.state);

        let mut text = format!(
            "{}: {}",
            state.account_type.get_title(),
            hex_encode(&state.account_key)
        );

        if state.is_expired() {
            text = format!("Expired: {text}");
        }

        if state.is_banned() {
            text = format!("Banned: {text}");
        }

        if state.account_type.is_null_account() {
            text = format!("THIS IS NULL ACCOUNT: {text}");
        }

        text
    }

    pub fn get_status_info(&self) -> (bool, String) {
        match lock(&self.state).check_functional() {
            Ok(()) => (true, "account is functional".to_string()),
            Err(error) => (false, error.to_string()),
        }
    }

    pub fn has_permission(&self, content_type: ContentType, action: PermissionAction) -> bool {
        let mut state = lock(&self.state);

        if state.is_admin() {
            return true;
        }

        if state.is_banned() || state.is_expired() {
            return false;
        }

        state.account_type.has_permission(content_type, action)
    }

    pub fn is_banned(&self) -> bool {
        lock(&self.state).is_banned()
    }

    pub fn is_dirty(&self) -> bool {
        lock(&self.state).dirty
    }

    pub fn is_expired(&self) -> bool {
        lock(&self.state).is_expired()
    }

    pub fn is_functional(&self) -> bool {
        lock(&self.state).check_functional().is_ok()
    }

    pub fn is_null_account(&self) -> bool {
        lock(&self.state).account_type.is_null_account()
    }

    pub fn is_unknown(&self) -> bool {
        lock(&self.state).created == 0
    }

    pub fn report_data_used(&self, num_bytes: i64) {
        let mut state = lock(&self.state);

        state.bandwidth_tracker.report_data_used(num_bytes);
        state.dirty = true;
    }

    pub fn report_request_used(&self) {
        let mut state = lock(&self.state);

        state.bandwidth_tracker.report_request_used(1);
        state.dirty = true;
    }

    pub fn set_bandwidth_tracker(&self, bandwidth_tracker: Arc<BandwidthTracker>) {
        let mut state = lock(&self.state);

        state.bandwidth_tracker = bandwidth_tracker;
        state.dirty = true;
    }

    pub fn set_clean(&self) {
        lock(&self.state).dirty = false;
    }

    pub fn set_expires(&self, expires: Option<i64>) {
        let mut state = lock(&self.state);

        state.expires = expires;
        state.dirty = true;
    }

    pub fn set_message(&self, message: impl Into<String>, created: i64) {
        let mut state = lock(&self.state);

        state.message = message.into();
        state.message_created = created;
        state.dirty = true;
    }

    pub fn to_string(&self) -> String {
        let (title, created, is_null) = {
            let state = lock(&self.state);

            (
                state.account_type.get_title().to_string(),
                state.created,
                state.account_type.is_null_account(),
            )
        };

        if is_null {
            return "This is the NULL ACCOUNT. It takes possession of old content to anonymise it. It cannot be modified.".to_string();
        }

        format!(
            "{} -- created {}",
            title,
            timestamp_to_pretty_time_delta(Some(created), &PrettyTimeDeltaOptions::default())
        )
    }

    pub fn unban(&self) {
        let mut state = lock(&self.state);

        state.banned_info = None;
        state.dirty = true;
    }

    pub fn generate_account_from_serialisable_tuple(info: &Value) -> HydrusResult<Self> {
        let items = info_array(info, "Account serialisable tuple")?;

        let account_key = hex_bytes(
            element(items, 0, "Account serialisable tuple")?,
            "Account key",
        )?;

        let serialisable_account_type = element(items, 1, "Account serialisable tuple")?;

        let account_type = Self::account_type_from_serialisable(serialisable_account_type)?;

        let created = as_i64(
            element(items, 2, "Account serialisable tuple")?,
            "Account created",
        )?;
        let expires = opt_i64(
            element(items, 3, "Account serialisable tuple")?,
            "Account expires",
        )?;

        let dictionary_string = as_str(
            element(items, 4, "Account serialisable tuple")?,
            "Account dictionary",
        )?;
        let dictionary = dictionary_from_string(dictionary_string)?;

        Self::generate_account_from_tuple(account_key, Arc::new(account_type), created, expires, dictionary)
    }

    pub fn generate_account_from_tuple(
        account_key: Vec<u8>,
        account_type: Arc<AccountType>,
        created: i64,
        expires: Option<i64>,
        mut dictionary: SerialisableDictionary,
    ) -> HydrusResult<Self> {
        if !dict_contains(&dictionary, "message") {
            dict_insert(&mut dictionary, "message", "");
            dict_insert(&mut dictionary, "message_created", 0_i64);
        }

        let banned_info = Self::banned_info_from_slot(dict_get(&dictionary, "banned_info"))?;

        let bandwidth_tracker = match dict_get(&dictionary, "bandwidth_tracker") {
            Some(value) => clone_object_from_slot::<BandwidthTracker>(
                value,
                SERIALISABLE_TYPE_BANDWIDTH_TRACKER,
                "Account bandwidth tracker",
            )?,
            None => Arc::new(BandwidthTracker::new()),
        };

        let account = Self::new(account_key, account_type, created, expires);

        account.set_bandwidth_tracker(bandwidth_tracker);

        if let Some(info) = banned_info {
            account.ban(info.reason, info.created, info.expires);
        }

        let message = dict_get_string(&dictionary, "message").unwrap_or_default();
        let message_created = dict_get_i64(&dictionary, "message_created").unwrap_or(0);

        account.set_message(message, message_created);
        account.set_clean();

        Ok(account)
    }

    pub fn generate_serialisable_tuple_from_account(account: &Account) -> HydrusResult<Value> {
        let (account_key, account_type, created, expires, dictionary) =
            Self::generate_tuple_from_account(account)?;

        Ok(json!([
            hex_encode(&account_key),
            account_type.get_serialisable_tuple(),
            created,
            expires,
            dictionary.dump_to_string(),
        ]))
    }

    pub fn generate_tuple_from_account(
        account: &Account,
    ) -> HydrusResult<(Vec<u8>, Arc<AccountType>, i64, Option<i64>, SerialisableDictionary)> {
        let (account_key, account_type, created, expires, banned_info, bandwidth_tracker, message, message_created) = {
            let state = lock(&account.state);

            (
                state.account_key.clone(),
                state.account_type.clone(),
                state.created,
                state.expires,
                state.banned_info.clone(),
                state.bandwidth_tracker.clone(),
                state.message.clone(),
                state.message_created,
            )
        };

        let mut dictionary = SerialisableDictionary::new();

        let banned_info = match banned_info {
            Some(info) => json!([info.reason, info.created, info.expires]),
            None => Value::Null,
        };

        dict_insert(&mut dictionary, "banned_info", banned_info);
        dict_insert(
            &mut dictionary,
            "bandwidth_tracker",
            SerialisableValue::Object(bandwidth_tracker),
        );
        dict_insert(&mut dictionary, "message", message);
        dict_insert(&mut dictionary, "message_created", message_created);

        let dictionary = duplicate_dictionary(&dictionary)?;

        Ok((account_key, account_type, created, expires, dictionary))
    }

    pub fn generate_unknown_account(account_key: Vec<u8>) -> Self {
        Self::new(
            account_key,
            Arc::new(AccountType::generate_unknown_account_type()),
            0,
            None,
        )
    }

    fn account_type_from_serialisable(info: &Value) -> HydrusResult<AccountType> {
        let is_legacy = info
            .as_array()
            .and_then(|items| items.first())
            .and_then(Value::as_str)
            .is_some();

        if is_legacy {
            let items = info_array(info, "Legacy account type")?;

            let encoded_key = as_str(element(items, 0, "Legacy account type")?, "Account type key")?;
            let title = as_str(element(items, 1, "Legacy account type")?, "Account type title")?;
            let dictionary_string = as_str(
                element(items, 2, "Legacy account type")?,
                "Account type dictionary",
            )?;

            let account_type_key = hex_decode(encoded_key)
                .map_err(|_| serialisation_error("Could not decode that account type key!"))?;

            return convert_legacy_account_type(account_type_key, title, dictionary_string);
        }

        object_from_tuple::<AccountType>(info, false, "Account type")
    }

    fn banned_info_from_slot(value: Option<&SerialisableValue>) -> HydrusResult<Option<BannedInfo>> {
        let Some(value) = value else {
            return Ok(None);
        };

        // A dictionary slot that held a JSON array may come back as a
        // `SerialisableList` after a round-trip; both forms mean the same
        // thing here.
        let Some(json) = slot_to_json(value) else {
            return Ok(None);
        };

        if json.is_null() {
            return Ok(None);
        }

        let items = info_array(&json, "Banned info")?;

        Ok(Some(BannedInfo {
            reason: as_str(element(items, 0, "Banned info")?, "Ban reason")?.to_string(),
            created: as_i64(element(items, 1, "Banned info")?, "Ban created")?,
            expires: opt_i64(element(items, 2, "Banned info")?, "Ban expires")?,
        }))
    }
}

/// Legacy `( account_type_key, title, dictionary_string )` conversion.
pub fn convert_legacy_account_type(
    account_type_key: Vec<u8>,
    title: &str,
    dictionary_string: &str,
) -> HydrusResult<AccountType> {
    let dictionary = dictionary_from_string(dictionary_string)?;

    let permissions_value = dict_get(&dictionary, "permissions")
        .ok_or_else(|| serialisation_error("Legacy account type had no permissions!"))?;

    let permissions_json = slot_to_json(permissions_value)
        .ok_or_else(|| serialisation_error("Legacy permissions were not JSON!"))?;

    let (permissions, _is_null) = AccountType::permissions_from_info(&permissions_json)?;

    let bandwidth_rules = match dict_get(&dictionary, "bandwidth_rules") {
        Some(value) => clone_object_from_slot::<BandwidthRules>(
            value,
            SERIALISABLE_TYPE_BANDWIDTH_RULES,
            "Legacy bandwidth rules",
        )?,
        None => Arc::new(BandwidthRules::new()),
    };

    Ok(AccountType::new(
        account_type_key,
        title,
        permissions,
        bandwidth_rules,
        (0, 86400),
        Arc::new(BandwidthTracker::new()),
    ))
}

// ---------------------------------------------------------------------------
// Content
// ---------------------------------------------------------------------------

/// The data carried by a [`Content`] object.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub enum ContentData {
    #[default]
    None,
    Files(Vec<Vec<u8>>),
    Mapping(String, Vec<u8>),
    Mappings(String, Vec<Vec<u8>>),
    TagParents(String, String),
    TagSiblings(String, String),
}

/// `Content`.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Content {
    content_type: ContentType,
    content_data: ContentData,
}

impl Default for Content {
    fn default() -> Self {
        Self {
            content_type: ContentType::Unknown,
            content_data: ContentData::None,
        }
    }
}

impl Content {
    pub fn files(hashes: Vec<Vec<u8>>) -> Self {
        Self {
            content_type: ContentType::Files,
            content_data: ContentData::Files(hashes),
        }
    }

    pub fn mapping(tag: impl Into<String>, hash: Vec<u8>) -> Self {
        Self {
            content_type: ContentType::Mapping,
            content_data: ContentData::Mapping(tag.into(), hash),
        }
    }

    pub fn mappings(tag: impl Into<String>, hashes: Vec<Vec<u8>>) -> Self {
        Self {
            content_type: ContentType::Mappings,
            content_data: ContentData::Mappings(tag.into(), hashes),
        }
    }

    pub fn tag_parents(child: impl Into<String>, parent: impl Into<String>) -> Self {
        Self {
            content_type: ContentType::TagParents,
            content_data: ContentData::TagParents(child.into(), parent.into()),
        }
    }

    pub fn tag_siblings(old_tag: impl Into<String>, new_tag: impl Into<String>) -> Self {
        Self {
            content_type: ContentType::TagSiblings,
            content_data: ContentData::TagSiblings(old_tag.into(), new_tag.into()),
        }
    }

    pub fn get_content_data(&self) -> &ContentData {
        &self.content_data
    }

    pub fn get_content_type(&self) -> ContentType {
        self.content_type
    }

    pub fn get_hashes(&self) -> Vec<Vec<u8>> {
        match &self.content_data {
            ContentData::Files(hashes) => hashes.clone(),
            ContentData::Mapping(_tag, hash) => vec![hash.clone()],
            ContentData::Mappings(_tag, hashes) => hashes.clone(),
            _ => Vec::new(),
        }
    }

    pub fn get_actual_weight(&self) -> usize {
        match self.content_type {
            ContentType::Files | ContentType::Mappings => self.get_hashes().len(),
            _ => 1,
        }
    }

    pub fn get_virtual_weight(&self) -> usize {
        match self.content_type {
            ContentType::Files | ContentType::Mappings => self.get_hashes().len(),
            ContentType::TagParents => 5000,
            ContentType::TagSiblings => 5,
            ContentType::Mapping => 1,
            _ => 1,
        }
    }

    pub fn has_hashes(&self) -> bool {
        matches!(
            self.content_type,
            ContentType::Files | ContentType::Mapping | ContentType::Mappings
        )
    }

    pub fn mappings_data(&self) -> Option<(&str, &[Vec<u8>])> {
        match &self.content_data {
            ContentData::Mappings(tag, hashes) => Some((tag, hashes)),
            _ => None,
        }
    }

    pub fn iterate_uploadable_chunks(&self) -> Vec<Content> {
        match &self.content_data {
            ContentData::Files(hashes) => split_list_into_chunks(hashes, 100)
                .into_iter()
                .map(Self::files)
                .collect(),
            ContentData::Mappings(tag, hashes) => split_list_into_chunks(hashes, 500)
                .into_iter()
                .map(|chunk| Self::mappings(tag.clone(), chunk))
                .collect(),
            _ => vec![self.clone()],
        }
    }

    pub fn to_string(&self) -> String {
        match &self.content_data {
            ContentData::Files(hashes) => {
                format!("FILES: {} files", base_to_human_int(hashes.len() as i128))
            }
            ContentData::Mapping(tag, hash) => {
                format!("MAPPING: {tag} for {}", hex_encode(hash))
            }
            ContentData::Mappings(tag, hashes) => {
                format!(
                    "MAPPINGS: {tag} for {} files",
                    base_to_human_int(hashes.len() as i128)
                )
            }
            ContentData::TagParents(child, parent) => {
                format!("PARENT: \"{child}\" -> \"{parent}\"")
            }
            ContentData::TagSiblings(old_tag, new_tag) => {
                format!("SIBLING: \"{old_tag}\" -> \"{new_tag}\"")
            }
            ContentData::None => "Content: unknown".to_string(),
        }
    }
}

impl std::fmt::Display for Content {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.to_string())
    }
}

impl SerialisableObject for Content {
    fn serialisable_type(&self) -> u16 {
        SERIALISABLE_TYPE_CONTENT
    }

    fn serialisable_version(&self) -> u16 {
        1
    }

    fn serialisable_name(&self) -> &'static str {
        "Content"
    }

    fn get_serialisable_info(&self) -> Value {
        let serialisable_content = match &self.content_data {
            ContentData::Files(hashes) => json!(encode_hashes(hashes)),
            ContentData::Mapping(tag, hash) => json!([tag, hex_encode(hash)]),
            ContentData::Mappings(tag, hashes) => json!([tag, encode_hashes(hashes)]),
            ContentData::TagParents(old_tag, new_tag) => json!([old_tag, new_tag]),
            ContentData::TagSiblings(old_tag, new_tag) => json!([old_tag, new_tag]),
            ContentData::None => Value::Null,
        };

        json!([self.content_type.as_int(), serialisable_content])
    }

    fn initialise_from_serialisable_info(&mut self, info: &Value) -> HydrusResult<()> {
        let items = info_array(info, "Content info")?;

        let content_type = ContentType::try_from_int(as_u16(
            element(items, 0, "Content info")?,
            "Content type",
        )?)
        .ok_or_else(|| serialisation_error("Unknown content type!"))?;

        let data = element(items, 1, "Content info")?;

        self.content_type = content_type;
        self.content_data = match content_type {
            ContentType::Files => ContentData::Files(decode_hashes(data, "Content files")?),
            ContentType::Mapping => {
                let pair = info_array(data, "Content mapping")?;
                ContentData::Mapping(
                    as_str(element(pair, 0, "Content mapping")?, "Mapping tag")?.to_string(),
                    hex_bytes(element(pair, 1, "Content mapping")?, "Mapping hash")?,
                )
            }
            ContentType::Mappings => {
                let pair = info_array(data, "Content mappings")?;
                ContentData::Mappings(
                    as_str(element(pair, 0, "Content mappings")?, "Mappings tag")?.to_string(),
                    decode_hashes(element(pair, 1, "Content mappings")?, "Mappings hashes")?,
                )
            }
            ContentType::TagParents => {
                let pair = info_array(data, "Content tag parents")?;
                ContentData::TagParents(
                    as_str(element(pair, 0, "Content tag parents")?, "Parent child")?.to_string(),
                    as_str(element(pair, 1, "Content tag parents")?, "Parent parent")?.to_string(),
                )
            }
            ContentType::TagSiblings => {
                let pair = info_array(data, "Content tag siblings")?;
                ContentData::TagSiblings(
                    as_str(element(pair, 0, "Content tag siblings")?, "Sibling old")?.to_string(),
                    as_str(element(pair, 1, "Content tag siblings")?, "Sibling new")?.to_string(),
                )
            }
            _ => ContentData::None,
        };

        Ok(())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

// ---------------------------------------------------------------------------
// AccountIdentifier
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum AccountIdentifierData {
    AccountKey(Vec<u8>),
    Content(Content),
}

/// `AccountIdentifier`.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct AccountIdentifier {
    data: AccountIdentifierData,
}

impl Default for AccountIdentifier {
    fn default() -> Self {
        Self {
            data: AccountIdentifierData::AccountKey(Vec::new()),
        }
    }
}

impl AccountIdentifier {
    pub const TYPE_ACCOUNT_KEY: u8 = 1;
    pub const TYPE_CONTENT: u8 = 2;

    pub fn new_account_key(account_key: Vec<u8>) -> Self {
        Self {
            data: AccountIdentifierData::AccountKey(account_key),
        }
    }

    pub fn new_content(content: Content) -> Self {
        Self {
            data: AccountIdentifierData::Content(content),
        }
    }

    pub fn get_account_key(&self) -> HydrusResult<&[u8]> {
        match &self.data {
            AccountIdentifierData::AccountKey(key) => Ok(key),
            AccountIdentifierData::Content(_) => Err(HydrusException::new(
                ErrorKind::Unknown,
                "This Account Identifier does not have an account id!",
            )),
        }
    }

    pub fn get_content(&self) -> HydrusResult<&Content> {
        match &self.data {
            AccountIdentifierData::Content(content) => Ok(content),
            AccountIdentifierData::AccountKey(_) => Err(HydrusException::new(
                ErrorKind::Unknown,
                "This Account Identifier does not have content!",
            )),
        }
    }

    pub fn get_data(&self) -> &AccountIdentifierData {
        &self.data
    }

    pub fn has_account_key(&self) -> bool {
        matches!(self.data, AccountIdentifierData::AccountKey(_))
    }

    pub fn has_content(&self) -> bool {
        matches!(self.data, AccountIdentifierData::Content(_))
    }
}

impl SerialisableObject for AccountIdentifier {
    fn serialisable_type(&self) -> u16 {
        SERIALISABLE_TYPE_ACCOUNT_IDENTIFIER
    }

    fn serialisable_version(&self) -> u16 {
        1
    }

    fn serialisable_name(&self) -> &'static str {
        "Account Identifier"
    }

    fn get_serialisable_info(&self) -> Value {
        match &self.data {
            AccountIdentifierData::AccountKey(key) => {
                json!([Self::TYPE_ACCOUNT_KEY, hex_encode(key)])
            }
            AccountIdentifierData::Content(content) => {
                json!([Self::TYPE_CONTENT, content.get_serialisable_tuple()])
            }
        }
    }

    fn initialise_from_serialisable_info(&mut self, info: &Value) -> HydrusResult<()> {
        let items = info_array(info, "Account identifier info")?;

        let identifier_type = as_u8(
            element(items, 0, "Account identifier info")?,
            "Account identifier type",
        )?;
        let data = element(items, 1, "Account identifier info")?;

        self.data = match identifier_type {
            Self::TYPE_ACCOUNT_KEY => {
                AccountIdentifierData::AccountKey(hex_bytes(data, "Account identifier key")?)
            }
            Self::TYPE_CONTENT => {
                AccountIdentifierData::Content(object_from_tuple::<Content>(
                    data,
                    false,
                    "Account identifier content",
                )?)
            }
            _ => {
                return Err(serialisation_error(
                    "Unknown account identifier type!",
                ));
            }
        };

        Ok(())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

// ---------------------------------------------------------------------------
// ContentUpdate
// ---------------------------------------------------------------------------

/// One row added to a [`ContentUpdate`].
#[derive(Clone, Debug)]
pub struct ContentUpdateRow {
    pub content_type: ContentType,
    pub action: ContentUpdate,
    pub data: Value,
}

/// `ContentUpdate`.
#[derive(Clone, Debug, Default)]
pub struct ContentUpdatePackage {
    content_data: IndexMap<ContentType, IndexMap<ContentUpdate, Vec<Value>>>,
}

impl ContentUpdatePackage {
    pub fn new() -> Self {
        Self::default()
    }

    fn get_content(&self, content_type: ContentType, action: ContentUpdate) -> &[Value] {
        self.content_data
            .get(&content_type)
            .and_then(|actions| actions.get(&action))
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    pub fn add_row(&mut self, content_type: ContentType, action: ContentUpdate, data: Value) {
        self.content_data
            .entry(content_type)
            .or_default()
            .entry(action)
            .or_default()
            .push(data);
    }

    pub fn get_deleted_files(&self) -> &[Value] {
        self.get_content(ContentType::Files, ContentUpdate::Delete)
    }

    pub fn get_deleted_mappings(&self) -> &[Value] {
        self.get_content(ContentType::Mappings, ContentUpdate::Delete)
    }

    pub fn get_deleted_tag_parents(&self) -> &[Value] {
        self.get_content(ContentType::TagParents, ContentUpdate::Delete)
    }

    pub fn get_deleted_tag_siblings(&self) -> &[Value] {
        self.get_content(ContentType::TagSiblings, ContentUpdate::Delete)
    }

    pub fn get_new_files(&self) -> &[Value] {
        self.get_content(ContentType::Files, ContentUpdate::Add)
    }

    pub fn get_new_mappings(&self) -> &[Value] {
        self.get_content(ContentType::Mappings, ContentUpdate::Add)
    }

    pub fn get_new_tag_parents(&self) -> &[Value] {
        self.get_content(ContentType::TagParents, ContentUpdate::Add)
    }

    pub fn get_new_tag_siblings(&self) -> &[Value] {
        self.get_content(ContentType::TagSiblings, ContentUpdate::Add)
    }

    pub fn get_num_rows(&self, content_types_to_count: Option<&[ContentType]>) -> usize {
        let mut num = 0;

        for (content_type, actions) in &self.content_data {
            if content_types_to_count.is_some_and(|types| !types.contains(content_type)) {
                continue;
            }

            for data in actions.values() {
                if *content_type == ContentType::Mappings {
                    num += data
                        .iter()
                        .map(|row| {
                            row.as_array()
                                .and_then(|items| items.get(1))
                                .and_then(Value::as_array)
                                .map_or(0, Vec::len)
                        })
                        .sum::<usize>();
                } else {
                    num += data.len();
                }
            }
        }

        num
    }
}

impl std::ops::Deref for ContentUpdatePackage {
    type Target = IndexMap<ContentType, IndexMap<ContentUpdate, Vec<Value>>>;

    fn deref(&self) -> &Self::Target {
        &self.content_data
    }
}

impl SerialisableObject for ContentUpdatePackage {
    fn serialisable_type(&self) -> u16 {
        SERIALISABLE_TYPE_CONTENT_UPDATE
    }

    fn serialisable_version(&self) -> u16 {
        1
    }

    fn serialisable_name(&self) -> &'static str {
        "Content Update"
    }

    fn get_serialisable_info(&self) -> Value {
        let groups: Vec<Value> = self
            .content_data
            .iter()
            .map(|(content_type, actions)| {
                let action_pairs: Vec<Value> = actions
                    .iter()
                    .map(|(action, data)| json!([action.as_int(), data]))
                    .collect();

                json!([content_type.as_int(), action_pairs])
            })
            .collect();

        Value::Array(groups)
    }

    fn initialise_from_serialisable_info(&mut self, info: &Value) -> HydrusResult<()> {
        let groups = info_array(info, "Content update info")?;

        self.content_data.clear();

        for group in groups {
            let pair = info_array(group, "Content update group")?;

            let content_type = ContentType::try_from_int(as_u16(
                element(pair, 0, "Content update group")?,
                "Content update content type",
            )?)
            .ok_or_else(|| serialisation_error("Unknown content update content type!"))?;

            let actions = info_array(
                element(pair, 1, "Content update group")?,
                "Content update actions",
            )?;

            let mut actions_to_data = IndexMap::new();

            for action in actions {
                let action_pair = info_array(action, "Content update action")?;

                let action = ContentUpdate::try_from_int(as_u16(
                    element(&action_pair, 0, "Content update action")?,
                    "Content update action",
                )?)
                .ok_or_else(|| serialisation_error("Unknown content update action!"))?;

                let data = info_array(
                    element(&action_pair, 1, "Content update action")?,
                    "Content update action data",
                )?
                .clone();

                actions_to_data.insert(action, data);
            }

            self.content_data.insert(content_type, actions_to_data);
        }

        Ok(())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

// ---------------------------------------------------------------------------
// Credentials
// ---------------------------------------------------------------------------

/// `Credentials`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Credentials {
    host: Option<String>,
    port: Option<u16>,
    access_key: Option<Vec<u8>>,
}

impl Credentials {
    pub fn new(host: Option<String>, port: Option<u16>, access_key: Option<Vec<u8>>) -> Self {
        Self {
            host,
            port,
            access_key,
        }
    }

    pub fn get_access_key(&self) -> Option<&[u8]> {
        self.access_key.as_deref()
    }

    pub fn get_address(&self) -> (Option<&str>, Option<u16>) {
        (self.host.as_deref(), self.port)
    }

    pub fn get_connection_string(&self) -> String {
        let mut connection_string = String::new();

        if let Some(access_key) = &self.access_key {
            connection_string.push_str(&hex_encode(access_key));
            connection_string.push('@');
        }

        connection_string.push_str(self.host.as_deref().unwrap_or(""));
        connection_string.push(':');
        connection_string.push_str(&self.port.unwrap_or(0).to_string());

        connection_string
    }

    pub fn get_ported_address(&self) -> String {
        let host = self.host.as_deref().unwrap_or("");
        let stripped_host = host.strip_suffix('/').unwrap_or(host);
        let port = self.port.unwrap_or(0);

        if stripped_host.contains('/') {
            let (actual_host, gubbins) = host.split_once('/').unwrap_or((host, ""));

            format!("{actual_host}:{port}/{gubbins}")
        } else {
            format!("{host}:{port}")
        }
    }

    pub fn has_access_key(&self) -> bool {
        self.access_key.is_some()
    }

    pub fn set_access_key(&mut self, access_key: Option<Vec<u8>>) {
        self.access_key = access_key;
    }

    pub fn set_address(&mut self, host: impl Into<String>, port: u16) {
        self.host = Some(host.into());
        self.port = Some(port);
    }

    pub fn generate_credentials_from_connection_string(
        connection_string: Option<&str>,
    ) -> HydrusResult<Self> {
        let (host, port, access_key) = Self::parse_connection_string(connection_string)?;

        Ok(Self::new(Some(host), Some(port), access_key))
    }

    /// `parse_connection_string( connection_string )`.
    ///
    /// Python has an apparent bug here: after splitting the access key off it
    /// parses the *whole* string as `host:port`, leaving the key in the host.
    /// We parse the address half, which is what the function should do.
    pub fn parse_connection_string(
        connection_string: Option<&str>,
    ) -> HydrusResult<(String, u16, Option<Vec<u8>>)> {
        let Some(connection_string) = connection_string else {
            return Ok(("hostname".to_string(), 80, None));
        };

        let (address, access_key) = match connection_string.split_once('@') {
            Some((encoded_access_key, address)) => {
                if encoded_access_key.is_empty() {
                    (address, None)
                } else {
                    let access_key = hex_decode(encoded_access_key).map_err(|_| {
                        data_missing(
                            "Could not parse that access key! It should be a 64 character hexadecimal string!",
                        )
                    })?;

                    (address, if access_key.is_empty() { None } else { Some(access_key) })
                }
            }
            None => (connection_string, None),
        };

        let Some((host, port)) = address.split_once(':') else {
            return Err(data_missing(
                "Could not parse that port! It should be an integer between 0 and 65535!",
            ));
        };

        let port: i64 = port.parse().map_err(|_| {
            data_missing("Could not parse that port! It should be an integer between 0 and 65535!")
        })?;

        if !(0..=65535).contains(&port) {
            return Err(data_missing(
                "Could not parse that port! It should be an integer between 0 and 65535!",
            ));
        }

        let host = if host == "localhost" {
            "127.0.0.1".to_string()
        } else {
            host.to_string()
        };

        Ok((host, port as u16, access_key))
    }
}

impl SerialisableObject for Credentials {
    fn serialisable_type(&self) -> u16 {
        SERIALISABLE_TYPE_CREDENTIALS
    }

    fn serialisable_version(&self) -> u16 {
        1
    }

    fn serialisable_name(&self) -> &'static str {
        "Credentials"
    }

    fn get_serialisable_info(&self) -> Value {
        json!([
            self.host,
            self.port,
            self.access_key.as_ref().map(|key| hex_encode(key)),
        ])
    }

    fn initialise_from_serialisable_info(&mut self, info: &Value) -> HydrusResult<()> {
        let items = info_array(info, "Credentials info")?;

        let host = element(items, 0, "Credentials info")?;
        self.host = if host.is_null() {
            None
        } else {
            Some(as_str(host, "Credentials host")?.to_string())
        };

        let port = element(items, 1, "Credentials info")?;
        self.port = if port.is_null() {
            None
        } else {
            Some(as_u16(port, "Credentials port")?)
        };

        let access_key = element(items, 2, "Credentials info")?;
        self.access_key = if access_key.is_null() {
            None
        } else {
            Some(hex_bytes(access_key, "Credentials access key")?)
        };

        Ok(())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

// ---------------------------------------------------------------------------
// DefinitionsUpdate
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum DefinitionValue {
    Hash(Vec<u8>),
    Tag(String),
}

/// One row added to a [`DefinitionsUpdate`].
#[derive(Clone, Debug)]
pub struct DefinitionsUpdateRow {
    pub definitions_type: DefinitionsType,
    pub key: i64,
    pub value: DefinitionValue,
}

/// `DefinitionsUpdate`.
#[derive(Clone, Debug, Default)]
pub struct DefinitionsUpdate {
    hash_ids_to_hashes: IndexMap<i64, Vec<u8>>,
    tag_ids_to_tags: IndexMap<i64, String>,
}

impl DefinitionsUpdate {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_row(
        &mut self,
        definitions_type: DefinitionsType,
        key: i64,
        value: DefinitionValue,
    ) -> HydrusResult<()> {
        match (definitions_type, value) {
            (DefinitionsType::Hashes, DefinitionValue::Hash(hash)) => {
                self.hash_ids_to_hashes.insert(key, hash);
            }
            (DefinitionsType::Tags, DefinitionValue::Tag(tag)) => {
                self.tag_ids_to_tags.insert(key, tag);
            }
            _ => {
                return Err(HydrusException::new(
                    ErrorKind::Unknown,
                    "Definitions update row had a mismatched type and value!",
                ));
            }
        }

        Ok(())
    }

    pub fn get_hash_ids_to_hashes(&self) -> &IndexMap<i64, Vec<u8>> {
        &self.hash_ids_to_hashes
    }

    pub fn get_num_rows(&self) -> usize {
        self.hash_ids_to_hashes.len() + self.tag_ids_to_tags.len()
    }

    pub fn get_tag_ids_to_tags(&self) -> &IndexMap<i64, String> {
        &self.tag_ids_to_tags
    }
}

impl SerialisableObject for DefinitionsUpdate {
    fn serialisable_type(&self) -> u16 {
        SERIALISABLE_TYPE_DEFINITIONS_UPDATE
    }

    fn serialisable_version(&self) -> u16 {
        1
    }

    fn serialisable_name(&self) -> &'static str {
        "Definitions Update"
    }

    fn get_serialisable_info(&self) -> Value {
        let mut info = Vec::new();

        if !self.hash_ids_to_hashes.is_empty() {
            let hashes: Vec<Value> = self
                .hash_ids_to_hashes
                .iter()
                .map(|(hash_id, hash)| json!([hash_id, hex_encode(hash)]))
                .collect();

            info.push(json!([DefinitionsType::Hashes.as_int(), hashes]));
        }

        if !self.tag_ids_to_tags.is_empty() {
            let tags: Vec<Value> = self
                .tag_ids_to_tags
                .iter()
                .map(|(tag_id, tag)| json!([tag_id, tag]))
                .collect();

            info.push(json!([DefinitionsType::Tags.as_int(), tags]));
        }

        Value::Array(info)
    }

    fn initialise_from_serialisable_info(&mut self, info: &Value) -> HydrusResult<()> {
        let groups = info_array(info, "Definitions update info")?;

        self.hash_ids_to_hashes.clear();
        self.tag_ids_to_tags.clear();

        for group in groups {
            let pair = info_array(group, "Definitions update group")?;

            let definitions_type = DefinitionsType::try_from_int(as_u8(
                element(pair, 0, "Definitions update group")?,
                "Definitions update type",
            )?)
            .ok_or_else(|| serialisation_error("Unknown definitions update type!"))?;

            let definitions = info_array(
                element(pair, 1, "Definitions update group")?,
                "Definitions update rows",
            )?;

            for definition in definitions {
                let row = info_array(definition, "Definitions update row")?;

                let key = as_i64(
                    element(row, 0, "Definitions update row")?,
                    "Definitions update key",
                )?;
                let value = element(row, 1, "Definitions update row")?;

                match definitions_type {
                    DefinitionsType::Hashes => {
                        self.hash_ids_to_hashes
                            .insert(key, hex_bytes(value, "Definitions update hash")?);
                    }
                    DefinitionsType::Tags => {
                        self.tag_ids_to_tags
                            .insert(key, as_str(value, "Definitions update tag")?.to_string());
                    }
                }
            }
        }

        Ok(())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

// ---------------------------------------------------------------------------
// ClientToServerUpdate
// ---------------------------------------------------------------------------

/// `ClientToServerUpdate`.
#[derive(Clone, Debug, Default)]
pub struct ClientToServerUpdate {
    actions_to_contents_and_reasons: IndexMap<ContentUpdate, Vec<(Content, String)>>,
}

impl ClientToServerUpdate {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_content(
        &mut self,
        action: ContentUpdate,
        content: Content,
        reason: impl Into<String>,
    ) {
        self.actions_to_contents_and_reasons
            .entry(action)
            .or_default()
            .push((content, reason.into()));
    }

    pub fn apply_tag_filter_to_pending_mappings(&mut self, tag_filter: &TagFilter) {
        let Some(contents_and_reasons) = self
            .actions_to_contents_and_reasons
            .get_mut(&ContentUpdate::Pend)
        else {
            return;
        };

        contents_and_reasons.retain(|(content, _reason)| match content.mappings_data() {
            Some((tag, _hashes)) => tag_filter.tag_ok(tag, false),
            None => true,
        });
    }

    /// `get_content_data_iterator( content_type, action )`, eagerly collected.
    pub fn get_content_data_iterator(
        &self,
        content_type: ContentType,
        action: ContentUpdate,
    ) -> Vec<(&ContentData, &str)> {
        self.actions_to_contents_and_reasons
            .get(&action)
            .into_iter()
            .flatten()
            .filter(|(content, _reason)| content.get_content_type() == content_type)
            .map(|(content, reason)| (content.get_content_data(), reason.as_str()))
            .collect()
    }

    pub fn get_hashes(&self) -> HashSet<Vec<u8>> {
        let mut hashes = HashSet::new();

        for contents_and_reasons in self.actions_to_contents_and_reasons.values() {
            for (content, _reason) in contents_and_reasons {
                hashes.extend(content.get_hashes());
            }
        }

        hashes
    }

    pub fn has_content(&self) -> bool {
        !self.actions_to_contents_and_reasons.is_empty()
    }

    /// `iterate_all_actions_and_contents_and_reasons`, eagerly collected.
    pub fn iterate_all_actions_and_contents_and_reasons(
        &self,
    ) -> Vec<(ContentUpdate, &Content, &str)> {
        self.actions_to_contents_and_reasons
            .iter()
            .flat_map(|(action, contents_and_reasons)| {
                contents_and_reasons
                    .iter()
                    .map(move |(content, reason)| (*action, content, reason.as_str()))
            })
            .collect()
    }
}

impl SerialisableObject for ClientToServerUpdate {
    fn serialisable_type(&self) -> u16 {
        SERIALISABLE_TYPE_CLIENT_TO_SERVER_UPDATE
    }

    fn serialisable_version(&self) -> u16 {
        1
    }

    fn serialisable_name(&self) -> &'static str {
        "Client To Server Update"
    }

    fn get_serialisable_info(&self) -> Value {
        let info: Vec<Value> = self
            .actions_to_contents_and_reasons
            .iter()
            .map(|(action, contents_and_reasons)| {
                let serialisable_contents_and_reasons: Vec<Value> = contents_and_reasons
                    .iter()
                    .map(|(content, reason)| {
                        json!([content.get_serialisable_tuple(), reason])
                    })
                    .collect();

                json!([action.as_int(), serialisable_contents_and_reasons])
            })
            .collect();

        Value::Array(info)
    }

    fn initialise_from_serialisable_info(&mut self, info: &Value) -> HydrusResult<()> {
        let groups = info_array(info, "Client to server update info")?;

        self.actions_to_contents_and_reasons.clear();

        for group in groups {
            let pair = info_array(group, "Client to server update group")?;

            let action = ContentUpdate::try_from_int(as_u16(
                element(pair, 0, "Client to server update group")?,
                "Client to server update action",
            )?)
            .ok_or_else(|| serialisation_error("Unknown client to server update action!"))?;

            let contents_and_reasons = info_array(
                element(pair, 1, "Client to server update group")?,
                "Client to server update contents",
            )?;

            let mut parsed = Vec::with_capacity(contents_and_reasons.len());

            for item in contents_and_reasons {
                let item_pair = info_array(item, "Client to server update item")?;

                let content = object_from_tuple::<Content>(
                    element(&item_pair, 0, "Client to server update item")?,
                    false,
                    "Client to server update content",
                )?;
                let reason = as_str(
                    element(&item_pair, 1, "Client to server update item")?,
                    "Client to server update reason",
                )?
                .to_string();

                parsed.push((content, reason));
            }

            self.actions_to_contents_and_reasons.insert(action, parsed);
        }

        Ok(())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

// ---------------------------------------------------------------------------
// Metadata
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MetadataRow {
    pub update_hashes: Vec<Vec<u8>>,
    pub begin: i64,
    pub end: i64,
}

struct MetadataState {
    metadata: IndexMap<i64, MetadataRow>,
    next_update_due: i64,
    update_hashes: HashSet<Vec<u8>>,
    update_hashes_ordered: Vec<Vec<u8>>,
    biggest_end: Option<i64>,
}

impl MetadataState {
    fn new() -> Self {
        Self {
            metadata: IndexMap::new(),
            next_update_due: 0,
            update_hashes: HashSet::new(),
            update_hashes_ordered: Vec::new(),
            biggest_end: None,
        }
    }

    fn calculate_biggest_end(&self) -> Option<i64> {
        let biggest_index = self.metadata.keys().max()?;

        self.metadata.get(biggest_index).map(|row| row.end)
    }

    fn recalc_hashes(&mut self) {
        self.update_hashes = HashSet::new();
        self.update_hashes_ordered = Vec::new();

        let mut rows: Vec<(i64, &MetadataRow)> =
            self.metadata.iter().map(|(index, row)| (*index, row)).collect();

        rows.sort_by_key(|(index, _)| *index);

        for (_index, row) in rows {
            self.update_hashes.extend(row.update_hashes.iter().cloned());
            self.update_hashes_ordered
                .extend(row.update_hashes.iter().cloned());
        }
    }
}

fn next_update_due_time(next_update_due: i64, from_client: bool) -> i64 {
    let delay = if from_client {
        UPDATE_CHECKING_PERIOD * 2
    } else {
        10
    };

    next_update_due + delay
}

/// `Metadata`.
pub struct Metadata {
    state: Mutex<MetadataState>,
}

impl Default for Metadata {
    fn default() -> Self {
        Self::empty()
    }
}

impl Metadata {
    pub fn empty() -> Self {
        Self {
            state: Mutex::new(MetadataState::new()),
        }
    }

    pub fn from_rows(metadata: IndexMap<i64, MetadataRow>, next_update_due: i64) -> Self {
        let mut state = MetadataState::new();

        state.metadata = metadata;
        state.next_update_due = next_update_due;
        state.recalc_hashes();
        state.biggest_end = state.calculate_biggest_end();

        Self {
            state: Mutex::new(state),
        }
    }

    pub fn append_update(
        &self,
        update_hashes: Vec<Vec<u8>>,
        begin: i64,
        end: i64,
        next_update_due: i64,
    ) {
        let mut state = lock(&self.state);

        let update_index = state.metadata.len() as i64;

        state.update_hashes.extend(update_hashes.iter().cloned());
        state
            .update_hashes_ordered
            .extend(update_hashes.iter().cloned());

        state.metadata.insert(
            update_index,
            MetadataRow {
                update_hashes,
                begin,
                end,
            },
        );

        state.next_update_due = next_update_due;
        state.biggest_end = Some(end);
    }

    pub fn calculate_new_next_update_due(&self, update_period: i64) {
        let mut state = lock(&self.state);

        state.next_update_due = match state.biggest_end {
            None => 0,
            Some(biggest_end) => biggest_end + update_period,
        };
    }

    pub fn get_earliest_timestamp_for_these_hashes(&self, hashes: &[Vec<u8>]) -> i64 {
        let hashes: HashSet<Vec<u8>> = hashes.iter().cloned().collect();

        let state = lock(&self.state);

        let mut rows: Vec<(i64, &MetadataRow)> =
            state.metadata.iter().map(|(index, row)| (*index, row)).collect();

        rows.sort_by_key(|(index, _)| *index);

        for (_index, row) in rows {
            if row.update_hashes.iter().any(|hash| hashes.contains(hash)) {
                return row.end;
            }
        }

        0
    }

    pub fn get_next_update_index(&self) -> i64 {
        lock(&self.state).metadata.len() as i64
    }

    pub fn get_next_update_begin(&self) -> i64 {
        let state = lock(&self.state);

        match state.biggest_end {
            None => get_now(),
            Some(biggest_end) => biggest_end + 1,
        }
    }

    pub fn get_next_update_due_string(&self, from_client: bool) -> String {
        let state = lock(&self.state);

        if state.next_update_due == 0 {
            return "have not yet synced metadata".to_string();
        }

        let Some(biggest_end) = state.biggest_end else {
            return "the metadata appears to be uninitialised".to_string();
        };

        let update_due = next_update_due_time(state.next_update_due, from_client);

        let s = if time_has_passed(Some(update_due)) {
            "checking for updates imminently".to_string()
        } else {
            format!(
                "checking for updates {}",
                timestamp_to_pretty_time_delta(
                    Some(update_due),
                    &PrettyTimeDeltaOptions::default()
                )
            )
        };

        format!(
            "metadata synced up to {}, {}",
            timestamp_to_pretty_time_delta(Some(biggest_end), &PrettyTimeDeltaOptions::default()),
            s
        )
    }

    pub fn get_num_update_hashes(&self) -> usize {
        lock(&self.state).update_hashes.len()
    }

    pub fn get_slice(&self, from_update_index: i64) -> Self {
        let (metadata, next_update_due) = {
            let state = lock(&self.state);

            let metadata = state
                .metadata
                .iter()
                .filter(|(index, _row)| **index >= from_update_index)
                .map(|(index, row)| (*index, row.clone()))
                .collect::<IndexMap<_, _>>();

            (metadata, state.next_update_due)
        };

        Self::from_rows(metadata, next_update_due)
    }

    pub fn get_update_hashes(
        &self,
        update_index: Option<i64>,
    ) -> HydrusResult<HashSet<Vec<u8>>> {
        let state = lock(&self.state);

        match update_index {
            None => Ok(state.update_hashes.clone()),
            Some(update_index) => {
                let row = state.metadata.get(&update_index).ok_or_else(|| {
                    HydrusException::new(ErrorKind::DataMissing, "That update does not exist!")
                })?;

                Ok(row.update_hashes.iter().cloned().collect())
            }
        }
    }

    pub fn get_update_index_begin_and_end(
        &self,
        update_index: i64,
    ) -> HydrusResult<(i64, i64)> {
        let state = lock(&self.state);

        state
            .metadata
            .get(&update_index)
            .map(|row| (row.begin, row.end))
            .ok_or_else(|| {
                HydrusException::new(
                    ErrorKind::DataMissing,
                    "That update index does not seem to exist!",
                )
            })
    }

    pub fn get_update_indices_and_times(&self) -> Vec<(i64, i64, i64)> {
        let state = lock(&self.state);

        state
            .metadata
            .iter()
            .map(|(index, row)| (*index, row.begin, row.end))
            .collect()
    }

    pub fn get_update_indices_and_hashes(&self) -> Vec<(i64, Vec<Vec<u8>>)> {
        let state = lock(&self.state);

        state
            .metadata
            .iter()
            .map(|(index, row)| (*index, row.update_hashes.clone()))
            .collect()
    }

    pub fn has_done_initial_sync(&self) -> bool {
        lock(&self.state).next_update_due != 0
    }

    pub fn has_update_hash(&self, update_hash: &[u8]) -> bool {
        lock(&self.state).update_hashes.contains(update_hash)
    }

    pub fn sort_content_hashes_and_content_types<T>(
        &self,
        content_hashes_and_content_types: Vec<(Vec<u8>, T)>,
    ) -> Vec<(Vec<u8>, T)> {
        let state = lock(&self.state);

        let mut content_hashes_to_content_types: HashMap<Vec<u8>, T> =
            content_hashes_and_content_types.into_iter().collect();

        let mut result = Vec::new();

        for update_hash in &state.update_hashes_ordered {
            if let Some(content_type) = content_hashes_to_content_types.remove(update_hash.as_slice())
            {
                result.push((update_hash.clone(), content_type));
            }
        }

        result
    }

    pub fn update_asap(&self) {
        // Not 0; that is reserved for "never synced".
        lock(&self.state).next_update_due = 1;
    }

    pub fn update_due(&self, from_client: bool) -> bool {
        let state = lock(&self.state);

        time_has_passed(Some(next_update_due_time(
            state.next_update_due,
            from_client,
        )))
    }

    pub fn update_from_slice(&self, metadata_slice: &Metadata) {
        let mut state = lock(&self.state);

        let (rows, new_next_update_due) = {
            let slice_state = lock(&metadata_slice.state);

            (
                slice_state
                    .metadata
                    .iter()
                    .map(|(index, row)| (*index, row.clone()))
                    .collect::<Vec<_>>(),
                slice_state.next_update_due,
            )
        };

        for (index, row) in rows {
            state.metadata.insert(index, row);
        }

        let new_next_update_due = if time_has_passed(Some(new_next_update_due)) {
            get_now() + 100000
        } else {
            new_next_update_due
        };

        state.next_update_due = new_next_update_due;
        state.biggest_end = state.calculate_biggest_end();
        state.recalc_hashes();
    }

    pub fn update_is_empty(&self, update_index: i64) -> HydrusResult<bool> {
        Ok(self.get_update_hashes(Some(update_index))?.is_empty())
    }
}

impl SerialisableObject for Metadata {
    fn serialisable_type(&self) -> u16 {
        SERIALISABLE_TYPE_METADATA
    }

    fn serialisable_version(&self) -> u16 {
        1
    }

    fn serialisable_name(&self) -> &'static str {
        "Metadata"
    }

    fn get_serialisable_info(&self) -> Value {
        let state = lock(&self.state);

        let serialisable_metadata: Vec<Value> = state
            .metadata
            .iter()
            .map(|(update_index, row)| {
                json!([
                    update_index,
                    encode_hashes(&row.update_hashes),
                    row.begin,
                    row.end,
                ])
            })
            .collect();

        json!([serialisable_metadata, state.next_update_due])
    }

    fn initialise_from_serialisable_info(&mut self, info: &Value) -> HydrusResult<()> {
        let items = info_array(info, "Metadata info")?;

        let serialisable_metadata = info_array(
            element(items, 0, "Metadata info")?,
            "Metadata update rows",
        )?;

        let mut metadata = IndexMap::new();

        for row in serialisable_metadata {
            let row = info_array(row, "Metadata update row")?;

            let update_index = as_i64(
                element(&row, 0, "Metadata update row")?,
                "Metadata update index",
            )?;
            let update_hashes = decode_hashes(
                element(&row, 1, "Metadata update row")?,
                "Metadata update hashes",
            )?;
            let begin = as_i64(
                element(&row, 2, "Metadata update row")?,
                "Metadata update begin",
            )?;
            let end = as_i64(
                element(&row, 3, "Metadata update row")?,
                "Metadata update end",
            )?;

            metadata.insert(
                update_index,
                MetadataRow {
                    update_hashes,
                    begin,
                    end,
                },
            );
        }

        let next_update_due = as_i64(
            element(items, 1, "Metadata info")?,
            "Metadata next update due",
        )?;

        let mut state = lock(&self.state);

        state.metadata = metadata;
        state.next_update_due = next_update_due;
        state.recalc_hashes();
        state.biggest_end = state.calculate_biggest_end();

        Ok(())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

// ---------------------------------------------------------------------------
// PetitionHeader
// ---------------------------------------------------------------------------

/// `PetitionHeader`.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PetitionHeader {
    pub content_type: ContentType,
    pub status: ContentStatus,
    pub account_key: Vec<u8>,
    pub reason: String,
}

impl Default for PetitionHeader {
    fn default() -> Self {
        Self {
            content_type: ContentType::Mappings,
            status: ContentStatus::Petitioned,
            account_key: Vec::new(),
            reason: String::new(),
        }
    }
}

impl PetitionHeader {
    pub fn new(
        content_type: ContentType,
        status: ContentStatus,
        account_key: Vec<u8>,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            content_type,
            status,
            account_key,
            reason: reason.into(),
        }
    }
}

impl SerialisableObject for PetitionHeader {
    fn serialisable_type(&self) -> u16 {
        SERIALISABLE_TYPE_PETITION_HEADER
    }

    fn serialisable_version(&self) -> u16 {
        1
    }

    fn serialisable_name(&self) -> &'static str {
        "Petitions Header"
    }

    fn get_serialisable_info(&self) -> Value {
        json!([
            self.content_type.as_int(),
            self.status.as_int(),
            hex_encode(&self.account_key),
            self.reason,
        ])
    }

    fn initialise_from_serialisable_info(&mut self, info: &Value) -> HydrusResult<()> {
        let items = info_array(info, "Petition header info")?;

        self.content_type = ContentType::try_from_int(as_u16(
            element(items, 0, "Petition header info")?,
            "Petition header content type",
        )?)
        .ok_or_else(|| serialisation_error("Unknown petition header content type!"))?;

        self.status = ContentStatus::try_from_int(as_u8(
            element(items, 1, "Petition header info")?,
            "Petition header status",
        )?)
        .ok_or_else(|| serialisation_error("Unknown petition header status!"))?;

        self.account_key = hex_bytes(
            element(items, 2, "Petition header info")?,
            "Petition header account key",
        )?;

        self.reason = as_str(
            element(items, 3, "Petition header info")?,
            "Petition header reason",
        )?
        .to_string();

        Ok(())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

// ---------------------------------------------------------------------------
// Petition
// ---------------------------------------------------------------------------

fn serialisable_list_from_tuple(value: &Value, what: &str) -> HydrusResult<SerialisableList> {
    object_from_tuple::<SerialisableList>(value, false, what)
}

fn contents_from_serialisable_list(list: &SerialisableList) -> HydrusResult<Vec<Content>> {
    let mut contents = Vec::new();

    for value in list.as_slice() {
        if let Some(object) = value.as_object() {
            if let Some(content) = object.as_any().downcast_ref::<Content>() {
                contents.push(content.clone());

                continue;
            }
        }

        if let Some(json) = value.as_json() {
            contents.push(object_from_tuple::<Content>(
                json,
                false,
                "Petition content",
            )?);

            continue;
        }

        return Err(serialisation_error("Petition content was not a content object!"));
    }

    Ok(contents)
}

fn content_list_tuple(contents: &[Content]) -> Value {
    let mut list = SerialisableList::new();

    for content in contents {
        list.push(SerialisableValue::Object(Arc::new(content.clone())));
    }

    list.get_serialisable_tuple()
}

/// `Petition`.
pub struct Petition {
    petitioner_account: Arc<Account>,
    petition_header: PetitionHeader,
    actions_and_contents: Vec<(ContentUpdate, Vec<Content>)>,
    completed_actions_to_contents: IndexMap<ContentUpdate, Vec<Content>>,
}

impl Default for Petition {
    fn default() -> Self {
        Self {
            petitioner_account: Arc::new(Account::generate_unknown_account(Vec::new())),
            petition_header: PetitionHeader::default(),
            actions_and_contents: Vec::new(),
            completed_actions_to_contents: IndexMap::new(),
        }
    }
}

impl PartialEq for Petition {
    fn eq(&self, other: &Self) -> bool {
        self.petition_header == other.petition_header
    }
}

impl Eq for Petition {}

impl std::hash::Hash for Petition {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.petition_header.hash(state);
    }
}

impl Petition {
    pub fn new(
        petitioner_account: Arc<Account>,
        petition_header: PetitionHeader,
        actions_and_contents: Vec<(ContentUpdate, Vec<Content>)>,
    ) -> Self {
        Self {
            petitioner_account,
            petition_header,
            actions_and_contents,
            completed_actions_to_contents: IndexMap::new(),
        }
    }

    pub fn approve(&mut self, action: ContentUpdate, content: Content) {
        self.completed_actions_to_contents
            .entry(action)
            .or_default()
            .push(content);
    }

    pub fn approve_all(&mut self) {
        let actions_and_contents = self.actions_and_contents.clone();

        for (action, contents) in actions_and_contents {
            for content in contents {
                self.approve(action, content);
            }
        }
    }

    pub fn deny(&mut self, action: ContentUpdate, content: Content) -> HydrusResult<()> {
        let denial_action = match action {
            ContentUpdate::Pend => ContentUpdate::DenyPend,
            ContentUpdate::Petition => ContentUpdate::DenyPetition,
            _ => {
                return Err(HydrusException::new(
                    ErrorKind::Unknown,
                    format!("Petition came with unexpected action: {action}"),
                ));
            }
        };

        self.completed_actions_to_contents
            .entry(denial_action)
            .or_default()
            .push(content);

        Ok(())
    }

    pub fn deny_all(&mut self) -> HydrusResult<()> {
        let actions_and_contents = self.actions_and_contents.clone();

        for (action, contents) in actions_and_contents {
            for content in contents {
                self.deny(action, content)?;
            }
        }

        Ok(())
    }

    pub fn get_completed_uploadable_client_to_server_updates(&self) -> Vec<ClientToServerUpdate> {
        fn break_contents_into_chunks(contents: &[Content]) -> Vec<Vec<Content>> {
            let mut chunks_of_contents = Vec::new();
            let mut chunk_of_contents = Vec::new();
            let mut weight_of_current_chunk = 0;

            for content in contents {
                for content_chunk in content.iterate_uploadable_chunks() {
                    chunk_of_contents.push(content_chunk);

                    weight_of_current_chunk += content.get_virtual_weight();

                    if weight_of_current_chunk > 50 {
                        chunks_of_contents.push(std::mem::take(&mut chunk_of_contents));
                        weight_of_current_chunk = 0;
                    }
                }
            }

            if !chunk_of_contents.is_empty() {
                chunks_of_contents.push(chunk_of_contents);
            }

            chunks_of_contents
        }

        let mut updates = Vec::new();

        // Make sure we delete before we add.
        for action in [
            ContentUpdate::DenyPetition,
            ContentUpdate::DenyPend,
            ContentUpdate::Petition,
            ContentUpdate::Pend,
        ] {
            let Some(contents) = self.completed_actions_to_contents.get(&action) else {
                continue;
            };

            if contents.is_empty() {
                continue;
            }

            for chunk_of_contents in break_contents_into_chunks(contents) {
                let mut update = ClientToServerUpdate::new();

                for content in chunk_of_contents {
                    update.add_content(action, content, self.petition_header.reason.clone());
                }

                updates.push(update);
            }
        }

        updates
    }

    pub fn get_contents(&self, action: ContentUpdate) -> &[Content] {
        self.actions_and_contents
            .iter()
            .rev()
            .find(|(candidate, _contents)| *candidate == action)
            .map(|(_action, contents)| contents.as_slice())
            .unwrap_or(&[])
    }

    pub fn get_actions_and_contents(&self) -> &[(ContentUpdate, Vec<Content>)] {
        &self.actions_and_contents
    }

    pub fn get_petitioner_account(&self) -> &Arc<Account> {
        &self.petitioner_account
    }

    pub fn get_petition_header(&self) -> &PetitionHeader {
        &self.petition_header
    }

    pub fn get_reason(&self) -> &str {
        &self.petition_header.reason
    }

    pub fn get_actual_content_weight(&self) -> usize {
        self.actions_and_contents
            .iter()
            .flat_map(|(_action, contents)| contents.iter())
            .map(Content::get_actual_weight)
            .sum()
    }

    pub fn get_content_summary(&self) -> String {
        let num_sub_petitions: usize = self
            .actions_and_contents
            .iter()
            .map(|(_action, contents)| contents.len())
            .sum();

        if self.petition_header.content_type == ContentType::Mappings && num_sub_petitions > 1 {
            format!(
                "{} mappings in {} petitions",
                base_to_human_int(self.get_actual_content_weight() as i128),
                base_to_human_int(num_sub_petitions as i128)
            )
        } else {
            format!(
                "{} {}",
                base_to_human_int(self.get_actual_content_weight() as i128),
                self.petition_header.content_type.as_str()
            )
        }
    }
}

impl SerialisableObject for Petition {
    fn serialisable_type(&self) -> u16 {
        SERIALISABLE_TYPE_PETITION
    }

    fn serialisable_version(&self) -> u16 {
        3
    }

    fn serialisable_name(&self) -> &'static str {
        "Petition"
    }

    fn get_serialisable_info(&self) -> Value {
        let serialisable_petitioner_account =
            Account::generate_serialisable_tuple_from_account(&self.petitioner_account)
                .unwrap_or(Value::Null);

        let serialisable_actions_and_contents: Vec<Value> = self
            .actions_and_contents
            .iter()
            .map(|(action, contents)| json!([action.as_int(), content_list_tuple(contents)]))
            .collect();

        json!([
            serialisable_petitioner_account,
            self.petition_header.get_serialisable_tuple(),
            serialisable_actions_and_contents,
        ])
    }

    fn initialise_from_serialisable_info(&mut self, info: &Value) -> HydrusResult<()> {
        let items = info_array(info, "Petition info")?;

        let petitioner_account = Account::generate_account_from_serialisable_tuple(element(
            items,
            0,
            "Petition info",
        )?)?;

        let petition_header = object_from_tuple::<PetitionHeader>(
            element(items, 1, "Petition info")?,
            false,
            "Petition header",
        )?;

        let serialisable_actions_and_contents = info_array(
            element(items, 2, "Petition info")?,
            "Petition actions and contents",
        )?;

        let mut actions_and_contents = Vec::new();

        for group in serialisable_actions_and_contents {
            let pair = info_array(group, "Petition action and contents")?;

            let action = ContentUpdate::try_from_int(as_u16(
                element(&pair, 0, "Petition action and contents")?,
                "Petition action",
            )?)
            .ok_or_else(|| serialisation_error("Unknown petition action!"))?;

            let list = serialisable_list_from_tuple(
                element(&pair, 1, "Petition action and contents")?,
                "Petition contents",
            )?;

            actions_and_contents.push((action, contents_from_serialisable_list(&list)?));
        }

        self.petitioner_account = Arc::new(petitioner_account);
        self.petition_header = petition_header;
        self.actions_and_contents = actions_and_contents;
        self.completed_actions_to_contents = IndexMap::new();

        Ok(())
    }

    fn update_serialisable_info(
        &self,
        version: u16,
        old_info: Value,
    ) -> HydrusResult<(u16, Value)> {
        if version == 1 {
            let items = info_array(&old_info, "Petition v1 info")?;

            let action = ContentUpdate::try_from_int(as_u16(
                element(&items, 0, "Petition v1 info")?,
                "Petition v1 action",
            )?)
            .ok_or_else(|| serialisation_error("Unknown petition v1 action!"))?;

            let account_info = element(&items, 1, "Petition v1 info")?.clone();
            let reason = element(&items, 2, "Petition v1 info")?.clone();
            let serialisable_contents = info_array(
                element(&items, 3, "Petition v1 info")?,
                "Petition v1 contents",
            )?;

            let mut contents = Vec::new();

            for content_info in serialisable_contents {
                contents.push(object_from_tuple::<Content>(
                    content_info,
                    false,
                    "Petition v1 content",
                )?);
            }

            let new_info = json!([
                account_info,
                reason,
                [[action.as_int(), content_list_tuple(&contents)]],
            ]);

            return Ok((2, new_info));
        }

        if version == 2 {
            // Python never implemented this step (it raises), but a v2 petition
            // has enough information to build the v3 header.
            let items = info_array(&old_info, "Petition v2 info")?;

            let account_info = element(&items, 0, "Petition v2 info")?.clone();
            let reason = as_str(element(&items, 1, "Petition v2 info")?, "Petition v2 reason")?
                .to_string();
            let actions = element(&items, 2, "Petition v2 info")?.clone();

            let account = Account::generate_account_from_serialisable_tuple(&account_info)?;

            let mut content_type = ContentType::Mappings;

            if let Some(groups) = actions.as_array() {
                if let Some(first) = groups.first().and_then(Value::as_array) {
                    if let Some(contents_info) = first.get(1) {
                        if let Ok(list) =
                            serialisable_list_from_tuple(contents_info, "Petition v2 contents")
                        {
                            if let Some(first_content) = contents_from_serialisable_list(&list)
                                .ok()
                                .and_then(|contents| contents.first().cloned())
                            {
                                content_type = first_content.get_content_type();
                            }
                        }
                    }
                }
            }

            let header = PetitionHeader::new(
                content_type,
                ContentStatus::Petitioned,
                account.get_account_key(),
                reason,
            );

            return Ok((3, json!([account_info, header.get_serialisable_tuple(), actions])));
        }

        Ok((version, old_info))
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

// ---------------------------------------------------------------------------
// UpdateBuilder
// ---------------------------------------------------------------------------

pub trait UpdateAccumulator: Default {
    type Row;

    fn push_row(&mut self, row: Self::Row);

    fn num_rows(&self) -> usize;
}

impl UpdateAccumulator for ContentUpdatePackage {
    type Row = ContentUpdateRow;

    fn push_row(&mut self, row: Self::Row) {
        self.add_row(row.content_type, row.action, row.data);
    }

    fn num_rows(&self) -> usize {
        self.get_num_rows(None)
    }
}

impl UpdateAccumulator for DefinitionsUpdate {
    type Row = DefinitionsUpdateRow;

    fn push_row(&mut self, row: Self::Row) {
        // The row already carries a matching type/value pair; a mismatch can
        // only happen if a caller constructs one by hand. Ignore it here so
        // `UpdateBuilder` mirrors Python's unchecked `add_row`.
        let _ = self.add_row(row.definitions_type, row.key, row.value);
    }

    fn num_rows(&self) -> usize {
        self.get_num_rows()
    }
}

/// `UpdateBuilder`.
pub struct UpdateBuilder<T: UpdateAccumulator> {
    max_rows: usize,
    updates: Vec<T>,
    current_update: T,
    current_num_rows: usize,
}

impl<T: UpdateAccumulator> UpdateBuilder<T> {
    pub fn new(max_rows: usize) -> Self {
        Self {
            max_rows,
            updates: Vec::new(),
            current_update: T::default(),
            current_num_rows: 0,
        }
    }

    pub fn add_row(&mut self, row: T::Row, row_weight: usize) {
        self.current_update.push_row(row);
        self.current_num_rows += row_weight;

        if self.current_num_rows > self.max_rows {
            self.updates
                .push(std::mem::take(&mut self.current_update));
            self.current_num_rows = 0;
        }
    }

    pub fn finish(&mut self) {
        if self.current_update.num_rows() > 0 {
            self.updates
                .push(std::mem::take(&mut self.current_update));
        }

        self.current_update = T::default();
    }

    pub fn get_updates(&self) -> &[T] {
        &self.updates
    }

    pub fn into_updates(mut self) -> Vec<T> {
        self.finish();

        self.updates
    }
}

// ---------------------------------------------------------------------------
// Registration
// ---------------------------------------------------------------------------

fn ensure_registered() {
    static REGISTERED: OnceLock<()> = OnceLock::new();

    REGISTERED.get_or_init(|| {
        register_serialisable_type(SERIALISABLE_TYPE_ACCOUNT_IDENTIFIER, || {
            Box::new(AccountIdentifier::default())
        });
        register_serialisable_type(SERIALISABLE_TYPE_ACCOUNT_TYPE, || {
            Box::new(AccountType::default())
        });
        register_serialisable_type(SERIALISABLE_TYPE_CLIENT_TO_SERVER_UPDATE, || {
            Box::new(ClientToServerUpdate::default())
        });
        register_serialisable_type(SERIALISABLE_TYPE_CONTENT, || Box::new(Content::default()));
        register_serialisable_type(SERIALISABLE_TYPE_CONTENT_UPDATE, || {
            Box::new(ContentUpdatePackage::default())
        });
        register_serialisable_type(SERIALISABLE_TYPE_CREDENTIALS, || {
            Box::new(Credentials::default())
        });
        register_serialisable_type(SERIALISABLE_TYPE_DEFINITIONS_UPDATE, || {
            Box::new(DefinitionsUpdate::default())
        });
        register_serialisable_type(SERIALISABLE_TYPE_METADATA, || Box::new(Metadata::default()));
        register_serialisable_type(SERIALISABLE_TYPE_PETITION, || Box::new(Petition::default()));
        register_serialisable_type(SERIALISABLE_TYPE_PETITION_HEADER, || {
            Box::new(PetitionHeader::default())
        });
    });
}

/// Register every type in this module. Idempotent.
pub fn register_serialisable_types() {
    ensure_registered();
}

// ---------------------------------------------------------------------------
// Server service controller injection
// ---------------------------------------------------------------------------

/// A held `HG.server_busy` lock. Dropping it releases the lock.
pub trait ServerBusyGuard: Send {}

/// The handful of controller calls `ServerServiceRepository` makes.
///
/// Python reaches these through the global `HG.controller`; Rust injects them
/// once during server boot.
pub trait ServerServiceController: Send + Sync {
    fn try_lock_server_busy(&self) -> Option<Box<dyn ServerBusyGuard>>;

    fn nullify_history(&self, service_key: &[u8], begin: i64, end: i64) -> HydrusResult<()>;

    fn create_update(
        &self,
        service_key: &[u8],
        begin: i64,
        end: i64,
    ) -> HydrusResult<Vec<Vec<u8>>>;

    fn publish(&self, topic: &str);
}

fn controller_slot() -> &'static RwLock<Option<Arc<dyn ServerServiceController>>> {
    static CONTROLLER: OnceLock<RwLock<Option<Arc<dyn ServerServiceController>>>> = OnceLock::new();

    CONTROLLER.get_or_init(|| RwLock::new(None))
}

pub fn set_server_service_controller(controller: Option<Arc<dyn ServerServiceController>>) {
    *controller_slot()
        .write()
        .unwrap_or_else(|error| error.into_inner()) = controller;
}

pub fn server_service_controller() -> Option<Arc<dyn ServerServiceController>> {
    controller_slot()
        .read()
        .unwrap_or_else(|error| error.into_inner())
        .clone()
}

fn publish(topic: &str) {
    if let Some(controller) = server_service_controller() {
        controller.publish(topic);
    }
}

// ---------------------------------------------------------------------------
// Service factories
// ---------------------------------------------------------------------------

/// `get_possible_permissions( service_type )`.
pub fn get_possible_permissions(
    service_type: ServiceType,
) -> Vec<(ContentType, Vec<Option<PermissionAction>>)> {
    let mut permissions = vec![
        (
            ContentType::Accounts,
            vec![
                None,
                Some(PermissionAction::Create),
                Some(PermissionAction::Moderate),
            ],
        ),
        (
            ContentType::AccountTypes,
            vec![None, Some(PermissionAction::Moderate)],
        ),
        (
            ContentType::Options,
            vec![None, Some(PermissionAction::Moderate)],
        ),
    ];

    if service_type == ServiceType::FileRepository {
        permissions.push((
            ContentType::Files,
            vec![
                None,
                Some(PermissionAction::Petition),
                Some(PermissionAction::Create),
                Some(PermissionAction::Moderate),
            ],
        ));
    } else if service_type == ServiceType::TagRepository {
        permissions.push((
            ContentType::Mappings,
            vec![
                None,
                Some(PermissionAction::Petition),
                Some(PermissionAction::Create),
                Some(PermissionAction::Moderate),
            ],
        ));
        permissions.push((
            ContentType::TagParents,
            vec![
                None,
                Some(PermissionAction::Petition),
                Some(PermissionAction::Moderate),
            ],
        ));
        permissions.push((
            ContentType::TagSiblings,
            vec![
                None,
                Some(PermissionAction::Petition),
                Some(PermissionAction::Moderate),
            ],
        ));
    } else if service_type == ServiceType::ServerAdmin {
        permissions.push((
            ContentType::Services,
            vec![None, Some(PermissionAction::Moderate)],
        ));
    }

    permissions
}

/// `generate_default_service_dictionary( service_type )`.
pub fn generate_default_service_dictionary(
    service_type: ServiceType,
) -> SerialisableDictionary {
    ensure_registered();

    let mut dictionary = SerialisableDictionary::new();

    dict_insert(
        &mut dictionary,
        "bandwidth_tracker",
        SerialisableValue::Object(Arc::new(BandwidthTracker::new())),
    );

    if RESTRICTED_SERVICES.contains(&service_type) {
        dict_insert(
            &mut dictionary,
            "bandwidth_rules",
            SerialisableValue::Object(Arc::new(BandwidthRules::new())),
        );

        let mut service_options = SerialisableDictionary::new();

        dict_insert(
            &mut service_options,
            "server_message",
            "Welcome to the server!",
        );

        if REPOSITORIES.contains(&service_type) {
            let update_period = 100000;

            dict_insert(&mut service_options, "update_period", update_period);
            dict_insert(
                &mut service_options,
                "nullification_period",
                90 * 86400,
            );

            dict_insert(
                &mut dictionary,
                "next_nullification_update_index",
                0_i64,
            );

            let metadata = Metadata::default();
            let now = get_now();

            metadata.append_update(Vec::new(), 0, now, now + update_period);

            dict_insert(
                &mut dictionary,
                "metadata",
                SerialisableValue::Object(Arc::new(metadata)),
            );

            if service_type == ServiceType::FileRepository {
                dict_insert(&mut dictionary, "log_uploader_ips", false);
                dict_insert(&mut dictionary, "max_storage", Value::Null);
            }

            if service_type == ServiceType::TagRepository {
                dict_insert(
                    &mut service_options,
                    "tag_filter",
                    SerialisableValue::Object(Arc::new(TagFilter::new())),
                );
            }
        }

        dict_insert(
            &mut dictionary,
            "service_options",
            SerialisableValue::Object(Arc::new(service_options)),
        );

        if service_type == ServiceType::ServerAdmin {
            dict_insert(
                &mut dictionary,
                "server_bandwidth_tracker",
                SerialisableValue::Object(Arc::new(BandwidthTracker::new())),
            );
            dict_insert(
                &mut dictionary,
                "server_bandwidth_rules",
                SerialisableValue::Object(Arc::new(BandwidthRules::new())),
            );
        }
    }

    dictionary
}

/// `generate_service( service_key, service_type, name, port, dictionary )`.
pub fn generate_service(
    service_key: Vec<u8>,
    service_type: ServiceType,
    name: impl Into<String>,
    port: u16,
    dictionary: Option<SerialisableDictionary>,
) -> HydrusResult<ServerService> {
    ensure_registered();

    let dictionary =
        dictionary.unwrap_or_else(|| generate_default_service_dictionary(service_type));

    ServerService::new(service_key, service_type, name.into(), port, &dictionary)
}

/// `generate_service_from_serialisable_tuple( serialisable_info )`.
pub fn generate_service_from_serialisable_tuple(info: &Value) -> HydrusResult<ServerService> {
    let items = info_array(info, "Service serialisable tuple")?;

    let service_key = hex_bytes(
        element(items, 0, "Service serialisable tuple")?,
        "Service key",
    )
    .map_err(|_| {
        HydrusException::new(ErrorKind::BadRequest, "Could not decode that service key!")
    })?;

    let service_type = ServiceType::try_from_int(as_u16(
        element(items, 1, "Service serialisable tuple")?,
        "Service type",
    )?)
    .ok_or_else(|| serialisation_error("Unknown service type!"))?;

    let name = as_str(
        element(items, 2, "Service serialisable tuple")?,
        "Service name",
    )?
    .to_string();

    let port = as_i64(
        element(items, 3, "Service serialisable tuple")?,
        "Service port",
    )?;

    if !(0..=65535).contains(&port) {
        return Err(serialisation_error("Service port was out of range!"));
    }

    let dictionary_string = as_str(
        element(items, 4, "Service serialisable tuple")?,
        "Service dictionary",
    )?;
    let dictionary = dictionary_from_string(dictionary_string)?;

    generate_service(service_key, service_type, name, port as u16, Some(dictionary))
}

// ---------------------------------------------------------------------------
// Server services
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ServiceKind {
    Basic,
    Restricted,
    RepositoryTag,
    RepositoryFile,
    Admin,
}

impl ServiceKind {
    fn for_service_type(service_type: ServiceType) -> Self {
        match service_type {
            ServiceType::ServerAdmin => Self::Admin,
            ServiceType::TagRepository => Self::RepositoryTag,
            ServiceType::FileRepository => Self::RepositoryFile,
            service_type if RESTRICTED_SERVICES.contains(&service_type) => Self::Restricted,
            _ => Self::Basic,
        }
    }

    fn is_restricted(self) -> bool {
        !matches!(self, Self::Basic)
    }

    fn is_repository(self) -> bool {
        matches!(self, Self::RepositoryTag | Self::RepositoryFile)
    }
}

struct ServerServiceState {
    name: String,
    port: u16,
    bandwidth_tracker: Arc<BandwidthTracker>,
    bandwidth_rules: Option<Arc<BandwidthRules>>,
    service_options: Option<SerialisableDictionary>,
    metadata: Option<Arc<Metadata>>,
    next_nullification_update_index: i64,
    log_uploader_ips: bool,
    max_storage: Option<i64>,
    server_bandwidth_tracker: Option<Arc<BandwidthTracker>>,
    server_bandwidth_rules: Option<Arc<BandwidthRules>>,
    dirty: bool,
}

impl ServerServiceState {
    fn service_option_i64(&self, key: &str) -> Option<i64> {
        self.service_options
            .as_ref()
            .and_then(|options| dict_get_i64(options, key))
    }
}

/// `ServerService` and its restricted/repository/admin forms.
pub struct ServerService {
    service_key: Vec<u8>,
    service_type: ServiceType,
    kind: ServiceKind,
    state: Mutex<ServerServiceState>,
}

fn load_service_state(
    kind: ServiceKind,
    state: &mut ServerServiceState,
    dictionary: &SerialisableDictionary,
) -> HydrusResult<()> {
    let tracker_value = dict_get(dictionary, "bandwidth_tracker").ok_or_else(|| {
        serialisation_error("Service dictionary had no bandwidth tracker!")
    })?;

    state.bandwidth_tracker = clone_object_from_slot::<BandwidthTracker>(
        tracker_value,
        SERIALISABLE_TYPE_BANDWIDTH_TRACKER,
        "Service bandwidth tracker",
    )?;

    if kind.is_restricted() {
        let mut service_options = match dict_get(dictionary, "service_options") {
            Some(value) => dictionary_from_slot(value)?,
            None => SerialisableDictionary::new(),
        };

        if !dict_contains(&service_options, "server_message") {
            dict_insert(&mut service_options, "server_message", "");
        }

        state.service_options = Some(service_options);

        let rules_value = dict_get(dictionary, "bandwidth_rules").ok_or_else(|| {
            serialisation_error("Restricted service dictionary had no bandwidth rules!")
        })?;

        state.bandwidth_rules = Some(clone_object_from_slot::<BandwidthRules>(
            rules_value,
            SERIALISABLE_TYPE_BANDWIDTH_RULES,
            "Service bandwidth rules",
        )?);
    }

    if kind.is_repository() {
        let default_nullification_period =
            dict_get_i64(dictionary, "nullification_period").unwrap_or(90 * 86400);

        {
            let options = state.service_options.as_mut().ok_or_else(|| {
                serialisation_error("Repository service had no service options!")
            })?;

            if !dict_contains(options, "update_period") {
                dict_insert(options, "update_period", 100000_i64);
            }

            if !dict_contains(options, "nullification_period") {
                dict_insert(options, "nullification_period", default_nullification_period);
            }
        }

        state.next_nullification_update_index =
            dict_get_i64(dictionary, "next_nullification_update_index").unwrap_or(0);

        state.metadata = Some(match dict_get(dictionary, "metadata") {
            Some(value) => clone_object_from_slot::<Metadata>(
                value,
                SERIALISABLE_TYPE_METADATA,
                "Repository metadata",
            )?,
            None => Arc::new(Metadata::default()),
        });
    }

    if kind == ServiceKind::RepositoryTag {
        let options = state
            .service_options
            .as_mut()
            .ok_or_else(|| serialisation_error("Tag repository had no service options!"))?;

        if !dict_contains(options, "tag_filter") {
            dict_insert(
                options,
                "tag_filter",
                SerialisableValue::Object(Arc::new(TagFilter::new())),
            );
        }
    }

    if kind == ServiceKind::RepositoryFile {
        state.log_uploader_ips = dict_get_bool(dictionary, "log_uploader_ips").unwrap_or(false);
        state.max_storage = dict_get(dictionary, "max_storage")
            .and_then(SerialisableValue::as_json)
            .and_then(Value::as_i64);
    }

    if kind == ServiceKind::Admin {
        let tracker_value = dict_get(dictionary, "server_bandwidth_tracker")
            .ok_or_else(|| serialisation_error("Admin service had no server bandwidth tracker!"))?;
        state.server_bandwidth_tracker = Some(clone_object_from_slot::<BandwidthTracker>(
            tracker_value,
            SERIALISABLE_TYPE_BANDWIDTH_TRACKER,
            "Server bandwidth tracker",
        )?);

        let rules_value = dict_get(dictionary, "server_bandwidth_rules")
            .ok_or_else(|| serialisation_error("Admin service had no server bandwidth rules!"))?;
        state.server_bandwidth_rules = Some(clone_object_from_slot::<BandwidthRules>(
            rules_value,
            SERIALISABLE_TYPE_BANDWIDTH_RULES,
            "Server bandwidth rules",
        )?);
    }

    Ok(())
}

impl ServerService {
    pub fn new(
        service_key: Vec<u8>,
        service_type: ServiceType,
        name: String,
        port: u16,
        dictionary: &SerialisableDictionary,
    ) -> HydrusResult<Self> {
        ensure_registered();

        let kind = ServiceKind::for_service_type(service_type);

        let mut state = ServerServiceState {
            name,
            port,
            bandwidth_tracker: Arc::new(BandwidthTracker::new()),
            bandwidth_rules: None,
            service_options: None,
            metadata: None,
            next_nullification_update_index: 0,
            log_uploader_ips: false,
            max_storage: None,
            server_bandwidth_tracker: None,
            server_bandwidth_rules: None,
            dirty: false,
        };

        load_service_state(kind, &mut state, dictionary)?;

        Ok(Self {
            service_key,
            service_type,
            kind,
            state: Mutex::new(state),
        })
    }

    fn serialisable_dictionary(&self) -> SerialisableDictionary {
        let state = lock(&self.state);

        let mut dictionary = SerialisableDictionary::new();

        dict_insert(
            &mut dictionary,
            "bandwidth_tracker",
            SerialisableValue::Object(state.bandwidth_tracker.clone()),
        );

        if self.kind.is_restricted() {
            if let Some(bandwidth_rules) = &state.bandwidth_rules {
                dict_insert(
                    &mut dictionary,
                    "bandwidth_rules",
                    SerialisableValue::Object(bandwidth_rules.clone()),
                );
            }

            if let Some(service_options) = &state.service_options {
                dict_insert(
                    &mut dictionary,
                    "service_options",
                    SerialisableValue::Object(Arc::new(service_options.clone())),
                );

                dict_insert(
                    &mut dictionary,
                    "server_message",
                    dict_get_string(service_options, "server_message").unwrap_or_default(),
                );
            }
        }

        if self.kind.is_repository() {
            if let Some(metadata) = &state.metadata {
                dict_insert(
                    &mut dictionary,
                    "metadata",
                    SerialisableValue::Object(metadata.clone()),
                );
            }

            dict_insert(
                &mut dictionary,
                "next_nullification_update_index",
                state.next_nullification_update_index,
            );
        }

        if self.kind == ServiceKind::RepositoryFile {
            dict_insert(&mut dictionary, "log_uploader_ips", state.log_uploader_ips);
            dict_insert(
                &mut dictionary,
                "max_storage",
                match state.max_storage {
                    Some(max_storage) => json!(max_storage),
                    None => Value::Null,
                },
            );
        }

        if self.kind == ServiceKind::Admin {
            if let Some(tracker) = &state.server_bandwidth_tracker {
                dict_insert(
                    &mut dictionary,
                    "server_bandwidth_tracker",
                    SerialisableValue::Object(tracker.clone()),
                );
            }

            if let Some(rules) = &state.server_bandwidth_rules {
                dict_insert(
                    &mut dictionary,
                    "server_bandwidth_rules",
                    SerialisableValue::Object(rules.clone()),
                );
            }
        }

        dictionary
    }

    fn set_dirty(&self) {
        lock(&self.state).dirty = true;
    }

    pub fn allows_non_local_connections(&self) -> bool {
        true
    }

    pub fn bandwidth_ok(&self) -> bool {
        let state = lock(&self.state);

        match &state.bandwidth_rules {
            Some(bandwidth_rules) => {
                bandwidth_rules.can_start_request_default(&state.bandwidth_tracker)
            }
            None => true,
        }
    }

    pub fn duplicate(&self) -> HydrusResult<Self> {
        let dictionary = duplicate_dictionary(&self.serialisable_dictionary())?;

        let (name, port) = {
            let state = lock(&self.state);

            (state.name.clone(), state.port)
        };

        generate_service(
            self.service_key.clone(),
            self.service_type,
            name,
            port,
            Some(dictionary),
        )
    }

    pub fn get_name(&self) -> String {
        lock(&self.state).name.clone()
    }

    pub fn get_port(&self) -> u16 {
        lock(&self.state).port
    }

    pub fn get_service_key(&self) -> &[u8] {
        &self.service_key
    }

    pub fn get_service_type(&self) -> ServiceType {
        self.service_type
    }

    pub fn is_dirty(&self) -> bool {
        lock(&self.state).dirty
    }

    pub fn logs_requests(&self) -> bool {
        true
    }

    pub fn report_data_used(&self, num_bytes: i64) {
        let mut state = lock(&self.state);

        state.bandwidth_tracker.report_data_used(num_bytes);
        state.dirty = true;
    }

    pub fn report_request_used(&self) {
        let mut state = lock(&self.state);

        state.bandwidth_tracker.report_request_used(1);
        state.dirty = true;
    }

    pub fn set_clean(&self) {
        lock(&self.state).dirty = false;
    }

    pub fn set_name(&self, name: impl Into<String>) {
        let mut state = lock(&self.state);

        state.name = name.into();
        state.dirty = true;
    }

    pub fn set_port(&self, port: u16) {
        let mut state = lock(&self.state);

        state.port = port;
        state.dirty = true;
    }

    pub fn supports_cors(&self) -> bool {
        false
    }

    pub fn to_serialisable_tuple(&self) -> Value {
        let (name, port) = {
            let state = lock(&self.state);

            (state.name.clone(), state.port)
        };

        let dictionary_string = self.serialisable_dictionary().dump_to_string();

        json!([
            hex_encode(&self.service_key),
            self.service_type.as_int(),
            name,
            port,
            dictionary_string,
        ])
    }

    pub fn to_tuple(
        &self,
    ) -> HydrusResult<(Vec<u8>, ServiceType, String, u16, SerialisableDictionary)> {
        let (name, port) = {
            let state = lock(&self.state);

            (state.name.clone(), state.port)
        };

        let dictionary = duplicate_dictionary(&self.serialisable_dictionary())?;

        Ok((self.service_key.clone(), self.service_type, name, port, dictionary))
    }

    pub fn use_normie_eris(&self) -> bool {
        false
    }

    fn repository_metadata(&self) -> HydrusResult<Arc<Metadata>> {
        lock(&self.state)
            .metadata
            .clone()
            .ok_or_else(|| HydrusException::new(ErrorKind::Unknown, "This service has no metadata!"))
    }

    pub fn get_metadata(&self) -> HydrusResult<Arc<Metadata>> {
        self.repository_metadata()
    }

    pub fn get_metadata_slice(&self, from_update_index: i64) -> HydrusResult<Metadata> {
        Ok(self.repository_metadata()?.get_slice(from_update_index))
    }

    pub fn get_nullification_period(&self) -> i64 {
        lock(&self.state)
            .service_option_i64("nullification_period")
            .unwrap_or(90 * 86400)
    }

    pub fn get_update_period(&self) -> i64 {
        lock(&self.state)
            .service_option_i64("update_period")
            .unwrap_or(100000)
    }

    pub fn has_update_hash(&self, update_hash: &[u8]) -> HydrusResult<bool> {
        Ok(self.repository_metadata()?.has_update_hash(update_hash))
    }

    pub fn nullify_history(&self) -> HydrusResult<()> {
        const MAX_WAIT_TIME_WHEN_HEAVY_UPDATES: f64 = 120.0;

        let time_started_nullifying = get_now();
        let time_to_stop_nullifying = time_started_nullifying + 3600;

        while !hydrus_globals::started_shutdown() {
            let metadata = self.repository_metadata()?;

            let next_update_index = metadata.get_next_update_index();

            let nullification_update_index = {
                let state = lock(&self.state);

                // We are caught up on a server with update times longer than
                // nullification_period.
                if state.next_nullification_update_index >= next_update_index {
                    return Ok(());
                }

                state.next_nullification_update_index
            };

            let (nullification_begin, nullification_end) =
                metadata.get_update_index_begin_and_end(nullification_update_index)?;

            let nullification_period = self.get_nullification_period();

            // It isn't time to do the next one yet!
            if !time_has_passed(Some(nullification_end + nullification_period)) {
                return Ok(());
            }

            let is_empty = metadata.update_is_empty(nullification_update_index)?;

            if is_empty {
                safe_print(format!(
                    "Account history for \"{}\" update {} was empty, so nothing to anonymise.",
                    self.get_name(),
                    nullification_update_index
                ));

                let mut state = lock(&self.state);

                state.next_nullification_update_index += 1;
                state.dirty = true;

                continue;
            }

            let Some(controller) = server_service_controller() else {
                return Err(HydrusException::new(
                    ErrorKind::Unknown,
                    "No server service controller is set!",
                ));
            };

            let Some(_busy_guard) = controller.try_lock_server_busy() else {
                return Ok(());
            };

            safe_print(format!(
                "Nullifying account history for \"{}\" update {}.",
                self.get_name(),
                nullification_update_index
            ));

            let update_started = get_now_float();

            controller.nullify_history(
                &self.service_key,
                nullification_begin,
                nullification_end,
            )?;

            let update_took = get_now_float() - update_started;

            {
                let mut state = lock(&self.state);

                safe_print(format!(
                    "Account history for \"{}\" update {} was anonymised in {}.",
                    state.name,
                    nullification_update_index,
                    time_delta_to_pretty_timedelta(Some(update_took), true, false)
                ));

                state.next_nullification_update_index += 1;
                state.dirty = true;
            }

            if time_has_passed(Some(time_to_stop_nullifying)) {
                return Ok(());
            }

            if update_took < 0.5 {
                continue;
            }

            let time_to_wait = update_took.min(MAX_WAIT_TIME_WHEN_HEAVY_UPDATES);

            let resume_timestamp = get_now_float() + time_to_wait;

            while !hydrus_globals::started_shutdown()
                && !crate::hydrus_time::time_has_passed_float(resume_timestamp)
            {
                std::thread::sleep(std::time::Duration::from_secs(1));
            }
        }

        Ok(())
    }

    pub fn set_nullification_period(&self, nullification_period: i64) {
        {
            let mut state = lock(&self.state);

            if let Some(options) = state.service_options.as_mut() {
                dict_insert(options, "nullification_period", nullification_period);
            }

            state.dirty = true;
        }

        publish("notify_new_nullification");
    }

    pub fn set_update_period(&self, update_period: i64) -> HydrusResult<()> {
        {
            let mut state = lock(&self.state);

            if let Some(options) = state.service_options.as_mut() {
                dict_insert(options, "update_period", update_period);
            }

            if let Some(metadata) = &state.metadata {
                metadata.calculate_new_next_update_due(update_period);
            }

            state.dirty = true;
        }

        publish("notify_new_repo_sync");

        Ok(())
    }

    pub fn sync(&self) -> HydrusResult<()> {
        let metadata = self.repository_metadata()?;

        let mut update_due = metadata.update_due(false);

        if !update_due {
            return Ok(());
        }

        let Some(controller) = server_service_controller() else {
            return Err(HydrusException::new(
                ErrorKind::Unknown,
                "No server service controller is set!",
            ));
        };

        let Some(_busy_guard) = controller.try_lock_server_busy() else {
            return Ok(());
        };

        let mut update_created = false;

        while update_due {
            let begin = metadata.get_next_update_begin();

            let update_period = self.get_update_period();

            let end = begin + update_period;

            let update_hashes = controller.create_update(&self.service_key, begin, end)?;

            update_created = true;

            let next_update_due = end + update_period;

            metadata.append_update(update_hashes, begin, end, next_update_due);

            update_due = metadata.update_due(false);
        }

        if update_created {
            controller.publish("notify_update_created");
        }

        self.set_dirty();

        Ok(())
    }

    pub fn get_tag_filter(&self) -> TagFilter {
        let state = lock(&self.state);

        let Some(options) = &state.service_options else {
            return TagFilter::new();
        };

        let Some(value) = dict_get(options, "tag_filter") else {
            return TagFilter::new();
        };

        let Some(object) = value.as_object() else {
            return TagFilter::new();
        };

        object
            .as_any()
            .downcast_ref::<TagFilter>()
            .cloned()
            .unwrap_or_else(TagFilter::new)
    }

    pub fn set_tag_filter(&self, tag_filter: TagFilter) {
        let mut state = lock(&self.state);

        if let Some(options) = state.service_options.as_mut() {
            dict_insert(
                options,
                "tag_filter",
                SerialisableValue::Object(Arc::new(tag_filter)),
            );
        }

        state.dirty = true;
    }

    pub fn log_uploader_ips(&self) -> bool {
        lock(&self.state).log_uploader_ips
    }

    pub fn get_max_storage(&self) -> Option<i64> {
        lock(&self.state).max_storage
    }

    pub fn server_bandwidth_ok(&self) -> bool {
        let state = lock(&self.state);

        match (&state.server_bandwidth_rules, &state.server_bandwidth_tracker) {
            (Some(rules), Some(tracker)) => rules.can_start_request_default(tracker),
            _ => true,
        }
    }

    pub fn server_report_data_used(&self, num_bytes: i64) {
        let mut state = lock(&self.state);

        if let Some(tracker) = &state.server_bandwidth_tracker {
            tracker.report_data_used(num_bytes);
        }

        state.dirty = true;
    }

    pub fn server_report_request_used(&self) {
        let mut state = lock(&self.state);

        if let Some(tracker) = &state.server_bandwidth_tracker {
            tracker.report_request_used(1);
        }

        state.dirty = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hydrus_constants::{ContentStatus, DefinitionsType, Filter};

    fn permissions(
        entries: &[(ContentType, Option<PermissionAction>)],
    ) -> IndexMap<ContentType, Option<PermissionAction>> {
        entries.iter().copied().collect()
    }

    fn account_type_with(entries: &[(ContentType, Option<PermissionAction>)]) -> Arc<AccountType> {
        Arc::new(AccountType::generate_new_account_type(
            "test user",
            permissions(entries),
            Arc::new(BandwidthRules::new()),
        ))
    }

    fn roundtrip<T>(object: &T) -> T
    where
        T: SerialisableObject + Default,
    {
        register_serialisable_types();

        let mut restored = T::default();

        restored
            .initialise_from_serialisable_info(&object.get_serialisable_info())
            .expect("serialisable round-trip should succeed");

        restored
    }

    // -----------------------------------------------------------------------
    // Service dictionaries and ServerService
    // -----------------------------------------------------------------------

    #[test]
    fn default_service_dictionaries_generate_working_services() {
        register_serialisable_types();

        let file_dictionary =
            generate_default_service_dictionary(ServiceType::FileRepository);

        let service = generate_service(
            vec![1, 2, 3],
            ServiceType::FileRepository,
            "my file repo",
            45871,
            Some(file_dictionary),
        )
        .unwrap();

        assert_eq!(service.get_name(), "my file repo");
        assert_eq!(service.get_port(), 45871);
        assert_eq!(service.get_service_key(), &[1, 2, 3]);
        assert_eq!(service.get_service_type(), ServiceType::FileRepository);
        assert_eq!(service.get_update_period(), 100000);
        assert_eq!(service.get_nullification_period(), 90 * 86400);
        assert!(!service.log_uploader_ips());
        assert_eq!(service.get_max_storage(), None);
        assert!(!service.supports_cors());

        let metadata = service.get_metadata().unwrap();
        assert_eq!(metadata.get_next_update_index(), 1);
        assert!(metadata.has_done_initial_sync());
        assert!(service.has_update_hash(&[]).unwrap() == false);

        let tag_dictionary =
            generate_default_service_dictionary(ServiceType::TagRepository);

        let tag_service = generate_service(
            vec![4, 5, 6],
            ServiceType::TagRepository,
            "my tag repo",
            45872,
            Some(tag_dictionary),
        )
        .unwrap();

        assert!(tag_service.get_tag_filter().tag_ok("anything", false));

        let admin_service = generate_service(
            vec![7, 8, 9],
            ServiceType::ServerAdmin,
            "my admin",
            45873,
            None,
        )
        .unwrap();

        assert!(admin_service.allows_non_local_connections());
        assert!(admin_service.server_bandwidth_ok());
        assert!(admin_service.bandwidth_ok());
    }

    #[test]
    fn service_round_trips_through_tuple_and_serialisable_tuple() {
        register_serialisable_types();

        let service = generate_service(
            vec![10, 11, 12],
            ServiceType::FileRepository,
            "round trip repo",
            12345,
            None,
        )
        .unwrap();

        service.set_name("renamed repo");
        service.set_port(23456);

        let tuple = service.to_tuple().unwrap();

        assert_eq!(tuple.0, vec![10, 11, 12]);
        assert_eq!(tuple.1, ServiceType::FileRepository);
        assert_eq!(tuple.2, "renamed repo");
        assert_eq!(tuple.3, 23456);

        let rebuilt = generate_service(tuple.0, tuple.1, tuple.2, tuple.3, Some(tuple.4)).unwrap();

        assert_eq!(rebuilt.get_name(), "renamed repo");
        assert_eq!(rebuilt.get_port(), 23456);
        assert_eq!(rebuilt.get_update_period(), 100000);

        let serialisable = service.to_serialisable_tuple();

        let from_tuple = generate_service_from_serialisable_tuple(&serialisable).unwrap();

        assert_eq!(from_tuple.get_name(), "renamed repo");
        assert_eq!(from_tuple.get_port(), 23456);

        let duplicate = service.duplicate().unwrap();

        assert_eq!(duplicate.get_name(), "renamed repo");
        assert_eq!(duplicate.get_port(), 23456);
        assert_eq!(duplicate.get_service_key(), &[10, 11, 12]);
    }

    #[test]
    fn service_dirty_and_bandwidth_reporting_work() {
        let service = generate_service(
            vec![13],
            ServiceType::TagRepository,
            "dirty repo",
            1234,
            None,
        )
        .unwrap();

        assert!(!service.is_dirty());

        service.report_request_used();
        service.report_data_used(1024);

        assert!(service.is_dirty());

        service.set_clean();

        assert!(!service.is_dirty());

        service.set_update_period(200000).unwrap();

        assert_eq!(service.get_update_period(), 200000);
        assert!(service.is_dirty());

        service.set_nullification_period(86400);

        assert_eq!(service.get_nullification_period(), 86400);
    }

    // -----------------------------------------------------------------------
    // AccountType
    // -----------------------------------------------------------------------

    #[test]
    fn account_type_permissions_follow_the_permission_ladder() {
        let account_type = account_type_with(&[(ContentType::Files, Some(PermissionAction::Create))]);

        assert!(account_type.has_permission(ContentType::Files, PermissionAction::Petition));
        assert!(account_type.has_permission(ContentType::Files, PermissionAction::Create));
        assert!(!account_type.has_permission(ContentType::Files, PermissionAction::Moderate));
        assert!(!account_type.has_permission(ContentType::Mappings, PermissionAction::Petition));
        assert!(!account_type.is_null_account());

        let moderate = account_type_with(&[(ContentType::Files, Some(PermissionAction::Moderate))]);

        assert!(moderate.has_permission(ContentType::Files, PermissionAction::Petition));
        assert!(moderate.has_permission(ContentType::Files, PermissionAction::Create));
        assert!(moderate.has_permission(ContentType::Files, PermissionAction::Moderate));

        let none = account_type_with(&[(ContentType::Files, None)]);

        assert!(!none.has_permission(ContentType::Files, PermissionAction::Petition));
    }

    #[test]
    fn null_and_unknown_account_types_behave_as_expected() {
        let null_type = AccountType::generate_null_account_type();

        assert!(null_type.is_null_account());
        assert!(!null_type.has_permission(ContentType::Files, PermissionAction::Petition));
        assert_eq!(
            null_type.get_permission_strings(),
            vec!["is null account, cannot do anything".to_string()]
        );
        assert!(!null_type.supports_auto_create_account());

        let unknown = AccountType::generate_unknown_account_type();

        assert!(!unknown.is_null_account());
        assert_eq!(unknown.get_title(), "unknown account");
        assert!(!unknown.bandwidth_ok(&BandwidthTracker::new()));
    }

    #[test]
    fn admin_account_types_have_the_expected_permissions() {
        let repo_admin =
            AccountType::generate_admin_account_type(ServiceType::FileRepository).unwrap();

        assert!(repo_admin.has_permission(ContentType::Accounts, PermissionAction::Moderate));
        assert!(repo_admin.has_permission(ContentType::AccountTypes, PermissionAction::Moderate));
        assert!(repo_admin.has_permission(ContentType::Options, PermissionAction::Moderate));
        assert!(repo_admin.has_permission(ContentType::Files, PermissionAction::Moderate));
        assert!(!repo_admin.is_null_account());

        let admin_admin =
            AccountType::generate_admin_account_type(ServiceType::ServerAdmin).unwrap();

        assert!(admin_admin.has_permission(ContentType::Services, PermissionAction::Moderate));

        let error = match AccountType::generate_admin_account_type(ServiceType::LocalFileDomain) {
            Ok(_) => panic!("non-server service types should not get an admin account type"),
            Err(error) => error,
        };

        assert_eq!(error.kind, ErrorKind::Unknown);
    }

    #[test]
    fn account_type_serialises_and_updates_from_v1() {
        register_serialisable_types();

        let account_type = AccountType::generate_new_account_type(
            "custom",
            permissions(&[
                (ContentType::Files, Some(PermissionAction::Petition)),
                (ContentType::Mappings, None),
            ]),
            Arc::new(BandwidthRules::new()),
        );

        let restored = roundtrip(&account_type);

        assert_eq!(restored.get_account_type_key(), account_type.get_account_type_key());
        assert_eq!(restored.get_title(), "custom");
        assert_eq!(restored.get_permissions(), account_type.get_permissions());
        assert!(!restored.is_null_account());

        let info = account_type.get_serialisable_info();
        let items = info.as_array().unwrap();

        let v1_info = json!([
            items[0].clone(),
            items[1].clone(),
            [
                [ContentType::AccountTypes.as_int(), PermissionAction::Moderate.as_int()],
                [ContentType::Files.as_int(), PermissionAction::Petition.as_int()],
            ],
            items[3].clone(),
            items[4].clone(),
            items[5].clone(),
        ]);

        let (version, updated) = account_type.update_serialisable_info(1, v1_info).unwrap();

        assert_eq!(version, 2);

        let mut restored_v1 = AccountType::default();
        restored_v1.initialise_from_serialisable_info(&updated).unwrap();

        assert!(restored_v1.has_permission(ContentType::Options, PermissionAction::Moderate));
        assert!(restored_v1.has_permission(
            ContentType::AccountTypes,
            PermissionAction::Moderate
        ));
        assert!(restored_v1.has_permission(ContentType::Files, PermissionAction::Petition));
    }

    #[test]
    fn null_account_type_round_trips_through_serialisation() {
        let null_type = AccountType::generate_null_account_type();

        let restored = roundtrip(&null_type);

        assert!(restored.is_null_account());
        assert_eq!(restored.get_title(), "null account");
    }

    // -----------------------------------------------------------------------
    // Account
    // -----------------------------------------------------------------------

    #[test]
    fn admin_account_can_do_anything_when_functional() {
        let admin_type =
            Arc::new(AccountType::generate_admin_account_type(ServiceType::ServerAdmin).unwrap());

        let account = Account::new(vec![1; 32], admin_type, get_now(), None);

        assert!(!account.is_unknown());
        assert!(!account.is_banned());
        assert!(!account.is_expired());
        assert!(account.is_functional());
        assert!(account.check_functional().is_ok());

        assert!(account.has_permission(ContentType::Mappings, PermissionAction::Moderate));
        assert!(account.check_permission(ContentType::Mappings, PermissionAction::Moderate).is_ok());
        assert!(account.check_at_least_one_permission(&[(
            ContentType::Services,
            PermissionAction::Moderate
        )])
        .is_ok());
    }

    #[test]
    fn accounts_validate_bans_and_expiry() {
        let account = Account::new(
            vec![2; 32],
            account_type_with(&[(ContentType::Files, Some(PermissionAction::Petition))]),
            get_now(),
            None,
        );

        assert!(account.is_functional());

        account.ban("being a menace", get_now(), None);

        assert!(account.is_banned());
        assert!(!account.is_functional());
        assert!(!account.has_permission(ContentType::Files, PermissionAction::Petition));

        let error = account.check_functional().unwrap_err();
        assert_eq!(error.kind, ErrorKind::InsufficientCredentials);

        let error = account
            .check_permission(ContentType::Files, PermissionAction::Petition)
            .unwrap_err();
        assert_eq!(error.kind, ErrorKind::InsufficientCredentials);

        account.unban();

        assert!(!account.is_banned());
        assert!(account.is_functional());

        account.set_expires(Some(get_now() - 10));

        assert!(account.is_expired());
        assert!(!account.is_functional());
        assert!(!account.has_permission(ContentType::Files, PermissionAction::Petition));

        let error = account.check_functional().unwrap_err();
        assert_eq!(error.kind, ErrorKind::InsufficientCredentials);

        account.set_expires(Some(get_now() + 10000));

        assert!(!account.is_expired());
        assert!(account.is_functional());
    }

    #[test]
    fn expired_bans_are_lifted_automatically() {
        let account = Account::new(
            vec![3; 32],
            account_type_with(&[(ContentType::Files, Some(PermissionAction::Petition))]),
            get_now(),
            None,
        );

        account.ban("old ban", get_now() - 100, Some(get_now() - 10));

        assert!(!account.is_banned());
        assert!(account.get_banned_info().is_none());
        assert!(account.is_functional());
    }

    #[test]
    fn unknown_and_bandwidth_limited_accounts_are_not_functional() {
        let unknown = Account::generate_unknown_account(vec![4; 32]);

        assert!(unknown.is_unknown());
        assert!(!unknown.is_functional());

        let error = unknown.check_functional().unwrap_err();
        assert_eq!(error.kind, ErrorKind::Conflict);

        let limited_type =
            Arc::new(AccountType::generate_unknown_account_type());

        let limited = Account::new(vec![5; 32], limited_type, get_now(), None);

        let error = limited.check_functional().unwrap_err();
        assert_eq!(error.kind, ErrorKind::Bandwidth);
    }

    #[test]
    fn account_messages_and_titles_work() {
        let account = Account::new(
            vec![0xab; 32],
            account_type_with(&[(ContentType::Files, Some(PermissionAction::Petition))]),
            get_now(),
            None,
        );

        account.set_message("hello there", 123);

        assert_eq!(
            account.get_message_and_timestamp(),
            ("hello there".to_string(), 123)
        );

        assert!(account.is_dirty());
        account.set_clean();
        assert!(!account.is_dirty());

        account.report_data_used(100);
        account.report_request_used();

        assert!(account.is_dirty());

        let title = account.get_single_line_title();

        assert!(title.contains("test user"));
        assert!(title.contains(&hex_encode(&[0xab; 32])));
    }

    #[test]
    fn accounts_round_trip_through_serialisation() {
        register_serialisable_types();

        let account = Account::new(
            vec![6; 32],
            account_type_with(&[
                (ContentType::Files, Some(PermissionAction::Create)),
                (ContentType::Mappings, Some(PermissionAction::Petition)),
            ]),
            get_now() - 100,
            Some(get_now() + 100000),
        );

        account.ban("rule breaking", get_now() - 50, None);
        account.set_message("be nice", 42);
        account.report_data_used(4096);

        let tuple = Account::generate_serialisable_tuple_from_account(&account).unwrap();

        let restored = Account::generate_account_from_serialisable_tuple(&tuple).unwrap();

        assert_eq!(restored.get_account_key(), account.get_account_key());
        assert_eq!(restored.get_created(), account.get_created());
        assert_eq!(restored.get_expires(), account.get_expires());
        assert_eq!(restored.get_message_and_timestamp(), ("be nice".to_string(), 42));
        assert_eq!(restored.get_banned_info(), account.get_banned_info());
        assert!(restored.is_banned());
        assert_eq!(
            restored.get_bandwidth_tracker().get_all_usage(BandwidthType::Data),
            account
                .get_bandwidth_tracker()
                .get_all_usage(BandwidthType::Data)
        );
        assert!(!restored.is_dirty());
    }

    #[test]
    fn legacy_account_types_and_accounts_convert() {
        register_serialisable_types();

        let key = vec![0xee; 32];

        let mut dictionary = SerialisableDictionary::new();

        dict_insert(
            &mut dictionary,
            "permissions",
            json!([
                [ContentType::Files.as_int(), PermissionAction::Create.as_int()],
                [ContentType::Mappings.as_int(), Value::Null],
            ]),
        );
        dict_insert(
            &mut dictionary,
            "bandwidth_rules",
            SerialisableValue::Object(Arc::new(BandwidthRules::new())),
        );

        let dictionary_string = dictionary.dump_to_string();

        let account_type =
            convert_legacy_account_type(key.clone(), "legacy user", &dictionary_string).unwrap();

        assert_eq!(account_type.get_account_type_key(), key);
        assert_eq!(account_type.get_title(), "legacy user");
        assert!(account_type.has_permission(ContentType::Files, PermissionAction::Create));
        assert!(!account_type.has_permission(ContentType::Mappings, PermissionAction::Petition));

        // Older account tuples embedded the account type as the legacy triple.
        let legacy_type_info = json!([
            hex_encode(&key),
            "legacy user",
            dictionary_string,
        ]);

        let empty_dictionary = SerialisableDictionary::new();

        let account_info = json!([
            hex_encode(&vec![0xdd; 32]),
            legacy_type_info,
            get_now(),
            Value::Null,
            empty_dictionary.dump_to_string(),
        ]);

        let account = Account::generate_account_from_serialisable_tuple(&account_info).unwrap();

        assert_eq!(account.get_account_key(), vec![0xdd; 32]);
        assert_eq!(account.get_account_type().get_title(), "legacy user");
        assert!(account.has_permission(ContentType::Files, PermissionAction::Create));
    }

    // -----------------------------------------------------------------------
    // Content
    // -----------------------------------------------------------------------

    #[test]
    fn content_variants_report_their_data_and_weights() {
        let hash_a = vec![1, 2, 3];
        let hash_b = vec![4, 5, 6];

        let files = Content::files(vec![hash_a.clone(), hash_b.clone()]);

        assert_eq!(files.get_content_type(), ContentType::Files);
        assert_eq!(files.get_hashes(), vec![hash_a.clone(), hash_b.clone()]);
        assert_eq!(files.get_actual_weight(), 2);
        assert_eq!(files.get_virtual_weight(), 2);
        assert!(files.has_hashes());

        let mapping = Content::mapping("blue eyes", hash_a.clone());

        assert_eq!(mapping.get_content_type(), ContentType::Mapping);
        assert_eq!(mapping.get_hashes(), vec![hash_a.clone()]);
        assert_eq!(mapping.get_actual_weight(), 1);
        assert_eq!(mapping.get_virtual_weight(), 1);

        let mappings = Content::mappings("blue eyes", vec![hash_a.clone(), hash_b.clone()]);

        assert_eq!(mappings.get_content_type(), ContentType::Mappings);
        assert_eq!(mappings.get_actual_weight(), 2);
        assert_eq!(mappings.get_virtual_weight(), 2);
        assert_eq!(
            mappings.mappings_data(),
            Some(("blue eyes", &[hash_a.clone(), hash_b.clone()][..]))
        );

        let parents = Content::tag_parents("blue eyes", "eyes");

        assert_eq!(parents.get_content_type(), ContentType::TagParents);
        assert_eq!(parents.get_actual_weight(), 1);
        assert_eq!(parents.get_virtual_weight(), 5000);
        assert!(!parents.has_hashes());

        let siblings = Content::tag_siblings("blue eyes", "eyes");

        assert_eq!(siblings.get_virtual_weight(), 5);
        assert!(siblings.to_string().contains("SIBLING"));
    }

    #[test]
    fn content_splits_into_uploadable_chunks() {
        let hashes: Vec<Vec<u8>> = (0..250).map(|i| vec![i as u8]).collect();

        let chunks = Content::files(hashes).iterate_uploadable_chunks();

        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0].get_hashes().len(), 100);
        assert_eq!(chunks[1].get_hashes().len(), 100);
        assert_eq!(chunks[2].get_hashes().len(), 50);

        let hashes: Vec<Vec<u8>> = (0..1200).map(|i| vec![i as u8]).collect();

        let chunks = Content::mappings("tag", hashes).iterate_uploadable_chunks();

        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0].get_hashes().len(), 500);
        assert_eq!(chunks[1].get_hashes().len(), 500);
        assert_eq!(chunks[2].get_hashes().len(), 200);

        let mapping = Content::mapping("tag", vec![1]);

        assert_eq!(mapping.iterate_uploadable_chunks(), vec![mapping.clone()]);
    }

    #[test]
    fn content_round_trips_through_serialisation() {
        let hash_a = vec![7; 32];
        let hash_b = vec![8; 32];

        for content in [
            Content::files(vec![hash_a.clone(), hash_b.clone()]),
            Content::mapping("blue eyes", hash_a.clone()),
            Content::mappings("blue eyes", vec![hash_a.clone(), hash_b.clone()]),
            Content::tag_parents("blue eyes", "eyes"),
            Content::tag_siblings("blue eyes", "eyes"),
        ] {
            let restored = roundtrip(&content);

            assert_eq!(restored, content);
        }
    }

    // -----------------------------------------------------------------------
    // AccountIdentifier
    // -----------------------------------------------------------------------

    #[test]
    fn account_identifier_holds_one_of_two_data_types() {
        let key_identifier = AccountIdentifier::new_account_key(vec![9; 32]);

        assert!(key_identifier.has_account_key());
        assert!(!key_identifier.has_content());
        assert_eq!(key_identifier.get_account_key().unwrap(), &[9; 32]);
        assert_eq!(key_identifier.get_content().unwrap_err().kind, ErrorKind::Unknown);

        let content = Content::mapping("blue eyes", vec![10; 32]);
        let content_identifier = AccountIdentifier::new_content(content.clone());

        assert!(content_identifier.has_content());
        assert!(!content_identifier.has_account_key());
        assert_eq!(content_identifier.get_content().unwrap(), &content);
        assert_eq!(
            content_identifier.get_account_key().unwrap_err().kind,
            ErrorKind::Unknown
        );

        assert_eq!(roundtrip(&key_identifier), key_identifier);
        assert_eq!(roundtrip(&content_identifier), content_identifier);
    }

    // -----------------------------------------------------------------------
    // ContentUpdatePackage
    // -----------------------------------------------------------------------

    #[test]
    fn content_update_package_counts_and_serves_rows() {
        let mut update = ContentUpdatePackage::new();

        assert_eq!(update.get_num_rows(None), 0);

        update.add_row(ContentType::Files, ContentUpdate::Add, json!("hash_a"));
        update.add_row(ContentType::Files, ContentUpdate::Delete, json!("hash_b"));
        update.add_row(
            ContentType::Mappings,
            ContentUpdate::Add,
            json!(["blue eyes", ["hash_c", "hash_d"]]),
        );
        update.add_row(
            ContentType::Mappings,
            ContentUpdate::Delete,
            json!(["blue eyes", ["hash_e"]]),
        );

        assert_eq!(update.get_new_files(), &[json!("hash_a")]);
        assert_eq!(update.get_deleted_files(), &[json!("hash_b")]);
        assert_eq!(
            update.get_new_mappings(),
            &[json!(["blue eyes", ["hash_c", "hash_d"]])]
        );
        assert_eq!(
            update.get_deleted_mappings(),
            &[json!(["blue eyes", ["hash_e"]])]
        );

        // Mappings count by the number of hashes they carry.
        assert_eq!(update.get_num_rows(None), 1 + 1 + 2 + 1);
        assert_eq!(update.get_num_rows(Some(&[ContentType::Files])), 2);
        assert_eq!(update.get_num_rows(Some(&[ContentType::Mappings])), 3);

        let restored = roundtrip(&update);

        assert_eq!(restored.get_num_rows(None), 5);
        assert_eq!(restored.get_new_files(), update.get_new_files());
        assert_eq!(restored.get_deleted_mappings(), update.get_deleted_mappings());
    }

    // -----------------------------------------------------------------------
    // Credentials
    // -----------------------------------------------------------------------

    #[test]
    fn credentials_parse_connection_strings() {
        assert_eq!(
            Credentials::parse_connection_string(None).unwrap(),
            ("hostname".to_string(), 80, None)
        );

        assert_eq!(
            Credentials::parse_connection_string(Some("127.0.0.1:45871")).unwrap(),
            ("127.0.0.1".to_string(), 45871, None)
        );

        assert_eq!(
            Credentials::parse_connection_string(Some("localhost:8080")).unwrap(),
            ("127.0.0.1".to_string(), 8080, None)
        );

        let key = vec![0xab; 32];

        let (host, port, access_key) = Credentials::parse_connection_string(Some(&format!(
            "{}@example.com:1234",
            hex_encode(&key)
        )))
        .unwrap();

        assert_eq!(host, "example.com");
        assert_eq!(port, 1234);
        assert_eq!(access_key, Some(key));

        assert!(Credentials::parse_connection_string(Some("no-port")).is_err());
        assert!(Credentials::parse_connection_string(Some("host:notaport")).is_err());
        assert!(Credentials::parse_connection_string(Some("host:70000")).is_err());
    }

    #[test]
    fn credentials_build_and_round_trip_connection_strings() {
        let key = vec![0xcd; 32];

        let mut credentials = Credentials::new(Some("example.com".to_string()), Some(1234), None);

        assert!(!credentials.has_access_key());
        assert_eq!(credentials.get_connection_string(), "example.com:1234");
        assert_eq!(credentials.get_ported_address(), "example.com:1234");

        credentials.set_access_key(Some(key.clone()));

        assert!(credentials.has_access_key());
        assert_eq!(
            credentials.get_connection_string(),
            format!("{}@example.com:1234", hex_encode(&key))
        );

        let parsed =
            Credentials::generate_credentials_from_connection_string(Some(&format!(
                "{}@example.com:1234",
                hex_encode(&key)
            )))
            .unwrap();

        assert_eq!(parsed, credentials);

        let mut path_credentials =
            Credentials::new(Some("example.com/sub/path/".to_string()), Some(1234), None);

        assert_eq!(
            path_credentials.get_ported_address(),
            "example.com:1234/sub/path/"
        );

        path_credentials.set_address("other.example", 4321);

        assert_eq!(
            path_credentials.get_address(),
            (Some("other.example"), Some(4321))
        );

        assert_eq!(roundtrip(&credentials), credentials);

        let empty = Credentials::new(None, None, None);

        assert_eq!(empty.get_connection_string(), ":0");
        assert_eq!(empty.get_ported_address(), ":0");
    }

    // -----------------------------------------------------------------------
    // DefinitionsUpdate
    // -----------------------------------------------------------------------

    #[test]
    fn definitions_update_collects_hashes_and_tags() {
        let mut update = DefinitionsUpdate::new();

        update
            .add_row(DefinitionsType::Hashes, 1, DefinitionValue::Hash(vec![1, 2]))
            .unwrap();
        update
            .add_row(
                DefinitionsType::Tags,
                2,
                DefinitionValue::Tag("blue eyes".to_string()),
            )
            .unwrap();

        assert_eq!(update.get_num_rows(), 2);
        assert_eq!(update.get_hash_ids_to_hashes().get(&1), Some(&vec![1, 2]));
        assert_eq!(
            update.get_tag_ids_to_tags().get(&2),
            Some(&"blue eyes".to_string())
        );

        let error = update
            .add_row(
                DefinitionsType::Hashes,
                3,
                DefinitionValue::Tag("mismatched".to_string()),
            )
            .unwrap_err();

        assert_eq!(error.kind, ErrorKind::Unknown);

        let restored = roundtrip(&update);

        assert_eq!(restored.get_num_rows(), 2);
        assert_eq!(restored.get_hash_ids_to_hashes(), update.get_hash_ids_to_hashes());
        assert_eq!(restored.get_tag_ids_to_tags(), update.get_tag_ids_to_tags());
    }

    // -----------------------------------------------------------------------
    // ClientToServerUpdate
    // -----------------------------------------------------------------------

    #[test]
    fn client_to_server_update_filters_and_iterates() {
        let mut update = ClientToServerUpdate::new();

        assert!(!update.has_content());

        update.add_content(
            ContentUpdate::Pend,
            Content::mappings("blue eyes", vec![vec![1; 32]]),
            "because",
        );
        update.add_content(
            ContentUpdate::Pend,
            Content::mappings("red eyes", vec![vec![2; 32]]),
            "because",
        );
        update.add_content(
            ContentUpdate::Delete,
            Content::files(vec![vec![3; 32]]),
            "duplicate",
        );

        assert!(update.has_content());
        assert_eq!(
            update.get_hashes(),
            HashSet::from([vec![1; 32], vec![2; 32], vec![3; 32]])
        );

        let mut filter = TagFilter::new();
        filter.set_rules(["blue eyes"], Filter::Blacklist);

        update.apply_tag_filter_to_pending_mappings(&filter);

        assert!(!update.get_hashes().contains(&vec![1; 32]));
        assert!(update.get_hashes().contains(&vec![2; 32]));

        let all = update.iterate_all_actions_and_contents_and_reasons();

        assert_eq!(all.len(), 2);
        assert_eq!(all[0].0, ContentUpdate::Pend);
        assert_eq!(all[0].2, "because");

        let data = update.get_content_data_iterator(ContentType::Mappings, ContentUpdate::Pend);

        assert_eq!(data.len(), 1);
        assert_eq!(data[0].1, "because");

        let restored = roundtrip(&update);

        assert_eq!(restored.get_hashes(), update.get_hashes());
        assert_eq!(
            restored.iterate_all_actions_and_contents_and_reasons().len(),
            update.iterate_all_actions_and_contents_and_reasons().len()
        );
    }

    // -----------------------------------------------------------------------
    // Metadata
    // -----------------------------------------------------------------------

    #[test]
    fn metadata_tracks_updates_and_slices() {
        let metadata = Metadata::empty();

        assert_eq!(metadata.get_next_update_index(), 0);
        assert_eq!(metadata.get_num_update_hashes(), 0);
        assert!(!metadata.has_done_initial_sync());
        assert!(metadata.update_due(false));

        let due = get_now() + 100000;

        metadata.append_update(vec![vec![1]], 100, 200, due);

        assert_eq!(metadata.get_next_update_index(), 1);
        assert_eq!(metadata.get_num_update_hashes(), 1);
        assert!(metadata.has_done_initial_sync());
        assert!(metadata.has_update_hash(&[1]));
        assert!(!metadata.has_update_hash(&[2]));
        assert_eq!(
            metadata.get_update_index_begin_and_end(0).unwrap(),
            (100, 200)
        );
        assert_eq!(
            metadata.get_update_hashes(Some(0)).unwrap(),
            HashSet::from([vec![1]])
        );
        assert!(!metadata.update_is_empty(0).unwrap());
        assert_eq!(metadata.get_earliest_timestamp_for_these_hashes(&[vec![1]]), 200);
        assert_eq!(metadata.get_earliest_timestamp_for_these_hashes(&[vec![9]]), 0);
        assert_eq!(metadata.get_next_update_begin(), 201);
        assert!(!metadata.update_due(false));

        metadata.append_update(vec![vec![2]], 201, 300, get_now() + 100000);

        assert_eq!(metadata.get_num_update_hashes(), 2);
        assert_eq!(
            metadata.get_update_indices_and_times(),
            vec![(0, 100, 200), (1, 201, 300)]
        );
        assert_eq!(
            metadata.get_update_indices_and_hashes(),
            vec![(0, vec![vec![1]]), (1, vec![vec![2]])]
        );

        let sorted = metadata.sort_content_hashes_and_content_types(vec![
            (vec![2], "second"),
            (vec![1], "first"),
        ]);

        assert_eq!(sorted, vec![(vec![1], "first"), (vec![2], "second")]);

        let slice = metadata.get_slice(1);

        assert_eq!(slice.get_next_update_index(), 1);
        assert_eq!(slice.get_update_indices_and_times(), vec![(1, 201, 300)]);

        let combined = Metadata::empty();
        combined.update_from_slice(&metadata);

        assert_eq!(combined.get_update_indices_and_times(), metadata.get_update_indices_and_times());
        assert_eq!(combined.get_num_update_hashes(), 2);

        metadata.update_asap();

        assert!(metadata.update_due(false));
    }

    #[test]
    fn metadata_calculates_new_due_and_round_trips() {
        register_serialisable_types();

        let mut rows = IndexMap::new();

        rows.insert(
            0,
            MetadataRow {
                update_hashes: vec![vec![1], vec![2]],
                begin: 10,
                end: 20,
            },
        );
        rows.insert(
            1,
            MetadataRow {
                update_hashes: vec![vec![3]],
                begin: 21,
                end: 30,
            },
        );

        let metadata = Metadata::from_rows(rows, get_now() + 1000);

        assert_eq!(metadata.get_next_update_index(), 2);
        assert_eq!(metadata.get_num_update_hashes(), 3);

        metadata.calculate_new_next_update_due(50);

        // biggest_end + update_period
        assert_eq!(metadata.get_next_update_begin(), 31);
        assert!(metadata.update_due(false));

        let restored = roundtrip(&metadata);

        assert_eq!(
            restored.get_update_indices_and_times(),
            metadata.get_update_indices_and_times()
        );
        assert_eq!(
            restored.get_update_indices_and_hashes(),
            metadata.get_update_indices_and_hashes()
        );
        assert!(restored.has_done_initial_sync());
        assert_eq!(restored.get_next_update_index(), 2);
    }

    #[test]
    fn metadata_empty_update_is_empty() {
        let metadata = Metadata::empty();

        metadata.append_update(Vec::new(), 100, 200, get_now() + 1000);

        assert!(metadata.update_is_empty(0).unwrap());
        assert!(metadata.get_update_hashes(Some(0)).unwrap().is_empty());
        assert_eq!(
            metadata.get_update_hashes(Some(99)).unwrap_err().kind,
            ErrorKind::DataMissing
        );
        assert_eq!(
            metadata.get_update_index_begin_and_end(99).unwrap_err().kind,
            ErrorKind::DataMissing
        );
    }

    // -----------------------------------------------------------------------
    // Petition
    // -----------------------------------------------------------------------

    #[test]
    fn petition_approves_denies_and_chunks_uploads() {
        let header = PetitionHeader::new(
            ContentType::Mappings,
            ContentStatus::Petitioned,
            vec![1; 32],
            "these are bad",
        );

        let mapping = Content::mapping("blue eyes", vec![2; 32]);

        let mut petition = Petition::new(
            Arc::new(Account::generate_unknown_account(vec![1; 32])),
            header.clone(),
            vec![(ContentUpdate::Pend, vec![mapping.clone()])],
        );

        assert_eq!(petition.get_petition_header(), &header);
        assert_eq!(petition.get_reason(), "these are bad");
        assert_eq!(petition.get_contents(ContentUpdate::Pend), &[mapping.clone()]);
        assert_eq!(petition.get_contents(ContentUpdate::Petition), &[]);
        assert_eq!(petition.get_actual_content_weight(), 1);
        assert!(petition.get_content_summary().contains("mappings"));

        petition.approve_all();

        let updates = petition.get_completed_uploadable_client_to_server_updates();

        assert_eq!(updates.len(), 1);
        assert_eq!(updates[0].get_hashes(), HashSet::from([vec![2; 32]]));

        let contents = updates[0].iterate_all_actions_and_contents_and_reasons();

        assert_eq!(contents[0].0, ContentUpdate::Pend);
        assert_eq!(contents[0].2, "these are bad");

        let mut denied = Petition::new(
            Arc::new(Account::generate_unknown_account(vec![1; 32])),
            header,
            vec![(ContentUpdate::Pend, vec![mapping.clone()])],
        );

        denied.deny_all().unwrap();

        let updates = denied.get_completed_uploadable_client_to_server_updates();

        assert_eq!(updates.len(), 1);
        assert_eq!(
            updates[0].iterate_all_actions_and_contents_and_reasons()[0].0,
            ContentUpdate::DenyPend
        );

        let error = denied.deny(ContentUpdate::Add, mapping).unwrap_err();

        assert_eq!(error.kind, ErrorKind::Unknown);
    }

    #[test]
    fn petition_breaks_large_contents_into_chunks() {
        let header = PetitionHeader::new(
            ContentType::Files,
            ContentStatus::Petitioned,
            vec![1; 32],
            "bad files",
        );

        let hashes: Vec<Vec<u8>> = (0..120).map(|i| vec![i as u8; 32]).collect();

        let mut petition = Petition::new(
            Arc::new(Account::generate_unknown_account(vec![1; 32])),
            header,
            vec![(ContentUpdate::Petition, vec![Content::files(hashes)])],
        );

        petition.approve_all();

        let updates = petition.get_completed_uploadable_client_to_server_updates();

        // Content::files splits at 100 hashes, giving two updates.
        assert_eq!(updates.len(), 2);
        assert_eq!(updates[0].get_hashes().len(), 100);
        assert_eq!(updates[1].get_hashes().len(), 20);
    }

    #[test]
    fn petition_serialises_and_updates_from_v1() {
        register_serialisable_types();

        let header = PetitionHeader::new(
            ContentType::Mappings,
            ContentStatus::Petitioned,
            vec![1; 32],
            "these are bad",
        );

        let mapping = Content::mapping("blue eyes", vec![2; 32]);

        let petition = Petition::new(
            Arc::new(Account::generate_unknown_account(vec![1; 32])),
            header,
            vec![(ContentUpdate::Pend, vec![mapping.clone()])],
        );

        let restored = roundtrip(&petition);

        assert_eq!(restored.get_petition_header(), petition.get_petition_header());
        assert_eq!(
            restored.get_petitioner_account().get_account_key(),
            petition.get_petitioner_account().get_account_key()
        );
        assert_eq!(
            restored.get_actions_and_contents(),
            petition.get_actions_and_contents()
        );

        let account_info = Account::generate_serialisable_tuple_from_account(
            petition.get_petitioner_account(),
        )
        .unwrap();

        let v1_info = json!([
            ContentUpdate::Pend.as_int(),
            account_info,
            "these are bad",
            [mapping.get_serialisable_tuple()],
        ]);

        let (version, updated) = petition.update_serialisable_info(1, v1_info.clone()).unwrap();

        assert_eq!(version, 2);

        let mut from_v1 = Petition::default();
        from_v1
            .initialise_from_serialisable_tuple(1, &v1_info, false)
            .unwrap();

        assert_eq!(from_v1.get_reason(), "these are bad");
        assert_eq!(from_v1.get_contents(ContentUpdate::Pend), &[mapping]);

        // A v2 petition also upgrades to the v3 header format.
        let (version, v3_info) = petition.update_serialisable_info(2, updated.clone()).unwrap();

        assert_eq!(version, 3);

        let mut from_v2 = Petition::default();
        from_v2
            .initialise_from_serialisable_tuple(2, &updated, false)
            .unwrap();

        assert_eq!(
            from_v2.get_petition_header().content_type,
            ContentType::Mapping
        );
        assert_eq!(from_v2.get_petition_header().status, ContentStatus::Petitioned);

        let mut from_v3 = Petition::default();
        from_v3
            .initialise_from_serialisable_tuple(3, &v3_info, false)
            .unwrap();

        assert_eq!(
            from_v3.get_petition_header().content_type,
            ContentType::Mapping
        );
    }

    // -----------------------------------------------------------------------
    // UpdateBuilder
    // -----------------------------------------------------------------------

    #[test]
    fn update_builder_flushes_when_full() {
        let mut builder = UpdateBuilder::<ContentUpdatePackage>::new(2);

        for i in 0..4 {
            builder.add_row(
                ContentUpdateRow {
                    content_type: ContentType::Files,
                    action: ContentUpdate::Add,
                    data: json!(format!("hash_{i}")),
                },
                1,
            );
        }

        let updates = builder.into_updates();

        // Python flushes once the count is strictly greater than max_rows.
        assert_eq!(updates.len(), 2);
        assert_eq!(updates[0].get_num_rows(None), 3);
        assert_eq!(updates[1].get_num_rows(None), 1);

        let mut defs_builder = UpdateBuilder::<DefinitionsUpdate>::new(1);

        for i in 0..2 {
            defs_builder.add_row(
                DefinitionsUpdateRow {
                    definitions_type: DefinitionsType::Tags,
                    key: i,
                    value: DefinitionValue::Tag(format!("tag_{i}")),
                },
                1,
            );
        }

        defs_builder.finish();

        assert_eq!(defs_builder.get_updates().len(), 1);
        assert_eq!(defs_builder.get_updates()[0].get_num_rows(), 2);
    }

    #[test]
    fn update_builder_drops_an_empty_finish() {
        let mut builder = UpdateBuilder::<ContentUpdatePackage>::new(10);

        builder.finish();

        assert!(builder.get_updates().is_empty());

        builder.add_row(
            ContentUpdateRow {
                content_type: ContentType::Files,
                action: ContentUpdate::Add,
                data: json!("hash"),
            },
            5,
        );

        builder.finish();

        assert_eq!(builder.get_updates().len(), 1);
    }

    // -----------------------------------------------------------------------
    // ServerService sync/nullify controller plumbing
    // -----------------------------------------------------------------------

    static CONTROLLER_LOCK: Mutex<()> = Mutex::new(());

    struct TestBusyGuard;

    impl ServerBusyGuard for TestBusyGuard {}

    #[derive(Default)]
    struct FakeServerController {
        created_updates: Mutex<Vec<(Vec<u8>, i64, i64)>>,
        nullified: Mutex<Vec<(Vec<u8>, i64, i64)>>,
        published: Mutex<Vec<String>>,
        busy_available: bool,
    }

    impl ServerServiceController for FakeServerController {
        fn try_lock_server_busy(&self) -> Option<Box<dyn ServerBusyGuard>> {
            if self.busy_available {
                Some(Box::new(TestBusyGuard))
            } else {
                None
            }
        }

        fn nullify_history(
            &self,
            service_key: &[u8],
            begin: i64,
            end: i64,
        ) -> HydrusResult<()> {
            self.nullified
                .lock()
                .unwrap()
                .push((service_key.to_vec(), begin, end));

            Ok(())
        }

        fn create_update(
            &self,
            service_key: &[u8],
            begin: i64,
            end: i64,
        ) -> HydrusResult<Vec<Vec<u8>>> {
            self.created_updates
                .lock()
                .unwrap()
                .push((service_key.to_vec(), begin, end));

            Ok(vec![vec![0xaa; 32]])
        }

        fn publish(&self, topic: &str) {
            self.published.lock().unwrap().push(topic.to_string());
        }
    }

    #[test]
    fn repository_sync_requires_a_controller_when_due() {
        let _guard = CONTROLLER_LOCK.lock().unwrap_or_else(|error| error.into_inner());

        set_server_service_controller(None);

        let service = generate_service(
            vec![20],
            ServiceType::TagRepository,
            "sync repo",
            1000,
            None,
        )
        .unwrap();

        // Fresh metadata is not due yet, so sync is a quiet no-op.
        assert!(service.sync().is_ok());

        service.get_metadata().unwrap().update_asap();

        let error = service.sync().unwrap_err();

        assert_eq!(error.kind, ErrorKind::Unknown);
        assert!(error.to_string().contains("No server service controller"));

        set_server_service_controller(None);
    }

    #[test]
    fn repository_sync_creates_updates_while_due() {
        let _guard = CONTROLLER_LOCK.lock().unwrap_or_else(|error| error.into_inner());

        let controller = Arc::new(FakeServerController {
            busy_available: true,
            ..Default::default()
        });

        set_server_service_controller(Some(controller.clone()));

        let service = generate_service(
            vec![21],
            ServiceType::FileRepository,
            "sync repo",
            1001,
            None,
        )
        .unwrap();

        service.get_metadata().unwrap().update_asap();

        assert!(!service.is_dirty());

        service.sync().unwrap();

        assert!(service.is_dirty());
        assert_eq!(service.get_metadata().unwrap().get_next_update_index(), 2);
        assert!(service.has_update_hash(&[0xaa; 32]).unwrap());

        let created = controller.created_updates.lock().unwrap().clone();

        assert_eq!(created.len(), 1);
        assert_eq!(created[0].0, vec![21]);
        assert_eq!(created[0].2 - created[0].1, 100000);

        assert_eq!(
            controller.published.lock().unwrap().clone(),
            vec!["notify_update_created".to_string()]
        );

        set_server_service_controller(None);
    }

    #[test]
    fn repository_sync_is_skipped_when_server_is_busy() {
        let _guard = CONTROLLER_LOCK.lock().unwrap_or_else(|error| error.into_inner());

        let controller = Arc::new(FakeServerController {
            busy_available: false,
            ..Default::default()
        });

        set_server_service_controller(Some(controller.clone()));

        let service = generate_service(
            vec![22],
            ServiceType::TagRepository,
            "busy repo",
            1002,
            None,
        )
        .unwrap();

        service.get_metadata().unwrap().update_asap();

        service.sync().unwrap();

        assert_eq!(service.get_metadata().unwrap().get_next_update_index(), 1);
        assert!(!service.is_dirty());
        assert!(controller.created_updates.lock().unwrap().is_empty());

        set_server_service_controller(None);
    }

    #[test]
    fn nullify_history_walks_empty_updates_and_stops_when_caught_up() {
        let _guard = CONTROLLER_LOCK.lock().unwrap_or_else(|error| error.into_inner());

        set_server_service_controller(None);

        let mut dictionary =
            generate_default_service_dictionary(ServiceType::TagRepository);

        let old_end = get_now() - 200 * 86400;

        let metadata = Metadata::empty();
        metadata.append_update(Vec::new(), 0, old_end, get_now() + 100000);
        metadata.append_update(Vec::new(), 0, old_end, get_now() + 100000);

        dict_insert(
            &mut dictionary,
            "metadata",
            SerialisableValue::Object(Arc::new(metadata)),
        );

        let service = generate_service(
            vec![23],
            ServiceType::TagRepository,
            "nullify repo",
            1003,
            Some(dictionary),
        )
        .unwrap();

        // Both updates are empty and old enough to nullify, so the code walks
        // past them without needing a controller.
        service.nullify_history().unwrap();

        assert_eq!(
            service.get_metadata().unwrap().get_next_update_index(),
            2
        );

        // Caught up: no further work.
        service.nullify_history().unwrap();

        set_server_service_controller(None);
    }

    #[test]
    fn nullify_history_calls_the_controller_for_old_non_empty_updates() {
        let _guard = CONTROLLER_LOCK.lock().unwrap_or_else(|error| error.into_inner());

        let controller = Arc::new(FakeServerController {
            busy_available: true,
            ..Default::default()
        });

        set_server_service_controller(Some(controller.clone()));

        let mut dictionary =
            generate_default_service_dictionary(ServiceType::TagRepository);

        let old_end = get_now() - 200 * 86400;

        let metadata = Metadata::empty();
        metadata.append_update(Vec::new(), 0, old_end, get_now() + 100000);
        metadata.append_update(vec![vec![0xbb; 32]], 0, old_end, get_now() + 100000);

        dict_insert(
            &mut dictionary,
            "metadata",
            SerialisableValue::Object(Arc::new(metadata)),
        );

        let service = generate_service(
            vec![24],
            ServiceType::TagRepository,
            "nullify repo",
            1004,
            Some(dictionary),
        )
        .unwrap();

        service.nullify_history().unwrap();

        let nullified = controller.nullified.lock().unwrap().clone();

        assert_eq!(nullified.len(), 1);
        assert_eq!(nullified[0].0, vec![24]);
        assert_eq!(service.get_metadata().unwrap().get_next_update_index(), 2);

        set_server_service_controller(None);
    }
}
