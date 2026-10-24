//! Rust port of `hydrus/core/HydrusSerialisable.py`.
//!
//! Python models every persistent object as a subclass of `SerialisableBase`,
//! with class attributes naming its type and version and a virtual method that
//! produces/consumes the raw JSON-able "serialisable info". Rust has no
//! inheritance, so the same contract is split into:
//!
//! * [`SerialisableObject`] -- the object-safe trait every ported type
//!   implements. It provides the tuple/dump/hash/update plumbing as default
//!   methods, exactly like `SerialisableBase`.
//! * [`register_serialisable_type`] -- the replacement for
//!   `SERIALISABLE_TYPES_TO_OBJECT_TYPES`. Types self-register; a missing
//!   registration is a `SerialisationException`, just like Python's `KeyError`.
//!
//! Raw serialisable info is still `serde_json::Value`, so a dumped object can
//! be loaded by the Python client and vice versa. [`python_json_dumps`]
//! reproduces Python's `json.dumps` separators and `ensure_ascii` escaping so
//! that [`SerialisableObject::get_serialised_hash`] stays stable across the
//! port for the value types hydrus actually uses.

use std::any::Any;
use std::collections::HashMap;
use std::fmt::Write as _;
use std::hash::{Hash, Hasher};
use std::sync::{Arc, OnceLock, RwLock};

use indexmap::IndexMap;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::hydrus_compression;
use crate::hydrus_data::{hex_decode, hex_encode, safe_print};
use crate::hydrus_exceptions::{ErrorKind, HydrusException, HydrusResult};
use crate::hydrus_numbers;

pub const META_SERIALISABLE_TYPE_JSON_OK: u8 = 0;
pub const META_SERIALISABLE_TYPE_JSON_BYTES: u8 = 1;
pub const META_SERIALISABLE_TYPE_HYDRUS_SERIALISABLE: u8 = 2;

