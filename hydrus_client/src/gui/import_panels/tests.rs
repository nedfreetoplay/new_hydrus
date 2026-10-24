use super::*;

#[test]
fn duplicate_pairs_advance_and_count() {
    let mut panel = DuplicatesPanelState::new();
    panel.load_pairs([(vec![1], vec![2]), (vec![3], vec![4])]);

    assert!(panel.decide(&DuplicatePairDecision::Approve));
    assert_eq!(panel.progress(), (1, 2));
    assert!(panel.decide(&DuplicatePairDecision::Skip));
    assert_eq!(panel.progress(), (2, 2));
    assert!(panel.is_finished());
}

#[test]
fn duplicate_decisions_require_a_pair() {
    let mut panel = DuplicatesPanelState::new();

    assert!(!panel.decide(&DuplicatePairDecision::Approve));
}

#[test]
fn import_counts_are_per_tab() {
    let mut panel = ImportPanelState::new();
    panel.set_count(ImportTab::Gallery, 2);
    panel.set_count(ImportTab::Watcher, 3);

    assert_eq!(panel.get_count(ImportTab::Gallery), 2);
    assert_eq!(panel.total_importers(), 5);

    panel.select_tab(ImportTab::Watcher);
    panel.toggle_pause();

    assert_eq!(panel.selected_tab, Some(ImportTab::Watcher));
    assert!(panel.paused);
}

#[test]
fn parsing_panel_tracks_formula_and_result() {
    let mut panel = ParsingPanelState::new();
    panel.set_formulas(["thread".to_string(), "post".to_string()]);

    assert_eq!(panel.get_selected_formula(), Some("thread"));
    assert!(panel.select_formula(1));
    assert!(!panel.select_formula(2));

    panel.set_test_text("<html></html>");
    panel.set_last_result("no results");

    assert_eq!(panel.test_text, "<html></html>");
    assert_eq!(panel.last_result.as_deref(), Some("no results"));
}
