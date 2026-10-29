//! Application styling from `hydrus/client/gui/ClientGUIStyle.py`.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientGUIStyle {
    pub darkmode: bool,
    pub style_name: String,
}

impl Default for ClientGUIStyle {
    fn default() -> Self {
        Self {
            darkmode: false,
            style_name: "windowsvista".to_string(),
        }
    }
}

impl ClientGUIStyle {
    pub fn darkmode() -> Self {
        Self {
            darkmode: true,
            ..Default::default()
        }
    }

    pub fn stylesheet(&self) -> String {
        // The Python client always prepends static/qss/default_hydrus.qss; the
        // default light theme adds nothing else on top of the native palette.
        const HYDRUS_DEFAULT_QSS: &str = "
            QPushButton#HydrusOnOffButton[hydrus_on=true] { color: #008000; }
            QPushButton#HydrusOnOffButton[hydrus_on=false] { color: #800000; }
        ";

        if self.darkmode {
            format!(
                "{HYDRUS_DEFAULT_QSS}
            QWidget {{ background-color: #343434; color: #d9e0e6; }}
            QTabWidget::pane {{ border: 1px solid #4b4b4b; }}
            QTabBar::tab:selected {{ background: #4b4b4b; }}
            QMenuBar, QMenu, QStatusBar {{ background-color: #2b2b2b; color: #d9e0e6; }}
            QTreeWidget {{ background-color: #2b2b2b; alternate-background-color: #343434; }}
            "
            )
        } else {
            HYDRUS_DEFAULT_QSS.to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn light_and_dark_stylesheets_differ() {
        let light = ClientGUIStyle::default();
        let dark = ClientGUIStyle::darkmode();

        assert!(light.stylesheet().contains("HydrusOnOffButton"));
        assert!(dark.stylesheet().contains("#343434"));
        assert_ne!(light.stylesheet(), dark.stylesheet());
    }

    #[test]
    fn windows_vista_is_the_default_style() {
        assert_eq!(ClientGUIStyle::default().style_name, "windowsvista");
    }
}