pub const SERIALISABLE_TYPE_BASE: u16 = 0;
pub const SERIALISABLE_TYPE_BASE_NAMED: u16 = 1;
pub const SERIALISABLE_TYPE_SHORTCUT_SET: u16 = 2;
pub const SERIALISABLE_TYPE_SUBSCRIPTION_LEGACY: u16 = 3;
pub const SERIALISABLE_TYPE_PERIODIC: u16 = 4;
pub const SERIALISABLE_TYPE_GALLERY_IDENTIFIER: u16 = 5;
pub const SERIALISABLE_TYPE_TAG_IMPORT_OPTIONS_LEGACY: u16 = 6;
pub const SERIALISABLE_TYPE_FILE_IMPORT_OPTIONS_LEGACY: u16 = 7;
pub const SERIALISABLE_TYPE_FILE_SEED_CACHE: u16 = 8;
pub const SERIALISABLE_TYPE_HDD_IMPORT: u16 = 9;
pub const SERIALISABLE_TYPE_SERVER_TO_CLIENT_CONTENT_UPDATE_PACKAGE: u16 = 10;
pub const SERIALISABLE_TYPE_SERVER_TO_CLIENT_SERVICE_UPDATE_PACKAGE: u16 = 11;
pub const SERIALISABLE_TYPE_MANAGEMENT_CONTROLLER: u16 = 12;
pub const SERIALISABLE_TYPE_GUI_SESSION_LEGACY: u16 = 13;
pub const SERIALISABLE_TYPE_PREDICATE: u16 = 14;
pub const SERIALISABLE_TYPE_FILE_SEARCH_CONTEXT: u16 = 15;
pub const SERIALISABLE_TYPE_EXPORT_FOLDER: u16 = 16;
pub const SERIALISABLE_TYPE_WATCHER_IMPORT: u16 = 17;
pub const SERIALISABLE_TYPE_SIMPLE_DOWNLOADER_IMPORT: u16 = 18;
pub const SERIALISABLE_TYPE_IMPORT_FOLDER: u16 = 19;
pub const SERIALISABLE_TYPE_MULTIPLE_GALLERY_IMPORT: u16 = 20;
pub const SERIALISABLE_TYPE_DICTIONARY: u16 = 21;
pub const SERIALISABLE_TYPE_CLIENT_OPTIONS: u16 = 22;
pub const SERIALISABLE_TYPE_CONTENT: u16 = 23;
pub const SERIALISABLE_TYPE_PETITION: u16 = 24;
pub const SERIALISABLE_TYPE_ACCOUNT_IDENTIFIER: u16 = 25;
pub const SERIALISABLE_TYPE_LIST: u16 = 26;
pub const SERIALISABLE_TYPE_PARSE_FORMULA_HTML: u16 = 27;
pub const SERIALISABLE_TYPE_URLS_IMPORT: u16 = 28;
pub const SERIALISABLE_TYPE_PARSE_NODE_CONTENT_LINK: u16 = 29;
pub const SERIALISABLE_TYPE_CONTENT_PARSER: u16 = 30;
pub const SERIALISABLE_TYPE_PARSE_FORMULA_JSON: u16 = 31;
pub const SERIALISABLE_TYPE_PARSE_ROOT_FILE_LOOKUP: u16 = 32;
pub const SERIALISABLE_TYPE_BYTES_DICT: u16 = 33;
pub const SERIALISABLE_TYPE_CONTENT_UPDATE: u16 = 34;
pub const SERIALISABLE_TYPE_CREDENTIALS: u16 = 35;
pub const SERIALISABLE_TYPE_DEFINITIONS_UPDATE: u16 = 36;
pub const SERIALISABLE_TYPE_METADATA: u16 = 37;
pub const SERIALISABLE_TYPE_BANDWIDTH_RULES: u16 = 38;
pub const SERIALISABLE_TYPE_BANDWIDTH_TRACKER: u16 = 39;
pub const SERIALISABLE_TYPE_CLIENT_TO_SERVER_UPDATE: u16 = 40;
pub const SERIALISABLE_TYPE_SHORTCUT: u16 = 41;
pub const SERIALISABLE_TYPE_APPLICATION_COMMAND: u16 = 42;
pub const SERIALISABLE_TYPE_DUPLICATE_CONTENT_MERGE_OPTIONS: u16 = 43;
pub const SERIALISABLE_TYPE_TAG_FILTER: u16 = 44;
pub const SERIALISABLE_TYPE_NETWORK_BANDWIDTH_MANAGER_LEGACY: u16 = 45;
pub const SERIALISABLE_TYPE_NETWORK_SESSION_MANAGER_LEGACY: u16 = 46;
pub const SERIALISABLE_TYPE_NETWORK_CONTEXT: u16 = 47;
pub const SERIALISABLE_TYPE_NETWORK_LOGIN_MANAGER: u16 = 48;
pub const SERIALISABLE_TYPE_MEDIA_SORT: u16 = 49;
pub const SERIALISABLE_TYPE_URL_CLASS: u16 = 50;
pub const SERIALISABLE_TYPE_STRING_MATCH: u16 = 51;
pub const SERIALISABLE_TYPE_CHECKER_OPTIONS: u16 = 52;
pub const SERIALISABLE_TYPE_NETWORK_DOMAIN_MANAGER: u16 = 53;
pub const SERIALISABLE_TYPE_SUBSCRIPTION_QUERY_LEGACY: u16 = 54;
pub const SERIALISABLE_TYPE_STRING_CONVERTER: u16 = 55;
pub const SERIALISABLE_TYPE_FILENAME_TAGGING_OPTIONS: u16 = 56;
pub const SERIALISABLE_TYPE_FILE_SEED: u16 = 57;
pub const SERIALISABLE_TYPE_PAGE_PARSER: u16 = 58;
pub const SERIALISABLE_TYPE_PARSE_FORMULA_ZIPPER: u16 = 59;
pub const SERIALISABLE_TYPE_PARSE_FORMULA_CONTEXT_VARIABLE: u16 = 60;
pub const SERIALISABLE_TYPE_TAG_SUMMARY_GENERATOR: u16 = 61;
pub const SERIALISABLE_TYPE_PARSE_RULE_HTML: u16 = 62;
pub const SERIALISABLE_TYPE_SIMPLE_DOWNLOADER_PARSE_FORMULA: u16 = 63;
pub const SERIALISABLE_TYPE_MULTIPLE_WATCHER_IMPORT: u16 = 64;
pub const SERIALISABLE_TYPE_SERVICE_TAG_IMPORT_OPTIONS: u16 = 65;
pub const SERIALISABLE_TYPE_GALLERY_SEED: u16 = 66;
pub const SERIALISABLE_TYPE_GALLERY_SEED_LOG: u16 = 67;
pub const SERIALISABLE_TYPE_GALLERY_IMPORT: u16 = 68;
pub const SERIALISABLE_TYPE_GALLERY_URL_GENERATOR: u16 = 69;
pub const SERIALISABLE_TYPE_NESTED_GALLERY_URL_GENERATOR: u16 = 70;
pub const SERIALISABLE_TYPE_DOMAIN_METADATA_PACKAGE: u16 = 71;
pub const SERIALISABLE_TYPE_LOGIN_CREDENTIAL_DEFINITION: u16 = 72;
pub const SERIALISABLE_TYPE_LOGIN_SCRIPT_DOMAIN: u16 = 73;
pub const SERIALISABLE_TYPE_LOGIN_STEP: u16 = 74;
pub const SERIALISABLE_TYPE_CLIENT_API_MANAGER: u16 = 75;
pub const SERIALISABLE_TYPE_CLIENT_API_PERMISSIONS: u16 = 76;
pub const SERIALISABLE_TYPE_SERVICE_KEYS_TO_TAGS: u16 = 77;
pub const SERIALISABLE_TYPE_MEDIA_COLLECT: u16 = 78;
pub const SERIALISABLE_TYPE_TAG_DISPLAY_MANAGER: u16 = 79;
pub const SERIALISABLE_TYPE_TAG_CONTEXT: u16 = 80;
pub const SERIALISABLE_TYPE_FAVOURITE_SEARCH_MANAGER: u16 = 81;
pub const SERIALISABLE_TYPE_NOTE_IMPORT_OPTIONS_LEGACY: u16 = 82;
pub const SERIALISABLE_TYPE_STRING_SPLITTER: u16 = 83;
pub const SERIALISABLE_TYPE_STRING_PROCESSOR: u16 = 84;
pub const SERIALISABLE_TYPE_TAG_AUTOCOMPLETE_OPTIONS: u16 = 85;
pub const SERIALISABLE_TYPE_SUBSCRIPTION_QUERY_LOG_CONTAINER: u16 = 86;
pub const SERIALISABLE_TYPE_SUBSCRIPTION_QUERY_HEADER: u16 = 87;
pub const SERIALISABLE_TYPE_SUBSCRIPTION: u16 = 88;
pub const SERIALISABLE_TYPE_FILE_SEED_CACHE_STATUS: u16 = 89;
pub const SERIALISABLE_TYPE_SUBSCRIPTION_CONTAINER: u16 = 90;
pub const SERIALISABLE_TYPE_COLUMN_LIST_STATUS: u16 = 91;
pub const SERIALISABLE_TYPE_COLUMN_LIST_MANAGER: u16 = 92;
pub const SERIALISABLE_TYPE_NUMBER_TEST: u16 = 93;
pub const SERIALISABLE_TYPE_NETWORK_BANDWIDTH_MANAGER: u16 = 94;
pub const SERIALISABLE_TYPE_NETWORK_SESSION_MANAGER: u16 = 95;
pub const SERIALISABLE_TYPE_NETWORK_SESSION_MANAGER_SESSION_CONTAINER: u16 = 96;
pub const SERIALISABLE_TYPE_NETWORK_BANDWIDTH_MANAGER_TRACKER_CONTAINER: u16 = 97;
pub const SERIALISABLE_TYPE_SIDECAR_EXPORTER: u16 = 98;
pub const SERIALISABLE_TYPE_STRING_SORTER: u16 = 99;
pub const SERIALISABLE_TYPE_STRING_SLICER: u16 = 100;
pub const SERIALISABLE_TYPE_TAG_SORT: u16 = 101;
pub const SERIALISABLE_TYPE_ACCOUNT_TYPE: u16 = 102;
pub const SERIALISABLE_TYPE_LOCATION_CONTEXT: u16 = 103;
pub const SERIALISABLE_TYPE_GUI_SESSION_CONTAINER: u16 = 104;
pub const SERIALISABLE_TYPE_GUI_SESSION_PAGE_DATA: u16 = 105;
pub const SERIALISABLE_TYPE_GUI_SESSION_CONTAINER_PAGE_NOTEBOOK: u16 = 106;
pub const SERIALISABLE_TYPE_GUI_SESSION_CONTAINER_PAGE_SINGLE: u16 = 107;
pub const SERIALISABLE_TYPE_PRESENTATION_IMPORT_OPTIONS: u16 = 108;
pub const SERIALISABLE_TYPE_METADATA_SINGLE_FILE_ROUTER: u16 = 109;
pub const SERIALISABLE_TYPE_METADATA_SINGLE_FILE_IMPORTER_TXT: u16 = 110;
pub const SERIALISABLE_TYPE_METADATA_SINGLE_FILE_IMPORTER_MEDIA_TAGS: u16 = 111;
pub const SERIALISABLE_TYPE_STRING_TAG_FILTER: u16 = 112;
pub const SERIALISABLE_TYPE_METADATA_SINGLE_FILE_EXPORTER_JSON: u16 = 113;
pub const SERIALISABLE_TYPE_METADATA_SINGLE_FILE_IMPORTER_JSON: u16 = 114;
pub const SERIALISABLE_TYPE_METADATA_SINGLE_FILE_EXPORTER_MEDIA_TAGS: u16 = 115;
pub const SERIALISABLE_TYPE_METADATA_SINGLE_FILE_EXPORTER_TXT: u16 = 116;
pub const SERIALISABLE_TYPE_METADATA_SINGLE_FILE_EXPORTER_MEDIA_URLS: u16 = 117;
pub const SERIALISABLE_TYPE_METADATA_SINGLE_FILE_IMPORTER_MEDIA_URLS: u16 = 118;
pub const SERIALISABLE_TYPE_METADATA_SINGLE_FILE_EXPORTER_MEDIA_NOTES: u16 = 119;
pub const SERIALISABLE_TYPE_METADATA_SINGLE_FILE_IMPORTER_MEDIA_NOTES: u16 = 120;
pub const SERIALISABLE_TYPE_TIMESTAMP_DATA: u16 = 121;
pub const SERIALISABLE_TYPE_METADATA_SINGLE_FILE_EXPORTER_MEDIA_TIMESTAMPS: u16 = 122;
pub const SERIALISABLE_TYPE_METADATA_SINGLE_FILE_IMPORTER_MEDIA_TIMESTAMPS: u16 = 123;
pub const SERIALISABLE_TYPE_PETITION_HEADER: u16 = 124;
pub const SERIALISABLE_TYPE_STRING_JOINER: u16 = 125;
pub const SERIALISABLE_TYPE_FILE_FILTER: u16 = 126;
pub const SERIALISABLE_TYPE_URL_CLASS_PARAMETER_FIXED_NAME: u16 = 127;
pub const SERIALISABLE_TYPE_DUPLICATES_AUTO_RESOLUTION_RULE: u16 = 128;
pub const SERIALISABLE_TYPE_DUPLICATES_AUTO_RESOLUTION_PAIR_SELECTOR: u16 = 129;
pub const SERIALISABLE_TYPE_DUPLICATES_AUTO_RESOLUTION_PAIR_COMPARATOR_ONE_FILE_METADATA_CONDITIONAL: u16 = 130;
pub const SERIALISABLE_TYPE_DUPLICATES_AUTO_RESOLUTION_PAIR_COMPARATOR_TWO_FILES_RELATIVE_FILE_INFO: u16 = 131;
pub const SERIALISABLE_TYPE_METADATA_CONDITIONAL: u16 = 132;
pub const SERIALISABLE_TYPE_PARSE_FORMULA_NESTED: u16 = 133;
pub const SERIALISABLE_TYPE_POTENTIAL_DUPLICATES_SEARCH_CONTEXT: u16 = 134;
pub const SERIALISABLE_TYPE_SUBSIDIARY_PAGE_PARSER: u16 = 135;
pub const SERIALISABLE_TYPE_PARSE_FORMULA_STATIC: u16 = 136;
pub const SERIALISABLE_TYPE_DUPLICATES_AUTO_RESOLUTION_PAIR_COMPARATOR_TWO_FILES_RELATIVE_HARDCODED: u16 = 137;
pub const SERIALISABLE_TYPE_DUPLICATES_AUTO_RESOLUTION_PAIR_COMPARATOR_TWO_FILES_RELATIVE_VISUAL_DUPLICATES: u16 = 138;
pub const SERIALISABLE_TYPE_URL_DOMAIN_MASK: u16 = 139;
pub const SERIALISABLE_TYPE_DUPLICATES_AUTO_RESOLUTION_PAIR_COMPARATOR_OR: u16 = 140;
pub const SERIALISABLE_TYPE_DUPLICATES_AUTO_RESOLUTION_PAIR_COMPARATOR_AND: u16 = 141;
pub const SERIALISABLE_TYPE_SERVICE_SPECIFIER: u16 = 142;
pub const SERIALISABLE_TYPE_IMPORT_OPTIONS_CONTAINER: u16 = 143;
pub const SERIALISABLE_TYPE_IMPORT_OPTIONS_MANAGER: u16 = 144;
pub const SERIALISABLE_TYPE_PREFETCH_IMPORT_OPTIONS: u16 = 145;
pub const SERIALISABLE_TYPE_NETWORK_CONTEXT_SETTINGS: u16 = 146;
pub const SERIALISABLE_TYPE_NETWORK_CONTEXT_STATUS: u16 = 147;
pub const SERIALISABLE_TYPE_FILE_FILTERING_IMPORT_OPTIONS: u16 = 148;
pub const SERIALISABLE_TYPE_LOCATION_IMPORT_OPTIONS: u16 = 149;
pub const SERIALISABLE_TYPE_TAG_FILTERING_IMPORT_OPTIONS: u16 = 150;
pub const SERIALISABLE_TYPE_TAG_IMPORT_OPTIONS: u16 = 151;
pub const SERIALISABLE_TYPE_DUPLICATES_AUTO_RESOLUTION_PAIR_COMPARATOR_ONE_FILE_HARDCODED: u16 =
    152;
