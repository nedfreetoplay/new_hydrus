//! Split out from the parent module.

use crate::client_constants;

use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageTreeNode {
    pub name: String,
    pub page_index: Option<usize>,
    pub children: Vec<PageTreeNode>,
}

impl PageTreeNode {
    pub fn new(name: impl Into<String>, page_index: Option<usize>) -> Self {
        Self {
            name: name.into(),
            page_index,
            children: Vec::new(),
        }
    }

    pub fn add_child(&mut self, node: PageTreeNode) {
        self.children.push(node);
    }

    pub fn find(&self, name: &str) -> Option<&PageTreeNode> {
        if self.name == name {
            return Some(self);
        }

        self.children.iter().find_map(|child| child.find(name))
    }

    pub fn flatten_names(&self) -> Vec<&str> {
        let mut names = vec![self.name.as_str()];

        for child in &self.children {
            names.extend(child.flatten_names());
        }

        names
    }

    pub fn count_pages(&self) -> usize {
        usize::from(self.page_index.is_some())
            + self
                .children
                .iter()
                .map(PageTreeNode::count_pages)
                .sum::<usize>()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PagesTree {
    pub roots: Vec<PageTreeNode>,
}

impl PagesTree {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_root(&mut self, node: PageTreeNode) {
        self.roots.push(node);
    }

    pub fn find(&self, name: &str) -> Option<&PageTreeNode> {
        self.roots.iter().find_map(|node| node.find(name))
    }

    pub fn flatten_names(&self) -> Vec<&str> {
        self.roots
            .iter()
            .flat_map(PageTreeNode::flatten_names)
            .collect()
    }

    pub fn count_pages(&self) -> usize {
        self.roots.iter().map(PageTreeNode::count_pages).sum()
    }

    pub fn from_notebook(notebook: &PagesNotebook) -> Self {
        let mut tree = Self::new();

        for (index, page) in notebook.get_pages().iter().enumerate() {
            tree.add_root(PageTreeNode::new(page.name.clone(), Some(index)));
        }

        tree
    }
}
