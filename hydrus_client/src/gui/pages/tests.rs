use super::*;

fn notebook() -> PagesNotebook {
    let mut notebook = PagesNotebook::new();
    notebook.add_page(Page::new("one", PageType::Search));
    notebook.add_page(Page::new("two", PageType::Import));
    notebook.add_page(Page::new("three", PageType::Gallery));
    notebook
}

#[test]
fn adding_selects_the_new_page() {
    let mut notebook = PagesNotebook::new();

    assert_eq!(notebook.add_page(Page::new("one", PageType::Search)), 0);
    assert_eq!(notebook.get_current_index(), Some(0));
    assert_eq!(notebook.get_current_page().unwrap().name, "one");
}

#[test]
fn closing_shifts_current_index() {
    let mut notebook = notebook();
    notebook.set_current_index(2);

    notebook.close_page(0);

    assert_eq!(notebook.get_current_index(), Some(1));
    assert_eq!(notebook.len(), 2);
}

#[test]
fn closing_current_page_selects_a_neighbour() {
    let mut notebook = notebook();
    notebook.set_current_index(1);

    notebook.close_page(1);

    assert_eq!(notebook.get_current_index(), Some(1));
    assert_eq!(notebook.get_current_page().unwrap().name, "three");
}

#[test]
fn closing_the_last_page_clears_selection() {
    let mut notebook = PagesNotebook::new();
    notebook.add_page(Page::new("only", PageType::Search));

    notebook.close_page(0);

    assert_eq!(notebook.get_current_index(), None);
    assert!(notebook.is_empty());
}

#[test]
fn moving_a_page_updates_selection() {
    let mut notebook = notebook();

    assert!(notebook.move_page(0, 2));
    assert_eq!(notebook.get_current_page().unwrap().name, "one");
    assert_eq!(notebook.get_pages()[2].name, "one");
}

#[test]
fn page_state_helpers_match_python() {
    let mut page = Page::new("search", PageType::Search);
    page.set_page_state(client_constants::PAGE_STATE_SEARCHING);

    assert!(page.is_searching());

    page.set_hashes(vec![vec![1], vec![2]]);
    assert_eq!(page.get_num_files(), 2);
}

#[test]
fn page_tree_supports_nesting_and_lookup() {
    let mut root = PageTreeNode::new("pages", None);
    let mut child = PageTreeNode::new("downloads", None);
    child.add_child(PageTreeNode::new("gallery", Some(0)));
    root.add_child(child);

    assert!(root.find("gallery").is_some());
    assert_eq!(root.flatten_names(), vec!["pages", "downloads", "gallery"]);
    assert_eq!(root.count_pages(), 1);
}

#[test]
fn tree_can_be_built_from_notebook() {
    let mut notebook = PagesNotebook::new();
    notebook.add_page(Page::new("one", PageType::Search));
    notebook.add_page(Page::new("two", PageType::Import));

    let tree = PagesTree::from_notebook(&notebook);

    assert_eq!(tree.roots.len(), 2);
    assert_eq!(tree.count_pages(), 2);
}