pub const SERIALISABLE_TYPE_NOTE_IMPORT_OPTIONS: u16 = 153;
pub const SERIALISABLE_TYPE_NETWORK_CONTEXT_RECORD: u16 = 154;
pub const SERIALISABLE_TYPE_EXECUTABLE_MANAGER: u16 = 155;
pub const SERIALISABLE_TYPE_EXECUTABLE_CALLABLE: u16 = 156;
pub const SERIALISABLE_TYPE_EXECUTABLE_CALL_LOCAL_PROCESS_CALL: u16 = 157;
pub const SERIALISABLE_TYPE_ID_AND_NAME: u16 = 158;
pub const SERIALISABLE_TYPE_EXECUTABLE_CALL_LOCAL_PROCESS_INPUT_TEMPLATE_PARAM_PROCESSING_RULE:
    u16 = 159;
pub const SERIALISABLE_TYPE_EXECUTABLE_CALL_LOCAL_PROCESS_DEFAULT_LAUNCH_FILE: u16 = 160;
pub const SERIALISABLE_TYPE_EXECUTABLE_CALL_LOCAL_PROCESS_DEFAULT_LAUNCH_URL: u16 = 161;
pub const SERIALISABLE_TYPE_EXTERNAL_PROGRAMS_IMPORT_OPTIONS: u16 = 162;
pub const SERIALISABLE_TYPE_EXTERNAL_PROGRAMS_IMPORT_OPTIONS_SINGLE_ENTRY: u16 = 163;

/// A failure that Python reports as a bare `KeyError` on the type table.
fn unknown_type_error(serialisable_type: u16) -> HydrusException {
    HydrusException::new(
        ErrorKind::Serialisation,
        format!("No serialisable object is registered for type {serialisable_type}!"),
    )
}

fn serialisation_error(message: impl Into<String>) -> HydrusException {
    HydrusException::new(ErrorKind::Serialisation, message)
}

fn json_u16(value: &Value, what: &str) -> HydrusResult<u16> {
    value
        .as_u64()
        .and_then(|n| u16::try_from(n).ok())
        .ok_or_else(|| serialisation_error(format!("{what} was not a valid small integer!")))
}

//region the object contract

/// Object-safe equivalent of `SerialisableBase`.
///
/// Implementors only provide their type/version/name, the raw info conversion
/// and `as_any`. Everything else (tuple layout, compression, duplication,
/// hashing, version migration and the future-version warning) is defaulted.
pub trait SerialisableObject: Send + Sync {
    fn serialisable_type(&self) -> u16;

    fn serialisable_version(&self) -> u16 {
        1
    }

    fn serialisable_name(&self) -> &'static str {
        "Base Serialisable Object"
    }

    /// `_get_serialisable_info`.
    fn get_serialisable_info(&self) -> Value;

    /// `_initialise_from_serialisable_info`.
    fn initialise_from_serialisable_info(&mut self, serialisable_info: &Value) -> HydrusResult<()>;

    /// `_update_serialisable_info`. Returns the new version and info.
    fn update_serialisable_info(
        &self,
        version: u16,
        old_serialisable_info: Value,
    ) -> HydrusResult<(u16, Value)> {
        Ok((version, old_serialisable_info))
    }

    /// `None` for unnamed objects, `Some` for `SerialisableBaseNamed` ports.
    fn get_name(&self) -> Option<&str> {
        None
    }

    fn set_name(&mut self, _name: &str) {}

    fn as_any(&self) -> &dyn Any;

    fn as_any_mut(&mut self) -> &mut dyn Any;

    fn get_serialisable_description(&self) -> String {
        match self.get_name() {
            Some(name) => format!(
                "{} ({}) \"{}\"",
                self.serialisable_name(),
                self.serialisable_type(),
                name
            ),
            None => format!(
                "{} ({})",
                self.serialisable_name(),
                self.serialisable_type()
            ),
        }
    }

    /// `get_serialisable_tuple`: `(type, [name,] version, info)`.
    fn get_serialisable_tuple(&self) -> Value {
        let info = self.get_serialisable_info();

        match self.get_name() {
            Some(name) => json!([
                self.serialisable_type(),
                name,
                self.serialisable_version(),
                info
            ]),
            None => json!([self.serialisable_type(), self.serialisable_version(), info]),
        }
    }

    fn dump_to_string(&self) -> String {
        python_json_dumps(&self.get_serialisable_tuple())
    }

    fn dump_to_network_bytes(&self) -> HydrusResult<Vec<u8>> {
        hydrus_compression::compress_string_to_bytes(&self.dump_to_string())
    }

    fn duplicate(&self) -> HydrusResult<Box<dyn SerialisableObject>> {
        create_from_string(&self.dump_to_string(), false)
    }

    fn get_serialised_hash(&self) -> Vec<u8> {
        Sha256::digest(self.dump_to_string().as_bytes()).to_vec()
    }

    /// `initialise_from_serialisable_info`, including the version-update loop
    /// and the warnings/errors around future versions.
    fn initialise_from_serialisable_tuple(
        &mut self,
        original_version: u16,
        serialisable_info: &Value,
        raise_error_on_future_version: bool,
    ) -> HydrusResult<()> {
        let supported_version = self.serialisable_version();
        let object_is_newer = original_version > supported_version;

        if object_is_newer {
            if raise_error_on_future_version {
                let message = format!(
                    "Unfortunately, an object of type {} could not be loaded because it was created in a client/server that uses an updated version of that object! We support up to version {}, but the object was version {}.\n\nPlease update your client/server to import this object.",
                    self.serialisable_name(),
                    supported_version,
                    original_version
                );

                return Err(serialisation_error(message));
            }

            safe_print(format!(
                "An object of type {} was created in a client/server that uses an updated version of that object! We support versions up to {}, but the object was version {}. For now, we will try to continue work, but things may break. If you know why this has occured, please correct it. If you do not, please let hydrus dev know.",
                self.serialisable_name(),
                supported_version,
                original_version
            ));
        }

        let mut current_version = original_version;
        let mut info = serialisable_info.clone();

        while current_version < supported_version {
            let (new_version, new_info) = self
                .update_serialisable_info(current_version, info)
                .map_err(|_| {
                    serialisation_error(format!(
                        "Could not update this object of type {} from version {} to {}!",
                        self.serialisable_name(),
                        original_version,
                        supported_version
                    ))
                })?;

            // Guard against an implementation forgetting to raise the version.
            if new_version <= current_version {
                return Err(serialisation_error(format!(
                    "Object of type {} did not advance past version {current_version} while updating!",
                    self.serialisable_name()
                )));
            }

            current_version = new_version;
            info = new_info;
        }

        self.initialise_from_serialisable_info(&info).map_err(|_| {
            if object_is_newer {
                serialisation_error(format!(
                    "An object of type {} was created in a client/server that uses an updated version of that object! We support versions up to {}, but the object was version {}. I tried to load it, but the initialisation failed. You probably need to update your client/server.",
                    self.serialisable_name(),
                    supported_version,
                    original_version
                ))
            } else {
                serialisation_error(format!(
                    "Could not initialise this object of type {}!",
                    self.serialisable_name()
                ))
            }
        })
    }
}

//endregion

//region type registry / factories

type PlainFactory = fn() -> Box<dyn SerialisableObject>;
type NamedFactory = fn(&str) -> Box<dyn SerialisableObject>;

#[derive(Clone, Copy)]
enum FactoryEntry {
    Plain(PlainFactory),
    Named(NamedFactory),
}

fn type_registry() -> &'static RwLock<HashMap<u16, FactoryEntry>> {
    static REGISTRY: OnceLock<RwLock<HashMap<u16, FactoryEntry>>> = OnceLock::new();

    REGISTRY.get_or_init(|| RwLock::new(HashMap::new()))
}

fn with_registry<R>(read: impl FnOnce(&HashMap<u16, FactoryEntry>) -> R) -> R {
    let guard = type_registry().read().unwrap_or_else(|e| e.into_inner());

    read(&guard)
}

/// Register an unnamed type. Returns `false` if the type was already taken.
pub fn register_serialisable_type(serialisable_type: u16, factory: PlainFactory) -> bool {
    let mut guard = type_registry().write().unwrap_or_else(|e| e.into_inner());

    if guard.contains_key(&serialisable_type) {
        return false;
    }

    guard.insert(serialisable_type, FactoryEntry::Plain(factory));

    true
}

/// Register a named type (a `SerialisableBaseNamed` port).
pub fn register_named_serialisable_type(serialisable_type: u16, factory: NamedFactory) -> bool {
    let mut guard = type_registry().write().unwrap_or_else(|e| e.into_inner());

    if guard.contains_key(&serialisable_type) {
        return false;
    }

    guard.insert(serialisable_type, FactoryEntry::Named(factory));

    true
}

pub fn serialisable_type_is_registered(serialisable_type: u16) -> bool {
    with_registry(|registry| registry.contains_key(&serialisable_type))
}

/// The set of currently registered type ids, for diagnostics.
pub fn registered_serialisable_types() -> Vec<u16> {
    let mut types: Vec<u16> = with_registry(|registry| registry.keys().copied().collect());

    types.sort_unstable();

    types
}

fn instantiate_serialisable(
    serialisable_type: u16,
    name: Option<&str>,
) -> HydrusResult<Box<dyn SerialisableObject>> {
    let entry = with_registry(|registry| registry.get(&serialisable_type).copied())
        .ok_or_else(|| unknown_type_error(serialisable_type))?;

    match entry {
        FactoryEntry::Plain(factory) => Ok(factory()),
        FactoryEntry::Named(factory) => {
            let name = name.ok_or_else(|| {
                serialisation_error(format!(
                    "Serialisable type {serialisable_type} is named but the tuple had no name!"
                ))
            })?;

            Ok(factory(name))
        }
    }
}

