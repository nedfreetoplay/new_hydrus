//! Undo stacks for closed pages and search-predicate edits (WS-01.9).
//!
//! Mirrors the Python client (`ClientGUI._closed_pages`,
//! `_predicate_history_added_to_search` /
//! `_predicate_history_removed_from_search`): both histories live only in
//! memory, are unbounded, grow with dedupe-move-to-end (predicates) or plain
//! append (closed pages), restore LIFO by default but allow restoring any
//! entry by index from the `undo` menu, and are cleared explicitly (`clear
//! all` / `clear history`). Nothing here is persisted in the session, same
//! as in Python.

/// Clamps a remembered insert position to a live container length, mirroring
/// `min(index, count)` in `PagesNotebook.TryToUncloseThisPage`.
pub fn clamp_insert_index(index: usize, len: usize) -> usize {
    index.min(len)
}

/// One predicate remembered together with the token position it occupied in
/// the query string, so undo can put it back at the same place.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PredicateEntry {
    pub predicate: String,
    pub position: usize,
}

impl PredicateEntry {
    pub fn new(predicate: impl Into<String>, position: usize) -> Self {
        Self {
            predicate: predicate.into(),
            position,
        }
    }
}

/// In-memory add/remove history of search predicates.
///
/// Python keeps two unbounded lists with dedupe-move-to-end on record and
/// restores any entry by menu index; undo re-enters the predicate on the
/// current page (which toggles it back) and re-runs the search.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PredicateHistory {
    added: Vec<PredicateEntry>,
    removed: Vec<PredicateEntry>,
}

impl PredicateHistory {
    pub fn new() -> Self {
        Self::default()
    }

    /// Records predicates added to the search. Duplicates move to the end,
    /// exactly like `SetPredicateHistoryAdded` in Python.
    pub fn record_added(&mut self, entries: impl IntoIterator<Item = PredicateEntry>) {
        for entry in entries {
            self.added
                .retain(|existing| existing.predicate != entry.predicate);
            self.added.push(entry);
        }
    }

    /// Records predicates removed from the search (`SetPredicateHistoryRemoved`).
    pub fn record_removed(&mut self, entries: impl IntoIterator<Item = PredicateEntry>) {
        for entry in entries {
            self.removed
                .retain(|existing| existing.predicate != entry.predicate);
            self.removed.push(entry);
        }
    }

    pub fn added(&self) -> &[PredicateEntry] {
        &self.added
    }

    pub fn removed(&self) -> &[PredicateEntry] {
        &self.removed
    }

    pub fn is_empty(&self) -> bool {
        self.added.is_empty() && self.removed.is_empty()
    }

    /// Takes one entry by menu index (LIFO default is `len() - 1`), mirroring
    /// `_UndoSearchPredAdd` / `_UndoSearchPredRemove` in Python.
    pub fn take_added(&mut self, index: usize) -> Option<PredicateEntry> {
        if index < self.added.len() {
            Some(self.added.remove(index))
        } else {
            None
        }
    }

    pub fn take_removed(&mut self, index: usize) -> Option<PredicateEntry> {
        if index < self.removed.len() {
            Some(self.removed.remove(index))
        } else {
            None
        }
    }

    /// Clears both lists (`clear history` menu item in Python).
    pub fn clear(&mut self) {
        self.added.clear();
        self.removed.clear();
    }
}

/// Splits a query string into predicate tokens the same way the search box
/// shows them: whitespace-separated, order preserved.
pub fn split_predicates(query: &str) -> Vec<String> {
    query.split_whitespace().map(str::to_string).collect()
}

/// Diffs an old token list against a new one, returning `(added, removed)`
/// with the positions each token had in its own list. Duplicate tokens are
/// diffed by count: only the surplus occurrences are reported.
pub fn diff_predicates(
    old: &[String],
    new: &[String],
) -> (Vec<PredicateEntry>, Vec<PredicateEntry>) {
    use std::collections::HashMap;

    let mut remaining: HashMap<&str, usize> = HashMap::new();
    for token in old {
        *remaining.entry(token.as_str()).or_default() += 1;
    }
    let mut added = Vec::new();
    for (position, token) in new.iter().enumerate() {
        match remaining.get_mut(token.as_str()) {
            Some(count) if *count > 0 => *count -= 1,
            _ => added.push(PredicateEntry::new(token.clone(), position)),
        }
    }

    let mut surplus: HashMap<&str, usize> = HashMap::new();
    for token in new {
        *surplus.entry(token.as_str()).or_default() += 1;
    }
    let mut removed = Vec::new();
    for (position, token) in old.iter().enumerate() {
        match surplus.get_mut(token.as_str()) {
            Some(count) if *count > 0 => *count -= 1,
            _ => removed.push(PredicateEntry::new(token.clone(), position)),
        }
    }

    (added, removed)
}

