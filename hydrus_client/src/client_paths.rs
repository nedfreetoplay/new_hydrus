//! OS integration from `hydrus/client/ClientPaths.py`.

use std::path::Path;
use std::process::Command;

use hydrus_core::hydrus_constants;
use hydrus_core::hydrus_exceptions::{ErrorKind, HydrusException, HydrusResult};
use hydrus_core::hydrus_paths;

pub const CAN_OPEN_FILE_LOCATION: bool =
    hydrus_constants::PLATFORM_WINDOWS || hydrus_constants::PLATFORM_MACOS;

/// `delete_path( path, always_delete_fully = False )`
pub fn delete_path(
    path: &Path,
    always_delete_fully: bool,
    delete_to_recycle_bin: bool,
) -> HydrusResult<()> {
    let deleted = if delete_to_recycle_bin && !always_delete_fully {
        hydrus_paths::recycle_path(path)
    } else {
        hydrus_paths::delete_path(path)
    };

    if deleted {
        Ok(())
    } else {
        Err(HydrusException::new(
            ErrorKind::FileMissing,
            format!("Could not delete {}", path.display()),
        ))
    }
}

/// `get_default_launch_path()`
pub fn get_default_launch_path() -> &'static str {
    if hydrus_constants::PLATFORM_WINDOWS {
        "windows is called directly"
    } else if hydrus_constants::PLATFORM_MACOS || hydrus_constants::PLATFORM_LINUX {
        "open \"%path%\""
    } else {
        "xdg-open \"%path%\""
    }
}

/// `open_file_location( path )`
pub fn open_file_location(path: &Path) -> HydrusResult<()> {
    if !path.exists() {
        return Err(HydrusException::new(
            ErrorKind::FileMissing,
            format!("File does not exist: {}", path.display()),
        ));
    }

    hydrus_paths::open_file_location(path);

    Ok(())
}

pub fn open_file_locations(paths: &[impl AsRef<Path>]) -> HydrusResult<()> {
    for path in paths {
        open_file_location(path.as_ref())?;
    }

    Ok(())
}

/// `open_native_file_properties( path )`
pub fn open_native_file_properties(path: &Path) -> HydrusResult<()> {
    if hydrus_constants::PLATFORM_WINDOWS {
        Command::new("explorer.exe")
            .arg(format!("/select,{}", path.display()))
            .spawn()
            .map_err(|error| HydrusException::new(ErrorKind::FileMissing, error.to_string()))?;
    }

    Ok(())
}

/// `open_file_with_dialog( path )`
pub fn open_file_with_dialog(path: &Path) -> HydrusResult<()> {
    open_file_location(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn delete_removes_a_file() {
        let path =
            std::env::temp_dir().join(format!("hydrus_client_delete_{}.tmp", std::process::id()));
        std::fs::write(&path, b"x").unwrap();

        delete_path(&path, true, false).unwrap();

        assert!(!path.exists());
    }

    #[test]
    fn missing_file_is_an_error() {
        let path = std::env::temp_dir().join("hydrus_client_missing_file_xyz");
        let _ = std::fs::remove_file(&path);

        assert!(open_file_location(&path).is_err());
    }

    #[test]
    fn launch_path_is_platform_specific() {
        assert!(!get_default_launch_path().is_empty());
    }
}
