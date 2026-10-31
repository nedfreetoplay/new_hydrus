//! Export folders and their periodic synchronisation.
//!
//! The filesystem work lives in [`execute`], which runs on the background
//! worker. This module owns the folder entries, their JSON file, the filename
//! phrase parser and the small amount of state that Qt renders.

use std::collections::BTreeSet;
use std::path::{Component, Path, PathBuf};

use hydrus_core::hydrus_constants::{ContentStatus, Mime};
use hydrus_core::hydrus_time;
use serde::{Deserialize, Serialize};

use crate::controller::Controller;
use crate::db::files_storage;

use super::model::ExportFoldersPage;
use super::worker::AsyncJob;
use super::{AppState, PageNode, UiCommand, query, render};

const FILENAME: &str = "export folders.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportFolderEntry {
    pub name: String,
    pub path: PathBuf,
    /// Search box text; empty exports every file in "my files".
    #[serde(default)]
    pub query: String,
    /// Filename phrase, e.g. `[creator] {hash}`.
    #[serde(default = "default_phrase")]
    pub phrase: String,
    /// `copy` keeps the source folder as it is, `synchronise` deletes files
    /// that are no longer in the search result.
    #[serde(default = "default_type")]
    pub export_type: String,
    /// `none`, `json` or `text`.
    #[serde(default = "default_sidecar")]
    pub sidecar_format: String,
    #[serde(default)]
    pub overwrite_sidecars: bool,
    #[serde(default)]
    pub delete_from_client: bool,
    #[serde(default = "default_true")]
    pub run_regularly: bool,
    #[serde(default = "default_period")]
    pub period: i64,
    #[serde(default)]
    pub paused: bool,
    #[serde(default)]
    pub last_check: i64,
    #[serde(default)]
    pub last_exported: usize,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub last_error: String,
    /// Not persisted: true while the worker is running this folder.
    #[serde(skip)]
    pub running: bool,
}

fn default_phrase() -> String {
    "{hash}".to_string()
}

fn default_type() -> String {
    "copy".to_string()
}

fn default_sidecar() -> String {
    "none".to_string()
}

fn default_period() -> i64 {
    3600
}

fn default_true() -> bool {
    true
}

impl ExportFolderEntry {
    pub fn new(
        name: impl Into<String>,
        path: impl Into<PathBuf>,
        period: i64,
        phrase: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            path: path.into(),
            query: String::new(),
            phrase: phrase.into(),
            export_type: default_type(),
            sidecar_format: default_sidecar(),
            overwrite_sidecars: false,
            delete_from_client: false,
            run_regularly: true,
            period: period.max(60),
            paused: false,
            last_check: 0,
            last_exported: 0,
            status: "ready".to_string(),
            last_error: String::new(),
            running: false,
        }
    }

    pub fn is_synchronise(&self) -> bool {
        self.export_type == "synchronise"
    }

    pub fn is_due(&self, now: i64) -> bool {
        !self.paused
            && self.run_regularly
            && !self.running
            && now >= self.last_check + self.period.max(60)
            && self.path.is_dir()
    }
}

/// One parsed piece of a filename phrase.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PhraseTerm {
    Text(String),
    /// `[creator]` expands to the sorted subtags of that namespace.
    Namespace(String),
    /// `{hash}`, `{file_id}`, `{#}`, `{tags}`, `{nn tags}`.
    Predicate(String),
    /// `(tag)` expands to the subtag when the file has that tag.
    Tag(String),
}

/// Parses `[namespace]`, `{predicate}` and `(tag)` out of a phrase. An
/// unclosed bracket is a user error and is reported instead of silently
/// producing a broken filename.
pub fn parse_phrase(phrase: &str) -> Result<Vec<PhraseTerm>, String> {
    let mut terms = Vec::new();
    let mut text = String::new();
    let mut chars = phrase.chars().peekable();

    while let Some(character) = chars.next() {
        let (kind, closing) = match character {
            '[' => (1u8, ']'),
            '{' => (2u8, '}'),
            '(' => (3u8, ')'),
            ']' | '}' | ')' => {
                return Err(format!("unexpected '{character}' in the filename phrase"));
            }
            _ => {
                text.push(character);
                continue;
            }
        };

        if !text.is_empty() {
            terms.push(PhraseTerm::Text(std::mem::take(&mut text)));
        }

        let mut value = String::new();

        loop {
            match chars.next() {
                Some(next) if next == closing => break,
                Some(next) => value.push(next),
                None => {
                    return Err(format!(
                        "the filename phrase has an unclosed '{character}'"
                    ));
                }
            }
        }

        terms.push(match kind {
            1 => PhraseTerm::Namespace(value),
            2 => PhraseTerm::Predicate(value),
            _ => PhraseTerm::Tag(value),
        });
    }

    if !text.is_empty() {
        terms.push(PhraseTerm::Text(text));
    }

    if terms.is_empty() {
        return Err("the filename phrase is empty".to_string());
    }

    Ok(terms)
}

fn subtag(tag: &str) -> &str {
    tag.split_once(':').map(|(_, subtag)| subtag).unwrap_or(tag)
}

fn clean_tag_text(text: &str) -> String {
    text.replace(['\\', '/'], "_")
}

