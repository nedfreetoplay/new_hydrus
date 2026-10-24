//! Edit/manage/review panel state from
//! `hydrus/client/gui/panels/ClientGUIScrolledPanels.py`.

use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanelMode {
    Edit,
    Manage,
    Review,
}

impl PanelMode {
    pub fn has_commit(self) -> bool {
        !matches!(self, Self::Review)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ScrolledPanel {
    pub mode: Option<PanelMode>,
    values: BTreeMap<String, String>,
    original_values: BTreeMap<String, String>,
}

impl ScrolledPanel {
    pub fn new(mode: PanelMode) -> Self {
        Self {
            mode: Some(mode),
            ..Default::default()
        }
    }

    pub fn set_value(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.values.insert(key.into(), value.into());
    }

    pub fn get_value(&self, key: &str) -> Option<&str> {
        self.values.get(key).map(String::as_str)
    }

    pub fn get_values(&self) -> &BTreeMap<String, String> {
        &self.values
    }

    pub fn has_changes(&self) -> bool {
        self.values != self.original_values
    }

    /// Marks the current values as the committed baseline.
    pub fn commit(&mut self) -> bool {
        if self.mode.is_some_and(|mode| !mode.has_commit()) {
            return false;
        }

        self.original_values = self.values.clone();

        true
    }

    /// Reverts to the last committed values.
    pub fn discard(&mut self) {
        self.values = self.original_values.clone();
    }

    pub fn is_edit(self) -> bool {
        self.mode == Some(PanelMode::Edit)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn changes_are_tracked_until_commit() {
        let mut panel = ScrolledPanel::new(PanelMode::Edit);

        panel.set_value("name", "test");
        assert!(panel.has_changes());

        assert!(panel.commit());
        assert!(!panel.has_changes());

        panel.set_value("name", "changed");
        assert!(panel.has_changes());

        panel.discard();
        assert_eq!(panel.get_value("name"), Some("test"));
        assert!(!panel.has_changes());
    }

    #[test]
    fn review_panels_cannot_commit() {
        let mut panel = ScrolledPanel::new(PanelMode::Review);
        panel.set_value("name", "test");

        assert!(!panel.commit());
        assert!(panel.has_changes());
    }

    #[test]
    fn manage_panels_can_commit() {
        let mut panel = ScrolledPanel::new(PanelMode::Manage);
        panel.set_value("name", "test");

        assert!(panel.commit());
    }
}
