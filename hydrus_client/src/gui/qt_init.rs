//! Qt application initialisation from `hydrus/client/gui/QtInit.py`.

use hydrus_core::hydrus_exceptions::HydrusResult;

use crate::gui::qt::apply_application_settings;
use crate::gui::style::ClientGUIStyle;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QtInit {
    pub app_name: String,
    pub display_name: String,
    pub organization_name: String,
    pub style: ClientGUIStyle,
}

impl Default for QtInit {
    fn default() -> Self {
        Self {
            app_name: "hydrus_client".to_string(),
            display_name: "hydrus client".to_string(),
            organization_name: "Hydrus".to_string(),
            style: ClientGUIStyle::default(),
        }
    }
}

impl QtInit {
    pub fn new(style: ClientGUIStyle) -> Self {
        Self {
            style,
            ..Default::default()
        }
    }

    /// Applies the settings to the already-created `QApplication`.
    pub fn apply(&self) -> HydrusResult<()> {
        apply_application_settings(
            &self.app_name,
            &self.display_name,
            &self.organization_name,
            &self.style.style_name,
            &self.style.stylesheet(),
        );

        Ok(())
    }
}

/// Hydrus only supports Qt 6; this mirrors `QtInit`'s version gate.
pub fn is_supported_qt_version(version: &str) -> bool {
    version.starts_with("6.")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_the_client_branding() {
        let init = QtInit::default();

        assert_eq!(init.app_name, "hydrus_client");
        assert_eq!(init.display_name, "hydrus client");
    }

    #[test]
    fn only_qt6_is_supported() {
        assert!(is_supported_qt_version("6.12.0"));
        assert!(!is_supported_qt_version("5.15.2"));
    }
}