fn namespace_slice(tags: &[String], namespace: &str) -> String {
    let prefix = format!("{namespace}:");

    let mut subtags: Vec<&str> = tags
        .iter()
        .filter_map(|tag| tag.strip_prefix(&prefix))
        .collect();

    subtags.sort_unstable();

    clean_tag_text(&subtags.join(", "))
}

fn tags_for_predicate(tags: &[String], predicate: &str) -> Option<String> {
    match predicate {
        "tags" => {
            let mut subtags: Vec<&str> = tags.iter().map(|tag| subtag(tag)).collect();
            subtags.sort_unstable();
            Some(clean_tag_text(&subtags.join(", ")))
        }
        "nn tags" => {
            let mut plain: Vec<&str> = tags
                .iter()
                .filter(|tag| !tag.contains(':'))
                .map(String::as_str)
                .collect();
            plain.sort_unstable();
            Some(clean_tag_text(&plain.join(", ")))
        }
        _ => None,
    }
}

/// Expands one phrase into a filename (without extension). `index` is
/// one-based and feeds `{#}`.
pub fn filename_for(
    terms: &[PhraseTerm],
    hash: &[u8],
    hash_id: i64,
    index: usize,
    tags: &[String],
) -> String {
    let mut filename = String::new();

    for term in terms {
        match term {
            PhraseTerm::Text(text) => filename.push_str(text),
            PhraseTerm::Namespace(namespace) => {
                filename.push_str(&namespace_slice(tags, namespace));
            }
            PhraseTerm::Predicate(predicate) => match predicate.as_str() {
                "hash" => filename.push_str(&query::hex(hash)),
                "file_id" => filename.push_str(&hash_id.to_string()),
                "#" => filename.push_str(&index.to_string()),
                other => {
                    if let Some(text) = tags_for_predicate(tags, other) {
                        filename.push_str(&text);
                    }
                }
            },
            PhraseTerm::Tag(tag) => {
                let wanted = subtag(tag.trim());

                if tags.iter().any(|current| subtag(current) == wanted) {
                    filename.push_str(&clean_tag_text(wanted));
                }
            }
        }
    }

    while filename.starts_with('/') || filename.starts_with('\\') {
        filename.remove(0);
    }

    while filename.contains("//") {
        filename = filename.replace("//", "/");
    }

    while filename.contains("\\\\") {
        filename = filename.replace("\\\\", "\\");
    }

    if filename.trim().is_empty() {
        filename = query::hex(hash);
    }

    filename
}

/// Turns a generated filename into a relative, traversal-free path. Absolute
/// paths, `..` components and Windows-reserved characters are rejected or
/// sanitised, so a tag can never write outside the export directory.
pub fn safe_relative_path(filename: &str) -> Option<PathBuf> {
    let mut relative = PathBuf::new();

    for part in filename.split(['/', '\\']) {
        let part = part.trim();

        if part.is_empty() || part == "." {
            continue;
        }

        if part == ".." {
            return None;
        }

        let mut cleaned = String::with_capacity(part.len());

        for character in part.chars() {
            let invalid = character.is_control()
                || matches!(character, '<' | '>' | ':' | '"' | '|' | '?' | '*');

            cleaned.push(if invalid { '_' } else { character });
        }

        let cleaned = cleaned.trim_end_matches(['.', ' ']);

        if cleaned.is_empty() {
            continue;
        }

        relative.push(cleaned);
    }

    if relative.as_os_str().is_empty() {
        return None;
    }

    if relative
        .components()
        .any(|component| !matches!(component, Component::Normal(_)))
    {
        return None;
    }

    Some(relative)
}

fn elide_filename(filename: &str, limit: usize) -> String {
    let limit = limit.max(32);
    let length = filename.chars().count();

    if length <= limit {
        return filename.to_string();
    }

    let keep = limit.saturating_sub(3);
    let front = keep / 2;
    let back = keep - front;

    let prefix: String = filename.chars().take(front).collect();
    let suffix: String = filename
        .chars()
        .rev()
        .take(back)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();

    format!("{prefix}...{suffix}")
}

fn with_extension(filename: &str, mime: Mime) -> String {
    let extension = mime.extension();

    if extension.is_empty() || filename.to_ascii_lowercase().ends_with(extension) {
        return filename.to_string();
    }

    format!("{filename}{extension}")
}

/// Summary of one export run, reported back by the worker.
#[derive(Debug, Clone, Default)]
pub struct ExportResult {
    pub exported: usize,
    pub skipped: usize,
    pub sidecars: usize,
    pub deleted: usize,
    pub synchronised_deleted: usize,
    pub errors: Vec<String>,
}

impl ExportResult {
    pub fn message(&self) -> String {
        let mut parts = vec![format!("exported {}", self.exported)];

        if self.skipped > 0 {
            parts.push(format!("{} already up to date", self.skipped));
        }

        if self.sidecars > 0 {
            parts.push(format!("{} sidecars", self.sidecars));
        }

        if self.deleted > 0 {
            parts.push(format!("{} moved to trash", self.deleted));
        }

        if self.synchronised_deleted > 0 {
            parts.push(format!("{} stale files removed", self.synchronised_deleted));
        }

        if !self.errors.is_empty() {
            parts.push(format!("{} errors", self.errors.len()));
        }

        parts.join(", ")
    }
}

fn files_under(root: &Path, files: &mut BTreeSet<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(root) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();

        if path.is_dir() {
            files_under(&path, files);
        } else if path.is_file() {
            files.insert(path);
        }
    }
}

