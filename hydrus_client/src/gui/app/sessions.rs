//! Named GUI sessions and their backups.
//!
//! The live tree is always saved to `gui session.json`. Named sessions live in
//! `sessions/` and backups in `sessions/backups/`, which mirrors the
//! save/load/append/backup menu in `ClientGUI.py`.

use std::path::{Path, PathBuf};

use super::model::{AppState, Notebook, PageNode};
use super::session::{self, LoadedSession};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionInfo {
    pub name: String,
    pub pages: usize,
}

pub fn dir(state: &AppState) -> PathBuf {
    state.controller.db_dir().join("sessions")
}

pub fn backups_dir(state: &AppState) -> PathBuf {
    dir(state).join("backups")
}

pub fn list(state: &AppState) -> Vec<SessionInfo> {
    list_in(&dir(state))
}

pub fn list_backups(state: &AppState) -> Vec<SessionInfo> {
    list_in(&backups_dir(state))
}

fn list_in(path: &Path) -> Vec<SessionInfo> {
    let Ok(entries) = std::fs::read_dir(path) else {
        return Vec::new();
    };

    let mut sessions = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();

        if path.extension().and_then(|extension| extension.to_str()) != Some("json") {
            continue;
        }

        let Some(name) = path.file_stem().and_then(|stem| stem.to_str()) else {
            continue;
        };

        let pages = session::load_session(&path)
            .map(|loaded| count_pages(&loaded.root))
            .unwrap_or(0);

        sessions.push(SessionInfo {
            name: name.to_string(),
            pages,
        });
    }

    sessions.sort_by(|left, right| left.name.to_lowercase().cmp(&right.name.to_lowercase()));
    sessions
}

fn count_pages(notebook: &Notebook) -> usize {
    notebook
        .children
        .iter()
        .map(|child| match child {
            PageNode::Notebook(inner) => count_pages(inner),
            _ => 1,
        })
        .sum()
}

/// Turns a user-supplied name into something that is safe as a file name.
pub fn sanitise_name(name: &str) -> String {
    let mut sanitised: String = name
        .trim()
        .chars()
        .map(|character| {
            if character.is_control() || "<>:\"/\\|?*".contains(character) {
                '_'
            } else {
                character
            }
        })
        .collect();

    sanitised = sanitised.trim_matches(['.', ' ']).to_string();

    if sanitised.is_empty() {
        "session".to_string()
    } else {
        sanitised
    }
}

fn path_for(state: &AppState, name: &str) -> PathBuf {
    dir(state).join(format!("{}.json", sanitise_name(name)))
}

pub fn save_named(state: &mut AppState, name: &str) -> Result<String, String> {
    let name = sanitise_name(name);
    let path = path_for(state, &name);

    session::save_to(state, &path).map_err(|error| error.to_string())?;
    state.session_name = Some(name.clone());
    let _ = session::save(state);

    Ok(name)
}

pub fn load_named(state: &AppState, name: &str) -> Result<LoadedSession, String> {
    let path = path_for(state, name);
    session::load_session(&path).ok_or_else(|| format!("session '{name}' is missing or unreadable"))
}

pub fn load_backup(state: &AppState, name: &str) -> Result<LoadedSession, String> {
    let path = backups_dir(state).join(format!("{}.json", sanitise_name(name)));
    session::load_session(&path).ok_or_else(|| format!("backup '{name}' is missing or unreadable"))
}

pub fn delete_named(state: &AppState, name: &str) -> Result<(), String> {
    let path = path_for(state, name);

    if !path.is_file() {
        return Err(format!("session '{name}' is missing"));
    }

    std::fs::remove_file(path).map_err(|error| error.to_string())
}

/// Copies the live session file into `sessions/backups/`.
pub fn backup(state: &mut AppState) -> Result<String, String> {
    let folder = backups_dir(state);
    std::fs::create_dir_all(&folder).map_err(|error| error.to_string())?;

    let stamp = chrono::Local::now().format("%Y-%m-%d %H-%M-%S");
    let stem = sanitise_name(state.session_name.as_deref().unwrap_or("session"));
    let name = format!("{stem} {stamp}");
    let path = folder.join(format!("{name}.json"));

    session::save_to(state, &path).map_err(|error| error.to_string())?;

    Ok(name)
}

#[cfg(test)]
mod tests;
