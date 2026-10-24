//! Page model from `hydrus/client/gui/pages/ClientGUIPages.py`.
//!
//! The Qt page widgets build on this model. It keeps the navigation and page
//! lifecycle rules separate from rendering.

use crate::client_constants;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageType {
    Search,
    Import,
    Gallery,
    Watcher,
    Subscription,
    Duplicates,
    PageOfPages,
    Other,
}

impl PageType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Search => "search",
            Self::Import => "import",
            Self::Gallery => "gallery",
            Self::Watcher => "watcher",
            Self::Subscription => "subscription",
            Self::Duplicates => "duplicates",
            Self::PageOfPages => "page of pages",
            Self::Other => "other",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Page {
    pub name: String,
    pub page_type: PageType,
    pub page_state: i32,
    pub hashes: Vec<Vec<u8>>,
}

impl Page {
    pub fn new(name: impl Into<String>, page_type: PageType) -> Self {
        Self {
            name: name.into(),
            page_type,
            page_state: client_constants::PAGE_STATE_NORMAL,
            hashes: Vec::new(),
        }
    }

    pub fn set_name(&mut self, name: impl Into<String>) {
        self.name = name.into();
    }

    pub fn set_page_state(&mut self, page_state: i32) {
        self.page_state = page_state;
    }

    pub fn is_searching(&self) -> bool {
        self.page_state == client_constants::PAGE_STATE_SEARCHING
    }

    pub fn set_hashes(&mut self, hashes: Vec<Vec<u8>>) {
        self.hashes = hashes;
    }

    pub fn get_num_files(&self) -> usize {
        self.hashes.len()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PagesNotebook {
    pages: Vec<Page>,
    current_index: Option<usize>,
}

impl PagesNotebook {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_page(&mut self, page: Page) -> usize {
        self.pages.push(page);
        let index = self.pages.len() - 1;
        self.current_index = Some(index);
        index
    }

    pub fn close_page(&mut self, index: usize) -> Option<Page> {
        if index >= self.pages.len() {
            return None;
        }

        let page = self.pages.remove(index);

        match self.current_index {
            Some(current) if current > index => self.current_index = Some(current - 1),
            Some(current) if current == index => {
                self.current_index = if self.pages.is_empty() {
                    None
                } else {
                    Some(index.min(self.pages.len() - 1))
                };
            }
            _ => {}
        }

        Some(page)
    }

    pub fn get_page(&self, index: usize) -> Option<&Page> {
        self.pages.get(index)
    }

    pub fn get_page_mut(&mut self, index: usize) -> Option<&mut Page> {
        self.pages.get_mut(index)
    }

    pub fn get_current_page(&self) -> Option<&Page> {
        self.current_index.and_then(|index| self.pages.get(index))
    }

    pub fn get_current_page_mut(&mut self) -> Option<&mut Page> {
        self.current_index
            .and_then(|index| self.pages.get_mut(index))
    }

    pub fn set_current_index(&mut self, index: usize) -> bool {
        if index < self.pages.len() {
            self.current_index = Some(index);
            true
        } else {
            false
        }
    }

    pub fn get_current_index(&self) -> Option<usize> {
        self.current_index
    }

    pub fn get_pages(&self) -> &[Page] {
        &self.pages
    }

    pub fn len(&self) -> usize {
        self.pages.len()
    }

    pub fn is_empty(&self) -> bool {
        self.pages.is_empty()
    }

    pub fn rename_page(&mut self, index: usize, name: impl Into<String>) -> bool {
        if let Some(page) = self.pages.get_mut(index) {
            page.set_name(name);
            true
        } else {
            false
        }
    }

    pub fn move_page(&mut self, from: usize, to: usize) -> bool {
        if from >= self.pages.len() || to >= self.pages.len() || from == to {
            return false;
        }

        let page = self.pages.remove(from);
        self.pages.insert(to, page);
        self.current_index = Some(to);

        true
    }
}

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

#[cfg(test)]
mod tests {
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
}
