//! Application commands and shortcut sets (`ClientGUIShortcuts.py`).
//!
//! R2: all logic (defaults, validation, conflicts, import/export, options
//! persistence) lives here with unit tests. C++ only renders the dialog and
//! creates `QShortcut`s from the typed rows.

use serde_json::{Map, Value, json};

use crate::client_options::ClientOptions;
use crate::gui::qt::api::ffi::{
    MouseBindingRow, ShortcutBindingRow, ShortcutSetRow, ShortcutSetsDialogState,
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

/// Shortcut contexts, mirroring Python `shortcut_names_sorted` (subset we
/// implement; the rest fall back to `main_gui`). Each context inherits from
/// its parent when a lookup misses: `thumbnails`/`media_viewer` -> `media` ->
/// `main_gui` -> `default set main_gui`.
pub const CONTEXT_MAIN: &str = "main_gui";
pub const CONTEXT_MEDIA: &str = "media";
pub const CONTEXT_THUMBNAILS: &str = "thumbnails";
pub const CONTEXT_MEDIA_VIEWER: &str = "media_viewer";
pub const CONTEXT_PREVIEW_MEDIA: &str = "preview_media_window";
pub const CONTEXT_VIEWER_MEDIA: &str = "media_viewer_media_window";

pub fn all_contexts() -> Vec<String> {
    vec![
        CONTEXT_MAIN.to_string(),
        CONTEXT_MEDIA.to_string(),
        CONTEXT_THUMBNAILS.to_string(),
        CONTEXT_MEDIA_VIEWER.to_string(),
        CONTEXT_PREVIEW_MEDIA.to_string(),
        CONTEXT_VIEWER_MEDIA.to_string(),
    ]
}

fn parent_context(context: &str) -> Option<&'static str> {
    match context {
        s if s == CONTEXT_THUMBNAILS => Some(CONTEXT_MEDIA),
        s if s == CONTEXT_MEDIA_VIEWER => Some(CONTEXT_MEDIA),
        s if s == CONTEXT_PREVIEW_MEDIA => Some(CONTEXT_VIEWER_MEDIA),
        s if s == CONTEXT_VIEWER_MEDIA => Some(CONTEXT_MEDIA_VIEWER),
        s if s == CONTEXT_MEDIA => Some(CONTEXT_MAIN),
        _ => None,
    }
}

/// One mouse gesture: button + press type + modifiers.
/// Text form: `[ctrl+alt+shift+meta+]button press-type`, e.g.
/// `ctrl+left press`, `scroll up press`. Buttons: `left`, `right`,
/// `middle`, `back`, `forward`, `task`, `scroll up`, `scroll down`,
/// `scroll left`, `scroll right` (Python `shortcut_mouse_string_lookup` +
/// `shortcut_press_type_str_lookup`: `press`/`release`/`double`/`drag`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MouseGesture {
    pub button: String,
    pub press_type: String,
    pub modifiers: Vec<String>,
}

impl MouseGesture {
    pub fn to_text(&self) -> String {
        let mut parts: Vec<String> = self.modifiers.clone();
        parts.push(self.button.clone());
        format!("{} {}", parts.join("+"), self.press_type)
    }
}

pub fn normalise_mouse_button(button: &str) -> Option<String> {
    match button.trim().to_lowercase().replace('-', " ").replace('_', " ").as_str() {
        "left" | "left click" | "primary" | "primary click" => Some("left".to_string()),
        "right" | "right click" | "secondary" | "secondary click" => Some("right".to_string()),
        "middle" | "middle click" => Some("middle".to_string()),
        "back" => Some("back".to_string()),
        "forward" => Some("forward".to_string()),
        "task" | "task button" => Some("task".to_string()),
        "scroll up" | "wheel up" => Some("scroll up".to_string()),
        "scroll down" | "wheel down" => Some("scroll down".to_string()),
        "scroll left" => Some("scroll left".to_string()),
        "scroll right" => Some("scroll right".to_string()),
        _ => None,
    }
}

pub fn normalise_press_type(press: &str) -> Option<String> {
    match press.trim().to_lowercase().as_str() {
        "press" | "click" | "single" => Some("press".to_string()),
        "release" => Some("release".to_string()),
        "double" | "double click" | "double-click" => Some("double".to_string()),
        "drag" => Some("drag".to_string()),
        _ => None,
    }
}

