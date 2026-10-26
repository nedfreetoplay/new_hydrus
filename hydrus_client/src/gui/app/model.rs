//! Live page tree and media-viewer context of the client GUI.
//!
//! This mirrors the object graph the Python client keeps around its
//! `PagesNotebook`/`Page` widgets. Rendering lives in `render.rs`; this module
//! only owns state.

use std::sync::Arc;

use crate::controller::Controller;

#[derive(Debug, Clone)]
pub struct QueryPage {
    pub id: String,
    pub title: String,
    pub query: String,
    /// Name of the local file domain used by this page ("my files", "trash", ...).
    pub location: String,
    pub hashes: Vec<Vec<u8>>,
    pub focused: Option<Vec<u8>>,
    pub status: String,
    pub empty_text: String,
}

impl QueryPage {
    pub fn new(
        id: impl Into<String>,
        title: impl Into<String>,
        location: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            query: String::new(),
            location: location.into(),
            hashes: Vec::new(),
            focused: None,
            status: String::new(),
            empty_text: String::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Notebook {
    pub id: String,
    pub title: String,
    pub children: Vec<PageNode>,
    pub current: Option<String>,
}

impl Notebook {
    pub fn new(id: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            children: Vec::new(),
            current: None,
        }
    }

    pub fn add(&mut self, node: PageNode) {
        let id = node.id().to_string();
        self.children.push(node);
        self.current = Some(id);
    }
}

#[derive(Debug, Clone)]
pub struct PlaceholderPage {
    pub id: String,
    pub title: String,
    pub message: String,
    pub status: String,
}

#[derive(Debug, Clone)]
pub enum PageNode {
    Query(QueryPage),
    Notebook(Notebook),
    Placeholder(PlaceholderPage),
}

impl PageNode {
    pub fn id(&self) -> &str {
        match self {
            Self::Query(page) => &page.id,
            Self::Notebook(notebook) => &notebook.id,
            Self::Placeholder(page) => &page.id,
        }
    }