fn file_is_current(source: &Path, destination: &Path) -> bool {
    let (Ok(source_meta), Ok(dest_meta)) = (std::fs::metadata(source), std::fs::metadata(destination))
    else {
        return false;
    };

    if !dest_meta.is_file() || source_meta.len() != dest_meta.len() {
        return false;
    }

    match (source_meta.modified(), dest_meta.modified()) {
        (Ok(source_time), Ok(dest_time)) => {
            let difference = source_time
                .duration_since(dest_time)
                .or_else(|_| dest_time.duration_since(source_time))
                .map(|duration| duration.as_secs())
                .unwrap_or(0);

            difference <= 2
        }
        _ => true,
    }
}

fn write_sidecar(
    destination: &Path,
    format: &str,
    tags: &[String],
    urls: &[String],
    overwrite: bool,
) -> bool {
    if format == "none" {
        return false;
    }

    let sidecar = destination.with_extension(if format == "text" {
        "tags.txt"
    } else {
        "tags.json"
    });

    if sidecar.exists() && !overwrite {
        return false;
    }

    let contents = if format == "text" {
        let mut text = tags.join("\n");
        text.push('\n');
        text
    } else {
        serde_json::to_string_pretty(&serde_json::json!({
            "tags": tags,
            "urls": urls,
        }))
        .unwrap_or_else(|_| "{}".to_string())
    };

    match std::fs::write(&sidecar, contents) {
        Ok(()) => true,
        Err(_) => false,
    }
}

/// Runs one export folder. Called from the worker thread; the GUI only sees
/// the returned summary.
pub fn execute(controller: &Controller, folder: &ExportFolderEntry) -> ExportResult {
    execute_selected(controller, folder, None)
}

/// Runs one folder, optionally restricted to hashes chosen in the review
/// dialog.
pub fn execute_selected(
    controller: &Controller,
    folder: &ExportFolderEntry,
    selected: Option<&[Vec<u8>]>,
) -> ExportResult {
    let mut result = ExportResult::default();

    if let Err(error) = std::fs::create_dir_all(&folder.path) {
        result.errors.push(format!(
            "could not create {}: {error}",
            folder.path.display()
        ));
        return result;
    }

    // Normalise once so a relative destination can still be checked for path
    // escapes with `starts_with`.
    let root = std::fs::canonicalize(&folder.path).unwrap_or_else(|_| folder.path.clone());

    let mut hashes = match query::hashes_for(controller, &folder.query, "my files") {
        Ok(hashes) => hashes,
        Err(error) => {
            result.errors.push(format!("search failed: {error}"));
            return result;
        }
    };

    if let Some(selected) = selected {
        hashes.retain(|hash| selected.iter().any(|chosen| chosen == hash));
    }

    let terms = match parse_phrase(&folder.phrase) {
        Ok(terms) => terms,
        Err(error) => {
            result
                .errors
                .push(format!("invalid filename phrase: {error}"));
            return result;
        }
    };

    let limit = controller
        .options()
        .get_integer("export_filename_character_limit");

    let limit = if limit <= 0 { 255 } else { limit.clamp(32, 255) } as usize;

    let mut previous = BTreeSet::new();

    if folder.is_synchronise() {
        files_under(&root, &mut previous);
    }

    let mut synced = BTreeSet::new();
    let mut used_names = BTreeSet::new();

    for (index, hash) in hashes.iter().enumerate() {
        let Some(hash_id) = query::hash_id_for(controller, hash) else {
            result.errors.push(format!("{} has no database entry", query::hex(hash)));
            continue;
        };

        let info = query::file_info_for(controller, hash);
        let mime = info.as_ref().map(|info| info.mime).unwrap_or(Mime::Unknown);

        let Some(source) = query::file_path(controller, hash) else {
            result.errors.push(format!(
                "{} is missing from file storage",
                query::hex(hash)
            ));
            continue;
        };

        let tags = query::selection_tags(controller, hash);
        let filename = filename_for(&terms, hash, hash_id, index + 1, &tags);
        let filename = with_extension(&filename, mime);

        let mut filename = match safe_relative_path(&filename) {
            Some(filename) => filename,
            None => {
                result.errors.push(format!(
                    "{} produced an unsafe filename",
                    query::hex(hash)
                ));
                continue;
            }
        };

        // A phrase can map two files to one name; keep both by numbering the
        // later one, like Hydrus' `do_not_use_filenames` check.
        let parent = filename
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_default();
        let stem = filename
            .file_stem()
            .map(|stem| stem.to_string_lossy().to_string())
            .unwrap_or_else(|| query::hex(hash));
        let extension = filename
            .extension()
            .map(|extension| format!(".{}", extension.to_string_lossy()))
            .unwrap_or_default();

        let elided = elide_filename(&stem, limit);
        filename = parent.join(format!("{elided}{extension}"));

        let mut counter = 1;

        while !used_names.insert(filename.clone()) {
            filename = parent.join(format!("{elided} ({counter}){extension}"));
            counter += 1;
        }

        let destination = root.join(&filename);

        if !destination.starts_with(&root) {
            result.errors.push(format!(
                "{} resolved outside the export directory",
                query::hex(hash)
            ));
            continue;
        }

        if let Some(parent) = destination.parent() {
            if let Err(error) = std::fs::create_dir_all(parent) {
                result.errors.push(format!(
                    "could not create {}: {error}",
                    parent.display()
                ));
                continue;
            }
        }

        if file_is_current(&source, &destination) {
            result.skipped += 1;
        } else {
            match std::fs::copy(&source, &destination) {
                Ok(_) => result.exported += 1,
                Err(error) => {
                    result.errors.push(format!(
                        "could not copy {}: {error}",
                        source.display()
                    ));
                    continue;
                }
            }
        }

        synced.insert(destination.clone());

        if write_sidecar(
            &destination,
            &folder.sidecar_format,
            &tags,
            &query::urls_for(controller, hash),
            folder.overwrite_sidecars,
        ) {
            result.sidecars += 1;
        }

        if folder.delete_from_client && !folder.is_synchronise() {
            move_to_trash(controller, hash_id);
            result.deleted += 1;
        }
    }

    if folder.is_synchronise() {
        for stale in previous.difference(&synced) {
            if stale.starts_with(&root) && std::fs::remove_file(stale).is_ok() {
                result.synchronised_deleted += 1;
            }
        }

        remove_empty_directories(&root);
    }

    result
}

