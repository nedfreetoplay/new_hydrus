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
