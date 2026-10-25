//! GUI session persistence (`ClientGUISession` behaviour in the controller).

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::client_constants::PageState;
use crate::gui::pages::{Page, PageType, PagesNotebook};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionPage {
    pub name: String,
    pub page_type: String,
    pub page_state: i32,
    pub hashes: Vec<String>,
}

impl SessionPage {
    pub fn from_page(page: &Page) -> Self {
        Self {
            name: page.name.clone(),
            page_type: page.page_type.as_str().to_string(),
            page_state: page.page_state.as_int(),
            hashes: page.hashes.iter().map(|hash| hex_encode(hash)).collect(),
        }
    }

    pub fn to_page(&self) -> Page {
        let page_type = match self.page_type.as_str() {
            "search" => PageType::Search,
            "import" => PageType::Import,
            "gallery" => PageType::Gallery,
            "watcher" => PageType::Watcher,
            "subscription" => PageType::Subscription,
            "duplicates" => PageType::Duplicates,
            "page of pages" => PageType::PageOfPages,
            _ => PageType::Other,
        };

        let mut page = Page::new(self.name.clone(), page_type);
        page.set_page_state(PageState::try_from_int(self.page_state).unwrap_or(PageState::Normal));
        page.set_hashes(
            self.hashes
                .iter()
                .filter_map(|hash| hex_decode(hash))
                .collect(),
        );

        page
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GuiSession {
    pub pages: Vec<SessionPage>,
    pub current_index: Option<usize>,
}

impl GuiSession {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_notebook(notebook: &PagesNotebook) -> Self {
        Self {
            pages: notebook
                .get_pages()
                .iter()
                .map(SessionPage::from_page)
                .collect(),
            current_index: notebook.get_current_index(),
        }
    }

    pub fn to_notebook(&self) -> PagesNotebook {
        let mut notebook = PagesNotebook::new();

        for page in &self.pages {
            notebook.add_page(page.to_page());
        }

        if let Some(index) = self.current_index {
            notebook.set_current_index(index);
        }

        notebook
    }

    pub fn save_to_path(&self, path: impl AsRef<Path>) -> std::io::Result<()> {
        let path = path.as_ref();

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let dump = serde_json::to_string_pretty(self)
            .map_err(|error| std::io::Error::other(error.to_string()))?;

        std::fs::write(path, dump)
    }

    pub fn load_from_path(path: impl AsRef<Path>) -> std::io::Result<Self> {
        let dump = std::fs::read_to_string(path)?;

        serde_json::from_str(&dump).map_err(|error| std::io::Error::other(error.to_string()))
    }
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn hex_decode(value: &str) -> Option<Vec<u8>> {
    if value.len() % 2 != 0 {
        return None;
    }

    (0..value.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(&value[index..index + 2], 16).ok())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn notebook() -> PagesNotebook {
        let mut notebook = PagesNotebook::new();
        let mut page = Page::new("search", PageType::Search);
        page.set_hashes(vec![vec![0xab, 0xcd]]);
        notebook.add_page(page);
        notebook.add_page(Page::new("import", PageType::Import));
        notebook
    }

    #[test]
    fn notebook_round_trips_through_session() {
        let notebook = notebook();
        let session = GuiSession::from_notebook(&notebook);
        let restored = session.to_notebook();

        assert_eq!(restored.len(), 2);
        assert_eq!(restored.get_current_index(), Some(1));
        assert_eq!(restored.get_page(0).unwrap().hashes, vec![vec![0xab, 0xcd]]);
    }

    #[test]
    fn session_file_round_trips() {
        let path = std::env::temp_dir().join(format!(
            "hydrus_client_session_{}_{}.json",
            std::process::id(),
            rand::random::<u64>()
        ));
        let session = GuiSession::from_notebook(&notebook());

        session.save_to_path(&path).unwrap();
        let loaded = GuiSession::load_from_path(&path).unwrap();

        assert_eq!(loaded, session);

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn unknown_page_types_fall_back_to_other() {
        let page = SessionPage {
            name: "x".to_string(),
            page_type: "mystery".to_string(),
            page_state: 0,
            hashes: Vec::new(),
        }
        .to_page();

        assert_eq!(page.page_type, PageType::Other);
    }
}