/// Applies a predicate-undo entry to a token list: the entry is toggled back
/// (added predicates are removed again, removed ones are re-inserted at
/// their old position), mirroring `EnterPredicates` toggling in Python.
/// Returns true when the list changed and the page must be re-queried.
pub fn apply_predicate_undo(tokens: &mut Vec<String>, entry: &PredicateEntry) -> bool {
    if let Some(at) = tokens.iter().position(|token| token == &entry.predicate) {
        tokens.remove(at);
        return true;
    }

    let at = clamp_insert_index(entry.position, tokens.len());
    tokens.insert(at, entry.predicate.clone());
    true
}

/// Joins tokens back into a query string.
pub fn join_predicates(tokens: &[String]) -> String {
    tokens.join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lifo_order_preserved() {
        let mut history = PredicateHistory::new();
        history.record_added([PredicateEntry::new("a", 0)]);
        history.record_added([PredicateEntry::new("b", 1)]);

        assert_eq!(history.take_added(1).unwrap().predicate, "b");
        assert_eq!(history.take_added(0).unwrap().predicate, "a");
        assert!(history.is_empty());
    }

    #[test]
    fn record_dedupes_by_moving_to_end() {
        let mut history = PredicateHistory::new();
        history.record_added([PredicateEntry::new("a", 0)]);
        history.record_added([PredicateEntry::new("b", 1)]);
        history.record_added([PredicateEntry::new("a", 2)]);

        let names: Vec<_> = history
            .added()
            .iter()
            .map(|entry| entry.predicate.clone())
            .collect();
        assert_eq!(names, vec!["b", "a"]);
    }

    #[test]
    fn removed_list_is_independent() {
        let mut history = PredicateHistory::new();
        history.record_removed([PredicateEntry::new("x", 0)]);

        assert!(history.take_added(0).is_none());
        assert_eq!(history.take_removed(0).unwrap().predicate, "x");
        assert!(history.is_empty());
    }

    #[test]
    fn out_of_range_take_returns_none() {
        let mut history = PredicateHistory::new();
        history.record_added([PredicateEntry::new("a", 0)]);

        assert!(history.take_added(7).is_none());
        assert!(history.take_removed(0).is_none());
        assert_eq!(history.added().len(), 1);
    }

    #[test]
    fn clear_empties_both_lists() {
        let mut history = PredicateHistory::new();
        history.record_added([PredicateEntry::new("a", 0)]);
        history.record_removed([PredicateEntry::new("b", 0)]);
        history.clear();

        assert!(history.is_empty());
    }

    #[test]
    fn unbounded_growth_like_python() {
        let mut history = PredicateHistory::new();
        for i in 0..500 {
            history.record_added([PredicateEntry::new(format!("tag{i}"), i)]);
        }

        assert_eq!(history.added().len(), 500);
    }

    #[test]
    fn diff_reports_positions() {
        let old: Vec<String> = split_predicates("red blue green");
        let new: Vec<String> = split_predicates("red green yellow");

        let (added, removed) = diff_predicates(&old, &new);
        assert_eq!(added, vec![PredicateEntry::new("yellow", 2)]);
        assert_eq!(removed, vec![PredicateEntry::new("blue", 1)]);
    }

    #[test]
    fn diff_ignores_reorder_and_duplicate_counts() {
        let old: Vec<String> = split_predicates("a b a");
        let new: Vec<String> = split_predicates("b a a");

        let (added, removed) = diff_predicates(&old, &new);
        assert!(added.is_empty());
        assert!(removed.is_empty());
    }

    #[test]
    fn undo_of_add_removes_predicate() {
        let mut tokens: Vec<String> = split_predicates("red yellow");
        let entry = PredicateEntry::new("yellow", 1);

        assert!(apply_predicate_undo(&mut tokens, &entry));
        assert_eq!(tokens, split_predicates("red"));
    }

    #[test]
    fn undo_of_remove_restores_position() {
        let mut tokens: Vec<String> = split_predicates("red green");
        let entry = PredicateEntry::new("blue", 1);

        assert!(apply_predicate_undo(&mut tokens, &entry));
        assert_eq!(tokens, split_predicates("red blue green"));
    }

    #[test]
    fn undo_of_remove_clamps_past_the_end() {
        let mut tokens: Vec<String> = split_predicates("red");
        let entry = PredicateEntry::new("blue", 9);

        assert!(apply_predicate_undo(&mut tokens, &entry));
        assert_eq!(tokens, split_predicates("red blue"));
    }

    #[test]
    fn clamp_insert_index_matches_python_min() {
        assert_eq!(clamp_insert_index(2, 5), 2);
        assert_eq!(clamp_insert_index(9, 3), 3);
        assert_eq!(clamp_insert_index(0, 0), 0);
    }

    #[test]
    fn join_round_trips_split() {
        let tokens: Vec<String> = split_predicates("red  blue green");
        assert_eq!(join_predicates(&tokens), "red blue green");
    }
}
