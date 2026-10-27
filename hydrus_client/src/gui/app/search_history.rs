//! Recent search strings shared by all query pages.

use std::path::Path;

use super::model::AppState;

pub const LIMIT: usize = 50;

pub fn path(state: &AppState) -> std::path::PathBuf {
    state.controller.db_dir().join("search history.json")
}

pub fn load(path: &Path) -> Vec<String> {
    let Ok(text) = std::fs::read_to_string(path) else {
        return Vec::new();
    };

    serde_json::from_str(&text).unwrap_or_default()
}

pub fn persist(state: &AppState) -> std::io::Result<()> {
    let text = serde_json::to_string_pretty(&state.search_history)
        .map_err(|error| std::io::Error::other(error.to_string()))?;

    std::fs::write(path(state), text)
}

/// Puts `query` at the front, removes duplicates and drops old entries.
pub fn record(history: &mut Vec<String>, query: &str) {
    let query = query.trim();

    if query.is_empty() {
        return;
    }

    history.retain(|entry| entry != query);
    history.insert(0, query.to_string());
    history.truncate(LIMIT);
}

#[cfg(test)]
mod tests;
