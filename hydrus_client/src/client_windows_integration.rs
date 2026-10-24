//! Windows shell integration from `hydrus/client/ClientWindowsIntegration.py`.

use std::path::Path;
use std::process::Command;

use hydrus_core::hydrus_constants;
use hydrus_core::hydrus_exceptions::{ErrorKind, HydrusException, HydrusResult};

/// `OpenFileProperties( path )`
pub fn open_file_properties(path: &Path) -> HydrusResult<()> {
    if !hydrus_constants::PLATFORM_WINDOWS {
        return Err(unsupported(
            "file properties are only implemented on Windows",
        ));
    }

    if !path.exists() {
        return Err(HydrusException::new(
            ErrorKind::FileMissing,
            format!("File does not exist: {}", path.display()),
        ));
    }

    Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-Command",
            "Start-Process -FilePath $args[0] -Verb Properties",
            &path.display().to_string(),
        ])
        .spawn()
        .map_err(|error| HydrusException::new(ErrorKind::Unknown, error.to_string()))?;

    Ok(())
}

/// `OpenFileWith( path )`
pub fn open_file_with(path: &Path) -> HydrusResult<()> {
    if !hydrus_constants::PLATFORM_WINDOWS {
        return Err(unsupported("'open with' is only implemented on Windows"));
    }

    if !path.exists() {
        return Err(HydrusException::new(
            ErrorKind::FileMissing,
            format!("File does not exist: {}", path.display()),
        ));
    }

    Command::new("rundll32.exe")
        .arg("shell32.dll,OpenAs_RunDLL")
        .arg(path)
        .spawn()
        .map_err(|error| HydrusException::new(ErrorKind::Unknown, error.to_string()))?;

    Ok(())
}

/// `SetAppUserModelID`
///
/// A no-op until the client owns a taskbar/tray integration layer; the Python
/// implementation reaches into `ctypes`/`win32com` for this.
pub fn set_app_user_model_id(_app_id: &str) -> HydrusResult<()> {
    Ok(())
}

fn unsupported(message: &str) -> HydrusException {
    HydrusException::new(ErrorKind::Unknown, message)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_paths_are_rejected_before_spawning_shells() {
        let path = std::env::temp_dir().join("hydrus_client_definitely_missing_xyz");

        let _ = std::fs::remove_file(&path);

        if hydrus_constants::PLATFORM_WINDOWS {
            assert!(open_file_properties(&path).is_err());
            assert!(open_file_with(&path).is_err());
        }
    }

    #[test]
    fn app_user_model_id_is_accepted() {
        assert!(set_app_user_model_id("hydrus.client").is_ok());
    }
}
