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
        let state = dialog_state(&default_sets(), &KeySets::default());
        assert_eq!(state.active_name, "default");
        assert_eq!(state.sets.len(), 1);
        assert!(state.sets[0].active);
        assert!(!state.bindings.is_empty());
    }

    #[test]
    fn protected_set_cannot_be_renamed_or_deleted() {
        assert!(is_protected("default"));
        assert!(!is_protected("mine"));
        assert!(validate_set_name("  ", &[]).is_err());
    }

    #[test]
    fn duplicate_set_names_do_not_collide() {
        let mut sets = default_sets();
        let existing = sets.set_names();
        let fresh = duplicate_name(&existing, "default");
        assert_eq!(fresh, "default (1)");
        sets.sets.push(ShortcutSetData {
            name: fresh,
            bindings: Vec::new(),
        });
        assert_eq!(duplicate_name(&sets.set_names(), "default"), "default (2)");
    }

    #[test]
    fn set_combos_trims_empties_and_sorts() {
        let mut set = ShortcutSetData {
            name: "x".to_string(),
            bindings: Vec::new(),
        };
        set.set_combos("undo", vec!["Ctrl+Z".to_string(), "  ".to_string(), "Ctrl+Z".to_string()]);
        assert_eq!(set.combos_for("undo"), &["Ctrl+Z".to_string()]);
        set.set_combos("undo", Vec::new());
        assert!(set.combos_for("undo").is_empty());
    }

    #[test]
    fn import_rejects_bad_shapes() {
        assert!(import_set(&json!({"name": "x"})).is_err());
        assert!(import_set(&json!({"name": "", "bindings": {}})).is_err());
        assert!(import_set(&json!({"name": "x", "bindings": {"undo": "nope"}})).is_err());
    }

    #[test]
    fn mouse_gesture_round_trip() {
        let gesture = parse_mouse_gesture("ctrl+left press").unwrap();
        assert_eq!(gesture.button, "left");
        assert_eq!(gesture.press_type, "press");
        assert_eq!(gesture.to_text(), "ctrl+left press");
        assert_eq!(parse_mouse_gesture("Scroll Up Press").unwrap().button, "scroll up");
        assert_eq!(parse_mouse_gesture("left double").unwrap().press_type, "double");
        assert!(parse_mouse_gesture("left").is_err());
        assert!(parse_mouse_gesture("left squeeze").is_err());
        assert!(parse_mouse_gesture("banana press").is_err());
    }

    #[test]
    fn mouse_defaults_resolve_and_inherit() {
        let mouse = MouseSets::default();
        assert_eq!(mouse.resolve("default", "thumbnails", "left press").as_deref(), Some("select"));
        assert_eq!(mouse.resolve("default", "thumbnails", "left double").as_deref(), Some("open_viewer"));
        // `media` parent supplies open_viewer to thumbnails too; main_gui has no mouse rows.
        assert_eq!(mouse.resolve("default", "media", "right press").as_deref(), Some("show_context_menu"));
        assert!(mouse.resolve("default", "main_gui", "left press").is_none());
        assert_eq!(
            mouse.resolve("default", "media_viewer_media_window", "ctrl+scroll up press").as_deref(),
            Some("zoom_in")
        );
        // Plain wheel flips files (Python browser set); release opens the menu.
        assert_eq!(
            mouse.resolve("default", "media_viewer_browser", "scroll up press").as_deref(),
            Some("previous_file")
        );
        assert_eq!(
            mouse.resolve("default", "media_viewer_browser", "scroll down press").as_deref(),
            Some("next_file")
        );
        assert_eq!(
            mouse.resolve("default", "media_viewer_browser", "right release").as_deref(),
            Some("show_context_menu")
        );
        assert_eq!(
            mouse.resolve("default", "media_viewer_browser", "left double").as_deref(),
            Some("close_viewer")
        );
        // The media window inherits the browser bindings through the chain.
        assert_eq!(
            mouse.resolve("default", "media_viewer_media_window", "scroll down press").as_deref(),
            Some("next_file")
        );
        // Drag has no Python binding anywhere.
        assert!(mouse.resolve("default", "media_viewer_browser", "left drag").is_none());
        assert!(mouse.resolve("default", "thumbnails", "middle drag").is_none());
    }

    #[test]
    fn mouse_contexts_list_and_dialog_rows() {
        assert!(all_contexts().contains(&"thumbnails".to_string()));
        let state = dialog_state_context(&default_sets(), &KeySets::default(), &MouseSets::default(), "thumbnails");
        assert_eq!(state.active_context, "thumbnails");
        assert!(!state.mouse_bindings.is_empty());
        assert!(state.mouse_bindings.iter().any(|row| row.action == "open_viewer"
            && row.gesture_text.contains("left double")));
        assert!(state.contexts.contains(&"media_viewer".to_string()));
        assert!(state.contexts.contains(&"media_viewer_browser".to_string()));
    }

    #[test]
    fn key_defaults_match_python_browser_and_thumbnails() {
        let keys = KeySets::default();
        assert_eq!(keys.resolve("default", "media_viewer_browser", "Up").as_deref(), Some("previous_file"));
        assert_eq!(keys.resolve("default", "media_viewer_browser", "Left").as_deref(), Some("previous_file"));
        assert_eq!(keys.resolve("default", "media_viewer_browser", "PageUp").as_deref(), Some("previous_file"));
        assert_eq!(keys.resolve("default", "media_viewer_browser", "Down").as_deref(), Some("next_file"));
        assert_eq!(keys.resolve("default", "media_viewer_browser", "Right").as_deref(), Some("next_file"));
        assert_eq!(keys.resolve("default", "media_viewer_browser", "PageDown").as_deref(), Some("next_file"));
        assert_eq!(keys.resolve("default", "media_viewer_browser", "Home").as_deref(), Some("first_file"));
        assert_eq!(keys.resolve("default", "media_viewer_browser", "End").as_deref(), Some("last_file"));
        // No plain-key zoom in the browser set (Python zooms on F/+/-, not arrows).
        assert!(keys.resolve("default", "media_viewer_browser", "F").is_none());
        // Thumbnails: Enter opens the viewer.
        assert_eq!(keys.resolve("default", "thumbnails", "Enter").as_deref(), Some("open_viewer"));
        assert_eq!(keys.resolve("default", "thumbnails", "Return").as_deref(), Some("open_viewer"));
        // main_gui set has no viewer actions; media context has no keyboard rows.
        assert!(keys.resolve("default", "main_gui", "Left").is_none());
        assert!(keys.resolve("default", "media", "Left").is_none());
    }

    #[test]
    fn key_context_inherits_up_the_chain() {
        let keys = KeySets::default();
        // Media window inherits browser nav keys through media_viewer.
        assert_eq!(
            keys.resolve("default", "media_viewer_media_window", "Right").as_deref(),
            Some("next_file")
        );
        // Stored override wins over the default in the same context.
        let mut stored = KeySets::default();
        stored.rows.push((
            "default".to_string(),
            "media_viewer_browser".to_string(),
            "next_file".to_string(),
            vec!["N".to_string()],
        ));
        assert_eq!(stored.resolve("default", "media_viewer_browser", "N").as_deref(), Some("next_file"));
        // ...but the default still applies in sibling contexts.
        assert_eq!(stored.resolve("default", "media_viewer_browser", "Right").as_deref(), Some("next_file"));
    }

    #[test]
    fn key_sets_persist_rows() {
        let mut options = ClientOptions::new();
        let mut keys = KeySets::default();
        keys.rows.push((
            "default".to_string(),
            "thumbnails".to_string(),
            "open_viewer".to_string(),
            vec!["Space".to_string()],
        ));
        save_keys_to_options(&mut options, &keys);
        let loaded = load_keys_from_options(&options);
        assert_eq!(loaded, keys);
        assert_eq!(loaded.combos_for("default", "thumbnails", "open_viewer"), vec!["Space".to_string()]);
    }

    #[test]
    fn keyboard_context_rows_show_inheritance() {
        // Browser context shows defaults marked inherited.
        let state = dialog_state_context(
            &default_sets(),
            &KeySets::default(),
            &MouseSets::default(),
            "media_viewer_browser",
        );
        let next = state
            .context_bindings
            .iter()
            .find(|row| row.action == "next_file")
            .expect("browser next_file row");
        assert!(next.inherited);
        assert!(next.combo_text.contains("Right"));
        assert!(next.label.contains("inherited from media_viewer_browser"));
        // Main context rows stay the flat set bindings, never inherited.
        let main = dialog_state(&default_sets(), &KeySets::default());
        assert!(main.bindings.iter().all(|row| !row.inherited));
        assert!(main.context_bindings.is_empty());
        // A stored override clears the inherited flag for that action only.
        let mut keys = KeySets::default();
        keys.rows.push((
            "default".to_string(),
            "media_viewer_browser".to_string(),
            "next_file".to_string(),
            vec!["N".to_string()],
        ));
        let overridden = dialog_state_context(
            &default_sets(),
            &keys,
            &MouseSets::default(),
            "media_viewer_browser",
        );
        let next = overridden
            .context_bindings
            .iter()
            .find(|row| row.action == "next_file")
            .expect("overridden next_file row");
        assert!(!next.inherited);
        assert_eq!(next.combo_text, "N");
        let prev = overridden
            .context_bindings
            .iter()
            .find(|row| row.action == "previous_file")
            .expect("previous_file row");
        assert!(prev.inherited);
    }
}
