//! Splash and shortcut-editor state from `ClientGUISplash.py` and
//! `ClientGUIShortcutControls.py`.

use crate::gui::shortcuts::{
    Shortcut, SHORTCUT_MODIFIER_ALT, SHORTCUT_MODIFIER_CTRL, SHORTCUT_MODIFIER_SHIFT,
    SHORTCUT_PRESS_TYPE_PRESS, SHORTCUT_TYPE_KEYBOARD_CHARACTER,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SplashModel {
    pub title: String,
    pub status_lines: Vec<String>,
    pub progress: Option<(i64, i64)>,
    pub visible: bool,
}

impl Default for SplashModel {
    fn default() -> Self {
        Self {
            title: "Hydrus Client".to_string(),
            status_lines: Vec::new(),
            progress: None,
            visible: true,
        }
    }
}

impl SplashModel {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            ..Default::default()
        }
    }

    pub fn set_status(&mut self, status: impl Into<String>) {
        self.status_lines.clear();
        self.status_lines.push(status.into());
    }

    pub fn add_status(&mut self, status: impl Into<String>) {
        self.status_lines.push(status.into());
    }

    pub fn set_progress(&mut self, done: i64, total: i64) {
        self.progress = Some((done, total));
    }

    pub fn clear_progress(&mut self) {
        self.progress = None;
    }

    pub fn get_status(&self) -> Option<&str> {
        self.status_lines.last().map(String::as_str)
    }

    pub fn finish(&mut self) {
        self.visible = false;
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShortcutEditor {
    pub command_type: i32,
    pub shortcut_type: i32,
    pub shortcut_key: i32,
    pub modifiers: Vec<i32>,
    pub press_type: i32,
    pub listening: bool,
}

impl Default for ShortcutEditor {
    fn default() -> Self {
        Self {
            command_type: 0,
            shortcut_type: SHORTCUT_TYPE_KEYBOARD_CHARACTER,
            shortcut_key: b'g' as i32,
            modifiers: Vec::new(),
            press_type: SHORTCUT_PRESS_TYPE_PRESS,
            listening: false,
        }
    }
}

impl ShortcutEditor {
    pub fn new(command_type: i32) -> Self {
        Self {
            command_type,
            ..Default::default()
        }
    }

    pub fn begin_listening(&mut self) {
        self.listening = true;
    }

    pub fn stop_listening(&mut self) {
        self.listening = false;
    }

    pub fn set_key(&mut self, key: i32) {
        self.shortcut_key = key;
        self.listening = false;
    }

    pub fn toggle_modifier(&mut self, modifier: i32) {
        if let Some(index) = self.modifiers.iter().position(|value| *value == modifier) {
            self.modifiers.remove(index);
        } else {
            self.modifiers.push(modifier);
            self.modifiers.sort_unstable();
        }
    }

    pub fn clear_modifiers(&mut self) {
        self.modifiers.clear();
    }

    pub fn has_ctrl(&self) -> bool {
        self.modifiers.contains(&SHORTCUT_MODIFIER_CTRL)
    }

    pub fn has_shift(&self) -> bool {
        self.modifiers.contains(&SHORTCUT_MODIFIER_SHIFT)
    }

    pub fn has_alt(&self) -> bool {
        self.modifiers.contains(&SHORTCUT_MODIFIER_ALT)
    }

    pub fn to_shortcut(&self) -> Shortcut {
        Shortcut::new(
            self.shortcut_type,
            self.shortcut_key,
            self.press_type,
            self.modifiers.clone(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splash_tracks_status_and_progress() {
        let mut splash = SplashModel::default();
        splash.set_status("starting");
        splash.set_progress(1, 4);

        assert_eq!(splash.get_status(), Some("starting"));
        assert_eq!(splash.progress, Some((1, 4)));

        splash.finish();
        assert!(!splash.is_visible());
    }

    #[test]
    fn splash_status_can_append_lines() {
        let mut splash = SplashModel::default();
        splash.add_status("first");
        splash.add_status("second");

        assert_eq!(splash.status_lines.len(), 2);
        assert_eq!(splash.get_status(), Some("second"));
    }

    #[test]
    fn shortcut_editor_toggles_modifiers() {
        let mut editor = ShortcutEditor::new(5);

        editor.toggle_modifier(SHORTCUT_MODIFIER_SHIFT);
        editor.toggle_modifier(SHORTCUT_MODIFIER_CTRL);

        assert!(editor.has_ctrl());
        assert!(editor.has_shift());

        editor.toggle_modifier(SHORTCUT_MODIFIER_SHIFT);

        assert!(!editor.has_shift());
        assert_eq!(editor.modifiers, vec![SHORTCUT_MODIFIER_CTRL]);
    }

    #[test]
    fn shortcut_editor_builds_shortcut() {
        let mut editor = ShortcutEditor::new(1);
        editor.toggle_modifier(SHORTCUT_MODIFIER_CTRL);
        editor.set_key(b'Z' as i32);

        let shortcut = editor.to_shortcut();

        assert_eq!(shortcut.shortcut_key, b'z' as i32);
        assert_eq!(shortcut.to_string(), "Ctrl+z");
    }
}
