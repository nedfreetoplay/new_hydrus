//! String editors from `hydrus/client/gui/ClientGUIStringPanels.py`.

use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StringPanel {
    pub title: String,
    pub text: String,
    pub placeholder: String,
    pub allow_multiline: bool,
    pub max_chars: Option<usize>,
    pub submitted: bool,
}

impl StringPanel {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            text: String::new(),
            placeholder: String::new(),
            allow_multiline: false,
            max_chars: None,
            submitted: false,
        }
    }

    pub fn with_text(mut self, text: impl Into<String>) -> Self {
        self.text = text.into();
        self
    }

    pub fn with_multiline(mut self, allow: bool) -> Self {
        self.allow_multiline = allow;
        self
    }

    pub fn with_max_chars(mut self, max_chars: usize) -> Self {
        self.max_chars = Some(max_chars);
        self
    }

    pub fn set_text(&mut self, text: impl Into<String>) {
        self.text = text.into();
    }

    pub fn get_text(&self) -> &str {
        &self.text
    }

    pub fn is_valid(&self) -> bool {
        self.max_chars
            .is_none_or(|max_chars| self.text.chars().count() <= max_chars)
    }

    pub fn submit(&mut self) -> bool {
        if !self.is_valid() {
            return false;
        }

        self.submitted = true;
        true
    }

    pub fn cancel(&mut self) {
        self.submitted = false;
    }
}

/// `StringToStringDictPanel`
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StringToStringDictPanel {
    entries: BTreeMap<String, String>,
    original_entries: BTreeMap<String, String>,
}

impl StringToStringDictPanel {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_entries(entries: impl IntoIterator<Item = (String, String)>) -> Self {
        let entries: BTreeMap<String, String> = entries.into_iter().collect();

        Self {
            original_entries: entries.clone(),
            entries,
        }
    }

    pub fn set_entry(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.entries.insert(key.into(), value.into());
    }

    pub fn get_entry(&self, key: &str) -> Option<&str> {
        self.entries.get(key).map(String::as_str)
    }

    pub fn remove_entry(&mut self, key: &str) -> bool {
        self.entries.remove(key).is_some()
    }

    pub fn entries(&self) -> &BTreeMap<String, String> {
        &self.entries
    }

    pub fn has_changes(&self) -> bool {
        self.entries != self.original_entries
    }

    pub fn commit(&mut self) {
        self.original_entries = self.entries.clone();
    }

    pub fn discard(&mut self) {
        self.entries = self.original_entries.clone();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn string_panel_validates_max_length() {
        let mut panel = StringPanel::new("Name").with_max_chars(4);

        panel.set_text("abcd");
        assert!(panel.is_valid());
        assert!(panel.submit());

        panel.set_text("abcde");
        assert!(!panel.is_valid());
        assert!(!panel.submit());
    }

    #[test]
    fn cancel_resets_submitted_state() {
        let mut panel = StringPanel::new("Name").with_text("x");
        panel.submit();
        panel.cancel();

        assert!(!panel.submitted);
    }

    #[test]
    fn dict_panel_tracks_changes() {
        let mut panel = StringToStringDictPanel::from_entries([(
            "a".to_string(),
            "1".to_string(),
        )]);

        assert!(!panel.has_changes());

        panel.set_entry("a", "2");
        assert!(panel.has_changes());

        panel.discard();
        assert_eq!(panel.get_entry("a"), Some("1"));

        panel.set_entry("b", "3");
        panel.commit();
        assert!(!panel.has_changes());
    }

    #[test]
    fn dict_entries_can_be_removed() {
        let mut panel = StringToStringDictPanel::from_entries([(
            "a".to_string(),
            "1".to_string(),
        )]);

        assert!(panel.remove_entry("a"));
        assert!(!panel.remove_entry("a"));
        assert!(panel.entries().is_empty());
    }
}
