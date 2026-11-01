//! Application commands and shortcut sets (`ClientGUIShortcuts.py`).
//!
//! R2: all logic (defaults, validation, conflicts, import/export, options
//! persistence) lives here with unit tests. C++ only renders the dialog and
//! creates `QShortcut`s from the typed rows.

use serde_json::{Map, Value, json};

use crate::client_options::ClientOptions;
use crate::gui::qt::api::ffi::{
    ShortcutBindingRow, ShortcutSetRow, ShortcutSetsDialogState,
};

/// Options group carrying the sets. Fixed schema per set; the group itself is
/// the one arbitrary-shaped container (R1 exception is not needed: values are
/// fixed `{ name, active, bindings }` objects).
pub const SHORTCUTS_GROUP: &str = "shortcuts";
const SETS_KEY: &str = "sets";
const ACTIVE_KEY: &str = "active_set";

/// One named set: action key -> list of `QKeySequence` strings.
#[derive(Debug, Clone, PartialEq)]
pub struct ShortcutSetData {
    pub name: String,
    pub bindings: Vec<(String, Vec<String>)>,
}

impl ShortcutSetData {
    pub fn combos_for(&self, action: &str) -> &[String] {
        self.bindings
            .iter()
            .find(|(key, _)| key == action)
            .map(|(_, combos)| combos.as_slice())
            .unwrap_or(&[])
    }

    pub fn set_combos(&mut self, action: &str, combos: Vec<String>) {
        let mut combos = combos;
        combos.retain(|combo| !combo.trim().is_empty());
        combos.sort();
        combos.dedup();
        if let Some(entry) = self.bindings.iter_mut().find(|(key, _)| key == action) {
            entry.1 = combos;
        } else {
            self.bindings.push((action.to_string(), combos));
        }
        self.bindings.sort_by(|left, right| left.0.cmp(&right.0));
    }
}

/// Whole collection with the active set name.
#[derive(Debug, Clone, PartialEq)]
pub struct ShortcutSets {
    pub sets: Vec<ShortcutSetData>,
    pub active: String,
}

impl ShortcutSets {
    pub fn active_set(&self) -> Option<&ShortcutSetData> {
        self.sets.iter().find(|set| set.name == self.active)
    }

    pub fn set_names(&self) -> Vec<String> {
        self.sets.iter().map(|set| set.name.clone()).collect()
    }

    pub fn get(&self, name: &str) -> Option<&ShortcutSetData> {
        self.sets.iter().find(|set| set.name == name)
    }

    pub fn get_mut(&mut self, name: &str) -> Option<&mut ShortcutSetData> {
        self.sets.iter_mut().find(|set| set.name == name)
    }
}

/// Default main-window bindings, mirroring `build_shortcuts()` and Python's
/// default set for `main_gui` (action key -> `QKeySequence` text).
pub fn default_bindings() -> Vec<(String, Vec<String>)> {
    vec![
        ("new_page".to_string(), vec!["Ctrl+T".to_string()]),
        ("close_page".to_string(), vec!["Ctrl+W".to_string()]),
        ("refresh_page".to_string(), vec!["Ctrl+R".to_string(), "F5".to_string()]),
        ("undo".to_string(), vec!["Ctrl+Z".to_string()]),
        ("history_back".to_string(), vec!["Alt+Left".to_string()]),
        ("history_forward".to_string(), vec!["Alt+Right".to_string()]),
        ("page_manager".to_string(), vec!["Ctrl+Shift+P".to_string()]),
        ("command_palette".to_string(), vec!["Ctrl+K".to_string()]),
        ("duplicate_page".to_string(), vec!["Ctrl+Shift+T".to_string()]),
        ("import_files".to_string(), vec!["Ctrl+I".to_string()]),
        ("import_url".to_string(), vec!["Ctrl+U".to_string()]),
        ("new_import_folders".to_string(), vec!["Ctrl+Shift+I".to_string()]),
        ("options".to_string(), vec!["Ctrl+P".to_string()]),
        ("manage_tags_active".to_string(), vec!["Ctrl+M".to_string()]),
        ("archive_active".to_string(), vec!["F7".to_string()]),
        ("inbox_active".to_string(), vec!["F8".to_string()]),
        ("exit".to_string(), vec!["Ctrl+Q".to_string()]),
    ]
}

pub fn default_sets() -> ShortcutSets {
    let mut set = ShortcutSetData {
        name: "default".to_string(),
        bindings: Vec::new(),
    };
    for (action, combos) in default_bindings() {
        set.set_combos(&action, combos);
    }
    ShortcutSets {
        sets: vec![set],
        active: "default".to_string(),
    }
}

/// Python validation: non-empty name, no duplicate names.
pub fn validate_set_name(name: &str, existing: &[String]) -> Result<(), String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err("You must enter a name!".to_string());
    }
    if existing.iter().any(|other| other == trimmed) {
        return Err(format!("'{trimmed}' already exists!"));
    }
    Ok(())
}

