//! Duplicate/import/parsing panel state from `ClientGUIDuplicates*`,
//! `ClientGUIImport*` and `ClientGUIParsing*`.

use crate::duplicates::DuplicatePairDecision;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DuplicatesFilter {
    All,
    Potential,
    Duplicates,
    Alternates,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DuplicatesPanelState {
    pub pairs: Vec<(Vec<u8>, Vec<u8>)>,
    pub current_index: Option<usize>,
    pub decided: usize,
    pub skipped: usize,
    pub filter: Option<DuplicatesFilter>,
}

impl DuplicatesPanelState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn load_pairs(&mut self, pairs: impl IntoIterator<Item = (Vec<u8>, Vec<u8>)>) {
        self.pairs = pairs.into_iter().collect();
        self.current_index = if self.pairs.is_empty() { None } else { Some(0) };
        self.decided = 0;
        self.skipped = 0;
    }

    pub fn current_pair(&self) -> Option<&(Vec<u8>, Vec<u8>)> {
        self.current_index.and_then(|index| self.pairs.get(index))
    }

    pub fn decide(&mut self, decision: &DuplicatePairDecision) -> bool {
        if self.current_pair().is_none() {
            return false;
        }

        if decision.is_skip() {
            self.skipped += 1;
        } else {
            self.decided += 1;
        }

        self.advance();
        true
    }

    pub fn advance(&mut self) -> bool {
        self.current_index = match self.current_index {
            Some(index) if index + 1 < self.pairs.len() => Some(index + 1),
            _ => None,
        };

        self.current_index.is_some()
    }

    pub fn progress(&self) -> (usize, usize) {
        (self.decided + self.skipped, self.pairs.len())
    }

    pub fn is_finished(&self) -> bool {
        self.current_index.is_none() && !self.pairs.is_empty()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImportTab {
    Gallery,
    Watcher,
    Subscription,
    Local,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ImportPanelState {
    pub gallery_count: usize,
    pub watcher_count: usize,
    pub subscription_count: usize,
    pub local_count: usize,
    pub paused: bool,
    pub selected_tab: Option<ImportTab>,
}

impl ImportPanelState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_count(&mut self, tab: ImportTab, count: usize) {
        match tab {
            ImportTab::Gallery => self.gallery_count = count,
            ImportTab::Watcher => self.watcher_count = count,
            ImportTab::Subscription => self.subscription_count = count,
            ImportTab::Local => self.local_count = count,
        }
    }

    pub fn get_count(&self, tab: ImportTab) -> usize {
        match tab {
            ImportTab::Gallery => self.gallery_count,
            ImportTab::Watcher => self.watcher_count,
            ImportTab::Subscription => self.subscription_count,
            ImportTab::Local => self.local_count,
        }
    }

    pub fn select_tab(&mut self, tab: ImportTab) {
        self.selected_tab = Some(tab);
    }

    pub fn toggle_pause(&mut self) {
        self.paused = !self.paused;
    }

    pub fn total_importers(&self) -> usize {
        self.gallery_count + self.watcher_count + self.subscription_count + self.local_count
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ParsingPanelState {
    pub formula_names: Vec<String>,
    pub selected_formula: Option<usize>,
    pub test_text: String,
    pub last_result: Option<String>,
}

impl ParsingPanelState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_formulas(&mut self, names: impl IntoIterator<Item = String>) {
        self.formula_names = names.into_iter().collect();
        self.selected_formula = if self.formula_names.is_empty() {
            None
        } else {
            Some(0)
        };
    }

    pub fn select_formula(&mut self, index: usize) -> bool {
        if index < self.formula_names.len() {
            self.selected_formula = Some(index);
            true
        } else {
            false
        }
    }

    pub fn get_selected_formula(&self) -> Option<&str> {
        self.selected_formula
            .and_then(|index| self.formula_names.get(index))
            .map(String::as_str)
    }

    pub fn set_test_text(&mut self, text: impl Into<String>) {
        self.test_text = text.into();
    }

    pub fn set_last_result(&mut self, result: impl Into<String>) {
        self.last_result = Some(result.into());
    }
}

#[cfg(test)]
mod tests;
