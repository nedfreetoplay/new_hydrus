//! Local import review panel from
//! `hydrus/client/gui/panels/ClientGUILocalFileImports.py` and
//! `hydrus/client/importing/ClientImportLocalFileParse.py`.
//!
//! Python parses a queue of paths in a background thread and shows the result
//! in a review frame before importing. The port parses synchronously when paths
//! are added; the list is small enough for the dialog and every classification
//! is the same: good, empty, missing, unsupported, occupied or sidecar.
//!
//! The review state is deliberately ephemeral: Python creates a new panel for
//! every import, so nothing here is written to the session.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use hydrus_core::files::hydrus_file_handling as core_files;
use hydrus_core::hydrus_constants::{Mime, ServiceType, allowed_mimes};
use hydrus_core::hydrus_paths;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::importing::FilenameTaggingOptions;

use super::UiCommand;
use super::model::AppState;
use super::worker::AsyncJob;

/// Mirrors `ClientImportLocalFileParse.RESULT_*`.
pub const RESULT_NOT_PARSED: i32 = 0;
pub const RESULT_GOOD: i32 = 1;
pub const RESULT_EMPTY: i32 = 2;
pub const RESULT_MISSING: i32 = 3;
pub const RESULT_UNIMPORTABLE: i32 = 4;
pub const RESULT_OCCUPIED: i32 = 5;
pub const RESULT_SIDECAR: i32 = 6;
pub const RESULT_DIRECTORY: i32 = 7;

const SIDECAR_EXTENSIONS: &[&str] = &["txt", "json", "xml"];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LocalFileResult {
    NotParsed,
    Good,
    Empty,
    Missing,
    Unimportable,
    Occupied,
    Sidecar,
    Directory,
}

impl LocalFileResult {
    pub fn code(self) -> i32 {
        match self {
            Self::NotParsed => RESULT_NOT_PARSED,
            Self::Good => RESULT_GOOD,
            Self::Empty => RESULT_EMPTY,
            Self::Missing => RESULT_MISSING,
            Self::Unimportable => RESULT_UNIMPORTABLE,
            Self::Occupied => RESULT_OCCUPIED,
            Self::Sidecar => RESULT_SIDECAR,
            Self::Directory => RESULT_DIRECTORY,
        }
    }

    pub fn from_code(code: i32) -> Self {
        match code {
            RESULT_GOOD => Self::Good,
            RESULT_EMPTY => Self::Empty,
            RESULT_MISSING => Self::Missing,
            RESULT_UNIMPORTABLE => Self::Unimportable,
            RESULT_OCCUPIED => Self::Occupied,
            RESULT_SIDECAR => Self::Sidecar,
            RESULT_DIRECTORY => Self::Directory,
            _ => Self::NotParsed,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::NotParsed => "not yet parsed",
            Self::Good => "good (you should not see this)",
            Self::Empty => "PROBLEM: file empty",
            Self::Missing => "PROBLEM: file missing",
            Self::Unimportable => "PROBLEM: filetype unsupported",
            Self::Occupied => "PROBLEM: file in use by other program",
            Self::Sidecar => "sidecar",
            Self::Directory => "directory (you should not see this)",
        }
    }

    pub fn is_bad(self) -> bool {
        matches!(
            self,
            Self::Empty | Self::Missing | Self::Unimportable | Self::Occupied
        )
    }
}

#[derive(Debug, Clone)]
pub struct LocalFileParse {
    pub path: PathBuf,
    pub result: LocalFileResult,
    pub index: usize,
    pub mime: Option<Mime>,
    pub size: u64,
}