//endregion

//region parsing and dumping

pub fn create_from_serialisable_tuple(
    obj_tuple: &Value,
    raise_error_on_future_version: bool,
) -> HydrusResult<Box<dyn SerialisableObject>> {
    ensure_builtins_once();

    let items = obj_tuple
        .as_array()
        .ok_or_else(|| serialisation_error("Serialisable tuple was not a JSON array!"))?;

    let (serialisable_type, name, version, serialisable_info) =
        match items.len() {
            3 => (
                json_u16(&items[0], "serialisable type")?,
                None,
                json_u16(&items[1], "serialisable version")?,
                items[2].clone(),
            ),
            4 => (
                json_u16(&items[0], "serialisable type")?,
                Some(items[1].as_str().ok_or_else(|| {
                    serialisation_error("Serialisable tuple name was not a string!")
                })?),
                json_u16(&items[2], "serialisable version")?,
                items[3].clone(),
            ),
            _ => {
                return Err(serialisation_error(
                    "Serialisable tuple was not length 3 or 4!",
                ));
            }
        };

    let mut obj = instantiate_serialisable(serialisable_type, name)?;

    obj.initialise_from_serialisable_tuple(
        version,
        &serialisable_info,
        raise_error_on_future_version,
    )?;

    Ok(obj)
}

pub fn create_from_noneable_serialisable_tuple(
    obj_tuple_or_none: Option<&Value>,
    raise_error_on_future_version: bool,
) -> HydrusResult<Option<Box<dyn SerialisableObject>>> {
    match obj_tuple_or_none {
        None => Ok(None),
        Some(obj_tuple) => Ok(Some(create_from_serialisable_tuple(
            obj_tuple,
            raise_error_on_future_version,
        )?)),
    }
}

pub fn create_from_string(
    obj_string: &str,
    raise_error_on_future_version: bool,
) -> HydrusResult<Box<dyn SerialisableObject>> {
    let obj_tuple: Value = serde_json::from_str(obj_string)
        .map_err(|e| serialisation_error(format!("Could not parse serialised object: {e}")))?;

    create_from_serialisable_tuple(&obj_tuple, raise_error_on_future_version)
}

pub fn create_from_network_bytes(
    network_bytes: &[u8],
    raise_error_on_future_version: bool,
) -> HydrusResult<Box<dyn SerialisableObject>> {
    let obj_string = hydrus_compression::decompress_bytes_to_string(network_bytes)?;

    create_from_string(&obj_string, raise_error_on_future_version)
}

pub fn get_noneable_serialisable_tuple(obj: Option<&dyn SerialisableObject>) -> Option<Value> {
    obj.map(SerialisableObject::get_serialisable_tuple)
}

/// `set_non_dupe_name`, for named serialisables.
pub fn set_non_dupe_name(
    obj: &mut dyn SerialisableObject,
    disallowed_names: &std::collections::HashSet<String>,
    do_casefold: bool,
) {
    let Some(current) = obj.get_name() else {
        return;
    };

    let non_dupe_name =
        crate::hydrus_data::get_non_dupe_name(current, disallowed_names, do_casefold);

    obj.set_name(&non_dupe_name);
}

/// Python's `json.dumps` defaults: `, ` / `: ` separators and ASCII-only
/// output. Using the same representation keeps `get_serialised_hash` stable.
pub fn python_json_dumps(value: &Value) -> String {
    let mut out = String::new();

    write_json_value(&mut out, value);

    out
}

fn write_json_string(out: &mut String, s: &str) {
    out.push('"');

    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{08}' => out.push_str("\\b"),
            '\u{0c}' => out.push_str("\\f"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (' '..='~').contains(&c) => out.push(c),
            c => {
                let mut units = [0u16; 2];

                for unit in c.encode_utf16(&mut units) {
                    let _ = write!(out, "\\u{unit:04x}");
                }
            }
        }
    }

    out.push('"');
}

fn write_json_value(out: &mut String, value: &Value) {
    match value {
        Value::Null => out.push_str("null"),
        Value::Bool(true) => out.push_str("true"),
        Value::Bool(false) => out.push_str("false"),
        Value::Number(number) => out.push_str(&number.to_string()),
        Value::String(s) => write_json_string(out, s),
        Value::Array(items) => {
            out.push('[');

            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }

                write_json_value(out, item);
            }

            out.push(']');
        }
        Value::Object(map) => {
            out.push('{');

            for (i, (key, item)) in map.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }

                write_json_string(out, key);
                out.push_str(": ");
                write_json_value(out, item);
            }

            out.push('}');
        }
    }
}

//endregion

//region SerialisableValue

/// A single JSON-able slot: plain JSON, a bytestring, or a nested serialisable.
///
/// This replaces Python's duck-typed `isinstance` tests in
/// `convert_object_to_meta_serialisable_tuple`.
#[derive(Clone)]
pub enum SerialisableValue {
    Json(Value),
    Bytes(Vec<u8>),
    /// `Arc` keeps cloning cheap and matches Python's reference semantics for
    /// objects used as dictionary keys.
    Object(Arc<dyn SerialisableObject>),
}

impl std::fmt::Debug for dyn SerialisableObject {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.get_serialisable_description())
    }
}

impl SerialisableValue {
    pub fn is_bytes(&self) -> bool {
        matches!(self, Self::Bytes(_))
    }

    pub fn is_object(&self) -> bool {
        matches!(self, Self::Object(_))
    }

    pub fn as_json(&self) -> Option<&Value> {
        match self {
            Self::Json(value) => Some(value),
            _ => None,
        }
    }

    pub fn into_json(self) -> Option<Value> {
        match self {
            Self::Json(value) => Some(value),
            _ => None,
        }
    }

    pub fn as_bytes(&self) -> Option<&[u8]> {
        match self {
            Self::Bytes(bytes) => Some(bytes),
            _ => None,
        }
    }

    pub fn as_object(&self) -> Option<&dyn SerialisableObject> {
        match self {
            Self::Object(obj) => Some(obj.as_ref()),
            _ => None,
        }
    }

    /// `convert_object_to_meta_serialisable_tuple`.
    pub fn to_meta_tuple(&self) -> HydrusResult<Value> {
        Ok(match self {
            Self::Json(value) => json!([META_SERIALISABLE_TYPE_JSON_OK, value]),
            Self::Bytes(bytes) => {
                json!([META_SERIALISABLE_TYPE_JSON_BYTES, hex_encode(bytes)])
            }
            Self::Object(obj) => {
                json!([
                    META_SERIALISABLE_TYPE_HYDRUS_SERIALISABLE,
                    obj.get_serialisable_tuple()
                ])
            }
        })
    }

    /// `convert_meta_serialisable_tuple_to_object`.
    pub fn from_meta_tuple(meta_tuple: &Value) -> HydrusResult<Self> {
        let items = meta_tuple
            .as_array()
            .ok_or_else(|| serialisation_error("Meta serialisable tuple was not a JSON array!"))?;

        let metatype = items
            .first()
            .and_then(Value::as_u64)
            .ok_or_else(|| serialisation_error("Meta serialisable metatype was missing!"))?;

        let serialisable = items.get(1).ok_or_else(|| {
            serialisation_error("Meta serialisable tuple had no serialisable value!")
        })?;

        match metatype {
            m if m == u64::from(META_SERIALISABLE_TYPE_JSON_OK) => {
                Ok(Self::Json(serialisable.clone()))
            }
            m if m == u64::from(META_SERIALISABLE_TYPE_JSON_BYTES) => {
                let hex = serialisable.as_str().ok_or_else(|| {
                    serialisation_error("Serialised bytestring was not a hex string!")
                })?;
                let bytes = hex_decode(hex)
                    .map_err(|_| serialisation_error("Serialised bytestring was not valid hex!"))?;

                Ok(Self::Bytes(bytes))
            }
            m if m == u64::from(META_SERIALISABLE_TYPE_HYDRUS_SERIALISABLE) => Ok(Self::Object(
                Arc::from(create_from_serialisable_tuple(serialisable, false)?),
            )),
            _ => Ok(Self::Json(serialisable.clone())),
        }
    }

    /// Python coerces raw lists/dicts before serialising. Do the same so the
    /// resulting tuple matches the Python client's.
    fn coerce_for_serialisation(self) -> Self {
        match self {
            Self::Json(Value::Array(items)) => {
                Self::Object(Arc::new(SerialisableList::from_json_array(items)))
            }
            Self::Json(Value::Object(map)) => {
                Self::Object(Arc::new(SerialisableDictionary::from_json_object(map)))
            }
            other => other,
        }
    }
}

impl From<Value> for SerialisableValue {
    fn from(value: Value) -> Self {
        Self::Json(value)
    }
}

impl From<Vec<u8>> for SerialisableValue {
    fn from(value: Vec<u8>) -> Self {
        Self::Bytes(value)
    }
}

impl From<&[u8]> for SerialisableValue {
    fn from(value: &[u8]) -> Self {
        Self::Bytes(value.to_vec())
    }
}

impl From<&str> for SerialisableValue {
    fn from(value: &str) -> Self {
        Self::Json(Value::String(value.to_string()))
    }
}

impl From<String> for SerialisableValue {
    fn from(value: String) -> Self {
        Self::Json(Value::String(value))
    }
}

