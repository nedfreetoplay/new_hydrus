//! Split out from the parent module.

use super::*;

pub(super) fn modifier_string(modifier: i32) -> &'static str {
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

pub(super) fn mouse_string(mouse: i32) -> &'static str {
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

pub(super) fn special_key_string(key: i32) -> &'static str {
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
