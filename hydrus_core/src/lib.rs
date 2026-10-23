//! A faithful Rust port of hydrus's core library.
//!
//! The Python package is `hydrus.core`; each module maps to a snake_case Rust
//! module of the same name (`HydrusConstants.py` -> [`hydrus_constants`]).
//!
//! The directory layout mirrors the Python package: `hydrus/core/files`,
//! `hydrus/core/files/images`, `hydrus/core/networking` and
//! `hydrus/core/processes` become [`files`], [`files::images`], [`networking`]
//! and [`processes`]. Everything Python keeps at the top of `hydrus/core` stays
//! in this file's own directory. Each directory has a `mod.rs`, the analogue of
//! the Python package's `__init__.py`.
//!
//! Only the platform-independent "brains" are ported here. Anything that
//! needs Qt, numpy/PIL, ffmpeg or the client UI lives in the client crate.

pub mod files;
pub mod hydrus_boot;
pub mod hydrus_compression;
pub mod hydrus_constants;
pub mod hydrus_controller;
pub mod hydrus_data;
pub mod hydrus_date_time;
pub mod hydrus_db;
pub mod hydrus_db_base;
pub mod hydrus_db_module;
pub mod hydrus_db_populate_cache;
pub mod hydrus_encryption;
pub mod hydrus_environment;
pub mod hydrus_exceptions;
pub mod hydrus_globals;
pub mod hydrus_lists;
pub mod hydrus_logger;
pub mod hydrus_memory;
pub mod hydrus_numbers;
pub mod hydrus_paths;
pub mod hydrus_profiling;
pub mod hydrus_psutil;
pub mod hydrus_pubsub;
pub mod hydrus_rating_archive;
pub mod hydrus_serialisable;
pub mod hydrus_sessions;
pub mod hydrus_static_dir;
pub mod hydrus_tag_archive;
pub mod hydrus_tags;
pub mod hydrus_temp;
pub mod hydrus_text;
pub mod hydrus_time;
pub mod networking;
pub mod processes;

/// Register every ported serialisable type.
///
/// Python gets this for free at import time; Rust modules have no import side
/// effects, so binaries and tests call this once during boot.
pub fn initialise() {
    hydrus_serialisable::initialise_serialisable_registry();
    hydrus_tags::register_serialisable_types();
    networking::hydrus_networking::register_serialisable_types();
    networking::hydrus_network::register_serialisable_types();
}