fn move_to_trash(controller: &Controller, hash_id: i64) {
    let db = controller.db().db();

    if let Some(trash) = query::trash_service_id(controller) {
        let _ = files_storage::add_files(
            db,
            trash,
            ContentStatus::Current,
            &[hash_id],
            Some(hydrus_time::get_now()),
        );
    }

    if let Some(service) = query::local_file_service_id(controller, "my files") {
        let _ = files_storage::remove_files(db, service, ContentStatus::Current, &[hash_id]);
    }

    let _ = files_storage::set_inbox(db, &[hash_id], false);
}

fn remove_empty_directories(root: &Path) {
    fn visit(directory: &Path, root: &Path) {
        let Ok(entries) = std::fs::read_dir(directory) else {
            return;
        };

        for entry in entries.flatten() {
            let path = entry.path();

            if path.is_dir() {
                visit(&path, root);
            }
        }

        if directory != root {
            let _ = std::fs::remove_dir(directory);
        }
    }

    visit(root, root);
}

pub fn load(path: &Path) -> Vec<ExportFolderEntry> {
    let Ok(text) = std::fs::read_to_string(path) else {
        return Vec::new();
    };

    serde_json::from_str(&text).unwrap_or_default()
}

pub fn save(path: &Path, folders: &[ExportFolderEntry]) -> std::io::Result<()> {
    let dump = serde_json::to_string_pretty(folders).map_err(std::io::Error::other)?;
    std::fs::write(path, dump)
}

pub fn persist(state: &AppState) -> std::io::Result<()> {
    save(
        &state.controller.db_dir().join(FILENAME),
        &state.export_folders,
    )
}

fn default_phrase_from_options(state: &AppState) -> String {
    let phrase = state.controller.options().get_string("export_phrase");

    if phrase.trim().is_empty() {
        default_phrase()
    } else {
        phrase
    }
}

pub fn edit(state: &AppState, payload: &str) -> Vec<UiCommand> {
    let name = payload.trim();
    let existing = state
        .export_folders
        .iter()
        .find(|folder| folder.name == name)
        .cloned();

    let folder = match existing {
        Some(folder) => folder_json(&folder),
        None => {
            let default_path = state
                .controller
                .options()
                .get_noneable_string("export_path")
                .unwrap_or_default();
            let mut entry = ExportFolderEntry::new(
                "",
                default_path,
                3600,
                default_phrase_from_options(state),
            );
            entry.query = String::new();
            folder_json(&entry)
        }
    };

    vec![UiCommand::ExportFolderDialog(
        serde_json::json!({"folder": folder, "existing": !name.is_empty()}).to_string(),
    )]
}

/// Saves the values produced by the export folder dialog.
pub fn save_action(state: &mut AppState, payload: &str) -> Vec<UiCommand> {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(payload) else {
        return vec![UiCommand::NotifyMessage(
            "invalid export folder payload".to_string(),
        )];
    };

    let path_text = value
        .get("path")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .trim();

    if path_text.is_empty() {
        return vec![UiCommand::NotifyMessage(
            "choose an export folder first".to_string(),
        )];
    }

    let path = PathBuf::from(path_text);

    if path.exists() && !path.is_dir() {
        return vec![UiCommand::NotifyMessage(format!(
            "{} is not a directory",
            path.display()
        ))];
    }

    if let Err(error) = std::fs::create_dir_all(&path) {
        return vec![UiCommand::NotifyMessage(format!(
            "could not create {}: {error}",
            path.display()
        ))];
    }

    let phrase = value
        .get("phrase")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|phrase| !phrase.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| default_phrase_from_options(state));

    if let Err(error) = parse_phrase(&phrase) {
        return vec![UiCommand::NotifyMessage(format!(
            "invalid filename phrase: {error}"
        ))];
    }

    let original = value
        .get("original")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .trim();
    let name = value
        .get("name")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| {
            path.file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string()
        });

    let mut entry = state
        .export_folders
        .iter()
        .find(|folder| folder.name == original)
        .cloned()
        .unwrap_or_else(|| ExportFolderEntry::new(name.clone(), path.clone(), 3600, phrase.clone()));

    entry.name = name.clone();
    entry.path = path;
    entry.phrase = phrase;
    apply_payload(&mut entry, &value);

    state
        .export_folders
        .retain(|folder| folder.name != original && folder.name != name);
    state.export_folders.push(entry);
    state.export_folders.sort_by(|a, b| a.name.cmp(&b.name));
    state.message = format!("export folder '{name}' saved");
    let _ = persist(state);

    page_commands(state)
}

