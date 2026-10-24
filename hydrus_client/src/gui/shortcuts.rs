//! Keyboard/mouse shortcuts from `hydrus/client/gui/ClientGUIShortcuts.py`.

use std::collections::BTreeMap;

use serde_json::{Value, json};

use crate::client_application_command::ApplicationCommand;

pub const SHORTCUT_TYPE_KEYBOARD_CHARACTER: i32 = 0;
pub const SHORTCUT_TYPE_MOUSE: i32 = 1;
pub const SHORTCUT_TYPE_KEYBOARD_SPECIAL: i32 = 2;
pub const SHORTCUT_TYPE_NOT_ALLOWED: i32 = 3;

pub const SHORTCUT_PRESS_TYPE_PRESS: i32 = 0;
pub const SHORTCUT_PRESS_TYPE_RELEASE: i32 = 1;
pub const SHORTCUT_PRESS_TYPE_DOUBLE_CLICK: i32 = 2;
pub const SHORTCUT_PRESS_TYPE_DRAG: i32 = 3;

pub const SHORTCUT_MODIFIER_CTRL: i32 = 0;
pub const SHORTCUT_MODIFIER_ALT: i32 = 1;
pub const SHORTCUT_MODIFIER_SHIFT: i32 = 2;
pub const SHORTCUT_MODIFIER_KEYPAD: i32 = 3;
pub const SHORTCUT_MODIFIER_GROUP_SWITCH: i32 = 4;
pub const SHORTCUT_MODIFIER_META: i32 = 5;

pub const SHORTCUT_MOUSE_LEFT: i32 = 0;
pub const SHORTCUT_MOUSE_RIGHT: i32 = 1;
pub const SHORTCUT_MOUSE_MIDDLE: i32 = 2;
pub const SHORTCUT_MOUSE_SCROLL_UP: i32 = 3;
pub const SHORTCUT_MOUSE_SCROLL_DOWN: i32 = 4;
pub const SHORTCUT_MOUSE_SCROLL_LEFT: i32 = 5;
pub const SHORTCUT_MOUSE_SCROLL_RIGHT: i32 = 6;
pub const SHORTCUT_MOUSE_BACK: i32 = 7;
pub const SHORTCUT_MOUSE_FORWARD: i32 = 8;
pub const SHORTCUT_MOUSE_TASK: i32 = 9;

pub const SERIALISABLE_VERSION: u16 = 3;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Shortcut {
    pub shortcut_type: i32,
    pub shortcut_key: i32,
    pub shortcut_press_type: i32,
    pub modifiers: Vec<i32>,
}

impl Default for Shortcut {
    fn default() -> Self {
        Self::new(
            SHORTCUT_TYPE_KEYBOARD_SPECIAL,
            23,
            SHORTCUT_PRESS_TYPE_PRESS,
            Vec::new(),
        )
    }
}

impl Shortcut {
    pub fn new(
        shortcut_type: i32,
        shortcut_key: i32,
        shortcut_press_type: i32,
        mut modifiers: Vec<i32>,
    ) -> Self {
        let shortcut_key = if shortcut_type == SHORTCUT_TYPE_KEYBOARD_CHARACTER
            && (b'A' as i32..=b'Z' as i32).contains(&shortcut_key)
        {
            shortcut_key + 32
        } else {
            shortcut_key
        };

        modifiers.sort_unstable();
        modifiers.dedup();

        Self {
            shortcut_type,
            shortcut_key,
            shortcut_press_type,
            modifiers,
        }
    }

    pub fn keyboard_character(key: char) -> Self {
        Self::new(
            SHORTCUT_TYPE_KEYBOARD_CHARACTER,
            key as i32,
            SHORTCUT_PRESS_TYPE_PRESS,
            Vec::new(),
        )
    }

    pub fn to_string(&self) -> String {
        let mut parts: Vec<String> = self
            .modifiers
            .iter()
            .map(|modifier| modifier_string(*modifier).to_string())
            .collect();

        let key = match self.shortcut_type {
            SHORTCUT_TYPE_KEYBOARD_CHARACTER => char::from_u32(self.shortcut_key as u32)
                .unwrap_or('?')
                .to_string(),
            SHORTCUT_TYPE_MOUSE => mouse_string(self.shortcut_key).to_string(),
            _ => special_key_string(self.shortcut_key).to_string(),
        };

        parts.push(key);

        parts.join("+")
    }

    pub fn get_serialisable_info(&self) -> Value {
        json!([
            self.shortcut_type,
            self.shortcut_key,
            self.shortcut_press_type,
            self.modifiers
        ])
    }

    pub fn from_serialisable_info(info: &Value) -> Option<Self> {
        let values = info.as_array()?;

        Some(Self::new(
            values.first()?.as_i64()? as i32,
            values.get(1)?.as_i64()? as i32,
            values.get(2)?.as_i64()? as i32,
            values
                .get(3)?
                .as_array()?
                .iter()
                .filter_map(Value::as_i64)
                .map(|value| value as i32)
                .collect(),
        ))
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ShortcutSet {
    pub name: String,
    shortcuts_to_commands: BTreeMap<Shortcut, ApplicationCommand>,
}

impl ShortcutSet {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            shortcuts_to_commands: BTreeMap::new(),
        }
    }

    pub fn set_command(&mut self, shortcut: Shortcut, command: ApplicationCommand) {
        self.shortcuts_to_commands.insert(shortcut, command);
    }

    pub fn get_command(&self, shortcut: &Shortcut) -> Option<&ApplicationCommand> {
        self.shortcuts_to_commands.get(shortcut)
    }

    pub fn remove_command(&mut self, shortcut: &Shortcut) {
        self.shortcuts_to_commands.remove(shortcut);
    }

