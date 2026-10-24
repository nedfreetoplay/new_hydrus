//! HTTP/TLS integration for the server.
//!
//! The core crate deliberately stops at request/response policy. This module
//! supplies the actual listener: it accepts TLS connections, turns hyper
//! requests into [`HydrusRequest`] values, dispatches through the server route
//! tree, and writes the resulting [`RenderPlan`] back to the network.

use std::convert::Infallible;
use std::io::Cursor;
use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr};
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Duration;

use bytes::Bytes;
use http_body_util::{BodyExt, Full, Limited};
use hyper::body::Incoming;
use hyper::service::service_fn;
use hyper::{Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use indexmap::IndexMap;
use rustls::pki_types::CertificateDer;
use tokio::net::{TcpListener, TcpSocket};
use tokio_rustls::TlsAcceptor;

use hydrus_core::hydrus_constants::{SOFTWARE_VERSION, ServiceType};
use hydrus_core::hydrus_exceptions::{ErrorKind, HydrusException, HydrusResult};
use hydrus_core::networking::hydrus_network::ServerService;
use hydrus_core::networking::hydrus_network_variable_handling::{
    ParsedRequestArguments, parse_hydrus_network_get_args,
    parse_network_bytes_to_parsed_hydrus_args, percent_decode,
};
use hydrus_core::networking::hydrus_server::{
    FAT_HTTP_CHANNEL_MAX_LENGTH, HydrusService,
};
use hydrus_core::networking::hydrus_server_context_factory::{
    TlsContextSettings, generate_ssl_context_settings,
};
use hydrus_core::networking::hydrus_server_request::{HydrusRequest, RequestHeaders};
use hydrus_core::networking::hydrus_server_resources::{
    HydrusResourceService, RenderPlan, ResponseContext, ResponseCookie, build_render_plan,
    error_response,
};

use crate::networking::server_server::ServerResourceTree;
use crate::networking::server_server_resources::{ServerResourceRuntime, render_resource};
use crate::server_controller::Controller;

include!("parts/part_01.rs");

include!("parts/part_02.rs");

include!("parts/part_03.rs");

include!("parts/part_04.rs");

include!("parts/part_05.rs");

include!("parts/part_06.rs");

#[cfg(test)]
mod tests;