fn apply_payload(entry: &mut ExportFolderEntry, value: &serde_json::Value) {
    if let Some(query) = value.get("query").and_then(serde_json::Value::as_str) {
        entry.query = query.trim().to_string();
    }

    if let Some(export_type) = value
        .get("export_type")
        .and_then(serde_json::Value::as_str)
    {
        entry.export_type = match export_type {
            "synchronise" => "synchronise".to_string(),
            _ => "copy".to_string(),
        };
    }

    if let Some(format) = value
        .get("sidecar_format")
        .and_then(serde_json::Value::as_str)
    {
        entry.sidecar_format = match format {
            "json" | "text" => format.to_string(),
            _ => "none".to_string(),
        };
    }

    if let Some(overwrite) = value
        .get("overwrite_sidecars")
        .and_then(serde_json::Value::as_bool)
    {
        entry.overwrite_sidecars = overwrite;
    }

    if let Some(delete) = value
        .get("delete_from_client")
        .and_then(serde_json::Value::as_bool)
    {
        entry.delete_from_client = delete;
    }

    if let Some(regular) = value
        .get("run_regularly")
        .and_then(serde_json::Value::as_bool)
    {
        entry.run_regularly = regular;
    }

    if let Some(period) = value.get("period").and_then(serde_json::Value::as_i64) {
        entry.period = period.max(60);
    }
}

pub fn remove(state: &mut AppState, payload: &str) -> Vec<UiCommand> {
    let name = payload.trim();
    let before = state.export_folders.len();
    state.export_folders.retain(|folder| folder.name != name);

    state.message = if state.export_folders.len() == before {
        format!("export folder '{name}' was not found")
    } else {
        format!("export folder '{name}' removed")
    };
    let _ = persist(state);

    page_commands(state)
}

pub fn toggle(state: &mut AppState, payload: &str) -> Vec<UiCommand> {
    let name = payload.trim();

    if let Some(folder) = state
        .export_folders
        .iter_mut()
        .find(|folder| folder.name == name)
    {
        folder.paused = !folder.paused;
        folder.status = if folder.paused {
            "paused".to_string()
        } else {
            "ready".to_string()
        };
    }

    let _ = persist(state);
    page_commands(state)
}

pub fn reveal(state: &AppState, payload: &str) -> Vec<UiCommand> {
    let name = payload.trim();

    match state
        .export_folders
        .iter()
        .find(|folder| folder.name == name)
    {
        Some(folder) => vec![UiCommand::OpenPath(folder.path.display().to_string())],
        None => vec![UiCommand::NotifyMessage(
            "export folder is missing".to_string(),
        )],
    }
}

fn parse_run_payload(payload: &str) -> (String, Option<Vec<Vec<u8>>>) {
    let payload = payload.trim();

    if !payload.starts_with('{') {
        return (payload.to_string(), None);
    }

    let Ok(value) = serde_json::from_str::<serde_json::Value>(payload) else {
        return (payload.to_string(), None);
    };

    let name = value
        .get("name")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_string();

    let hashes = value
        .get("hashes")
        .and_then(serde_json::Value::as_array)
        .map(|hashes| {
            hashes
                .iter()
                .filter_map(serde_json::Value::as_str)
                .filter_map(query::unhex)
                .collect()
        });

    (name, hashes)
}

pub fn run_now(state: &mut AppState, payload: &str) -> Vec<UiCommand> {
    let (name, selected) = parse_run_payload(payload);
    let name = name.as_str();
    let phrase_default = default_phrase_from_options(state);

    let Some(folder) = state
        .export_folders
        .iter_mut()
        .find(|folder| folder.name == name)
    else {
        return vec![UiCommand::NotifyMessage(
            "export folder is missing".to_string(),
        )];
    };

    if state
        .controller
        .options()
        .get_boolean("pause_export_folders_sync")
    {
        folder.status = "paused by options".to_string();
        return vec![UiCommand::NotifyMessage(
            "export folders are paused in the options".to_string(),
        )];
    }

    if folder.running {
        return vec![UiCommand::NotifyMessage(format!(
            "export folder '{name}' is already running"
        ))];
    }

    if !folder.path.is_dir() {
        if let Err(error) = std::fs::create_dir_all(&folder.path) {
            folder.status = format!("missing destination: {error}");
            let _ = persist(state);
            return page_commands(state);
        }
    }

    if folder.phrase.trim().is_empty() {
        folder.phrase = phrase_default;
    }

    let job = AsyncJob::ExportFolder {
        name: folder.name.clone(),
        folder: folder.clone(),
        hashes: selected,
    };

    match state.runtime.submit(job) {
        Ok(()) => {
            folder.running = true;
            folder.status = "queued".to_string();
            folder.last_error.clear();
            state.message = format!("export folder '{name}' queued");
        }
        Err(error) => {
            folder.status = error.clone();
            folder.last_error = error.clone();
            state.message = error;
        }
    }

    let _ = persist(state);
    page_commands(state)
}

