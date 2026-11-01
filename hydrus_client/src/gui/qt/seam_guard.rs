//! Regression guard for the WS-15.2.5 typed-seam rule.
//!
//! The rule (docs/GUI_PORT_PLAN.md, docs/ARCHITECTURE.md): a new screen must
//! not add a `String`/`&str` JSON payload or a `QJsonObject` to the cxx seam.
//! Existing JSON seams are debt tracked in the plan; they are grandfathered
//! here by an explicit allowlist so the count can only go down.
//!
//! These tests read the C++/Rust sources from `CARGO_MANIFEST_DIR`. They do not
//! need Qt, a display, or a database, so they stay cheap and deterministic.

use std::path::{Path, PathBuf};

const MANIFEST_DIR: &str = env!("CARGO_MANIFEST_DIR");

/// Files whose `.cpp`/`.hpp` still contain `parse_object(` (WS-15.2 debt).
/// A file that is not listed must contain zero calls; a listed file must not
/// grow beyond its baseline. Converting a seam only lowers the number, so the
/// allowlist entry can be deleted once it reaches zero.
const PARSE_OBJECT_ALLOWLIST: &[(&str, usize)] = &[
    ("src/gui/qt/bridge_api.cpp", 13),
    ("src/gui/qt/support/strings.cpp", 1),
    ("src/gui/qt/support/strings.hpp", 1),
    ("src/gui/qt/typed_seams.cpp", 1),
];

/// Files whose `.cpp`/`.hpp` still build a `json_payload(...)` reverse payload.
const JSON_PAYLOAD_ALLOWLIST: &[(&str, usize)] = &[
    ("src/gui/qt/support/pairs.cpp", 1),
    ("src/gui/qt/support/strings.cpp", 1),
    ("src/gui/qt/support/strings.hpp", 1),
    ("src/gui/qt/account_types.cpp", 2),
    ("src/gui/qt/api_permissions.cpp", 1),
    ("src/gui/qt/auto_resolution_custom.cpp", 1),
    ("src/gui/qt/auto_resolution_merge.cpp", 1),
    ("src/gui/qt/auto_resolution_review.cpp", 2),
    ("src/gui/qt/auto_resolution_rule.cpp", 2),
    ("src/gui/qt/downloader_page_importers.cpp", 1),
    ("src/gui/qt/downloader_page_state.cpp", 1),
    ("src/gui/qt/duplicates_page_media.cpp", 2),
    ("src/gui/qt/export_folder_editor.cpp", 1),
    ("src/gui/qt/export_folder_sidecar.cpp", 1),
    ("src/gui/qt/export_review.cpp", 1),
    ("src/gui/qt/import_folder_editor.cpp", 1),
    ("src/gui/qt/import_options_dialog.cpp", 2),
    ("src/gui/qt/local_import_review.cpp", 1),
    ("src/gui/qt/local_import_tagging.cpp", 1),
    ("src/gui/qt/network_dialog.cpp", 1),
    ("src/gui/qt/network_tables.cpp", 9),
    ("src/gui/qt/notes_dialog.cpp", 1),
    ("src/gui/qt/options_dialog.cpp", 1),
    ("src/gui/qt/orphan_files.cpp", 2),
    ("src/gui/qt/repair_locations.cpp", 1),
    ("src/gui/qt/seed_cache_dialog.cpp", 1),
    ("src/gui/qt/services_dialog.cpp", 1),
    ("src/gui/qt/services_helpers.cpp", 1),
    ("src/gui/qt/services_server.cpp", 8),
    ("src/gui/qt/subscriptions_dialog.cpp", 2),
    ("src/gui/qt/subscriptions_page.cpp", 2),
    ("src/gui/qt/tag_filter.cpp", 3),
    ("src/gui/qt/tag_relations_dialog.cpp", 2),
];

/// `api.rs` structs that are still a typed envelope around one JSON blob
/// (`json: String`). Dialog contents are fixed `JObject` shapes in Python and
/// are scheduled for conversion in WS-15.2.2/15.2.3; new code must not add
/// another one. Keyed by `Struct.field`.
const JSON_FIELD_ALLOWLIST: &[&str] = &[
    "SubscriptionDialogRow.import_options_json",
    "PageRow.state",
    "ImportOptionsDialogState.json",
    "OptionsDialogState.json",
    "ServicesDialogState.json",
    "NetworkDialogState.json",
    "AccountTypesDialogState.json",
    "ApiPermissionsDialogState.json",
    "TagFilterDialogState.json",
    "TagRelationsDialogState.json",
    "ManageTagsDialogState.json",
    "TimestampsDialogState.json",
    "NotificationsDialogState.json",
    "ExportReviewDialogState.json",
    "RepairFileLocationsDialogState.json",
    "OrphanFilesDialogState.json",
    "DatabaseBackupDialogState.json",
    "LocalImportReviewDialogState.json",
    "LocalImportTaggingDialogState.json",
    "DuplicatesRulesDialogState.json",
    "DuplicatesMergeOptionsDialogState.json",
    "DuplicateCustomActionDialogState.json",
    "DuplicatesRulePreviewState.json",
    "DuplicateReviewDialogState.json",
];

