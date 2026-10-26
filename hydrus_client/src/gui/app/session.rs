//! Persistence of the page tree in `gui session.json`.

use std::path::Path;

use serde::{Deserialize, Serialize};

use super::model::{Notebook, PageNode, PlaceholderPage, QueryPage};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionFile {
    pub version: u32,
    pub root: SessionNode,
    pub active: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionNode {
    pub id: String,
    pub kind: String,
    pub title: String,
    #[serde(default)]
    pub query: Option<String>,
    #[serde(default)]
    pub location: Option<String>,
    #[serde(default)]
    pub message: Option<String>,
    #[serde(default)]
    pub current: Option<String>,
    #[serde(default)]
    pub children: Vec<SessionNode>,
}

pub fn load(path: &Path) -> Option<Notebook> {
    let text = std::fs::read_to_string(path).ok()?;
    let session: SessionFile = serde_json::from_str(&text).ok()?;
    Some(to_notebook(&session.root))
}

pub fn save(state: &super::model::AppState) -> std::io::Result<()> {
    let path = state.controller.db_dir().join("gui session.json");
    let session = SessionFile {
        version: 1,
        root: from_notebook(&state.root),
        active: state.active.clone(),
    };
    let dump = serde_json::to_string_pretty(&session).map_err(std::io::Error::other)?;
    std::fs::write(path, dump)
}

fn from_notebook(notebook: &Notebook) -> SessionNode {
    SessionNode {
        id: notebook.id.clone(),
        kind: "notebook".to_string(),
        title: notebook.title.clone(),
        query: None,
        location: None,
        message: None,
        current: notebook.current.clone(),
        children: notebook.children.iter().map(from_node).collect(),
    }
}

fn from_node(node: &PageNode) -> SessionNode {
    match node {
        PageNode::Query(page) => SessionNode {
            id: page.id.clone(),
            kind: "query".to_string(),
            title: page.title.clone(),
            query: Some(page.query.clone()),
            location: Some(page.location.clone()),
            message: None,
            current: None,
            children: Vec::new(),
        },
        PageNode::Notebook(notebook) => from_notebook(notebook),
        PageNode::Placeholder(page) => SessionNode {
            id: page.id.clone(),
            kind: "placeholder".to_string(),
            title: page.title.clone(),
            query: None,
            location: None,
            message: Some(page.message.clone()),
            current: None,
            children: Vec::new(),
        },
    }
}

fn to_notebook(node: &SessionNode) -> Notebook {
    let mut notebook = Notebook::new(node.id.clone(), node.title.clone());
    notebook.current = node.current.clone();

    for child in &node.children {
        notebook.children.push(to_node(child));
    }

    notebook
}

fn to_node(node: &SessionNode) -> PageNode {
    match node.kind.as_str() {
        "notebook" => PageNode::Notebook(to_notebook(node)),
        "query" => {
            let mut page = QueryPage::new(
                node.id.clone(),
                node.title.clone(),
                node.location
                    .clone()
                    .unwrap_or_else(|| "my files".to_string()),
            );
            page.query = node.query.clone().unwrap_or_default();
            PageNode::Query(page)
        }
        _ => PageNode::Placeholder(PlaceholderPage {
            id: node.id.clone(),
            title: node.title.clone(),
            message: node
                .message
                .clone()
                .unwrap_or_else(|| "This page type is not ported yet.".to_string()),
            status: String::new(),
        }),
    }
}

#[cfg(test)]
mod tests;
