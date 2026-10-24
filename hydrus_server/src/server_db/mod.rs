//! Rust port of the server database layer from
//! `hydrus/server/ServerDB.py`.
//!
//! This first slice ports the schema, service registry and repository table
//! creation. Account registration keys, sessions and the repository content
//! update machinery are kept for the next layers; the table layout is already
//! the one those layers expect.

#![allow(dead_code)] // The remaining API is wired up by the resource/session layers.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use chrono::Datelike;
use serde_json::json;
use sha2::{Digest, Sha256};

use hydrus_core::hydrus_constants::{
    DEFAULT_SERVER_ADMIN_PORT, FILE_REPOSITORY_SERVICE_INFO_TYPES, Mime, REPOSITORIES,
    SERVER_ADMIN_KEY, SOFTWARE_VERSION, ContentStatus, ContentType, ContentUpdate,
    DefinitionsType, ServiceInfoType, ServiceType, TAG_REPOSITORY_SERVICE_INFO_TYPES,
    TIMEZONE_LOCAL,
};
use hydrus_core::hydrus_data::generate_key;
use hydrus_core::hydrus_db::{DBController, DBHooks, HydrusDB, HydrusDBConfig};
use hydrus_core::hydrus_db_base::{DBBase, DBCursorTransactionWrapper};
use hydrus_core::hydrus_exceptions::{ErrorKind, HydrusException, HydrusResult};
use hydrus_core::hydrus_pubsub::Subscriber;
use hydrus_core::hydrus_serialisable::{
    SerialisableDictionary, SerialisableObject, create_from_string,
};
use hydrus_core::hydrus_sessions::{
    Account as SessionAccountTrait, ExistingSession, SessionController,
};
use hydrus_core::hydrus_time;
use hydrus_core::networking::hydrus_network::{
    Account as NetworkAccount, AccountType, ClientToServerUpdate, Content, ContentData,
    ContentUpdatePackage, ContentUpdateRow, DefinitionValue, DefinitionsUpdate,
    DefinitionsUpdateRow, Petition, PetitionHeader, ServerService, UpdateBuilder, generate_service,
    generate_service_from_serialisable_tuple,
};

use crate::server_files::ServerFiles;

/// The file dictionary Python passes to `_add_file`.
#[derive(Clone, Debug)]
pub struct ServerFileInput {
    pub hash: Vec<u8>,
    pub size: u64,
    pub mime: Mime,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub duration_ms: Option<i64>,
    pub num_frames: Option<u32>,
    pub num_words: Option<i64>,
    pub path: PathBuf,
    pub thumbnail: Option<Vec<u8>>,
}

include!("parts/part_01.rs");

include!("parts/part_02.rs");

include!("parts/part_03.rs");

include!("parts/part_04.rs");

include!("parts/part_05.rs");

include!("parts/part_06.rs");

include!("parts/part_07.rs");

include!("parts/part_08.rs");

include!("parts/part_09.rs");

include!("parts/part_10.rs");

include!("parts/part_11.rs");

include!("parts/part_12.rs");

include!("parts/part_13.rs");

include!("parts/part_14.rs");

include!("parts/part_15.rs");

include!("parts/part_16.rs");

include!("parts/part_17.rs");

include!("parts/part_18.rs");

include!("parts/part_18b.rs");

include!("parts/part_18c.rs");

include!("parts/part_19.rs");

include!("parts/part_20.rs");

include!("parts/part_21.rs");

include!("parts/part_22.rs");

include!("parts/part_23.rs");

include!("parts/part_24.rs");

include!("parts/part_25.rs");

include!("parts/part_26.rs");

include!("parts/part_27.rs");

include!("parts/part_28.rs");

include!("parts/part_29.rs");

include!("parts/part_30.rs");

include!("parts/part_30b.rs");

include!("parts/part_30c.rs");

include!("parts/part_30d.rs");

include!("parts/part_30e.rs");

include!("parts/part_31.rs");

include!("parts/part_32.rs");

include!("parts/part_33.rs");

include!("parts/part_34.rs");

include!("parts/part_35.rs");

include!("parts/part_36.rs");

include!("parts/part_37.rs");

include!("parts/part_38.rs");

include!("parts/part_39.rs");

include!("parts/part_40.rs");

#[cfg(test)]
mod tests;
