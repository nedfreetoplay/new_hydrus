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
            style_name: "Fusion".to_string(),
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
        if self.darkmode {
            "
            QWidget { background-color: #343434; color: #d9e0e6; }
            QTabWidget::pane { border: 1px solid #4b4b4b; }
            QTabBar::tab:selected { background: #4b4b4b; }
            QMenuBar, QMenu, QStatusBar { background-color: #2b2b2b; color: #d9e0e6; }
            QTreeWidget { background-color: #2b2b2b; alternate-background-color: #343434; }
            "
            .to_string()
        } else {
            "
            QWidget { background-color: #ffffff; color: #000000; }
            QTabWidget::pane { border: 1px solid #dfe3e6; }
            QTabBar::tab:selected { background: #d9f2ff; }
            QMenuBar, QMenu, QStatusBar { background-color: #f2f2f2; }
            QTreeWidget { background-color: #ffffff; alternate-background-color: #f7f7f7; }
            "
            .to_string()
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

        assert!(light.stylesheet().contains("#ffffff"));
        assert!(dark.stylesheet().contains("#343434"));
        assert_ne!(light.stylesheet(), dark.stylesheet());
    }

    #[test]
    fn fusion_is_the_default_style() {
        assert_eq!(ClientGUIStyle::default().style_name, "Fusion");
    }
}
