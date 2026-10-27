//! Saved favourite searches, mirroring Hydrus' favourite search manager.

use std::path::Path;

use serde::{Deserialize, Serialize};

use super::model::AppState;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FavouriteSearch {
    pub name: String,
    pub query: String,
    #[serde(default = "default_location")]
    pub location: String,
    #[serde(default)]
    pub tag_service: String,
}

fn default_location() -> String {
    "my files".to_string()
}

pub fn path(state: &AppState) -> std::path::PathBuf {
    state.controller.db_dir().join("favourite searches.json")
}

pub fn load(path: &Path) -> Vec<FavouriteSearch> {
    let Ok(text) = std::fs::read_to_string(path) else {
        return Vec::new();
    };

    serde_json::from_str(&text).unwrap_or_default()
}

pub fn persist(state: &AppState) -> std::io::Result<()> {
    let path = path(state);
    let text = serde_json::to_string_pretty(&state.favourites)
        .map_err(|error| std::io::Error::other(error.to_string()))?;

    std::fs::write(path, text)
}

pub fn add_or_replace(favourites: &mut Vec<FavouriteSearch>, favourite: FavouriteSearch) {
    if let Some(existing) = favourites
        .iter_mut()
        .find(|existing| existing.name == favourite.name)
    {
        *existing = favourite;
    } else {
        favourites.push(favourite);
        favourites.sort_by(|left, right| left.name.to_lowercase().cmp(&right.name.to_lowercase()));
    }
}

pub fn remove(favourites: &mut Vec<FavouriteSearch>, name: &str) -> bool {
    let before = favourites.len();
    favourites.retain(|favourite| favourite.name != name);
    favourites.len() != before
}

#[cfg(test)]
mod tests;