impl From<bool> for SerialisableValue {
    fn from(value: bool) -> Self {
        Self::Json(Value::Bool(value))
    }
}

macro_rules! serialisable_value_from_int {
    ($($ty:ty),* $(,)?) => {
        $(
            impl From<$ty> for SerialisableValue {
                fn from(value: $ty) -> Self {
                    Self::Json(Value::from(value))
                }
            }
        )*
    };
}

serialisable_value_from_int!(i8, i16, i32, i64, u8, u16, u32, u64, f32, f64);

impl From<Box<dyn SerialisableObject>> for SerialisableValue {
    fn from(value: Box<dyn SerialisableObject>) -> Self {
        Self::Object(Arc::from(value))
    }
}

impl From<Arc<dyn SerialisableObject>> for SerialisableValue {
    fn from(value: Arc<dyn SerialisableObject>) -> Self {
        Self::Object(value)
    }
}

impl std::fmt::Debug for SerialisableValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Json(value) => write!(f, "{value:?}"),
            Self::Bytes(bytes) => write!(f, "Bytes({})", hex_encode(bytes)),
            Self::Object(obj) => write!(f, "Object({})", obj.get_serialisable_description()),
        }
    }
}

impl PartialEq for SerialisableValue {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Json(a), Self::Json(b)) => a == b,
            (Self::Bytes(a), Self::Bytes(b)) => a == b,
            // Python objects compare by identity unless they define __eq__.
            (Self::Object(a), Self::Object(b)) => Arc::ptr_eq(a, b),
            _ => false,
        }
    }
}

impl Eq for SerialisableValue {}

impl Hash for SerialisableValue {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            Self::Json(value) => {
                0u8.hash(state);
                hash_json_value(value, state);
            }
            Self::Bytes(bytes) => {
                1u8.hash(state);
                bytes.hash(state);
            }
            Self::Object(obj) => {
                2u8.hash(state);
                (Arc::as_ptr(obj) as *const () as usize).hash(state);
            }
        }
    }
}

fn hash_json_value<H: Hasher>(value: &Value, state: &mut H) {
    match value {
        Value::Null => 0u8.hash(state),
        Value::Bool(b) => {
            1u8.hash(state);
            b.hash(state);
        }
        Value::Number(number) => {
            2u8.hash(state);
            match number.as_f64() {
                Some(f) => f.to_bits().hash(state),
                None => number.to_string().hash(state),
            }
        }
        Value::String(s) => {
            3u8.hash(state);
            s.hash(state);
        }
        Value::Array(items) => {
            4u8.hash(state);
            items.len().hash(state);

            for item in items {
                hash_json_value(item, state);
            }
        }
        Value::Object(map) => {
            5u8.hash(state);
            map.len().hash(state);

            for (key, item) in map {
                key.hash(state);
                hash_json_value(item, state);
            }
        }
    }
}

//endregion

//region SerialisableList

/// `SerialisableList`: an ordered list whose items may be JSON, bytes or
/// nested serialisables. Derefs to the inner vector for convenient access.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SerialisableList {
    items: Vec<SerialisableValue>,
}

impl SerialisableList {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_values(values: Vec<SerialisableValue>) -> Self {
        Self { items: values }
    }

    pub fn from_json_array(values: Vec<Value>) -> Self {
        Self {
            items: values.into_iter().map(SerialisableValue::Json).collect(),
        }
    }

    pub fn as_slice(&self) -> &[SerialisableValue] {
        &self.items
    }

    pub fn push(&mut self, value: impl Into<SerialisableValue>) {
        self.items.push(value.into());
    }

    pub fn into_inner(self) -> Vec<SerialisableValue> {
        self.items
    }

    fn describe_items(item_descriptions: impl Iterator<Item = String>) -> String {
        let mut counter: IndexMap<String, usize> = IndexMap::new();

        for description in item_descriptions {
            *counter.entry(description).or_insert(0) += 1;
        }

        let mut result = String::from("Serialisable List (26)\n");

        let lines: Vec<String> = counter
            .into_iter()
            .map(|(description, count)| {
                format!(
                    "{}: {}",
                    hydrus_numbers::base_to_human_int(count as i128),
                    description
                )
            })
            .collect();

        result.push_str(&lines.join("\n"));

        result
    }
}

impl std::ops::Deref for SerialisableList {
    type Target = Vec<SerialisableValue>;

    fn deref(&self) -> &Self::Target {
        &self.items
    }
}

impl std::ops::DerefMut for SerialisableList {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.items
    }
}

impl IntoIterator for SerialisableList {
    type Item = SerialisableValue;
    type IntoIter = std::vec::IntoIter<SerialisableValue>;

    fn into_iter(self) -> Self::IntoIter {
        self.items.into_iter()
    }
}

impl<'a> IntoIterator for &'a SerialisableList {
    type Item = &'a SerialisableValue;
    type IntoIter = std::slice::Iter<'a, SerialisableValue>;

    fn into_iter(self) -> Self::IntoIter {
        self.items.iter()
    }
}

impl Extend<SerialisableValue> for SerialisableList {
    fn extend<T: IntoIterator<Item = SerialisableValue>>(&mut self, iter: T) {
        self.items.extend(iter);
    }
}

impl FromIterator<SerialisableValue> for SerialisableList {
    fn from_iter<T: IntoIterator<Item = SerialisableValue>>(iter: T) -> Self {
        Self {
            items: iter.into_iter().collect(),
        }
    }
}

impl SerialisableObject for SerialisableList {
    fn serialisable_type(&self) -> u16 {
        SERIALISABLE_TYPE_LIST
    }

    fn serialisable_version(&self) -> u16 {
        3
    }

    fn serialisable_name(&self) -> &'static str {
        "Serialisable List"
    }

    fn get_serialisable_info(&self) -> Value {
        Value::Array(
            self.items
                .iter()
                .map(|item| {
                    item.clone()
                        .coerce_for_serialisation()
                        .to_meta_tuple()
                        .unwrap_or(Value::Null)
                })
                .collect(),
        )
    }

    fn initialise_from_serialisable_info(&mut self, serialisable_info: &Value) -> HydrusResult<()> {
        let meta_tuples = serialisable_info
            .as_array()
            .ok_or_else(|| serialisation_error("Serialisable list info was not an array!"))?;

        let mut have_shown_load_error = false;

        for meta_tuple in meta_tuples {
            match SerialisableValue::from_meta_tuple(meta_tuple) {
                Ok(value) => self.items.push(value),
                Err(e) => {
                    if !have_shown_load_error {
                        safe_print(
                            "An object in a list could not load. It has been discarded from the list. More may also have failed to load, but to stop error spam, they will go silently. Your client may be running on code versions behind its database. Depending on the severity of this error, you may need to rollback to a previous backup. If you have no backup, you may want to kill your hydrus process now to stop the cleansed list ever being saved back to the db.",
                        );
                        safe_print(e.to_string());

                        have_shown_load_error = true;
                    }
                }
            }
        }

        Ok(())
    }

    fn update_serialisable_info(
        &self,
        version: u16,
        old_serialisable_info: Value,
    ) -> HydrusResult<(u16, Value)> {
        match version {
            1 => {
                let serialised_objects = old_serialisable_info.as_array().ok_or_else(|| {
                    serialisation_error("Serialisable list v1 info was not an array!")
                })?;

                let new_info = Value::Array(
                    serialised_objects
                        .iter()
                        .map(|obj| json!([true, obj]))
                        .collect(),
                );

                Ok((2, new_info))
            }
            2 => {
                let serialised_object_tuples =
                    old_serialisable_info.as_array().ok_or_else(|| {
                        serialisation_error("Serialisable list v2 info was not an array!")
                    })?;

                let mut meta_items = Vec::with_capacity(serialised_object_tuples.len());

                for tuple in serialised_object_tuples {
                    let pair = tuple.as_array().ok_or_else(|| {
                        serialisation_error("Serialisable list v2 item was not an array!")
                    })?;

                    let is_serialised = pair.first().and_then(Value::as_bool).unwrap_or(false);
                    let serialised_obj = pair
                        .get(1)
                        .ok_or_else(|| {
                            serialisation_error("Serialisable list v2 item had no value!")
                        })?
                        .clone();

                    let metatype = if is_serialised {
                        META_SERIALISABLE_TYPE_HYDRUS_SERIALISABLE
                    } else {
                        META_SERIALISABLE_TYPE_JSON_OK
                    };

                    meta_items.push(json!([metatype, serialised_obj]));
                }

                Ok((3, Value::Array(meta_items)))
            }
            _ => Ok((version, old_serialisable_info)),
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn get_serialisable_description(&self) -> String {
        Self::describe_items(self.items.iter().map(|item| match item {
            SerialisableValue::Object(obj) => obj.get_serialisable_description(),
            other => format!("<{}>", value_kind_name(other)),
        }))
    }
}

fn value_kind_name(value: &SerialisableValue) -> &'static str {
    match value {
        SerialisableValue::Json(Value::Null) => "class 'NoneType'",
        SerialisableValue::Json(Value::Bool(_)) => "class 'bool'",
        SerialisableValue::Json(Value::Number(_)) => "class 'int'",
        SerialisableValue::Json(Value::String(_)) => "class 'str'",
        SerialisableValue::Json(Value::Array(_)) => "class 'list'",
        SerialisableValue::Json(Value::Object(_)) => "class 'dict'",
        SerialisableValue::Bytes(_) => "class 'bytes'",
        SerialisableValue::Object(_) => "SerialisableBase",
    }
}