fn normalise_mouse_modifier(part: &str) -> Option<String> {
    match part.trim().to_lowercase().as_str() {
        "ctrl" | "control" => Some("ctrl".to_string()),
        "alt" | "option" => Some("alt".to_string()),
        "shift" => Some("shift".to_string()),
        "meta" | "command" | "cmd" => Some("meta".to_string()),
        _ => None,
    }
}

/// Parse one gesture text. Returns Python-style error on bad shape.
pub fn parse_mouse_gesture(text: &str) -> Result<MouseGesture, String> {
    let text = text.trim();
    if text.is_empty() {
        return Err("Empty mouse gesture!".to_string());
    }
    let (head, press_raw) = text
        .rsplit_once(' ')
        .ok_or_else(|| format!("Bad mouse gesture '{text}'! Use '<button> <press/release/double/drag>'."))
        .map(|(head, press)| (head.trim(), press.trim()))?;
    let press_type = normalise_press_type(press_raw)
        .ok_or_else(|| format!("Bad press type '{press_raw}'! Use press, release, double or drag."))?;
    // Split head on '+'; the button may itself contain a space (`scroll up`).
    let parts: Vec<&str> = head.split('+').map(str::trim).filter(|part| !part.is_empty()).collect();
    if parts.is_empty() {
        return Err(format!("Bad mouse gesture '{text}'!"));
    }
    let mut modifiers: Vec<String> = Vec::new();
    let mut button_parts: Vec<String> = Vec::new();
    for part in parts {
        if let Some(modifier) = normalise_mouse_modifier(part) {
            if !button_parts.is_empty() {
                return Err(format!("Bad mouse gesture '{text}'! Modifiers come first."));
            }
            if !modifiers.contains(&modifier) {
                modifiers.push(modifier);
            }
        } else {
            button_parts.push(part.to_string());
        }
    }
    let button = normalise_mouse_button(&button_parts.join("+"))
        .ok_or_else(|| format!("Bad mouse button '{}'!", button_parts.join("+")))?;
    modifiers.sort();
    Ok(MouseGesture { button, press_type, modifiers })
}

/// Defaults per context, mirroring Python's default mouse shortcuts for the
/// canvas (viewer/media window) and the thumbnail grid.
pub fn default_mouse_bindings(context: &str) -> Vec<(String, Vec<String>)> {
    match context {
        s if s == CONTEXT_MEDIA_VIEWER || s == CONTEXT_VIEWER_MEDIA || s == CONTEXT_PREVIEW_MEDIA => vec![
            ("zoom_in".to_string(), vec!["scroll up press".to_string()]),
            ("zoom_out".to_string(), vec!["scroll down press".to_string()]),
            ("pan_drag".to_string(), vec!["middle drag".to_string(), "left drag".to_string()]),
            ("toggle_fullscreen".to_string(), vec!["left double".to_string()]),
            ("previous_file".to_string(), vec!["back press".to_string()]),
            ("next_file".to_string(), vec!["forward press".to_string()]),
            ("show_context_menu".to_string(), vec!["right press".to_string()]),
        ],
        s if s == CONTEXT_THUMBNAILS => vec![
            ("select".to_string(), vec!["left press".to_string()]),
            ("range_select".to_string(), vec!["shift+left press".to_string()]),
            ("toggle_select".to_string(), vec!["ctrl+left press".to_string()]),
            ("open_viewer".to_string(), vec!["left double".to_string()]),
            ("open_new_page".to_string(), vec!["middle press".to_string()]),
            ("show_context_menu".to_string(), vec!["right press".to_string()]),
        ],
        s if s == CONTEXT_MEDIA => vec![
            ("open_viewer".to_string(), vec!["left double".to_string()]),
            ("show_context_menu".to_string(), vec!["right press".to_string()]),
        ],
        _ => Vec::new(),
    }
}

/// Mouse label for an action key.
pub fn mouse_action_label(action: &str) -> String {
    match action {
        "zoom_in" => "zoom in".to_string(),
        "zoom_out" => "zoom out".to_string(),
        "pan_drag" => "pan (drag)".to_string(),
        "toggle_fullscreen" => "toggle fullscreen".to_string(),
        "previous_file" => "previous file".to_string(),
        "next_file" => "next file".to_string(),
        "select" => "select".to_string(),
        "range_select" => "range select".to_string(),
        "toggle_select" => "toggle select".to_string(),
        "open_viewer" => "open viewer".to_string(),
        "open_new_page" => "open in new page".to_string(),
        "show_context_menu" => "context menu".to_string(),
        other => other.replace('_', " "),
    }
}