pub fn run_all(state: &mut AppState) -> Vec<UiCommand> {
    let names: Vec<String> = state
        .export_folders
        .iter()
        .map(|folder| folder.name.clone())
        .collect();

    if names.is_empty() {
        return vec![UiCommand::NotifyMessage(
            "there are no export folders".to_string(),
        )];
    }

    let mut commands = Vec::new();

    for name in names {
        commands.extend(run_now(state, &name));
    }

    commands
}

pub fn tick(state: &mut AppState) -> Vec<UiCommand> {
    if state
        .controller
        .options()
        .get_boolean("pause_export_folders_sync")
    {
        return Vec::new();
    }

    let now = hydrus_time::get_now();
    let due: Vec<String> = state
        .export_folders
        .iter()
        .filter(|folder| folder.is_due(now))
        .map(|folder| folder.name.clone())
        .collect();

    if due.is_empty() {
        return Vec::new();
    }

    for name in due {
        run_now(state, &name);
    }

    page_commands(state)
}

pub fn apply_result(
    state: &mut AppState,
    name: &str,
    result: ExportResult,
    checked_at: i64,
) -> Vec<UiCommand> {
    let Some(folder) = state
        .export_folders
        .iter_mut()
        .find(|folder| folder.name == name)
    else {
        return Vec::new();
    };

    folder.running = false;
    folder.last_check = checked_at;
    folder.last_exported = result.exported;
    folder.status = result.message();
    folder.last_error = result.errors.first().cloned().unwrap_or_default();

    if !result.errors.is_empty() {
        state
            .notifications
            .push("error", format!("export folder '{name}': {}", result.errors[0]));
    }

    state.message = format!("export folder '{name}': {}", folder.status);
    let _ = persist(state);

    page_commands(state)
}

/// Builds the list shown by the review dialog: every file the folder would
/// export, its generated destination and whether the source and destination
/// already exist.
pub fn review(state: &AppState, payload: &str) -> Vec<UiCommand> {
    let name = payload.trim();

    let Some(folder) = state
        .export_folders
        .iter()
        .find(|folder| folder.name == name)
    else {
        return vec![UiCommand::NotifyMessage(
            "export folder is missing".to_string(),
        )];
    };

    let hashes = match query::hashes_for(&state.controller, &folder.query, "my files") {
        Ok(hashes) => hashes,
        Err(error) => {
            return vec![UiCommand::NotifyMessage(format!("search failed: {error}"))];
        }
    };

    let terms = match parse_phrase(&folder.phrase) {
        Ok(terms) => terms,
        Err(error) => {
            return vec![UiCommand::NotifyMessage(format!(
                "invalid filename phrase: {error}"
            ))];
        }
    };

    let limit = state
        .controller
        .options()
        .get_integer("export_filename_character_limit");
    let limit = if limit <= 0 { 255 } else { limit.clamp(32, 255) } as usize;

    let mut entries = Vec::with_capacity(hashes.len());
    let root = std::fs::canonicalize(&folder.path).unwrap_or_else(|_| folder.path.clone());

    for (index, hash) in hashes.iter().enumerate() {
        let Some(hash_id) = query::hash_id_for(&state.controller, hash) else {
            continue;
        };

        let info = query::file_info_for(&state.controller, hash);
        let mime = info.as_ref().map(|info| info.mime).unwrap_or(Mime::Unknown);
        let tags = query::selection_tags(&state.controller, hash);
        let filename = with_extension(
            &filename_for(&terms, hash, hash_id, index + 1, &tags),
            mime,
        );

        let destination = safe_relative_path(&filename)
            .map(|relative| root.join(relative))
            .map(|path| {
                let stem = path
                    .file_stem()
                    .map(|stem| stem.to_string_lossy().to_string())
                    .unwrap_or_default();
                let extension = path
                    .extension()
                    .map(|extension| format!(".{}", extension.to_string_lossy()))
                    .unwrap_or_default();
                let elided = elide_filename(&stem, limit);

                path.with_file_name(format!("{elided}{extension}"))
            });

        entries.push(serde_json::json!({
            "hash": query::hex(hash),
            "tags": tags,
            "mime": mime.mimetype_str(),
            "destination": destination.as_ref().map(|path| path.display().to_string()).unwrap_or_default(),
            "safe": destination.is_some(),
            "exists": destination.as_ref().is_some_and(|path| path.exists()),
            "source_exists": query::file_path(&state.controller, hash).is_some(),
        }));
    }

    vec![UiCommand::ExportReviewDialog(
        serde_json::json!({
            "name": folder.name,
            "destination": folder.path.display().to_string(),
            "query": folder.query,
            "files": entries,
        })
        .to_string(),
    )]
}

pub fn folder_json(folder: &ExportFolderEntry) -> serde_json::Value {
    let mut pretty_type = if folder.is_synchronise() {
        "synchronise".to_string()
    } else {
        "regular".to_string()
    };

    if folder.delete_from_client {
        pretty_type.push_str(" and deleting from the client!");
    }

    let pretty_period = if folder.run_regularly {
        pretty_period(folder.period)
    } else {
        "not running regularly".to_string()
    };

    serde_json::json!({
        "name": folder.name,
        "path": folder.path.display().to_string(),
        "query": folder.query,
        "phrase": folder.phrase,
        "export_type": folder.export_type,
        "sidecar_format": folder.sidecar_format,
        "overwrite_sidecars": folder.overwrite_sidecars,
        "delete_from_client": folder.delete_from_client,
        "run_regularly": folder.run_regularly,
        "period": folder.period,
        "paused": folder.paused,
        "last_check": folder.last_check,
        "last_exported": folder.last_exported,
        "status": folder.status,
        "last_error": folder.last_error,
        "running": folder.running,
        "exists": folder.path.is_dir(),
        "pretty_type": pretty_type,
        "pretty_period": pretty_period,
    })
}

