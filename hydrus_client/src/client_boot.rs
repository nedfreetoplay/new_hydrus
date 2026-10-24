use std::path::PathBuf;
use std::sync::OnceLock;

use hydrus_core::hydrus_exceptions::HydrusResult;
use hydrus_core::hydrus_logger::HydrusLogger;

static LOGGER: OnceLock<HydrusLogger> = OnceLock::new();

/// Performs the process-wide hydrus_core initialisation that must happen
/// before any client subsystem is constructed.
pub fn boot() -> HydrusResult<()> {
    let base_dir = base_dir();

    let dirs = hydrus_core::hydrus_constants::init_dirs(base_dir);
    let db_dir = dirs.default_db_dir.clone();
    hydrus_core::hydrus_globals::set_db_dir(db_dir.clone());
    hydrus_core::hydrus_globals::set_program_name("Hydrus Client");
    hydrus_core::initialise();
    hydrus_core::files::hydrus_file_handling::initialise_mimes_to_default_thumbnail_paths();
    crate::client_options::register_serialisable_type();

    let logger = HydrusLogger::new(db_dir, "client")?;
    let _ = LOGGER.set(logger);

    Ok(())
}

/// The process-wide client logger, available after [`boot`].
pub fn logger() -> Option<&'static HydrusLogger> {
    LOGGER.get()
}

fn base_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from("."))
}