//endregion

//region SerialisableDictionary

/// `SerialisableDictionary`. Insertion order is preserved (`IndexMap`), which
/// matches Python's `dict` and keeps dumps stable.
#[derive(Clone, Debug, Default)]
pub struct SerialisableDictionary {
    entries: IndexMap<SerialisableValue, SerialisableValue>,
}

impl SerialisableDictionary {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_json_object(map: serde_json::Map<String, Value>) -> Self {
        let mut dict = Self::new();

        for (key, value) in map {
            dict.insert(
                SerialisableValue::Json(Value::String(key)),
                SerialisableValue::Json(value),
            );
        }

        dict
    }

    pub fn insert(
        &mut self,
        key: impl Into<SerialisableValue>,
        value: impl Into<SerialisableValue>,
    ) -> Option<SerialisableValue> {
        self.entries.insert(key.into(), value.into())
    }

    pub fn get(&self, key: &SerialisableValue) -> Option<&SerialisableValue> {
        self.entries.get(key)
    }

    pub fn get_mut(&mut self, key: &SerialisableValue) -> Option<&mut SerialisableValue> {
        self.entries.get_mut(key)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn iter(&self) -> indexmap::map::Iter<'_, SerialisableValue, SerialisableValue> {
        self.entries.iter()
    }

    pub fn keys(&self) -> indexmap::map::Keys<'_, SerialisableValue, SerialisableValue> {
        self.entries.keys()
    }

    pub fn values(&self) -> indexmap::map::Values<'_, SerialisableValue, SerialisableValue> {
        self.entries.values()
    }

    pub fn into_inner(self) -> IndexMap<SerialisableValue, SerialisableValue> {
        self.entries
    }

    fn describe_entries(descriptions: impl Iterator<Item = (String, String)>) -> String {
        let mut counter: IndexMap<String, usize> = IndexMap::new();

        for (key, value) in descriptions {
            *counter.entry(format!("{key}: {value}")).or_insert(0) += 1;
        }

        let mut result = String::from("Serialisable Dictionary (21)\n");

        let lines: Vec<String> = counter
            .into_iter()
            .map(|(description, count)| {
                format!(
                    "{}: {}",
                    hydrus_numbers::base_to_human_int(count as i128),
                    description
                )
            })
            .collect();

        result.push_str(&lines.join("\n"));

        result
    }
}

impl std::ops::Deref for SerialisableDictionary {
    type Target = IndexMap<SerialisableValue, SerialisableValue>;

    fn deref(&self) -> &Self::Target {
        &self.entries
    }
}

impl std::ops::DerefMut for SerialisableDictionary {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.entries
    }
}

impl IntoIterator for SerialisableDictionary {
    type Item = (SerialisableValue, SerialisableValue);
    type IntoIter = indexmap::map::IntoIter<SerialisableValue, SerialisableValue>;

    fn into_iter(self) -> Self::IntoIter {
        self.entries.into_iter()
    }
}

impl SerialisableObject for SerialisableDictionary {
    fn serialisable_type(&self) -> u16 {
        SERIALISABLE_TYPE_DICTIONARY
    }

    fn serialisable_version(&self) -> u16 {
        2
    }

    fn serialisable_name(&self) -> &'static str {
        "Serialisable Dictionary"
    }

    fn get_serialisable_info(&self) -> Value {
        let mut meta_keys_and_meta_values = Vec::with_capacity(self.entries.len());

        for (key, value) in &self.entries {
            let meta_key = key.to_meta_tuple().unwrap_or(Value::Null);
            let meta_value = value
                .clone()
                .coerce_for_serialisation()
                .to_meta_tuple()
                .unwrap_or(Value::Null);

            meta_keys_and_meta_values.push(json!([meta_key, meta_value]));
        }

        Value::Array(meta_keys_and_meta_values)
    }

    fn initialise_from_serialisable_info(&mut self, serialisable_info: &Value) -> HydrusResult<()> {
        let pairs = serialisable_info
            .as_array()
            .ok_or_else(|| serialisation_error("Serialisable dictionary info was not an array!"))?;

        let mut have_shown_load_error = false;

        for pair in pairs {
            let pair = pair.as_array().ok_or_else(|| {
                serialisation_error("Serialisable dictionary pair was not an array!")
            })?;

            let (Some(meta_key), Some(meta_value)) = (pair.first(), pair.get(1)) else {
                return Err(serialisation_error(
                    "Serialisable dictionary pair was not length 2!",
                ));
            };

            let converted = (|| -> HydrusResult<(SerialisableValue, SerialisableValue)> {
                let key = SerialisableValue::from_meta_tuple(meta_key)?;

                // Python converts list keys to tuples. Value arrays already
                // compare by content, so they serve the same purpose here.
                let value = SerialisableValue::from_meta_tuple(meta_value)?;

                Ok((key, value))
            })();

            match converted {
                Ok((key, value)) => {
                    self.entries.insert(key, value);
                }
                Err(e) => {
                    if !have_shown_load_error {
                        safe_print(
                            "An object in a dictionary could not load. It has been discarded from the dictionary. More may also have failed to load, but to stop error spam, they will go silently. Your client may be running on code versions behind its database. Depending on the severity of this error, you may need to rollback to a previous backup. If you have no backup, you may want to kill your hydrus process now to stop the cleansed dictionary ever being saved back to the db.",
                        );
                        safe_print(e.to_string());

                        have_shown_load_error = true;
                    }
                }
            }
        }

        Ok(())
    }

    fn update_serialisable_info(
        &self,
        version: u16,
        old_serialisable_info: Value,
    ) -> HydrusResult<(u16, Value)> {
        if version != 1 {
            return Ok((version, old_serialisable_info));
        }

        let groups = old_serialisable_info.as_array().ok_or_else(|| {
            serialisation_error("Serialisable dictionary v1 info was not an array!")
        })?;

        if groups.len() != 4 {
            return Err(serialisation_error(
                "Serialisable dictionary v1 info was not length 4!",
            ));
        }

        let mut meta_keys_and_meta_values = Vec::new();

        // The v1 layout was four buckets of (key, value) pairs.
        for (bucket_index, group) in groups.iter().enumerate() {
            let pairs = group.as_array().ok_or_else(|| {
                serialisation_error("Serialisable dictionary v1 bucket was not an array!")
            })?;

            let (key_metatype, value_metatype) = match bucket_index {
                0 => (
                    META_SERIALISABLE_TYPE_JSON_OK,
                    META_SERIALISABLE_TYPE_JSON_OK,
                ),
                1 => (
                    META_SERIALISABLE_TYPE_JSON_OK,
                    META_SERIALISABLE_TYPE_HYDRUS_SERIALISABLE,
                ),
                2 => (
                    META_SERIALISABLE_TYPE_HYDRUS_SERIALISABLE,
                    META_SERIALISABLE_TYPE_JSON_OK,
                ),
                _ => (
                    META_SERIALISABLE_TYPE_HYDRUS_SERIALISABLE,
                    META_SERIALISABLE_TYPE_HYDRUS_SERIALISABLE,
                ),
            };

            for pair in pairs {
                let pair = pair.as_array().ok_or_else(|| {
                    serialisation_error("Serialisable dictionary v1 pair was not an array!")
                })?;

                let (Some(key), Some(value)) = (pair.first(), pair.get(1)) else {
                    return Err(serialisation_error(
                        "Serialisable dictionary v1 pair was not length 2!",
                    ));
                };

                meta_keys_and_meta_values
                    .push(json!([[key_metatype, key], [value_metatype, value]]));
            }
        }

        Ok((2, Value::Array(meta_keys_and_meta_values)))
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn get_serialisable_description(&self) -> String {
        Self::describe_entries(
            self.entries
                .iter()
                .map(|(key, value)| (describe_slot(key), describe_slot(value))),
        )
    }
}

fn describe_slot(value: &SerialisableValue) -> String {
    match value {
        SerialisableValue::Object(obj) => obj.get_serialisable_description(),
        other => value_kind_name(other).to_string(),
    }
}

//endregion

//region SerialisableBytesDictionary

#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum BytesDictionaryKey {
    Int(i64),
    Bytes(Vec<u8>),
}

impl BytesDictionaryKey {
    pub fn as_int(&self) -> Option<i64> {
        match self {
            Self::Int(value) => Some(*value),
            Self::Bytes(_) => None,
        }
    }

    pub fn as_bytes(&self) -> Option<&[u8]> {
        match self {
            Self::Int(_) => None,
            Self::Bytes(bytes) => Some(bytes),
        }
    }
}

impl From<i64> for BytesDictionaryKey {
    fn from(value: i64) -> Self {
        Self::Int(value)
    }
}

