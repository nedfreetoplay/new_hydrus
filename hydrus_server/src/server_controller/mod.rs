//! Rust port of `hydrus/server/ServerController.py`.
//!
//! The controller is being built up in layers. This first layer owns the
//! server directory layout and installs the process-global controller that
//! `ServerFiles` reads. The database, service life cycle and HTTP listeners
//! will be added on top of this bootstrap without changing that contract.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock, Weak};

use hydrus_core::hydrus_constants::ServiceType;
use hydrus_core::hydrus_db::DBController;
use hydrus_core::hydrus_exceptions::{ErrorKind, HydrusException, HydrusResult};
use hydrus_core::hydrus_pubsub::PubSubMessage;
use hydrus_core::hydrus_sessions::HydrusSessionManagerServer;
use hydrus_core::networking::hydrus_server_request::HydrusRequest;
use hydrus_core::networking::hydrus_server_resources::ResponseContext;
use hydrus_core::processes::hydrus_process;

use crate::networking::server_http;
use crate::networking::server_server::ServerResourceTree;
use crate::networking::server_server_resources::{ServerResourceRuntime, render_resource};
use crate::server_db::{ServerDB, ServerSessionController};
use crate::server_files::ServerFiles;
use crate::server_globals::{self, ServerController};

/// The server's `Controller`.
pub struct Controller {
    db_dir: PathBuf,
    files_dir: PathBuf,
    db: OnceLock<Arc<ServerDB>>,
    session_manager: OnceLock<Arc<HydrusSessionManagerServer>>,
    busy: Arc<AtomicBool>,
    shutdown_requested: Arc<AtomicBool>,
    core_controller: OnceLock<Arc<hydrus_core::hydrus_controller::HydrusController>>,
}

include!("parts/part_01.rs");

include!("parts/part_02.rs");

include!("parts/part_03.rs");

include!("parts/part_04.rs");

include!("parts/part_05.rs");

include!("parts/part_06.rs");

#[cfg(test)]
mod tests;