/// cxx functions that still take a `json: &str` argument (WS-15.2 debt).
const JSON_ARG_ALLOWLIST: &[&str] = &[
    "hydrus_set_tag_suggestions",
    "hydrus_show_autocomplete",
    "hydrus_update_import_folders",
    "hydrus_show_import_folders_dialog",
    "hydrus_update_import_folders_dialog",
    "hydrus_update_export_folders",
    "hydrus_show_export_folders_dialog",
    "hydrus_update_export_folders_dialog",
    "hydrus_show_import_folder",
    "hydrus_show_export_folder",
    "hydrus_show_notes",
    "hydrus_show_tag_dialog",
];

fn read(path: &Path) -> String {
    std::fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()))
}

fn relative(path: &Path) -> String {
    path.strip_prefix(MANIFEST_DIR)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn collect(dir: &Path, predicate: &dyn Fn(&Path) -> bool, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect(&path, predicate, out);
        } else if predicate(&path) {
            out.push(path);
        }
    }
}

fn cpp_sources() -> Vec<PathBuf> {
    let root = Path::new(MANIFEST_DIR).join("src/gui/qt");
    let mut files = Vec::new();
    collect(
        &root,
        &|path| {
            matches!(
                path.extension().and_then(|value| value.to_str()),
                Some("cpp" | "hpp")
            )
        },
        &mut files,
    );
    files.sort();
    files
}

fn count(haystack: &str, needle: &str) -> usize {
    haystack.matches(needle).count()
}

fn check_helper(needle: &str, allowlist: &[(&str, usize)]) -> Vec<String> {
    let mut violations = Vec::new();

    for path in cpp_sources() {
        let name = relative(&path);
        let found = count(&read(&path), needle);
        if found == 0 {
            continue;
        }

        match allowlist.iter().find(|(file, _)| *file == name) {
            Some((_, baseline)) if found <= *baseline => {}
            Some((_, baseline)) => violations.push(format!(
                "{name}: {found} `{needle}` (allowlist baseline {baseline})"
            )),
            None => violations.push(format!(
                "{name}: {found} `{needle}` (file is not allowlisted)"
            )),
        }
    }

    violations
}

#[test]
fn no_new_raw_json_seams_in_cpp() {
    let mut violations = Vec::new();

    for path in cpp_sources() {
        let source = read(&path);
        if source.contains("notify(QStringLiteral(") {
            violations.push(format!(
                "{}: `notify(QStringLiteral(` — actions must use `UiAction`",
                relative(&path)
            ));
        }
    }

    violations.extend(check_helper("parse_object(", PARSE_OBJECT_ALLOWLIST));
    violations.extend(check_helper("json_payload(", JSON_PAYLOAD_ALLOWLIST));

    assert!(
        violations.is_empty(),
        "raw JSON seams grew (WS-15.2.5). Convert the seam to a typed struct or \
         add a justified allowlist entry:\n{}",
        violations.join("\n")
    );
}

/// `json` / `*_json` / `state` string fields of `api.rs` structs. A new one
/// means a new JSON blob hidden inside a "typed" struct.
#[test]
fn no_new_json_blob_fields_in_api() {
    let source = read(&Path::new(MANIFEST_DIR).join("src/gui/qt/api.rs"));
    let mut violations = Vec::new();
    let mut current: Option<String> = None;

    for line in source.lines() {
        let trimmed = line.trim();

        if let Some(rest) = trimmed.strip_prefix("struct ") {
            current = rest
                .split(|c: char| !c.is_alphanumeric() && c != '_')
                .next()
                .filter(|name| !name.is_empty())
                .map(str::to_string);
            continue;
        }
        if trimmed == "}" {
            current = None;
            continue;
        }

        let Some(struct_name) = current.as_deref() else {
            continue;
        };
        if trimmed.starts_with("//") || !trimmed.ends_with(',') {
            continue;
        }
        let Some((field, value)) = trimmed.split_once(':') else {
            continue;
        };
        let field = field.trim();
        if value.trim().trim_end_matches(',') != "String" {
            continue;
        }
        if field != "json" && field != "state" && !field.ends_with("_json") {
            continue;
        }

        let key = format!("{struct_name}.{field}");
        if !JSON_FIELD_ALLOWLIST.contains(&key.as_str()) {
            violations.push(key);
        }
    }

    assert!(
        violations.is_empty(),
        "new JSON blob fields in api.rs (WS-15.2.5): {violations:?}. Type the \
         fixed fields individually; only a genuinely arbitrary dict may stay \
         as one explained `*_json` field."
    );
}

/// cxx functions that take a JSON string argument. New seams must pass a
/// struct from `api.rs` instead.
#[test]
fn no_new_json_string_arguments_in_bridge() {
    let source = read(&Path::new(MANIFEST_DIR).join("src/gui/qt/api.rs"));
    let declaration = regex::Regex::new(r"(?s)\bfn\s+(\w+)\s*\((.*?)\)\s*;").unwrap();
    let mut violations = Vec::new();

    for captures in declaration.captures_iter(&source) {
        let name = captures.get(1).unwrap().as_str();
        let arguments = captures.get(2).unwrap().as_str();
        if arguments.contains("json:") && !JSON_ARG_ALLOWLIST.contains(&name) {
            violations.push(name.to_string());
        }
    }

    assert!(
        violations.is_empty(),
        "cxx functions with a raw JSON argument (WS-15.2.5): {violations:?}"
    );
}
