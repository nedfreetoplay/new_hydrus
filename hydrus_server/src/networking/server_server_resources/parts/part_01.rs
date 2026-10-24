use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use indexmap::IndexMap;
use hydrus_core::hydrus_constants::{ContentType, Mime, PermissionAction, REPOSITORIES};
use hydrus_core::hydrus_data::hex_to_none_or_bytes;
use hydrus_core::hydrus_exceptions::{ErrorKind, HydrusException, HydrusResult};
use hydrus_core::hydrus_sessions::{Account as SessionAccountTrait, HydrusSessionManagerServer};
use hydrus_core::hydrus_time::get_now;
use hydrus_core::networking::hydrus_network::{Content, ServerService};
use hydrus_core::networking::hydrus_network_variable_handling::{
    ArgValue, ParsedRequestArguments, dump_hydrus_args_to_network_bytes,
};
use hydrus_core::networking::hydrus_server_request::HydrusRequest;
use hydrus_core::networking::hydrus_server_resources::{
    ResponseBody, ResponseContext, ResponseCookie, apply_user_agent_check,
    parse_hydrus_network_access_key,
};
use crate::networking::server_server::{ServerResourceKind, ServerRoute};
use crate::server_db::ServerDB;
use crate::server_files::ServerFiles;
pub struct ServerResourceRuntime {
    pub db: Arc<ServerDB>,
    pub session_manager: Arc<HydrusSessionManagerServer>,
    pub service: Arc<ServerService>,
    pub busy: Arc<AtomicBool>,
    pub shutdown_requested: Arc<AtomicBool>,
}
