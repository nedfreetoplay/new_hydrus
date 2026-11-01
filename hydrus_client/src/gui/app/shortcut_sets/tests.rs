#[cfg(test)]
mod shortcut_sets_tests {
    use super::super::*;
    use crate::client_options::ClientOptions;

    #[test]
    fn defaults_cover_main_actions() {
        let sets = default_sets();
        assert_eq!(sets.active, "default");
        let set = sets.active_set().unwrap();
        assert!(!set.combos_for("close_page").is_empty());
        assert!(set.combos_for("refresh_page").contains(&"F5".to_string()));
    }

    #[test]
    fn name_validation_rejects_empty_and_dupes() {
        assert!(validate_set_name("", &[]).is_err());
        assert!(validate_set_name("default", &["default".to_string()]).is_err());
        assert!(validate_set_name("mine", &["default".to_string()]).is_ok());
    }

    #[test]
    fn duplicate_name_increments() {
        let existing = vec!["copy (1)".to_string()];
        assert_eq!(duplicate_name(&existing, "copy"), "copy (2)");
    }

    #[test]
    fn conflicts_detect_shared_combos() {
        let set = ShortcutSetData {
            name: "x".to_string(),
            bindings: vec![
                ("close_page".to_string(), vec!["Ctrl+W".to_string()]),
                ("undo".to_string(), vec!["Ctrl+W".to_string()]),
            ],
        };
        let conflicts = find_conflicts(&set);
        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].0, "Ctrl+W");
    }

    #[test]
    fn normalise_combo_orders_modifiers() {
        assert_eq!(normalise_combo("shift+ctrl+a"), "Ctrl+Shift+A");
        assert_eq!(normalise_combo("  f5 "), "F5");
    }

    #[test]
    fn export_import_round_trip() {
        let set = ShortcutSetData {
            name: "mine".to_string(),
            bindings: vec![("undo".to_string(), vec!["Ctrl+Z".to_string()])],
        };
        let value = export_set(&set);
        let back = import_set(&value).unwrap();
        assert_eq!(back, set);
        assert!(import_set(&json!({})).is_err());
    }

    #[test]
    fn options_round_trip() {
        let sets = default_sets();
        let mut options = ClientOptions::new();
        save_to_options(&mut options, &sets);
        let loaded = load_from_options(&options);
        assert_eq!(loaded, sets);
    }

    #[test]
    fn load_falls_back_to_defaults() {
        let options = ClientOptions::new();
        let loaded = load_from_options(&options);
        assert_eq!(loaded.active, "default");
    }

    #[test]
    fn dialog_state_marks_active() {
        let state = dialog_state(&default_sets());
        assert_eq!(state.active_name, "default");
        assert_eq!(state.sets.len(), 1);
        assert!(state.sets[0].active);
        assert!(!state.bindings.is_empty());
    }
}
