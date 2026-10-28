//! Database backup, from `_BackupDatabase` in `ClientGUI.py`.
//!
//! Hydrus keeps the backup directory in `backup_path` and writes a single
//! `client.db` there. The Rust client has one database file, so a backup is a
//! `VACUUM INTO` copy: it is consistent while the client keeps running and it
//! does not need the database to be closed first.

use std::path::{Path, PathBuf};

use serde_json::json;

use super::{render, AppState, UiCommand};

pub fn dialog_json(state: &AppState) -> String {
    let options = state.controller.options();
    let path = options
        .get_noneable_string("backup_path")
        .filter(|path| !path.trim().is_empty());
    let last_backup = options
        .get_noneable_integer("last_backup_time")
        .map(|timestamp| {
            hydrus_core::hydrus_time::timestamp_to_pretty_time(Some(timestamp as f64), false, true)
        });

    json!({
        "path": path.clone().unwrap_or_default(),
        "db_dir": state.controller.db_dir().to_string_lossy(),
        "last_backup": last_backup.unwrap_or_else(|| "never".to_string()),
        "target": path
            .map(|path| Path::new(&path).join("client.db").to_string_lossy().to_string())
            .unwrap_or_default(),
    })
    .to_string()
}

pub fn open(state: &AppState) -> Vec<UiCommand> {
    vec![UiCommand::DatabaseBackupDialog(dialog_json(state))]
}

pub fn set_path(state: &mut AppState, payload: &str) -> Vec<UiCommand> {
    let path = payload.trim().trim_matches('"').to_string();

    if path.is_empty() {
        return notify(state, "choose a backup directory");
    }

    let directory = PathBuf::from(&path);

    if directory
        .components()
        .any(|component| component == std::path::Component::ParentDir)
    {
        return notify(state, "the backup directory is not valid");
    }

    if std::fs::create_dir_all(&directory).is_err() {
        return notify(state, "could not create the backup directory");
    }

    let mut options = state.controller.options();
    options.set_noneable_string("backup_path", Some(&path));
    state.controller.replace_options(options);

    if let Err(error) = state.controller.save_options() {
        return notify(state, format!("could not save the backup path: {error}"));
    }

    updated(state, format!("backup directory set to {path}"))
}

pub fn run(state: &mut AppState) -> Vec<UiCommand> {
    let options = state.controller.options();
    let Some(path) = options
        .get_noneable_string("backup_path")
        .filter(|path| !path.trim().is_empty())
    else {
        return notify(state, "no backup path is set");
    };

    let directory = PathBuf::from(&path);

    if let Err(error) = std::fs::create_dir_all(&directory) {
        return notify(
            state,
            format!("could not create the backup directory: {error}"),
        );
    }

    let target = directory.join("client.db");

    if same_file(&target, &state.controller.db_dir().join("client.db")) {
        return notify(state, "the backup path is the live database");
    }

    if target.exists() {
        if let Err(error) = std::fs::remove_file(&target) {
            return notify(state, format!("could not replace the old backup: {error}"));
        }
    }

    let escaped = target.to_string_lossy().replace('\'', "''");

    if let Err(error) = state
        .controller
        .db()
        .db()
        .execute_batch(&format!("VACUUM INTO '{escaped}';"))
    {
        return notify(state, format!("backup failed: {error}"));
    }

    let size = std::fs::metadata(&target)
        .map(|metadata| metadata.len())
        .unwrap_or(0);
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0);

    let mut options = state.controller.options();
    options.set_noneable_integer("last_backup_time", Some(now));
    state.controller.replace_options(options);
    let _ = state.controller.save_options();

    updated(
        state,
        format!(
            "database backed up to {} ({})",
            target.display(),
            human_bytes(size)
        ),
    )
}

fn same_file(left: &Path, right: &Path) -> bool {
    match (std::fs::canonicalize(left), std::fs::canonicalize(right)) {
        (Ok(left), Ok(right)) => left == right,
        _ => left == right,
    }
}