pub fn duplicate_name(existing: &[String], base: &str) -> String {
    let mut candidate = format!("{base} (1)");
    let mut counter = 1;
    while existing.iter().any(|other| other == &candidate) {
        counter += 1;
        candidate = format!("{base} ({counter})");
    }
    candidate
}

/// Find combos claimed by more than one action: `(combo, actions)`.
pub fn find_conflicts(set: &ShortcutSetData) -> Vec<(String, Vec<String>)> {
    let mut owners: std::collections::BTreeMap<String, Vec<String>> = Default::default();
    for (action, combos) in &set.bindings {
        for combo in combos {
            owners.entry(combo.clone()).or_default().push(action.clone());
        }
    }
    let mut conflicts: Vec<(String, Vec<String>)> = owners
        .into_iter()
        .filter(|(_, actions)| actions.len() > 1)
        .collect();
    conflicts.sort_by(|left, right| left.0.cmp(&right.0));
    conflicts
}

/// Normalise a `QKeySequence` string the way Python normalises shortcuts:
/// trim, collapse whitespace, Qt-style modifier names (`Ctrl+Shift+A`).
pub fn normalise_combo(combo: &str) -> String {
    let mut modifiers: Vec<String> = Vec::new();
    let mut keys: Vec<String> = Vec::new();
    for part in combo
        .split('+')
        .map(str::trim)
        .filter(|part| !part.is_empty())
    {
        match part.to_lowercase().as_str() {
            "ctrl" | "control" => modifiers.push("Ctrl".to_string()),
            "alt" | "option" => modifiers.push("Alt".to_string()),
            "shift" => modifiers.push("Shift".to_string()),
            "meta" | "command" => modifiers.push("Meta".to_string()),
            _ => {
                if part.len() == 1 {
                    keys.push(part.to_uppercase());
                } else {
                    let mut chars = part.chars();
                    match chars.next() {
                        Some(first) => {
                            let rest: String = chars.collect();
                            keys.push(format!("{}{}", first.to_uppercase(), rest));
                        }
                        None => keys.push(part.to_string()),
                    }
                }
            }
        }
    }
    modifiers.sort_by_key(|part| match part.as_str() {
        "Ctrl" => 0,
        "Alt" => 1,
        "Shift" => 2,
        "Meta" => 3,
        _ => 4,
    });
    modifiers.extend(keys);
    modifiers.join("+")
}

/// Export one set to the file format (same shape Python writes: name plus
/// action -> combos map).
pub fn export_set(set: &ShortcutSetData) -> Value {
    let mut bindings = Map::new();
    for (action, combos) in &set.bindings {
        bindings.insert(action.clone(), json!(combos));
    }
    json!({ "name": set.name, "bindings": Value::Object(bindings) })
}

/// Import one set from the file format. Returns an error string on bad shape.
pub fn import_set(value: &Value) -> Result<ShortcutSetData, String> {
    let name = value
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim();
    if name.is_empty() {
        return Err("That file has no set name!".to_string());
    }
    let bindings_value = value.get("bindings").and_then(Value::as_object).ok_or_else(|| {
        "That file has no bindings!".to_string()
    })?;
    let mut set = ShortcutSetData {
        name: name.to_string(),
        bindings: Vec::new(),
    };
    for (action, combos) in bindings_value {
        let combos = combos
            .as_array()
            .ok_or_else(|| format!("Bad combos for '{action}'!"))?;
        let combos: Vec<String> = combos
            .iter()
            .filter_map(Value::as_str)
            .map(|combo| normalise_combo(combo))
            .filter(|combo| !combo.is_empty())
            .collect();
        set.set_combos(action, combos);
    }
    Ok(set)
}

pub fn load_from_options(options: &ClientOptions) -> ShortcutSets {
    let mut defaults = default_sets();
    let Some(value) = options.get_group_value(SHORTCUTS_GROUP, SETS_KEY) else {
        return defaults;
    };
    let sets = value.as_array().cloned().unwrap_or_default();
    if sets.is_empty() {
        return defaults;
    }
    let mut loaded = Vec::new();
    for entry in &sets {
        let name = entry
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        if name.trim().is_empty() {
            continue;
        }
        let bindings = entry
            .get("bindings")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        let mut set = ShortcutSetData {
            name,
            bindings: Vec::new(),
        };
        for (action, combos) in &bindings {
            let combos: Vec<String> = combos
                .as_array()
                .cloned()
                .unwrap_or_default()
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect();
            set.set_combos(action, combos);
        }
        if !loaded.iter().any(|other: &ShortcutSetData| other.name == set.name) {
            loaded.push(set);
        }
    }
    if loaded.is_empty() {
        return defaults;
    }
    defaults.sets = loaded;
    let active = options
        .get_group_value(SHORTCUTS_GROUP, ACTIVE_KEY)
        .and_then(|value| value.as_str().map(str::to_string))
        .unwrap_or_default();
    if defaults.sets.iter().any(|set| set.name == active) {
        defaults.active = active;
    } else {
        defaults.active = defaults.sets[0].name.clone();
    }
    defaults
}