/// Python's `HydrusTime.TimeDeltaToPrettyTimeDelta`: up to two units, bigger
/// first, without the trailing zero units.
fn pretty_period(seconds: i64) -> String {
    if seconds <= 0 {
        return "0 seconds".to_string();
    }

    let mut remaining = seconds;
    let mut parts: Vec<String> = Vec::new();

    for (name, size) in [
        ("year", 31_557_600_i64),
        ("month", 2_629_800),
        ("day", 86_400),
        ("hour", 3_600),
        ("minute", 60),
        ("second", 1),
    ] {
        let quantity = remaining / size;
        remaining %= size;

        if quantity > 0 {
            let plural = if quantity > 1 { "s" } else { "" };
            parts.push(format!("{quantity} {name}{plural}"));

            if parts.len() == 2 {
                break;
            }
        } else if !parts.is_empty() {
            break;
        }
    }

    parts.join(" ")
}

pub fn folders_json(state: &AppState, page_id: &str) -> String {
    let folders: Vec<serde_json::Value> = state.export_folders.iter().map(folder_json).collect();

    serde_json::json!({
        "page_id": page_id,
        "title": state.find_export_folders(page_id).map(|page| page.title.as_str()).unwrap_or("export folders"),
        "status": state.find_export_folders(page_id).map(|page| page.status.as_str()).unwrap_or(""),
        "folders": folders,
    })
    .to_string()
}

/// Python's `edit export folders` manager dialog. The folder list is shared
/// with the legacy export folders page.
pub fn dialog_json(state: &AppState) -> String {
    let folders: Vec<serde_json::Value> = state.export_folders.iter().map(folder_json).collect();

    serde_json::json!({
        "folders": folders,
    })
    .to_string()
}

pub fn manage(state: &AppState) -> Vec<UiCommand> {
    vec![UiCommand::ExportFoldersDialog(dialog_json(state))]
}

pub fn create_page(state: &mut AppState, notebook_id: &str) -> Vec<UiCommand> {
    let id = state.new_id("p");
    let page = ExportFoldersPage::new(id.clone());

    let Some(notebook) = state.find_notebook_mut(notebook_id) else {
        return vec![UiCommand::NotifyMessage("notebook is missing".to_string())];
    };

    notebook.add(PageNode::ExportFolders(page));
    state.active = Some(id);
    page_commands(state)
}