/// Mouse bindings stored per named set: `set name -> context -> action -> gestures`.
/// Persisted inside the same `shortcuts` options group under `mouse_sets`
/// (fixed schema, no R1 exception: `{ set, context, action, gestures }` rows).
pub const MOUSE_SETS_KEY: &str = "mouse_sets";

#[derive(Debug, Clone, PartialEq, Default)]
pub struct MouseSets {
    /// (set name, context, action, gestures)
    pub rows: Vec<(String, String, String, Vec<String>)>,
}

impl MouseSets {
    pub fn gestures_for(&self, set: &str, context: &str, action: &str) -> Vec<String> {
        // Direct hit, else walk up the context chain, else defaults.
        let mut current = Some(context.to_string());
        while let Some(context_name) = current {
            if let Some(row) = self.rows.iter().find(|(set_name, row_context, row_action, _)| {
                set_name == set && row_context == &context_name && row_action == action
            }) {
                return row.3.clone();
            }
            current = parent_context(&context_name).map(str::to_string);
        }
        default_mouse_bindings(context)
            .into_iter()
            .find(|(key, _)| key == action)
            .map(|(_, gestures)| gestures)
            .unwrap_or_default()
    }

    /// Resolve a live gesture to an action: exact context first, then parents.
    pub fn resolve(&self, set: &str, context: &str, gesture_text: &str) -> Option<String> {
        let gesture = parse_mouse_gesture(gesture_text).ok()?.to_text();
        let mut current = Some(context.to_string());
        while let Some(context_name) = current {
            // Stored rows first.
            for (set_name, row_context, action, gestures) in &self.rows {
                if set_name == set && row_context == &context_name && gestures.iter().any(|owned| owned == &gesture) {
                    return Some(action.clone());
                }
            }
            // Then built-in defaults of that exact context.
            for (action, gestures) in default_mouse_bindings(&context_name) {
                if gestures.iter().any(|owned| owned == &gesture) {
                    return Some(action.clone());
                }
            }
            current = parent_context(&context_name).map(str::to_string);
        }
        None
    }

    pub fn conflicts_in(&self, set: &str, context: &str) -> Vec<(String, Vec<String>)> {
        let mut owners: std::collections::BTreeMap<String, Vec<String>> = Default::default();
        let mut actions: std::collections::BTreeSet<String> = Default::default();
        for (row_set, row_context, action, _) in &self.rows {
            if row_set == set && row_context == context {
                actions.insert(action.clone());
            }
        }
        for (action, _) in default_mouse_bindings(context) {
            if !actions.contains(&action) {
                actions.insert(action);
            }
        }
        for action in &actions {
            for gesture in self.gestures_for(set, context, action) {
                owners.entry(gesture).or_default().push(action.clone());
            }
        }
        let mut conflicts: Vec<(String, Vec<String>)> =
            owners.into_iter().filter(|(_, actions)| actions.len() > 1).collect();
        conflicts.sort_by(|left, right| left.0.cmp(&right.0));
        conflicts
    }
}

pub fn load_mouse_from_options(options: &ClientOptions) -> MouseSets {
    let mut sets = MouseSets::default();
    let Some(value) = options.get_group_value(SHORTCUTS_GROUP, MOUSE_SETS_KEY) else {
        return sets;
    };
    for entry in value.as_array().cloned().unwrap_or_default() {
        let get = |key: &str| entry.get(key).and_then(Value::as_str).unwrap_or_default().to_string();
        let set = get("set");
        let context = get("context");
        let action = get("action");
        if set.trim().is_empty() || action.trim().is_empty() {
            continue;
        }
        let gestures: Vec<String> = entry
            .get("gestures")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
            .iter()
            .filter_map(Value::as_str)
            .filter_map(|text| parse_mouse_gesture(text).ok().map(|gesture| gesture.to_text()))
            .collect();
        sets.rows.push((set, context, action, gestures));
    }
    sets
}

pub fn save_mouse_to_options(options: &mut ClientOptions, sets: &MouseSets) {
    let entries: Vec<Value> = sets
        .rows
        .iter()
        .map(|(set, context, action, gestures)| {
            json!({ "set": set, "context": context, "action": action, "gestures": gestures })
        })
        .collect();
    options.set_group_value(SHORTCUTS_GROUP, MOUSE_SETS_KEY, Value::Array(entries));
}

