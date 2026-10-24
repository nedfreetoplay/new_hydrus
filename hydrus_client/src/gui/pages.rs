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

mod trees;
pub use trees::*;

#[cfg(test)]
mod tests;
