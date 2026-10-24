//! Options panel registry/search from `hydrus/client/gui/panels/options/`.

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum OptionsPanelId {
    Gui,
    Media,
    Files,
    Importing,
    Networking,
    Duplicates,
    Tags,
    Maintenance,
    Advanced,
}

impl OptionsPanelId {
    pub fn title(self) -> &'static str {
        match self {
            Self::Gui => "gui",
            Self::Media => "media",
            Self::Files => "files",
            Self::Importing => "importing",
            Self::Networking => "networking",
            Self::Duplicates => "duplicates",
            Self::Tags => "tags",
            Self::Maintenance => "maintenance",
            Self::Advanced => "advanced",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OptionsPanelEntry {
    pub id: OptionsPanelId,
    pub title: String,
    pub keywords: Vec<String>,
}

impl OptionsPanelEntry {
    pub fn new(id: OptionsPanelId) -> Self {
        Self {
            id,
            title: id.title().to_string(),
            keywords: Vec::new(),
        }
    }

    pub fn with_keywords(mut self, keywords: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.keywords = keywords.into_iter().map(Into::into).collect();
        self
    }

    pub fn matches(&self, search_text: &str) -> bool {
        let needle = search_text.to_lowercase();

        self.title.to_lowercase().contains(&needle)
            || self
                .keywords
                .iter()
                .any(|keyword| keyword.to_lowercase().contains(&needle))
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OptionsPanelState {
    pub panels: Vec<OptionsPanelEntry>,
    pub selected: Option<OptionsPanelId>,
    pub search_text: String,
}

impl OptionsPanelState {
    pub fn new() -> Self {
        let panels = [
            OptionsPanelId::Gui,
            OptionsPanelId::Media,
            OptionsPanelId::Files,
            OptionsPanelId::Importing,
            OptionsPanelId::Networking,
            OptionsPanelId::Duplicates,
            OptionsPanelId::Tags,
            OptionsPanelId::Maintenance,
            OptionsPanelId::Advanced,
        ]
        .into_iter()
        .map(OptionsPanelEntry::new)
        .collect();

        Self {
            panels,
            selected: None,
            search_text: String::new(),
        }
    }

    pub fn set_search_text(&mut self, text: impl Into<String>) {
        self.search_text = text.into();
    }

    pub fn filtered_panels(&self) -> Vec<&OptionsPanelEntry> {
        if self.search_text.trim().is_empty() {
            return self.panels.iter().collect();
        }

        self.panels
            .iter()
            .filter(|panel| panel.matches(&self.search_text))
            .collect()
    }

    pub fn select(&mut self, id: OptionsPanelId) -> bool {
        if self.panels.iter().any(|panel| panel.id == id) {
            self.selected = Some(id);
            true
        } else {
            false
        }
    }

    pub fn get_selected(&self) -> Option<&OptionsPanelEntry> {
        self.selected
            .and_then(|id| self.panels.iter().find(|panel| panel.id == id))
    }

    pub fn select_first_match(&mut self) -> bool {
        let id = self.filtered_panels().first().map(|panel| panel.id);

        match id {
            Some(id) => {
                self.selected = Some(id);
                true
            }
            None => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_registry_contains_all_panels() {
        let state = OptionsPanelState::new();

        assert_eq!(state.panels.len(), 9);
        assert!(state.panels.iter().any(|panel| panel.id == OptionsPanelId::Networking));
    }

    #[test]
    fn search_filters_by_title_and_keywords() {
        let mut state = OptionsPanelState::new();
        state.panels[0].keywords.push("theme".to_string());

        state.set_search_text("theme");
        let filtered = state.filtered_panels();

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].id, OptionsPanelId::Gui);
    }

    #[test]
    fn empty_search_lists_everything() {
        let state = OptionsPanelState::new();

        assert_eq!(state.filtered_panels().len(), state.panels.len());
    }

    #[test]
    fn selection_and_first_match_work() {
        let mut state = OptionsPanelState::new();

        assert!(state.select(OptionsPanelId::Files));
        assert_eq!(state.get_selected().unwrap().title, "files");

        state.set_search_text("net");
        assert!(state.select_first_match());
        assert_eq!(state.selected, Some(OptionsPanelId::Networking));
    }
}