pub fn save_to_options(options: &mut ClientOptions, sets: &ShortcutSets) {
    let entries: Vec<Value> = sets
        .sets
        .iter()
        .map(|set| {
            let mut bindings = Map::new();
            for (action, combos) in &set.bindings {
                bindings.insert(action.clone(), json!(combos));
            }
            json!({ "name": set.name, "bindings": Value::Object(bindings) })
        })
        .collect();
    options.set_group_value(SHORTCUTS_GROUP, SETS_KEY, Value::Array(entries));
    options.set_group_value(SHORTCUTS_GROUP, ACTIVE_KEY, json!(sets.active));
}

/// Typed rows for the manage dialog (read-only editor in WS-13.12: sets list
/// plus bindings of the active set; conflicts listed as text).
pub fn dialog_state(sets: &ShortcutSets) -> ShortcutSetsDialogState {
    let set_rows: Vec<ShortcutSetRow> = sets
        .sets
        .iter()
        .map(|set| ShortcutSetRow {
            name: set.name.clone(),
            active: set.name == sets.active,
            count: set.bindings.iter().map(|(_, combos)| combos.len() as i64).sum(),
        })
        .collect();
    let active = sets.active_set();
    let bindings: Vec<ShortcutBindingRow> = active
        .map(|set| {
            set.bindings
                .iter()
                .map(|(action, combos)| ShortcutBindingRow {
                    action: action.clone(),
                    label: action_label(action),
                    combos: combos.clone(),
                    combo_text: combos.join(", "),
                })
                .collect()
        })
        .unwrap_or_default();
    let conflicts: Vec<String> = active
        .map(|set| {
            find_conflicts(set)
                .into_iter()
                .map(|(combo, actions)| format!("{combo}: {}", actions.join(", ")))
                .collect()
        })
        .unwrap_or_default();
    ShortcutSetsDialogState {
        sets: set_rows,
        bindings,
        active_name: sets.active.clone(),
        conflicts,
    }
}

/// Human label for an action key (mirrors the menu label where possible).
pub fn action_label(action: &str) -> String {    match action {
        "new_page" => "new page...".to_string(),
        "close_page" => "close page".to_string(),
        "refresh_page" => "refresh page".to_string(),
        "undo" => "undo".to_string(),
        "history_back" => "back".to_string(),
        "history_forward" => "forward".to_string(),
        "page_manager" => "page manager".to_string(),
        "command_palette" => "command palette".to_string(),
        "duplicate_page" => "duplicate page".to_string(),
        "import_files" => "import files...".to_string(),
        "import_url" => "import url...".to_string(),
        "new_import_folders" => "new import folders page".to_string(),
        "options" => "options...".to_string(),
        "manage_tags_active" => "manage tags".to_string(),
        "archive_active" => "archive".to_string(),
        "inbox_active" => "inbox".to_string(),
        "exit" => "exit".to_string(),
        other => other.replace('_', " "),
    }
}

/// Open the manage-shortcuts dialog (read-only editor in WS-13.12).
pub fn open_dialog(state: &mut crate::gui::app::model::AppState) -> Vec<crate::gui::app::UiCommand> {
    let sets = load_from_options(&state.controller.options());
    vec![crate::gui::app::UiCommand::ShortcutSetsDialog(dialog_state(&sets))]
}

/// Activate a set by name: persist, then re-apply without restart.
pub fn activate(
    state: &mut crate::gui::app::model::AppState,
    name: &str,
) -> Vec<crate::gui::app::UiCommand> {
    let mut sets = load_from_options(&state.controller.options());
    if sets.get(name).is_none() {
        state.message = format!("shortcut set '{name}' was not found");
        return vec![
            crate::gui::app::UiCommand::NotifyMessage(state.message.clone()),
            crate::gui::app::UiCommand::ShortcutSetsDialog(dialog_state(&sets)),
        ];
    }
    sets.active = name.to_string();
    let mut options = state.controller.options();
    save_to_options(&mut options, &sets);
    state.controller.replace_options(options);
    if let Err(error) = state.controller.save_options() {
        state.message = format!("could not save options: {error}");
    } else {
        state.message = format!("active shortcut set: {name}");
    }
    vec![
        crate::gui::app::UiCommand::ApplyShortcuts(dialog_state(&sets)),
        crate::gui::app::UiCommand::ShortcutSetsDialog(dialog_state(&sets)),
        crate::gui::app::UiCommand::NotifyMessage(state.message.clone()),
    ]
}

#[cfg(test)]
mod tests;