impl LocalFileParse {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            result: LocalFileResult::NotParsed,
            index: 0,
            mime: None,
            size: 0,
        }
    }

    pub fn pretty_mime(&self) -> String {
        match (self.result, self.mime) {
            (LocalFileResult::Good, Some(mime)) => mime.as_str().to_string(),
            _ => self.result.label().to_string(),
        }
    }

    pub fn size_display(&self) -> String {
        if matches!(
            self.result,
            LocalFileResult::Missing | LocalFileResult::Occupied
        ) {
            return "-".to_string();
        }

        crate::client_data::to_human_bytes(Some(self.size), 3)
    }

    /// `LocalFileParse.DoFileParse`
    fn parse(&mut self, comparable_sidecar_prefixes: &BTreeSet<PathBuf>) {
        let Ok(metadata) = std::fs::metadata(&self.path) else {
            self.result = LocalFileResult::Missing;
            return;
        };

        if metadata.is_dir() {
            self.result = LocalFileResult::Directory;
            return;
        }

        self.size = metadata.len();

        if looks_like_sidecar(&self.path, comparable_sidecar_prefixes) {
            self.result = LocalFileResult::Sidecar;
            return;
        }

        if !hydrus_paths::path_is_free(&self.path) {
            self.result = LocalFileResult::Occupied;
            return;
        }

        if self.size == 0 {
            self.result = LocalFileResult::Empty;
            return;
        }

        let name = self
            .path
            .file_name()
            .map(|name| name.to_string_lossy().to_ascii_lowercase())
            .unwrap_or_default();

        if name == "thumbs.db" {
            self.result = LocalFileResult::Unimportable;
            return;
        }

        match core_files::get_mime(&self.path, false) {
            Ok(mime) if allowed_mimes().contains(&mime) => {
                self.mime = Some(mime);
                self.result = LocalFileResult::Good;
            }
            _ => {
                self.result = LocalFileResult::Unimportable;
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct LocalImportReview {
    pub paths: Vec<LocalFileParse>,
    pub search_subdirectories: bool,
    pub delete_after_success: bool,
    /// Hex local tag service key -> filename tagging rules. Python keeps this
    /// per panel; it is not saved when the review closes.
    pub filename_tagging: BTreeMap<String, FilenameTaggingOptions>,
}

impl Default for LocalImportReview {
    fn default() -> Self {
        Self {
            paths: Vec::new(),
            search_subdirectories: true,
            delete_after_success: false,
            filename_tagging: BTreeMap::new(),
        }
    }
}

impl LocalImportReview {
    pub fn reset(&mut self) {
        self.paths.clear();
        self.search_subdirectories = true;
        self.delete_after_success = false;
        self.filename_tagging.clear();
    }

    /// Adds files and expands folders, then reparses the affected list.
    /// Duplicate paths are ignored, matching Python's `HasData` check.
    pub fn add_paths(&mut self, paths: &[PathBuf]) {
        let known: BTreeSet<PathBuf> = self.paths.iter().map(|parse| parse.path.clone()).collect();
        let mut new_paths: Vec<PathBuf> = Vec::new();

        for path in paths {
            let mut candidates = Vec::new();
            collect_candidates(path, self.search_subdirectories, &mut candidates);
            candidates.sort();

            for candidate in candidates {
                if known.contains(&candidate) || new_paths.contains(&candidate) {
                    continue;
                }

                new_paths.push(candidate);
            }
        }

        if new_paths.is_empty() {
            return;
        }

        let prefixes = comparable_sidecar_prefixes(
            self.paths
                .iter()
                .map(|parse| parse.path.clone())
                .chain(new_paths.iter().cloned()),
        );

        for path in new_paths {
            let mut parse = LocalFileParse::new(path);
            parse.parse(&prefixes);
            self.paths.push(parse);
        }

        self.reindex();
    }

    pub fn remove_paths(&mut self, paths: &[PathBuf]) -> usize {
        let before = self.paths.len();
        self.paths.retain(|parse| !paths.contains(&parse.path));
        self.reindex();
        before - self.paths.len()
    }

    pub fn good_paths(&self) -> Vec<PathBuf> {
        let mut paths: Vec<(usize, PathBuf)> = self
            .paths
            .iter()
            .filter(|parse| parse.result == LocalFileResult::Good)
            .map(|parse| (parse.index, parse.path.clone()))
            .collect();
        paths.sort_by_key(|(index, _)| *index);
        paths.into_iter().map(|(_, path)| path).collect()
    }

    pub fn sidecar_paths(&self) -> Vec<PathBuf> {
        self.paths
            .iter()
            .filter(|parse| parse.result == LocalFileResult::Sidecar)
            .map(|parse| parse.path.clone())
            .collect()
    }

    pub fn counts(&self) -> ReviewCounts {
        let mut counts = ReviewCounts {
            total: self.paths.len(),
            ..ReviewCounts::default()
        };

        for parse in &self.paths {
            match parse.result {
                LocalFileResult::Good => counts.good += 1,
                LocalFileResult::Empty => counts.empty += 1,
                LocalFileResult::Missing => counts.missing += 1,
                LocalFileResult::Unimportable => counts.unimportable += 1,
                LocalFileResult::Occupied => counts.occupied += 1,
                LocalFileResult::Sidecar => counts.sidecar += 1,
                LocalFileResult::Directory => counts.directory += 1,
                LocalFileResult::NotParsed => counts.not_parsed += 1,
            }
        }

        counts.bad = counts.empty + counts.missing + counts.unimportable + counts.occupied;
        counts
    }

    pub fn summary(&self) -> String {
        let counts = self.counts();

        if counts.total == 0 {
            return "waiting for paths to parse.".to_string();
        }

        let mut message = format!("{} files parsed", counts.total);

        if counts.bad > 0 {
            message.push_str(" - ");

            if counts.good > 0 {
                message.push_str(&format!("{} good | {} bad", counts.good, counts.bad));
            } else if counts.bad == counts.total {
                message.push_str("all bad");
            }

            message.push_str(": ");

            let mut comments = Vec::new();

            if counts.empty > 0 {
                comments.push(format!("{} were empty", counts.empty));
            }

            if counts.missing > 0 {
                comments.push(format!("{} were missing", counts.missing));
            }

            if counts.unimportable > 0 {
                comments.push(format!(
                    "{} had unsupported file types",
                    counts.unimportable
                ));
            }

            if counts.occupied > 0 {
                comments.push(format!(
                    "{} were inaccessible (maybe in use by another process)",
                    counts.occupied
                ));
            }

            message.push_str(&comments.join(", "));
        }

        if counts.sidecar > 0 {
            message.push_str(&format!(
                " - and looks like {} txt/json/xml sidecars",
                counts.sidecar
            ));
        }

        message.push('.');
        message
    }

    fn reindex(&mut self) {
        for (index, parse) in self.paths.iter_mut().enumerate() {
            parse.index = index + 1;
        }
    }

    /// Hex-keyed tagging map in the shape the worker wants.
    pub fn worker_filename_tagging(&self) -> BTreeMap<Vec<u8>, FilenameTaggingOptions> {
        self.filename_tagging
            .iter()
            .filter_map(|(key, options)| super::query::unhex(key).map(|key| (key, options.clone())))
            .collect()
    }
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct ReviewCounts {
    pub total: usize,
    pub good: usize,
    pub bad: usize,
    pub empty: usize,
    pub missing: usize,
    pub unimportable: usize,
    pub occupied: usize,
    pub sidecar: usize,
    pub directory: usize,
    pub not_parsed: usize,
}

pub fn dialog_json(state: &AppState) -> String {
    dialog_json_for(state, false, "")
}

fn dialog_json_for(state: &AppState, import_after_save: bool, note: &str) -> String {
    let review = &state.local_import_review;
    let counts = review.counts();

    let paths: Vec<Value> = review
        .paths
        .iter()
        .map(|parse| {
            json!({
                "index": parse.index,
                "path": parse.path.display().to_string(),
                "result": parse.result,
                "result_text": parse.pretty_mime(),
                "size": parse.size,
                "size_display": parse.size_display(),
                "bad": parse.result.is_bad(),
                "sidecar": parse.result == LocalFileResult::Sidecar,
            })
        })
        .collect();

    json!({
        "title": "review files to import",
        "summary": review.summary(),
        "paths": paths,
        "counts": counts,
        "search_subdirectories": review.search_subdirectories,
        "delete_after_success": review.delete_after_success,
        "filename_tagging": review.filename_tagging,
        "tag_services": tag_services(state),
        "can_import": counts.good > 0,
        "good_count": counts.good,
        "import_after_save": import_after_save,
        "note": note,
    })
    .to_string()
}

fn tag_services(state: &AppState) -> Vec<Value> {
    state
        .controller
        .services()
        .get_services(Some(&[ServiceType::LocalTag]))
        .into_iter()
        .map(|service| {
            json!({
                "key": super::query::hex(&service.service_key),
                "name": service.name,
            })
        })
        .collect()
}

pub fn open(state: &mut AppState) -> Vec<UiCommand> {
    state.local_import_review.reset();

    vec![UiCommand::LocalImportReviewDialog(dialog_json(state))]
}

pub fn open_paths(state: &mut AppState, payload: &str) -> Vec<UiCommand> {
    state.local_import_review.reset();
    add_paths(state, payload)
}

pub fn add_paths(state: &mut AppState, payload: &str) -> Vec<UiCommand> {
    let paths = parse_paths(payload);

    if paths.is_empty() {
        return vec![UiCommand::NotifyMessage(
            "no files or folders to add".to_string(),
        )];
    }

    state.local_import_review.add_paths(&paths);

    vec![UiCommand::LocalImportReviewDialog(dialog_json(state))]
}

pub fn remove_paths(state: &mut AppState, payload: &str) -> Vec<UiCommand> {
    let paths = parse_paths(payload);

    if paths.is_empty() {
        return vec![UiCommand::NotifyMessage(
            "select some paths first".to_string(),
        )];
    }

    let removed = state.local_import_review.remove_paths(&paths);

    vec![
        UiCommand::LocalImportReviewDialog(dialog_json(state)),
        UiCommand::NotifyMessage(format!("removed {removed} path(s)")),
    ]
}

pub fn set_search_subdirectories(state: &mut AppState, payload: &str) -> Vec<UiCommand> {
    let value = payload_value(payload);
    state.local_import_review.search_subdirectories = value
        .get("value")
        .and_then(Value::as_bool)
        .unwrap_or_else(|| payload.trim().eq_ignore_ascii_case("true"));

    vec![UiCommand::LocalImportReviewDialog(dialog_json(state))]
}

pub fn set_delete_after_success(state: &mut AppState, payload: &str) -> Vec<UiCommand> {
    let value = payload_value(payload);
    state.local_import_review.delete_after_success = value
        .get("value")
        .and_then(Value::as_bool)
        .unwrap_or_else(|| payload.trim().eq_ignore_ascii_case("true"));

    vec![UiCommand::LocalImportReviewDialog(dialog_json(state))]
}

pub fn tagging_open(state: &mut AppState) -> Vec<UiCommand> {
    if state.local_import_review.good_paths().is_empty() {
        return vec![UiCommand::NotifyMessage(
            "add some good files before editing tags".to_string(),
        )];
    }

    vec![UiCommand::LocalImportTaggingDialog(tagging_json(
        state, true,
    ))]
}

fn tagging_json(state: &AppState, import_after_save: bool) -> String {
    let example_path = state
        .local_import_review
        .good_paths()
        .first()
        .map(|path| path.display().to_string())
        .unwrap_or_default();

    json!({
        "title": "filename tagging",
        "filename_tagging": state.local_import_review.filename_tagging,
        "tag_services": tag_services(state),
        "example_path": example_path,
        "import_after_save": import_after_save,
    })
    .to_string()
}

pub fn tagging_save(state: &mut AppState, payload: &str) -> Vec<UiCommand> {
    let Ok(value) = serde_json::from_str::<Value>(payload) else {
        return vec![UiCommand::NotifyMessage(
            "could not read the filename tagging".to_string(),
        )];
    };

    let import_after_save = value
        .get("import_after_save")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    let tagging: BTreeMap<String, FilenameTaggingOptions> = value
        .get("filename_tagging")
        .cloned()
        .and_then(|value| serde_json::from_value(value).ok())
        .unwrap_or_default();

    state.local_import_review.filename_tagging = tagging
        .into_iter()
        .filter(|(_, options)| options.get_summary() != "no tags")
        .collect();

    if import_after_save {
        import_now(state)
    } else {
        vec![UiCommand::LocalImportReviewDialog(dialog_json(state))]
    }
}

pub fn import_now(state: &mut AppState) -> Vec<UiCommand> {
    let good_paths = state.local_import_review.good_paths();

    if good_paths.is_empty() {
        return vec![UiCommand::NotifyMessage(
            "there are no good files to import".to_string(),
        )];
    }

    if state.import_queue.running {
        return vec![UiCommand::NotifyMessage(
            "an import is already running".to_string(),
        )];
    }

    let job = AsyncJob::ImportPaths {
        paths: good_paths.clone(),
        folder: None,
        filename_tagging: state.local_import_review.worker_filename_tagging(),
        delete_after_success: state.local_import_review.delete_after_success,
    };

    if let Err(error) = state.runtime.submit(job) {
        return vec![UiCommand::NotifyMessage(error)];
    }

    let message = format!("importing {} files", good_paths.len());
    state.import_queue.running = true;
    state.import_queue.done = 0;
    state.import_queue.total = good_paths.len();
    state.import_queue.imported = 0;
    state.import_queue.failed = 0;
    state.import_queue.message = message.clone();
    state.message = message;

    vec![
        UiCommand::CloseLocalImportReview,
        UiCommand::NotifyMessage(format!("queued {} files", good_paths.len())),
    ]
}

fn parse_paths(payload: &str) -> Vec<PathBuf> {
    let value = payload_value(payload);

    let paths: Vec<PathBuf> = match value {
        Value::Array(values) => values
            .iter()
            .filter_map(Value::as_str)
            .map(str::trim)
            .filter(|path| !path.is_empty())
            .map(PathBuf::from)
            .collect(),
        Value::Object(object) => object
            .get("paths")
            .and_then(Value::as_array)
            .map(|values| {
                values
                    .iter()
                    .filter_map(Value::as_str)
                    .map(str::trim)
                    .filter(|path| !path.is_empty())
                    .map(PathBuf::from)
                    .collect()
            })
            .unwrap_or_default(),
        _ => payload
            .trim()
            .split('\n')
            .map(str::trim)
            .filter(|path| !path.is_empty())
            .map(PathBuf::from)
            .collect(),
    };

    paths
}

fn payload_value(payload: &str) -> Value {
    serde_json::from_str::<Value>(payload).unwrap_or(Value::Null)
}

fn collect_candidates(path: &Path, search_subdirectories: bool, out: &mut Vec<PathBuf>) {
    let Ok(metadata) = std::fs::metadata(path) else {
        out.push(path.to_path_buf());
        return;
    };

    if !metadata.is_dir() {
        out.push(path.to_path_buf());
        return;
    }

    let Ok(entries) = std::fs::read_dir(path) else {
        return;
    };

    for entry in entries.flatten() {
        let child = entry.path();
        let Ok(metadata) = entry.metadata() else {
            continue;
        };

        if metadata.is_dir() {
            if search_subdirectories {
                collect_candidates(&child, true, out);
            }
        } else if metadata.is_file() {
            out.push(child);
        }
    }
}

fn has_sidecar_extension(path: &Path) -> bool {
    path.extension()
        .map(|extension| extension.to_string_lossy().to_ascii_lowercase())
        .is_some_and(|extension| SIDECAR_EXTENSIONS.contains(&extension.as_str()))
}

fn comparable_prefix(path: &Path) -> Option<PathBuf> {
    let name = path.file_name()?.to_string_lossy();
    let stem = name.split('.').next().unwrap_or(&name);
    Some(path.parent()?.join(stem))
}

fn comparable_sidecar_prefixes(paths: impl Iterator<Item = PathBuf>) -> BTreeSet<PathBuf> {
    paths
        .filter(|path| !has_sidecar_extension(path))
        .filter_map(|path| comparable_prefix(&path))
        .collect()
}

fn looks_like_sidecar(path: &Path, comparable_sidecar_prefixes: &BTreeSet<PathBuf>) -> bool {
    has_sidecar_extension(path)
        && comparable_prefix(path)
            .is_some_and(|prefix| comparable_sidecar_prefixes.contains(&prefix))
}

#[cfg(test)]
mod tests;