impl From<Vec<u8>> for BytesDictionaryKey {
    fn from(value: Vec<u8>) -> Self {
        Self::Bytes(value)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum BytesDictionaryValue {
    None,
    Bytes(Vec<u8>),
    BytesList(Vec<Vec<u8>>),
}

impl BytesDictionaryValue {
    pub fn as_bytes(&self) -> Option<&[u8]> {
        match self {
            Self::Bytes(bytes) => Some(bytes),
            _ => None,
        }
    }

    pub fn as_bytes_list(&self) -> Option<&[Vec<u8>]> {
        match self {
            Self::BytesList(items) => Some(items),
            _ => None,
        }
    }
}

impl From<Vec<u8>> for BytesDictionaryValue {
    fn from(value: Vec<u8>) -> Self {
        Self::Bytes(value)
    }
}

/// `SerialisableBytesDictionary`: integer/bytes keys mapped to bytes, lists of
/// bytes or null. Kept as ordered pairs like Python's dict.
#[derive(Clone, Debug, Default)]
pub struct SerialisableBytesDictionary {
    entries: Vec<(BytesDictionaryKey, BytesDictionaryValue)>,
}

impl SerialisableBytesDictionary {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(
        &mut self,
        key: BytesDictionaryKey,
        value: BytesDictionaryValue,
    ) -> Option<BytesDictionaryValue> {
        if let Some((_, existing)) = self.entries.iter_mut().find(|(k, _)| *k == key) {
            return Some(std::mem::replace(existing, value));
        }

        self.entries.push((key, value));

        None
    }

    pub fn get(&self, key: &BytesDictionaryKey) -> Option<&BytesDictionaryValue> {
        self.entries.iter().find(|(k, _)| k == key).map(|(_, v)| v)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &(BytesDictionaryKey, BytesDictionaryValue)> {
        self.entries.iter()
    }
}

impl SerialisableObject for SerialisableBytesDictionary {
    fn serialisable_type(&self) -> u16 {
        SERIALISABLE_TYPE_BYTES_DICT
    }

    fn serialisable_version(&self) -> u16 {
        1
    }

    fn serialisable_name(&self) -> &'static str {
        "Serialisable Dictionary With Bytestring Key/Value Support"
    }

    fn get_serialisable_info(&self) -> Value {
        let pairs: Vec<Value> = self
            .entries
            .iter()
            .map(|(key, value)| {
                let encoded_key = match key {
                    BytesDictionaryKey::Int(value) => json!(value),
                    BytesDictionaryKey::Bytes(bytes) => json!(hex_encode(bytes)),
                };

                let encoded_value = match value {
                    BytesDictionaryValue::None => Value::Null,
                    BytesDictionaryValue::Bytes(bytes) => json!(hex_encode(bytes)),
                    BytesDictionaryValue::BytesList(items) => {
                        json!(items.iter().map(|b| hex_encode(b)).collect::<Vec<_>>())
                    }
                };

                json!([encoded_key, encoded_value])
            })
            .collect();

        Value::Array(pairs)
    }

    fn initialise_from_serialisable_info(&mut self, serialisable_info: &Value) -> HydrusResult<()> {
        let pairs = serialisable_info.as_array().ok_or_else(|| {
            serialisation_error("Serialisable bytes dictionary info was not an array!")
        })?;

        for pair in pairs {
            let pair = pair.as_array().ok_or_else(|| {
                serialisation_error("Serialisable bytes dictionary pair was not an array!")
            })?;

            let (Some(encoded_key), Some(encoded_value)) = (pair.first(), pair.get(1)) else {
                return Err(serialisation_error(
                    "Serialisable bytes dictionary pair was not length 2!",
                ));
            };

            let key = match encoded_key.as_i64() {
                Some(value) => BytesDictionaryKey::Int(value),
                None => {
                    let hex = encoded_key.as_str().ok_or_else(|| {
                        serialisation_error("Bytes dictionary key was not an int or hex string!")
                    })?;

                    BytesDictionaryKey::Bytes(hex_decode(hex).map_err(|_| {
                        serialisation_error("Bytes dictionary key was not valid hex!")
                    })?)
                }
            };

            let value = if encoded_value.is_null() {
                BytesDictionaryValue::None
            } else if let Some(items) = encoded_value.as_array() {
                let mut decoded = Vec::with_capacity(items.len());

                for item in items {
                    let hex = item.as_str().ok_or_else(|| {
                        serialisation_error("Bytes dictionary list item was not a hex string!")
                    })?;

                    decoded.push(hex_decode(hex).map_err(|_| {
                        serialisation_error("Bytes dictionary list item was not valid hex!")
                    })?);
                }

                BytesDictionaryValue::BytesList(decoded)
            } else {
                let hex = encoded_value.as_str().ok_or_else(|| {
                    serialisation_error("Bytes dictionary value was not a hex string!")
                })?;

                BytesDictionaryValue::Bytes(hex_decode(hex).map_err(|_| {
                    serialisation_error("Bytes dictionary value was not valid hex!")
                })?)
            };

            self.insert(key, value);
        }

        Ok(())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn get_serialisable_description(&self) -> String {
        let mut counter: IndexMap<String, usize> = IndexMap::new();

        for (key, value) in &self.entries {
            let key_str = match key {
                BytesDictionaryKey::Int(_) => "class 'int'".to_string(),
                BytesDictionaryKey::Bytes(_) => "class 'bytes'".to_string(),
            };

            let value_str = match value {
                BytesDictionaryValue::None => "class 'NoneType'".to_string(),
                BytesDictionaryValue::Bytes(_) => "class 'bytes'".to_string(),
                BytesDictionaryValue::BytesList(_) => "class 'list'".to_string(),
            };

            *counter
                .entry(format!("{key_str}: {value_str}"))
                .or_insert(0) += 1;
        }

        let mut result =
            String::from("Serialisable Dictionary With Bytestring Key/Value Support (33)\n");

        let lines: Vec<String> = counter
            .into_iter()
            .map(|(description, count)| {
                format!(
                    "{}: {}",
                    hydrus_numbers::base_to_human_int(count as i128),
                    description
                )
            })
            .collect();

        result.push_str(&lines.join("\n"));

        result
    }
}

//endregion

//region IdAndName

/// `IdAndName`: a bytestring id paired with a human name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IdAndName {
    pub object_id: Vec<u8>,
    pub name: String,
}

impl Default for IdAndName {
    fn default() -> Self {
        Self {
            object_id: b"abcd".to_vec(),
            name: "unknown".to_string(),
        }
    }
}

impl IdAndName {
    pub fn new(object_id: impl Into<Vec<u8>>, name: impl Into<String>) -> Self {
        Self {
            object_id: object_id.into(),
            name: name.into(),
        }
    }
}

impl Hash for IdAndName {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.object_id.hash(state);
    }
}

impl std::fmt::Display for IdAndName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "\"{}\" (id {})", self.name, hex_encode(&self.object_id))
    }
}

impl SerialisableObject for IdAndName {
    fn serialisable_type(&self) -> u16 {
        SERIALISABLE_TYPE_ID_AND_NAME
    }

    fn serialisable_version(&self) -> u16 {
        1
    }

