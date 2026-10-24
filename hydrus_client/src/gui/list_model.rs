//! List model helpers from `hydrus/client/gui/lists/ClientGUIListCtrl.py`,
//! `ClientGUIListBoxes.py` and `ClientGUIListManager.py`.

use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListColumn {
    pub name: String,
    pub sortable: bool,
}

impl ListColumn {
    pub fn new(name: impl Into<String>, sortable: bool) -> Self {
        Self {
            name: name.into(),
            sortable,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListRow {
    pub values: BTreeMap<String, String>,
}

impl ListRow {
    pub fn new(values: impl IntoIterator<Item = (String, String)>) -> Self {
        Self {
            values: values.into_iter().collect(),
        }
    }

    pub fn get(&self, column: &str) -> Option<&str> {
        self.values.get(column).map(String::as_str)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ListModel {
    pub columns: Vec<ListColumn>,
    rows: Vec<ListRow>,
    selected: BTreeSet<usize>,
    sort_column: Option<String>,
    sort_ascending: bool,
}

impl ListModel {
    pub fn new(columns: Vec<ListColumn>) -> Self {
        Self {
            columns,
            rows: Vec::new(),
            selected: BTreeSet::new(),
            sort_column: None,
            sort_ascending: true,
        }
    }

    pub fn add_row(&mut self, row: ListRow) -> usize {
        self.rows.push(row);
        self.rows.len() - 1
    }

    pub fn get_rows(&self) -> &[ListRow] {
        &self.rows
    }

    pub fn get_row(&self, index: usize) -> Option<&ListRow> {
        self.rows.get(index)
    }

    pub fn len(&self) -> usize {
        self.rows.len()
    }

    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    pub fn clear(&mut self) {
        self.rows.clear();
        self.selected.clear();
    }

    pub fn sort_by(&mut self, column: &str, ascending: bool) -> bool {
        if !self
            .columns
            .iter()
            .any(|candidate| candidate.name == column && candidate.sortable)
        {
            return false;
        }

        self.rows.sort_by(|a, b| {
            let left = a.values.get(column).cloned().unwrap_or_default();
            let right = b.values.get(column).cloned().unwrap_or_default();
            let ordering = left.cmp(&right);

            if ascending {
                ordering
            } else {
                ordering.reverse()
            }
        });

        self.sort_column = Some(column.to_string());
        self.sort_ascending = ascending;

        true
    }

    pub fn get_sort(&self) -> Option<(&str, bool)> {
        self.sort_column
            .as_deref()
            .map(|column| (column, self.sort_ascending))
    }

    pub fn select(&mut self, index: usize, additive: bool) {
        if index >= self.rows.len() {
            return;
        }

        if !additive {
            self.selected.clear();
        }

        self.selected.insert(index);
    }

    pub fn deselect(&mut self, index: usize) {
        self.selected.remove(&index);
    }

    pub fn clear_selection(&mut self) {
        self.selected.clear();
    }

    pub fn get_selected_indices(&self) -> &BTreeSet<usize> {
        &self.selected
    }

    pub fn get_selected_rows(&self) -> Vec<&ListRow> {
        self.selected
            .iter()
            .filter_map(|index| self.rows.get(*index))
            .collect()
    }

    pub fn filter_rows(&self, text: &str) -> Vec<usize> {
        let needle = text.to_lowercase();

        self.rows
            .iter()
            .enumerate()
            .filter(|(_, row)| {
                row.values
                    .values()
                    .any(|value| value.to_lowercase().contains(&needle))
            })
            .map(|(index, _)| index)
            .collect()
    }
}

/// Simple `ListBox` model: an ordered set of strings with a check state.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ListBox {
    items: Vec<String>,
    checked: BTreeSet<usize>,
}

impl ListBox {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_item(&mut self, item: impl Into<String>) -> usize {
        self.items.push(item.into());
        self.items.len() - 1
    }

    pub fn get_items(&self) -> &[String] {
        &self.items
    }

    pub fn remove_item(&mut self, index: usize) -> Option<String> {
        if index >= self.items.len() {
            return None;
        }

        self.checked.remove(&index);
        Some(self.items.remove(index))
    }

    pub fn set_checked(&mut self, index: usize, checked: bool) {
        if checked {
            self.checked.insert(index);
        } else {
            self.checked.remove(&index);
        }
    }

    pub fn is_checked(&self, index: usize) -> bool {
        self.checked.contains(&index)
    }

    pub fn get_checked_items(&self) -> Vec<&String> {
        self.checked
            .iter()
            .filter_map(|index| self.items.get(*index))
            .collect()
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

#[cfg(test)]
mod tests;