/// Save one mouse binding with Python-style conflict veto.
pub fn save_mouse_binding(
    state: &mut crate::gui::app::model::AppState,
    set_name: &str,
    context: &str,
    action: &str,
    gestures: &[String],
) -> Vec<crate::gui::app::UiCommand> {
    let mut mouse = load_mouse_from_options(&state.controller.options());
    let normalised: Vec<String> = gestures
        .iter()
        .filter_map(|text| parse_mouse_gesture(text).ok().map(|gesture| gesture.to_text()))
        .collect();
    if normalised.len() != gestures.iter().filter(|text| !text.trim().is_empty()).count() {
        state.message = "Bad mouse gesture! Use '<button> <press/release/double/drag>'.".to_string();
        return vec![crate::gui::app::UiCommand::NotifyMessage(state.message.clone())];
    }
    // Conflict veto within the same set+context (stored rows + defaults).
    let mut known: std::collections::BTreeMap<String, String> = Default::default();
    let mut actions: std::collections::BTreeSet<String> = Default::default();
    for (row_set, row_context, row_action, _) in &mouse.rows {
        if row_set == set_name && row_context == context && row_action != action {
            actions.insert(row_action.clone());
        }
    }
    for (default_action, _) in default_mouse_bindings(context) {
        if default_action != action && !actions.contains(&default_action) {
            actions.insert(default_action);
        }
    }
    for other in &actions {
        for gesture in mouse.gestures_for(set_name, context, other) {
            known.insert(gesture, other.clone());
        }
    }
    for gesture in &normalised {
        if let Some(other) = known.get(gesture) {
            let message = format!("The mouse gesture {gesture} is already used by '{other}'. The system only supports one command per gesture in a set for now, please remove one.");
            state.message = message.clone();
            return vec![crate::gui::app::UiCommand::NotifyMessage(message)];
        }
    }
    mouse.rows.retain(|(row_set, row_context, row_action, _)| {
        !(row_set == set_name && row_context == context && row_action == action)
    });
    mouse.rows.push((set_name.to_string(), context.to_string(), action.to_string(), normalised));
    mouse.rows.sort();
    let mut options = state.controller.options();
    save_mouse_to_options(&mut options, &mouse);
    state.controller.replace_options(options);
    let _ = state.controller.save_options();
    state.message = format!("updated mouse '{action}' in '{set_name}' ({context})");
    let sets = load_from_options(&state.controller.options());
    refreshed_context(&sets, &mouse, context, state.message.clone())
}

fn refreshed_context(
    sets: &ShortcutSets,
    mouse: &MouseSets,
    context: &str,
    message: String,
) -> Vec<crate::gui::app::UiCommand> {
    vec![
        crate::gui::app::UiCommand::ApplyShortcuts(dialog_state_context(sets, mouse, context)),
        crate::gui::app::UiCommand::ShortcutSetsDialog(dialog_state_context(sets, mouse, context)),
        crate::gui::app::UiCommand::NotifyMessage(message),
    ]
}

/// Typed rows for the manage dialog (read-only editor in WS-13.12: sets list
/// plus bindings of the active set; conflicts listed as text).
pub fn dialog_state(sets: &ShortcutSets) -> ShortcutSetsDialogState {
    dialog_state_context(sets, &MouseSets::default(), CONTEXT_MAIN)
}