fn human_bytes(bytes: u64) -> String {
    const UNITS: [&str; 4] = ["B", "KB", "MB", "GB"];
    let mut value = bytes as f64;
    let mut unit = 0;

    while value >= 1024.0 && unit + 1 < UNITS.len() {
        value /= 1024.0;
        unit += 1;
    }

    if unit == 0 {
        format!("{bytes} B")
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

fn updated(state: &AppState, message: impl Into<String>) -> Vec<UiCommand> {
    vec![
        UiCommand::UpdateDatabaseBackup(dialog_json(state)),
        UiCommand::Menus(render::menus_json(state)),
        UiCommand::Status(render::status_json(state)),
        UiCommand::NotifyMessage(message.into()),
    ]
}

fn notify(state: &AppState, message: impl Into<String>) -> Vec<UiCommand> {
    vec![
        UiCommand::NotifyMessage(message.into()),
        UiCommand::UpdateDatabaseBackup(dialog_json(state)),
        UiCommand::Menus(render::menus_json(state)),
        UiCommand::Status(render::status_json(state)),
    ]
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use serde_json::Value;

    use super::*;
    use crate::gui::app::AppState;

    fn temp_directory(label: &str) -> PathBuf {
        let directory = std::env::temp_dir().join(format!(
            "hydrus_backup_{label}_{}_{}",
            std::process::id(),
            rand::random::<u64>()
        ));
        std::fs::create_dir_all(&directory).unwrap();

        directory
    }

    fn test_state(directory: &Path) -> AppState {
        let _ = crate::client_options::register_serialisable_type();
        let controller = crate::controller::Controller::new(directory).unwrap();
        controller.init_model().unwrap();

        AppState::new(Arc::new(controller))
    }

    #[test]
    fn backup_writes_a_readable_copy_and_updates_the_timestamp() {
        let directory = temp_directory("run");
        let mut state = test_state(&directory);
        let target = directory.join("backup");

        let commands = set_path(&mut state, target.to_string_lossy().as_ref());
        assert!(commands.iter().any(
            |command| matches!(command, UiCommand::NotifyMessage(message) if message.contains("backup directory"))
        ), "{commands:?}");

        let commands = run(&mut state);
        assert!(commands.iter().any(
            |command| matches!(command, UiCommand::NotifyMessage(message) if message.contains("database backed up"))
        ), "{commands:?}");

        let copy = target.join("client.db");
        assert!(copy.exists());

        let connection = rusqlite::Connection::open(&copy).unwrap();
        let services: i64 = connection
            .query_row("SELECT COUNT(*) FROM services", [], |row| row.get(0))
            .unwrap();
        assert!(services >= 5, "backup should contain the service tables");

        let options = state.controller.options();
        assert!(options.get_noneable_integer("last_backup_time").is_some());
        assert_eq!(
            options.get_noneable_string("backup_path").as_deref(),
            Some(target.to_string_lossy().as_ref())
        );

        drop(connection);

        // A second run replaces the existing copy.
        let commands = run(&mut state);
        assert!(commands.iter().any(
            |command| matches!(command, UiCommand::NotifyMessage(message) if message.contains("database backed up"))
        ), "{commands:?}");

        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn backup_without_a_path_is_rejected() {
        let directory = temp_directory("missing");
        let mut state = test_state(&directory);

        let commands = run(&mut state);

        assert!(commands.iter().any(
            |command| matches!(command, UiCommand::NotifyMessage(message) if message.contains("no backup path"))
        ), "{commands:?}");

        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn the_live_database_cannot_be_the_backup_target() {
        let directory = temp_directory("live");
        let mut state = test_state(&directory);
        let db_dir = state.controller.db_dir().to_path_buf();

        set_path(&mut state, db_dir.to_string_lossy().as_ref());
        let commands = run(&mut state);

        assert!(commands.iter().any(
            |command| matches!(command, UiCommand::NotifyMessage(message) if message.contains("live database"))
        ), "{commands:?}");

        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn dialog_shows_the_target_and_last_backup() {
        let directory = temp_directory("dialog");
        let state = test_state(&directory);
        let value: Value = serde_json::from_str(&dialog_json(&state)).unwrap();

        assert_eq!(value["last_backup"], "never");
        assert!(value["target"].as_str().unwrap_or_default().is_empty());

        let _ = std::fs::remove_dir_all(directory);
    }
}
