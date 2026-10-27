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
