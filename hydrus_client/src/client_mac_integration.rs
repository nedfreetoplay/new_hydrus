//! macOS Quick Look integration from `hydrus/client/ClientMacIntegration.py`.
//!
//! The Python implementation binds PyObjC. Until the Rust client gains an
//! Objective-C layer, macOS asks the shell to preview the path and other
//! platforms report that the feature is unavailable.

use std::path::Path;
use std::process::Command;

use hydrus_core::hydrus_constants;
use hydrus_core::hydrus_exceptions::{ErrorKind, HydrusException, HydrusResult};

/// `show_quicklook_for_path( path )`
pub fn show_quicklook_for_path(path: &Path) -> HydrusResult<()> {
    if !hydrus_constants::PLATFORM_MACOS {
        return Err(HydrusException::new(
            ErrorKind::Unknown,
            "Quick Look is only available on macOS",
        ));
    }

    if !path.exists() {
        return Err(HydrusException::new(
            ErrorKind::FileMissing,
            format!("File does not exist: {}", path.display()),
        ));
    }

    Command::new("qlmanage")
        .arg("-p")
        .arg(path)
        .spawn()
        .map_err(|error| HydrusException::new(ErrorKind::Unknown, error.to_string()))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quicklook_is_not_available_off_macos() {
        if !hydrus_constants::PLATFORM_MACOS {
            assert!(show_quicklook_for_path(Path::new(".")).is_err());
        }
    }
}