pub fn dialog_state_context(
    sets: &ShortcutSets,
    mouse: &MouseSets,
    context: &str,
) -> ShortcutSetsDialogState {
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
    // Mouse rows: union of stored actions and defaults for this context chain.
    let mut mouse_actions: std::collections::BTreeSet<String> = Default::default();
    for (action, _) in default_mouse_bindings(CONTEXT_MAIN) {
        let _ = action;
    }
    let mut chain = Some(context.to_string());
    let mut chain_contexts: Vec<String> = Vec::new();
    while let Some(context_name) = chain {
        chain_contexts.push(context_name.clone());
        chain = parent_context(&context_name).map(str::to_string);
    }
    chain_contexts.push(CONTEXT_MAIN.to_string());
    for context_name in &chain_contexts {
        for (action, _) in default_mouse_bindings(context_name) {
            mouse_actions.insert(action);
        }
    }
    for (row_set, row_context, action, _) in &mouse.rows {
        if Some(row_set) == active.map(|set| &set.name) && chain_contexts.contains(row_context) {
            mouse_actions.insert(action.clone());
        }
    }
    let active_name = sets.active.clone();
    let mouse_bindings: Vec<MouseBindingRow> = mouse_actions
        .into_iter()
        .map(|action| {
            let gestures = mouse.gestures_for(&active_name, context, &action);
            MouseBindingRow {
                action: action.clone(),
                label: mouse_action_label(&action),
                gesture_text: gestures.join(", "),
                gestures,
                context: context.to_string(),
            }
        })
        .collect();
    let mut mouse_conflicts: Vec<String> = mouse
        .conflicts_in(&active_name, context)
        .into_iter()
        .map(|(gesture, actions)| format!("{gesture}: {}", actions.join(", ")))
        .collect();
    let mut all_conflicts = conflicts;
    all_conflicts.append(&mut mouse_conflicts);
    ShortcutSetsDialogState {
        sets: set_rows,
        bindings,
        active_name: sets.active.clone(),
        conflicts: all_conflicts,
        mouse_bindings,
        contexts: all_contexts(),
        active_context: context.to_string(),
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

pub const DEFAULT_SET_NAME: &str = "default";

/// Python: reserved `main_gui` set cannot be deleted/renamed; custom sets can.
pub fn is_protected(name: &str) -> bool {
    name == DEFAULT_SET_NAME
}

fn persist(state: &mut crate::gui::app::model::AppState, sets: &ShortcutSets) {
    let mut options = state.controller.options();
    save_to_options(&mut options, sets);
    state.controller.replace_options(options);
    let _ = state.controller.save_options();
}

fn refreshed(sets: &ShortcutSets, message: String) -> Vec<crate::gui::app::UiCommand> {
    vec![
        crate::gui::app::UiCommand::ApplyShortcuts(dialog_state(sets)),
        crate::gui::app::UiCommand::ShortcutSetsDialog(dialog_state(sets)),
        crate::gui::app::UiCommand::NotifyMessage(message),
    ]
}

pub fn create_set(
    state: &mut crate::gui::app::model::AppState,
    name: &str,
) -> Vec<crate::gui::app::UiCommand> {
    let mut sets = load_from_options(&state.controller.options());
    let trimmed = name.trim();
    let existing = sets.set_names();
    if let Err(error) = validate_set_name(trimmed, &existing) {
        state.message = error.clone();
        return vec![crate::gui::app::UiCommand::NotifyMessage(error)];
    }
    let mut fresh = ShortcutSetData { name: trimmed.to_string(), bindings: Vec::new() };
    for (action, combos) in default_bindings() {
        let _ = (action, combos);
    }
    // New sets start as a copy of the active set (Python `SetNonDupeName` flow).
    if let Some(active) = sets.active_set().cloned() {
        fresh.bindings = active.bindings;
    }
    sets.sets.push(fresh);
    sets.active = trimmed.to_string();
    persist(state, &sets);
    state.message = format!("created shortcut set '{trimmed}'");
    refreshed(&sets, state.message.clone())
}
pub fn open_dialog(state: &mut crate::gui::app::model::AppState) -> Vec<crate::gui::app::UiCommand> {
    let sets = load_from_options(&state.controller.options());
    vec![crate::gui::app::UiCommand::ShortcutSetsDialog(dialog_state(&sets))]
}

pub fn duplicate_set_full(
    state: &mut crate::gui::app::model::AppState,
    name: &str,
) -> Vec<crate::gui::app::UiCommand> {
    let mut sets = load_from_options(&state.controller.options());
    let Some(source) = sets.get(name).cloned() else {
        state.message = format!("shortcut set '{name}' was not found");
        return vec![crate::gui::app::UiCommand::NotifyMessage(state.message.clone())];
    };
    let existing = sets.set_names();
    let fresh_name = duplicate_name(&existing, &source.name);
    let mut fresh = source;
    fresh.name = fresh_name.clone();
    sets.sets.push(fresh);
    sets.active = fresh_name.clone();
    persist(state, &sets);
    state.message = format!("duplicated shortcut set as '{fresh_name}'");
    refreshed(&sets, state.message.clone())
}

pub fn rename_set(
    state: &mut crate::gui::app::model::AppState,
    old: &str,
    new: &str,
) -> Vec<crate::gui::app::UiCommand> {
    let mut sets = load_from_options(&state.controller.options());
    if is_protected(old) {
        state.message = "The default set cannot be renamed!".to_string();
        return vec![crate::gui::app::UiCommand::NotifyMessage(state.message.clone())];
    }
    let trimmed = new.trim().to_string();
    let mut existing: Vec<String> = sets.set_names();
    existing.retain(|other| other != old);
    if let Err(error) = validate_set_name(&trimmed, &existing) {
        state.message = error.clone();
        return vec![crate::gui::app::UiCommand::NotifyMessage(error)];
    }
    let Some(entry) = sets.get_mut(old) else {
        state.message = format!("shortcut set '{old}' was not found");
        return vec![crate::gui::app::UiCommand::NotifyMessage(state.message.clone())];
    };
    entry.name = trimmed.clone();
    if sets.active == old {
        sets.active = trimmed.clone();
    }
    persist(state, &sets);
    state.message = format!("renamed shortcut set to '{trimmed}'");
    refreshed(&sets, state.message.clone())
}

pub fn delete_set(
    state: &mut crate::gui::app::model::AppState,
    name: &str,
) -> Vec<crate::gui::app::UiCommand> {
    let mut sets = load_from_options(&state.controller.options());
    if is_protected(name) {
        state.message = "The default set cannot be deleted!".to_string();
        return vec![crate::gui::app::UiCommand::NotifyMessage(state.message.clone())];
    }
    let before = sets.sets.len();
    sets.sets.retain(|set| set.name != name);
    if sets.sets.len() == before {
        state.message = format!("shortcut set '{name}' was not found");
        return vec![crate::gui::app::UiCommand::NotifyMessage(state.message.clone())];
    }
    if sets.active == name {
        sets.active = sets.sets.first().map(|set| set.name.clone()).unwrap_or_else(|| DEFAULT_SET_NAME.to_string());
    }
    if sets.sets.is_empty() {
        sets = default_sets();
    }
    persist(state, &sets);
    state.message = format!("deleted shortcut set '{name}'");
    refreshed(&sets, state.message.clone())
}

pub fn save_binding(
    state: &mut crate::gui::app::model::AppState,
    set_name: &str,
    action: &str,
    combos: &[String],
) -> Vec<crate::gui::app::UiCommand> {
    let mut sets = load_from_options(&state.controller.options());
    let Some(entry) = sets.get_mut(set_name) else {
        state.message = format!("shortcut set '{set_name}' was not found");
        return vec![crate::gui::app::UiCommand::NotifyMessage(state.message.clone())];
    };
    let normalised: Vec<String> = combos.iter().map(|combo| normalise_combo(combo)).filter(|combo| !combo.is_empty()).collect();
    for combo in &normalised {
        for (other_action, other_combos) in &entry.bindings.clone() {
            if other_action != action && other_combos.iter().any(|owned| owned == combo) {
                let message = format!("The shortcut {combo} is already used by '{other_action}'. The system only supports one command per shortcut in a set for now, please remove one.");
                state.message = message.clone();
                return vec![crate::gui::app::UiCommand::NotifyMessage(message)];
            }
        }
    }
    entry.set_combos(action, normalised);
    persist(state, &sets);
    state.message = format!("updated '{action}' in '{set_name}'");
    refreshed(&sets, state.message.clone())
}

pub fn import_text(
    state: &mut crate::gui::app::model::AppState,
    text: &str,
) -> Vec<crate::gui::app::UiCommand> {
    let value: Value = match serde_json::from_str(text) {
        Ok(value) => value,
        Err(error) => {
            state.message = format!("Could not parse that file! ({error})");
            return vec![crate::gui::app::UiCommand::NotifyMessage(state.message.clone())];
        }
    };
    let mut imported = match import_set(&value) {
        Ok(set) => set,
        Err(error) => {
            state.message = error.clone();
            return vec![crate::gui::app::UiCommand::NotifyMessage(error)];
        }
    };
    let mut sets = load_from_options(&state.controller.options());
    let existing = sets.set_names();
    if existing.iter().any(|other| other == &imported.name) {
        imported.name = duplicate_name(&existing, &imported.name);
    }
    let name = imported.name.clone();
    sets.sets.push(imported);
    sets.active = name.clone();
    persist(state, &sets);
    state.message = format!("imported shortcut set '{name}'");
    refreshed(&sets, state.message.clone())
}

pub fn export_text(state: &mut crate::gui::app::model::AppState, name: &str) -> String {
    let sets = load_from_options(&state.controller.options());
    sets.get(name).map(|set| export_set(set).to_string()).unwrap_or_default()
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