fn page_commands(state: &AppState) -> Vec<UiCommand> {
    let ids = state.export_folder_page_ids();

    let mut commands = vec![UiCommand::BuildPages(render::pages_json(state))];

    for id in ids {
        commands.push(UiCommand::UpdateExportFolders(
            id.clone(),
            folders_json(state, &id),
        ));
    }

    // No-op in Qt when the manager dialog is closed.
    commands.push(UiCommand::UpdateExportFoldersDialog(dialog_json(state)));
    commands.push(UiCommand::Status(render::status_json(state)));
    commands
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tags() -> Vec<String> {
        vec![
            "creator:alice".to_string(),
            "creator:bob".to_string(),
            "blue eyes".to_string(),
            "series:one".to_string(),
        ]
    }

    #[test]
    fn phrases_are_split_into_namespaces_predicates_and_tags() {
        let terms = parse_phrase("[creator] - {hash} (blue eyes)").unwrap();

        assert_eq!(
            terms,
            vec![
                PhraseTerm::Namespace("creator".to_string()),
                PhraseTerm::Text(" - ".to_string()),
                PhraseTerm::Predicate("hash".to_string()),
                PhraseTerm::Text(" ".to_string()),
                PhraseTerm::Tag("blue eyes".to_string()),
            ]
        );

        assert!(parse_phrase("{hash").is_err());
        assert!(parse_phrase("").is_err());
    }

    #[test]
    fn filenames_expand_tags_and_numbers() {
        let terms = parse_phrase("[creator] {hash} {#} {tags} {nn tags} (blue eyes)").unwrap();
        let hash = vec![0xab, 0xcd];
        let filename = filename_for(&terms, &hash, 42, 7, &tags());

        assert_eq!(
            filename,
            "alice, bob abcd 7 alice, blue eyes, bob, one blue eyes blue eyes"
        );
    }

    #[test]
    fn tag_text_cannot_escape_the_export_directory() {
        let terms = parse_phrase("{tags}").unwrap();
        let tags = vec!["bad/name".to_string(), "worse\\name".to_string()];
        let filename = filename_for(&terms, &[1], 1, 1, &tags);
        let relative = safe_relative_path(&filename).unwrap();

        assert_eq!(relative, PathBuf::from("bad_name, worse_name"));
        assert!(safe_relative_path("../escape").is_none());
        assert!(safe_relative_path("a/../../b").is_none());
        assert_eq!(
            safe_relative_path("sub/dir/file.png").unwrap(),
            PathBuf::from("sub/dir/file.png")
        );
    }

    #[test]
    fn run_payload_accepts_a_name_or_a_reviewed_hash_list() {
        let (name, hashes) = parse_run_payload(" pictures ");
        assert_eq!(name, "pictures");
        assert!(hashes.is_none());

        let (name, hashes) = parse_run_payload(
            r#"{"name":"pictures","hashes":["abcd","zzzz"]}"#,
        );
        assert_eq!(name, "pictures");
        assert_eq!(hashes, Some(vec![vec![0xab, 0xcd]]));
    }

    #[test]
    fn export_folder_rows_match_python_display_text() {
        let mut folder = ExportFolderEntry::new("export folder", "out", 86_400, "{hash}");
        folder.query = String::new();

        let row = folder_json(&folder);
        assert_eq!(row["pretty_type"], "regular");
        assert_eq!(row["pretty_period"], "1 day");

        folder.export_type = "synchronise".to_string();
        folder.delete_from_client = true;
        assert_eq!(
            folder_json(&folder)["pretty_type"],
            "synchronise and deleting from the client!"
        );

        folder.run_regularly = false;
        assert_eq!(folder_json(&folder)["pretty_period"], "not running regularly");
    }

    #[test]
    fn periods_are_pretty_printed_like_python() {
        assert_eq!(pretty_period(300), "5 minutes");
        assert_eq!(pretty_period(3_600), "1 hour");
        assert_eq!(pretty_period(90_000), "1 day 1 hour");
        assert_eq!(pretty_period(172_800), "2 days");
        assert_eq!(pretty_period(45), "45 seconds");
    }

    #[test]
    fn folders_become_due_after_their_period() {
        let mut folder = ExportFolderEntry::new("out", ".", 600, "{hash}");
        folder.last_check = 1_000;

        assert!(!folder.is_due(1_599));
        assert!(folder.is_due(1_600));

        folder.paused = true;
        assert!(!folder.is_due(9_999));
    }

    #[test]
    fn synchronise_runs_delete_stale_files_and_empty_directories() {
        let root = std::env::temp_dir().join(format!(
            "hydrus_export_sync_{}_{}",
            std::process::id(),
            rand::random::<u64>()
        ));
        let nested = root.join("old");
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::write(nested.join("stale.jpg"), b"old").unwrap();
        std::fs::write(root.join("keep.jpg"), b"keep").unwrap();

        let mut previous = BTreeSet::new();
        files_under(&root, &mut previous);
        assert_eq!(previous.len(), 2);

        let mut synced = BTreeSet::new();
        synced.insert(root.join("keep.jpg"));

        for stale in previous.difference(&synced) {
            std::fs::remove_file(stale).unwrap();
        }
        remove_empty_directories(&root);

        assert!(!nested.exists());
        assert!(root.join("keep.jpg").exists());

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn exports_imported_files_and_skips_unchanged_runs() {
        use crate::controller::Controller;
        use crate::gui::app::import::import_paths;

        let directory = std::env::temp_dir().join(format!(
            "hydrus_export_run_{}_{}",
            std::process::id(),
            rand::random::<u64>()
        ));
        let source_directory = directory.join("source");
        let export_directory = directory.join("export");
        std::fs::create_dir_all(&source_directory).unwrap();
        std::fs::create_dir_all(&export_directory).unwrap();

        let _ = crate::client_options::register_serialisable_type();

        let controller = Controller::new(&directory).unwrap();
        controller.init_model().unwrap();

        let source = source_directory.join("source.png");
        image::RgbaImage::from_pixel(8, 6, image::Rgba([10, 20, 30, 255]))
            .save(&source)
            .unwrap();

        let summary = import_paths(&controller, &[source]);
        assert_eq!(summary.imported, 1, "{:?}", summary.errors);

        let hash_hex = query::hex(&summary.hashes[0]);
        let mut folder = ExportFolderEntry::new("out", &export_directory, 3600, "{hash}");
        folder.sidecar_format = "json".to_string();

        let result = execute(&controller, &folder);

        assert_eq!(result.exported, 1, "{:?}", result.errors);
        assert_eq!(result.errors, Vec::<String>::new());

        let exported = export_directory.join(format!("{hash_hex}.png"));
        assert!(exported.exists(), "the image was not copied");
        assert!(export_directory.join(format!("{hash_hex}.tags.json")).exists());

        folder.export_type = "synchronise".to_string();
        std::fs::write(export_directory.join("stale.txt"), b"stale").unwrap();

        let result = execute(&controller, &folder);

        assert_eq!(result.exported, 0);
        assert_eq!(result.skipped, 1);
        assert!(!export_directory.join("stale.txt").exists());

        drop(controller);
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn sidecars_are_written_once_unless_overwrite_is_set() {
        let directory = std::env::temp_dir().join(format!(
            "hydrus_export_sidecar_{}_{}",
            std::process::id(),
            rand::random::<u64>()
        ));
        std::fs::create_dir_all(&directory).unwrap();
        let media = directory.join("image.jpg");

        assert!(write_sidecar(
            &media,
            "json",
            &["blue eyes".to_string()],
            &[],
            false
        ));
        assert!(!write_sidecar(&media, "json", &[], &[], false));

        let sidecar = directory.join("image.tags.json");
        let text = std::fs::read_to_string(&sidecar).unwrap();
        assert!(text.contains("blue eyes"));

        assert!(write_sidecar(
            &media,
            "json",
            &["new".to_string()],
            &[],
            true
        ));
        assert!(std::fs::read_to_string(&sidecar).unwrap().contains("new"));

        let _ = std::fs::remove_dir_all(directory);
    }
}
