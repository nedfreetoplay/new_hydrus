use super::*;
use crate::controller::Controller;

fn test_state() -> (AppState, std::path::PathBuf) {
    let directory = std::env::temp_dir().join(format!(
        "hydrus_gui_pages_{}_{}",
        std::process::id(),
        rand::random::<u64>()
    ));
    let controller = Controller::new(&directory).unwrap();
    controller.init_model().unwrap();

    (AppState::new(controller.into()), directory)
}

fn add_query(notebook: &mut Notebook, id: &str) {
    notebook.add(PageNode::Query(QueryPage::new(id, id, "my files")));
}

#[test]
fn history_walks_back_and_forwards_and_truncates_on_new_page() {
    let mut history = PageHistory::default();

    history.record("p1");
    history.record("p2");
    history.record("p3");
    history.record("p3");

    assert!(history.can_back());
    assert!(!history.can_forward());
    assert_eq!(history.back().as_deref(), Some("p2"));
    assert_eq!(history.back().as_deref(), Some("p1"));
    assert_eq!(history.back(), None);
    assert_eq!(history.forward().as_deref(), Some("p2"));
    assert_eq!(history.forward().as_deref(), Some("p3"));

    history.record("p2");
    assert_eq!(history.entries(), &["p1", "p2", "p3", "p2"]);
    assert!(!history.can_forward());
}

#[test]
fn history_drops_closed_pages() {
    let mut history = PageHistory::default();

    history.record("p1");
    history.record("p2");
    history.record("p3");

    history.retain(|id| id != "p2");

    assert_eq!(history.entries(), &["p1", "p3"]);
    assert_eq!(history.back().as_deref(), Some("p1"));
}