    pub fn shortcuts(&self) -> impl Iterator<Item = &Shortcut> {
        self.shortcuts_to_commands.keys()
    }

    pub fn len(&self) -> usize {
        self.shortcuts_to_commands.len()
    }

    pub fn is_empty(&self) -> bool {
        self.shortcuts_to_commands.is_empty()
    }

    pub fn get_serialisable_info(&self) -> Value {
        let entries: Vec<Value> = self
            .shortcuts_to_commands
            .iter()
            .map(|(shortcut, command)| {
                json!([
                    shortcut.get_serialisable_info(),
                    {
                        "command_type": command.command_type,
                        "data": command.data,
                    }
                ])
            })
            .collect();

        Value::Array(entries)
    }

    pub fn from_serialisable_info(name: impl Into<String>, info: &Value) -> Option<Self> {
        let mut set = Self::new(name);
        let entries = info.as_array()?;

        for entry in entries {
            let pair = entry.as_array()?;
            let shortcut = Shortcut::from_serialisable_info(pair.first()?)?;
            let command_info = pair.get(1)?.as_object()?;
            let command = ApplicationCommand::new(
                command_info.get("command_type")?.as_i64()? as i32,
                command_info
                    .get("data")
                    .cloned()
                    .filter(|value| !value.is_null()),
            );

            set.set_command(shortcut, command);
        }

        Some(set)
    }
}

fn modifier_string(modifier: i32) -> &'static str {
    match modifier {
        SHORTCUT_MODIFIER_CTRL => "Ctrl",
        SHORTCUT_MODIFIER_ALT => "Alt",
        SHORTCUT_MODIFIER_SHIFT => "Shift",
        SHORTCUT_MODIFIER_KEYPAD => "Keypad",
        SHORTCUT_MODIFIER_GROUP_SWITCH => "Group",
        SHORTCUT_MODIFIER_META => "Meta",
        _ => "?",
    }
}

fn mouse_string(mouse: i32) -> &'static str {
    match mouse {
        SHORTCUT_MOUSE_LEFT => "Left Click",
        SHORTCUT_MOUSE_RIGHT => "Right Click",
        SHORTCUT_MOUSE_MIDDLE => "Middle Click",
        SHORTCUT_MOUSE_SCROLL_UP => "Scroll Up",
        SHORTCUT_MOUSE_SCROLL_DOWN => "Scroll Down",
        SHORTCUT_MOUSE_SCROLL_LEFT => "Scroll Left",
        SHORTCUT_MOUSE_SCROLL_RIGHT => "Scroll Right",
        SHORTCUT_MOUSE_BACK => "Back",
        SHORTCUT_MOUSE_FORWARD => "Forward",
        SHORTCUT_MOUSE_TASK => "Task",
        _ => "?",
    }
}

fn special_key_string(key: i32) -> &'static str {
    match key {
        0 => "Space",
        1 => "Backspace",
        2 => "Tab",
        3 => "Return",
        4 => "Enter",
        6 => "Escape",
        8 => "Delete",
        9 => "Up",
        10 => "Down",
        11 => "Left",
        12 => "Right",
        13 => "Home",
        14 => "End",
        15 => "Page Up",
        16 => "Page Down",
        17..=48 => "Function",
        _ => "?",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client_application_command::{SIMPLE_EXIT_APPLICATION, SIMPLE_UNDO};

    #[test]
    fn upper_characters_are_normalised() {
        let shortcut = Shortcut::new(
            SHORTCUT_TYPE_KEYBOARD_CHARACTER,
            b'G' as i32,
            SHORTCUT_PRESS_TYPE_PRESS,
            Vec::new(),
        );

        assert_eq!(shortcut.shortcut_key, b'g' as i32);
    }

    #[test]
    fn modifiers_are_sorted_and_deduped() {
        let shortcut = Shortcut::new(
            SHORTCUT_TYPE_KEYBOARD_CHARACTER,
            b'g' as i32,
            SHORTCUT_PRESS_TYPE_PRESS,
            vec![
                SHORTCUT_MODIFIER_SHIFT,
                SHORTCUT_MODIFIER_CTRL,
                SHORTCUT_MODIFIER_SHIFT,
            ],
        );

        assert_eq!(
            shortcut.modifiers,
            vec![SHORTCUT_MODIFIER_CTRL, SHORTCUT_MODIFIER_SHIFT]
        );
        assert_eq!(shortcut.to_string(), "Ctrl+Shift+g");
    }

    #[test]
    fn special_and_mouse_keys_have_names() {
        let f7 = Shortcut::default();
        let click = Shortcut::new(
            SHORTCUT_TYPE_MOUSE,
            SHORTCUT_MOUSE_MIDDLE,
            SHORTCUT_PRESS_TYPE_PRESS,
            Vec::new(),
        );

        assert_eq!(f7.to_string(), "Function");
        assert_eq!(click.to_string(), "Middle Click");
    }

    #[test]
    fn shortcut_set_round_trips() {
        let mut set = ShortcutSet::new("global");
        set.set_command(
            Shortcut::keyboard_character('g'),
            ApplicationCommand::simple(SIMPLE_EXIT_APPLICATION),
        );
        set.set_command(
            Shortcut::keyboard_character('z'),
            ApplicationCommand::simple(SIMPLE_UNDO),
        );

        let info = set.get_serialisable_info();
        let loaded = ShortcutSet::from_serialisable_info("global", &info).unwrap();

        assert_eq!(loaded.len(), 2);
        assert_eq!(
            loaded
                .get_command(&Shortcut::keyboard_character('z'))
                .unwrap()
                .command_type,
            SIMPLE_UNDO
        );
    }
}
