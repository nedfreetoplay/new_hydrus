use super::*;
use crate::gui::pages::{Page, PageType, PagesNotebook};

fn view_model(session: Option<&GuiSession>) -> MainViewModel {
    MainViewModel::new("Hydrus Client", "ready", session)
}

fn titles(model: &MainViewModel) -> Vec<&str> {
    model
        .panels
        .iter()
        .map(|panel| match panel {
            PanelViewModel::Form(form) | PanelViewModel::ScrolledForm(form) => form.title.as_str(),
            PanelViewModel::List(list) => list.title.as_str(),
            PanelViewModel::Media(media) => media.title.as_str(),
            PanelViewModel::Canvas(canvas) => canvas.title.as_str(),
        })
        .collect()
}

#[test]
fn session_pages_extend_the_sidebar_and_tabs() {
    let mut notebook = PagesNotebook::new();
    notebook.add_page(Page::new("search", PageType::Search));
    notebook.add_page(Page::new("import", PageType::Import));
    let session = GuiSession::from_notebook(&notebook);

    let model = view_model(Some(&session));

    assert_eq!(
        model.pages,
        vec!["search".to_string(), "import".to_string()]
    );
    assert!(model.sidebar.contains(&"search".to_string()));
    assert!(model.sidebar.contains(&"import".to_string()));
    assert_eq!(titles(&model)[0], "Welcome");
}

#[test]
fn without_a_session_the_frame_pages_are_used() {
    let model = view_model(None);

    assert_eq!(model.pages, vec!["Welcome".to_string()]);
    assert!(!model.sidebar.contains(&"Welcome".to_string()));
}

#[test]
fn panels_cover_every_rendered_page() {
    let model = view_model(None);
    let page_titles = titles(&model);

    for expected in [
        "Welcome",
        "Files",
        "Canvas",
        "Options",
        "List",
        "Networking",
        "Headers",
        "Login",
        "Duplicate Filter",
        "Duplicates",
        "Downloads",
        "Subscriptions",
        "Gallery",
        "Watchers",
        "Parsing",
        "Options panels",
        "String editor",
        "String dictionary",
        "Popups",
    ] {
        assert!(page_titles.contains(&expected), "missing {expected}");
    }
}

#[test]
fn list_rows_are_render_ready() {
    let model = view_model(None);
    let duplicates = model
        .panels
        .iter()
        .find_map(|panel| match panel {
            PanelViewModel::List(list) if list.title == "Duplicate Filter" => Some(list),
            _ => None,
        })
        .unwrap();

    assert_eq!(duplicates.columns, "File A,File B,Status");
    assert_eq!(duplicates.rows[0], "abcd01,abcd02,current");
    assert_eq!(duplicates.rows[1], "abcd03,abcd04,pending");
}