#[test]
fn selecting_a_page_selects_its_whole_notebook_chain() {
    let (mut state, directory) = test_state();

    let mut notebook = Notebook::new("n2", "group");
    add_query(&mut notebook, "p2");
    add_query(&mut notebook, "p3");
    state.root.add(PageNode::Notebook(notebook));

    assert!(state.select_page("p3"));

    assert_eq!(state.active.as_deref(), Some("p3"));
    assert_eq!(state.root.current.as_deref(), Some("n2"));
    assert_eq!(
        state.find_notebook("n2").unwrap().current.as_deref(),
        Some("p3")
    );

    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn undo_restores_a_closed_page_at_its_old_position() {
    let (mut state, directory) = test_state();

    let mut notebook = Notebook::new("n2", "group");
    add_query(&mut notebook, "p2");
    add_query(&mut notebook, "p3");
    state.root.add(PageNode::Notebook(notebook));

    let node = find_node(&state.root, "p3").unwrap().clone();
    state.remove_page("p3");
    state.push_closed_page("n2", 1, node);

    let restored = state.undo_closed_page().unwrap();

    assert_eq!(restored, "p3");
    assert_eq!(
        state
            .find_notebook("n2")
            .unwrap()
            .children
            .iter()
            .map(|child| child.id())
            .collect::<Vec<_>>(),
        vec!["p2", "p3"]
    );
    assert_eq!(state.active.as_deref(), Some("p3"));

    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn unclose_page_at_restores_any_entry_by_index() {
    let (mut state, directory) = test_state();

    let mut notebook = Notebook::new("n2", "group");
    add_query(&mut notebook, "p2");
    add_query(&mut notebook, "p3");
    add_query(&mut notebook, "p4");
    state.root.add(PageNode::Notebook(notebook));

    for id in ["p4", "p3"] {
        let node = find_node(&state.root, id).unwrap().clone();
        state.remove_page(id);
        state.push_closed_page("n2", 1, node);
    }

    // LIFO default restores the newest first.
    assert_eq!(state.unclose_page_at(1).as_deref(), Some("p3"));
    // The older entry is still available by its own index.
    assert_eq!(state.unclose_page_at(0).as_deref(), Some("p4"));
    assert!(state.closed_pages.is_empty());

    let ids = state
        .find_notebook("n2")
        .unwrap()
        .children
        .iter()
        .map(|child| child.id())
        .collect::<Vec<_>>();
    assert!(ids.contains(&"p3"));
    assert!(ids.contains(&"p4"));

    // Out-of-range index restores nothing; clear drops the rest.
    assert!(state.unclose_page_at(0).is_none());
    state.push_closed_page("n2", 0, find_node(&state.root, "p2").unwrap().clone());
    state.remove_page("p2");
    state.clear_closed_pages();
    assert!(state.closed_pages.is_empty());
    assert!(state.undo_closed_page().is_none());

    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn closed_pages_are_unbounded_like_python() {
    let (mut state, directory) = test_state();

    for i in 0..60 {
        let id = format!("extra{i}");
        state.root.add(PageNode::Query(QueryPage::new(
            id.clone(),
            id.clone(),
            "my files",
        )));
        let node = find_node(&state.root, &id).unwrap().clone();
        state.remove_page(&id);
        state.push_closed_page("root", 0, node);
    }

    assert_eq!(state.closed_pages.len(), 60);

    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn moves_pages_between_and_inside_notebooks() {
    let (mut state, directory) = test_state();

    add_query(&mut state.root, "p2");
    let mut notebook = Notebook::new("n2", "group");
    add_query(&mut notebook, "p3");
    state.root.add(PageNode::Notebook(notebook));

    state.move_page("p3", "p1", "before").unwrap();
    assert_eq!(state.root.children[0].id(), "p3");
    assert_eq!(state.root.children[1].id(), "p1");

    state.move_page("p3", "n2", "inside").unwrap();
    assert_eq!(state.root.children[0].id(), "p1");
    assert_eq!(state.find_notebook("n2").unwrap().children[0].id(), "p3");

    state.move_page("p3", "p2", "after").unwrap();
    assert_eq!(state.root.children[1].id(), "p2");
    assert_eq!(state.root.children[2].id(), "p3");

    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn refuses_to_move_a_notebook_into_itself() {
    let (mut state, directory) = test_state();

    let mut notebook = Notebook::new("n2", "group");
    add_query(&mut notebook, "p2");
    state.root.add(PageNode::Notebook(notebook));

    let error = state.move_page("n2", "p2", "inside").unwrap_err();

    assert!(error.contains("itself"));
    assert_eq!(state.find_notebook("n2").unwrap().children[0].id(), "p2");

    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn selection_opens_a_new_fixed_page_like_python() {
    let (mut state, directory) = test_state();

    let mut source = QueryPage::new("p2", "files", "my files");
    source.hashes = vec![vec![1], vec![2], vec![3]];
    source.selected = vec![vec![3], vec![1]];
    state.root.add(PageNode::Query(source));

    let new_id = state.open_selection_in_new_page_model("p2").unwrap();

    // Ordered like the page, not like the selection (Python ordered=True).
    let PageNode::Query(copy) = find_node(&state.root, &new_id).unwrap().clone() else {
        panic!("new page is not a query page");
    };
    assert_eq!(copy.fixed_hashes, Some(vec![vec![1], vec![3]]));
    assert_eq!(copy.hashes, vec![vec![1], vec![3]]);
    assert_eq!(copy.location, "my files");
    // Inserted right after the source and selected.
    let ids: Vec<&str> = state.root.children.iter().map(|child| child.id()).collect();
    let source_pos = ids.iter().position(|id| *id == "p2").unwrap();
    assert_eq!(ids[source_pos + 1], new_id);
    assert_eq!(state.active.as_deref(), Some(new_id.as_str()));

    // Empty selection falls back to the focused file; nothing -> error.
    state.root.add(PageNode::Query(QueryPage::new("p9", "files", "my files")));
    assert!(state.open_selection_in_new_page_model("p9").is_err());

    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn duplicating_a_notebook_gives_every_child_a_fresh_id() {
    let (mut state, directory) = test_state();

    let mut notebook = Notebook::new("n2", "group");
    add_query(&mut notebook, "p2");
    notebook.current = Some("p2".to_string());
    state.root.add(PageNode::Notebook(notebook));

    let copy_id = state.duplicate_page("n2").unwrap();

    assert_ne!(copy_id, "n2");
    assert_eq!(state.root.children.len(), 3);
    assert_eq!(state.root.children[2].id(), copy_id);

    match &state.root.children[2] {
        PageNode::Notebook(copy) => {
            assert_ne!(copy.id, "n2");
            assert_ne!(copy.children[0].id(), "p2");
            assert_eq!(copy.current.as_deref(), Some(copy.children[0].id()));
        }
        _ => panic!("copy is not a notebook"),
    }

    let _ = std::fs::remove_dir_all(directory);
}