    fn serialisable_name(&self) -> &'static str {
        "Serialisable Object ID and Name"
    }

    fn get_serialisable_info(&self) -> Value {
        json!([hex_encode(&self.object_id), self.name])
    }

    fn initialise_from_serialisable_info(&mut self, serialisable_info: &Value) -> HydrusResult<()> {
        let items = serialisable_info
            .as_array()
            .ok_or_else(|| serialisation_error("IdAndName info was not an array!"))?;

        let (Some(id_serialisable), Some(name)) = (items.first(), items.get(1)) else {
            return Err(serialisation_error("IdAndName info was not length 2!"));
        };

        let id_hex = id_serialisable
            .as_str()
            .ok_or_else(|| serialisation_error("IdAndName id was not a hex string!"))?;

        self.object_id = hex_decode(id_hex)
            .map_err(|_| serialisation_error("IdAndName id was not valid hex!"))?;
        self.name = name
            .as_str()
            .ok_or_else(|| serialisation_error("IdAndName name was not a string!"))?
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

//endregion

// Built-in registrations. Other modules register their own types when ported.
fn register_builtins() {
    register_serialisable_type(SERIALISABLE_TYPE_LIST, || Box::new(SerialisableList::new()));
    register_serialisable_type(SERIALISABLE_TYPE_DICTIONARY, || {
        Box::new(SerialisableDictionary::new())
    });
    register_serialisable_type(SERIALISABLE_TYPE_BYTES_DICT, || {
        Box::new(SerialisableBytesDictionary::new())
    });
    register_serialisable_type(SERIALISABLE_TYPE_ID_AND_NAME, || {
        Box::new(IdAndName::default())
    });
}

static BUILTINS: OnceLock<()> = OnceLock::new();

fn ensure_builtins_registered() {
    BUILTINS.get_or_init(register_builtins);
}

/// Call once from library initialisation; also called lazily by the public
/// loaders. Kept public so a binary can force it for diagnostics.
pub fn initialise_serialisable_registry() {
    ensure_builtins_registered();
}

// Make the built-ins available even if nobody calls the explicit initialiser.
// (`create_from_*` calls this via `instantiate_serialisable`.)
fn ensure_builtins_once() {
    ensure_builtins_registered();
}

#[cfg(test)]
mod tests {
    use super::*;

    // The loaders must have built-ins available before the first lookup.
    fn load(value: &Value) -> Box<dyn SerialisableObject> {
        ensure_builtins_once();

        create_from_serialisable_tuple(value, false).unwrap()
    }

    #[derive(Default)]
    struct TestNamed {
        name: String,
        number: i64,
    }

    impl SerialisableObject for TestNamed {
        fn serialisable_type(&self) -> u16 {
            16_000
        }

        fn serialisable_name(&self) -> &'static str {
            "Test Named"
        }

        fn get_serialisable_info(&self) -> Value {
            json!([self.number])
        }

        fn initialise_from_serialisable_info(&mut self, info: &Value) -> HydrusResult<()> {
            self.number = info[0].as_i64().unwrap_or_default();

            Ok(())
        }

        fn get_name(&self) -> Option<&str> {
            Some(&self.name)
        }

        fn set_name(&mut self, name: &str) {
            self.name = name.to_string();
        }

        fn as_any(&self) -> &dyn Any {
            self
        }

        fn as_any_mut(&mut self) -> &mut dyn Any {
            self
        }
    }

    fn test_named_factory(name: &str) -> Box<dyn SerialisableObject> {
        Box::new(TestNamed {
            name: name.to_string(),
            number: 0,
        })
    }

    #[test]
    fn python_json_dumps_matches_python_defaults() {
        let value = json!({
            "a": 1,
            "b": [true, null, "x"],
            "c": "caf\u{e9}"
        });

        // serde_json without preserve_order sorts keys; the escaping and
        // separators are what matter here.
        let dumped = python_json_dumps(&value);

        assert!(dumped.contains("\"caf\\u00e9\""));
        assert!(dumped.contains("\"a\": 1"));
        assert!(dumped.contains(", "));
    }

    #[test]
    fn id_and_name_round_trip() {
        ensure_builtins_once();

        let obj = IdAndName::new(vec![0x01, 0xab, 0xff], "hello");

        let dumped = obj.dump_to_string();
        let loaded = create_from_string(&dumped, false).unwrap();
        let loaded = loaded.as_any().downcast_ref::<IdAndName>().unwrap();

        assert_eq!(loaded, &obj);
        assert_eq!(loaded.to_string(), "\"hello\" (id 01abff)");
    }

    #[test]
    fn list_round_trip_keeps_nested_shapes() {
        ensure_builtins_once();

        let mut list = SerialisableList::new();
        list.push(1i64);
        list.push("two");
        list.push(vec![0xde, 0xad]);
        list.push(Value::from("plain"));
        list.push(SerialisableValue::Object(Arc::new(IdAndName::new(
            b"id".to_vec(),
            "inner",
        ))));

        let dumped = list.dump_to_string();
        let loaded = create_from_string(&dumped, false).unwrap();
        let loaded = loaded.as_any().downcast_ref::<SerialisableList>().unwrap();

        assert_eq!(loaded.len(), 5);
        assert_eq!(loaded[0].as_json(), Some(&json!(1)));
        assert_eq!(loaded[1].as_json(), Some(&json!("two")));
        assert_eq!(loaded[2].as_bytes(), Some(&[0xde, 0xad][..]));
        // A raw JSON string stays JSON; only lists/dicts/serialisables become objects.
        assert!(loaded[3].as_object().is_none());
        assert_eq!(
            loaded[4].as_object().unwrap().serialisable_type(),
            SERIALISABLE_TYPE_ID_AND_NAME
        );
    }

    #[test]
    fn list_upgrades_from_version_one() {
        // v1 held bare serialisable tuples, all of which were presumed
        // serialised. They become the v2 `(true, obj)` form and then v3 metas.
        let loaded = load(&json!([
            SERIALISABLE_TYPE_LIST,
            1,
            [
                [SERIALISABLE_TYPE_ID_AND_NAME, 1, ["00ff", "old"]],
                [SERIALISABLE_TYPE_ID_AND_NAME, 1, ["0102", "older"]]
            ]
        ]));
        let loaded = loaded.as_any().downcast_ref::<SerialisableList>().unwrap();

        assert_eq!(loaded.len(), 2);
        assert_eq!(
            loaded[0].as_object().unwrap().serialisable_type(),
            SERIALISABLE_TYPE_ID_AND_NAME
        );
    }

    #[test]
    fn list_upgrades_from_version_two() {
        let loaded = load(&json!([
            SERIALISABLE_TYPE_LIST,
            2,
            [
                [false, "raw"],
                [true, [SERIALISABLE_TYPE_ID_AND_NAME, 1, ["00ff", "named"]]]
            ]
        ]));

        let loaded = loaded.as_any().downcast_ref::<SerialisableList>().unwrap();

        assert_eq!(loaded[0].as_json(), Some(&json!("raw")));
        assert_eq!(
            loaded[1].as_object().unwrap().serialisable_type(),
            SERIALISABLE_TYPE_ID_AND_NAME
        );
    }

    #[test]
    fn dictionary_round_trip_with_bytes_and_nested_lists() {
        ensure_builtins_once();

        let mut dict = SerialisableDictionary::new();
        dict.insert("key", 5i64);
        dict.insert(
            SerialisableValue::Bytes(b"raw".to_vec()),
            SerialisableValue::Bytes(vec![1, 2, 3]),
        );
        dict.insert(
            "nested",
            SerialisableValue::Object(Arc::new(SerialisableList::from_values(vec![
                SerialisableValue::Json(json!(1)),
                SerialisableValue::Json(json!("x")),
            ]))),
        );

        let dumped = dict.dump_to_string();
        let loaded = create_from_string(&dumped, false).unwrap();
        let loaded = loaded
            .as_any()
            .downcast_ref::<SerialisableDictionary>()
            .unwrap();

        assert_eq!(loaded.len(), 3);
        assert_eq!(
            loaded
                .get(&SerialisableValue::Json(json!("key")))
                .and_then(SerialisableValue::as_json),
            Some(&json!(5))
        );
    }

    #[test]
    fn dictionary_upgrades_from_version_one() {
        let loaded = load(&json!([
            SERIALISABLE_TYPE_DICTIONARY,
            1,
            [
                [["a", 1]],
                [["b", [SERIALISABLE_TYPE_ID_AND_NAME, 1, ["00ff", "v"]]]],
                [[[SERIALISABLE_TYPE_ID_AND_NAME, 1, ["00ff", "n"]], "c"]],
                []
            ]
        ]));

        let loaded = loaded
            .as_any()
            .downcast_ref::<SerialisableDictionary>()
            .unwrap();

        assert_eq!(loaded.len(), 3);
    }

    #[test]
    fn bytes_dictionary_round_trip() {
        ensure_builtins_once();

        let mut dict = SerialisableBytesDictionary::new();
        dict.insert(5.into(), vec![1, 2].into());
        dict.insert(
            BytesDictionaryKey::Bytes(b"k".to_vec()),
            BytesDictionaryValue::BytesList(vec![vec![3], vec![4, 5]]),
        );
        dict.insert(7.into(), BytesDictionaryValue::None);

        let dumped = dict.dump_to_string();
        let loaded = create_from_string(&dumped, false).unwrap();
        let loaded = loaded
            .as_any()
            .downcast_ref::<SerialisableBytesDictionary>()
            .unwrap();

        assert_eq!(loaded.len(), 3);
        assert_eq!(
            loaded
                .get(&BytesDictionaryKey::Bytes(b"k".to_vec()))
                .and_then(BytesDictionaryValue::as_bytes_list),
            Some(&[vec![3], vec![4, 5]][..])
        );
    }

    #[test]
    fn named_tuple_round_trip() {
        ensure_builtins_once();
        register_named_serialisable_type(16_000, test_named_factory);

        let obj = TestNamed {
            name: "hello".to_string(),
            number: 42,
        };

        let tuple = obj.get_serialisable_tuple();
        let items = tuple.as_array().unwrap();

        assert_eq!(items.len(), 4);
        assert_eq!(items[1], json!("hello"));

        let loaded = create_from_serialisable_tuple(&tuple, false).unwrap();
        let loaded = loaded.as_any().downcast_ref::<TestNamed>().unwrap();

        assert_eq!(loaded.get_name(), Some("hello"));
        assert_eq!(loaded.number, 42);
    }

    #[test]
    fn future_version_raises_when_asked() {
        ensure_builtins_once();

        let tuple = json!([SERIALISABLE_TYPE_ID_AND_NAME, 99, ["abcd", "x"]]);

        let err = create_from_serialisable_tuple(&tuple, true).unwrap_err();

        assert_eq!(err.kind, ErrorKind::Serialisation);
        assert!(err.to_string().contains("updated version"));
    }

    #[test]
    fn unknown_type_is_a_serialisation_error() {
        ensure_builtins_once();

        let err = create_from_serialisable_tuple(&json!([60_000, 1, []]), false).unwrap_err();

        assert_eq!(err.kind, ErrorKind::Serialisation);
    }

    #[test]
    fn network_round_trip_compresses() {
        ensure_builtins_once();

        let obj = IdAndName::new(b"xyz".to_vec(), "net");
        let bytes = obj.dump_to_network_bytes().unwrap();

        assert_eq!(bytes[0], 0x78); // zlib header

        let loaded = create_from_network_bytes(&bytes, false).unwrap();

        assert_eq!(loaded.as_any().downcast_ref::<IdAndName>().unwrap(), &obj);
    }

    #[test]
    fn serialised_hash_is_sha256_of_dump() {
        ensure_builtins_once();

        let obj = IdAndName::new(b"id".to_vec(), "hashme");
        let hash = obj.get_serialised_hash();

        assert_eq!(hash.len(), 32);
        assert_eq!(
            hash,
            Sha256::digest(obj.dump_to_string().as_bytes()).to_vec()
        );
    }

    #[test]
    fn duplicate_recreates_independent_object() {
        ensure_builtins_once();

        let obj = IdAndName::new(b"id".to_vec(), "copy");
        let copy = obj.duplicate().unwrap();

        assert!(copy.as_any().downcast_ref::<IdAndName>().is_some());
    }
}