    pub fn title(&self) -> &str {
        match self {
            Self::Query(page) => &page.title,
            Self::Notebook(notebook) => &notebook.title,
            Self::Placeholder(page) => &page.title,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ViewerContext {
    pub page_id: String,
    pub hash: Vec<u8>,
}

pub struct AppState {
    pub controller: Arc<Controller>,
    pub root: Notebook,
    pub active: Option<String>,
    pub message: String,
    pub viewers: std::collections::BTreeMap<String, ViewerContext>,
    next_id: u64,
}

impl AppState {
    pub fn new(controller: Arc<Controller>) -> Self {
        let session_path = controller.db_dir().join("gui session.json");

        let mut state = Self {
            controller,
            root: Notebook::new("root", "top page notebook"),
            active: None,
            message: "ready".to_string(),
            viewers: Default::default(),
            next_id: 1,
        };

        if let Some(root) = crate::gui::app::session::load(&session_path) {
            state.root = root;
            state.reindex();
        } else {
            state.root = default_root();
        }

        state.active = current_leaf(&state.root).or_else(|| first_page_id(&state.root));

        state
    }

    pub fn new_id(&mut self, prefix: &str) -> String {
        let id = format!("{prefix}{}", self.next_id);
        self.next_id += 1;
        id
    }

    /// Makes the internal counter larger than every id in the loaded session.
    fn reindex(&mut self) {
        fn visit(node: &PageNode, max: &mut u64) {
            let digits = node
                .id()
                .trim_start_matches(|character: char| !character.is_ascii_digit());

            if let Ok(number) = digits.parse::<u64>() {
                *max = (*max).max(number);
            }

            if let PageNode::Notebook(notebook) = node {
                for child in &notebook.children {
                    visit(child, max);
                }
            }
        }

        let mut max = 1;

        for child in &self.root.children {
            visit(child, &mut max);
        }

        self.next_id = max + 1;
    }

    pub fn find_query(&self, id: &str) -> Option<&QueryPage> {
        find_node(&self.root, id).and_then(|node| match node {
            PageNode::Query(page) => Some(page),
            _ => None,
        })
    }

    pub fn find_query_mut(&mut self, id: &str) -> Option<&mut QueryPage> {
        find_node_mut(&mut self.root, id).and_then(|node| match node {
            PageNode::Query(page) => Some(page),
            _ => None,
        })
    }

    pub fn find_notebook(&self, id: &str) -> Option<&Notebook> {
        if id == self.root.id {
            return Some(&self.root);
        }

        find_node(&self.root, id).and_then(|node| match node {
            PageNode::Notebook(notebook) => Some(notebook),
            _ => None,
        })
    }

    pub fn find_notebook_mut(&mut self, id: &str) -> Option<&mut Notebook> {
        if id == self.root.id {
            return Some(&mut self.root);
        }

        find_node_mut(&mut self.root, id).and_then(|node| match node {
            PageNode::Notebook(notebook) => Some(notebook),
            _ => None,
        })
    }

    pub fn active_query(&self) -> Option<&QueryPage> {
        self.active.as_deref().and_then(|id| self.find_query(id))
    }

    pub fn active_notebook_id(&self) -> String {
        self.active
            .as_deref()
            .and_then(|id| parent_notebook_id(&self.root, id))
            .unwrap_or_else(|| self.root.id.clone())
    }

    pub fn remove_page(&mut self, page_id: &str) -> bool {
        remove_node(&mut self.root, page_id)
    }

    pub fn close_viewers_for_page(&mut self, page_id: &str) -> Vec<String> {
        let mut closed = Vec::new();

        self.viewers.retain(|viewer_id, context| {
            if context.page_id == page_id {
                closed.push(viewer_id.clone());
                false
            } else {
                true
            }
        });

        closed
    }
}

pub fn default_root() -> Notebook {
    let mut root = Notebook::new("root", "top page notebook");
    let mut page = QueryPage::new("p1", "files", "my files");
    page.query = String::new();
    root.add(PageNode::Query(page));
    root
}

/// Follows the selected child of each nested notebook down to a leaf page.
pub fn current_leaf(notebook: &Notebook) -> Option<String> {
    let id = notebook.current.as_ref()?;

    match find_node(notebook, id) {
        Some(PageNode::Notebook(inner)) => current_leaf(inner),
        Some(node) => Some(node.id().to_string()),
        None => None,
    }
}

pub fn first_page_id(notebook: &Notebook) -> Option<String> {
    for child in &notebook.children {
        match child {
            PageNode::Notebook(inner) => {
                if let Some(id) = first_page_id(inner) {
                    return Some(id);
                }
            }
            _ => return Some(child.id().to_string()),
        }
    }

    None
}

pub fn find_node<'a>(notebook: &'a Notebook, id: &str) -> Option<&'a PageNode> {
    for child in &notebook.children {
        if child.id() == id {
            return Some(child);
        }

        if let PageNode::Notebook(inner) = child {
            if let Some(node) = find_node(inner, id) {
                return Some(node);
            }
        }
    }

    None
}

pub fn find_node_mut<'a>(notebook: &'a mut Notebook, id: &str) -> Option<&'a mut PageNode> {
    for child in &mut notebook.children {
        if child.id() == id {
            return Some(child);
        }

        if let PageNode::Notebook(inner) = child {
            if let Some(node) = find_node_mut(inner, id) {
                return Some(node);
            }
        }
    }

    None
}

pub fn parent_notebook_id(notebook: &Notebook, id: &str) -> Option<String> {
    for child in &notebook.children {
        if child.id() == id {
            return Some(notebook.id.clone());
        }

        if let PageNode::Notebook(inner) = child {
            if let Some(found) = parent_notebook_id(inner, id) {
                return Some(found);
            }
        }
    }

    None
}

fn remove_node(notebook: &mut Notebook, id: &str) -> bool {
    if let Some(position) = notebook.children.iter().position(|child| child.id() == id) {
        notebook.children.remove(position);

        if notebook.current.as_deref() == Some(id) {
            notebook.current = notebook
                .children
                .get(position.min(notebook.children.len().saturating_sub(1)))
                .map(|node| node.id().to_string());
        }

        return true;
    }

    for child in &mut notebook.children {
        if let PageNode::Notebook(inner) = child {
            if remove_node(inner, id) {
                return true;
            }
        }
    }

    false
}
