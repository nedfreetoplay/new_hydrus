//! A faithful Rust port of hydrus's core library.
//!
//! The Python package is `hydrus.core`; each module maps to a snake_case Rust
//! module of the same name (`HydrusConstants.py` -> [`hydrus_constants`]).
//!
//! Only the platform-independent "brains" are ported here. Anything that
//! needs Qt, numpy/PIL, ffmpeg or the client UI lives in the client crate.

pub mod hydrus_animation_handling;
pub mod hydrus_archive_handling;
pub mod hydrus_audio_handling;
pub mod hydrus_blurhash;
pub mod hydrus_boot;
pub mod hydrus_clip_handling;
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
pub mod hydrus_ffmpeg;
pub mod hydrus_file_handling;
pub mod hydrus_ffmpeg_parsing;
pub mod hydrus_ffmpeg_rendering;
pub mod hydrus_files_physical_storage;
pub mod hydrus_flash_handling;
pub mod hydrus_globals;
pub mod hydrus_image_colours;
pub mod hydrus_image_handling;
pub mod hydrus_image_icc_profiles;
pub mod hydrus_image_icc_profiles_tag_slop;
pub mod hydrus_image_init;
pub mod hydrus_image_math_slop;
pub mod hydrus_image_metadata;
pub mod hydrus_image_normalisation;
pub mod hydrus_image_opening;
pub mod hydrus_krita_handling;
pub mod hydrus_lists;
pub mod hydrus_logger;
pub mod hydrus_memory;
pub mod hydrus_network;
pub mod hydrus_network_legacy;
pub mod hydrus_network_variable_handling;
pub mod hydrus_networking;
pub mod hydrus_numbers;
pub mod hydrus_office_open_xml_handling;
pub mod hydrus_ole_handling;
pub mod hydrus_ora_handling;
pub mod hydrus_paint_net_handling;
pub mod hydrus_paths;
pub mod hydrus_pdf_handling;
pub mod hydrus_profiling;
pub mod hydrus_procreate_handling;
pub mod hydrus_psd_handling;
pub mod hydrus_psutil;
pub mod hydrus_pubsub;
pub mod hydrus_process;
pub mod hydrus_rating_archive;
pub mod hydrus_serialisable;
pub mod hydrus_server;
pub mod hydrus_server_context_factory;
pub mod hydrus_server_request;
pub mod hydrus_server_resources;
pub mod hydrus_sessions;
pub mod hydrus_static_dir;
pub mod hydrus_subprocess;
pub mod hydrus_svg_handling;
pub mod hydrus_tag_archive;
pub mod hydrus_tags;
pub mod hydrus_temp;
pub mod hydrus_text;
pub mod hydrus_threading;
pub mod hydrus_time;
pub mod hydrus_ugoira_handling;
pub mod hydrus_video_handling;

/// Register every ported serialisable type.
///
/// Python gets this for free at import time; Rust modules have no import side
/// effects, so binaries and tests call this once during boot.
pub fn initialise() {
    hydrus_serialisable::initialise_serialisable_registry();
    hydrus_tags::register_serialisable_types();
    hydrus_networking::register_serialisable_types();
    hydrus_network::register_serialisable_types();
}
