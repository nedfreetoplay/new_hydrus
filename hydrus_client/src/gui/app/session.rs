//! Persistence of the page tree, downloaders and subscriptions in
//! `gui session.json`.

use std::path::Path;

use serde::{Deserialize, Serialize};

use super::model::{
    DownloadItem, DownloaderKind, DownloaderPage, DuplicatesPage, Notebook, PageNode,
    PlaceholderPage, QueryPage, SubscriptionEntry, SubscriptionsPage,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionFile {
    pub version: u32,
    pub root: SessionNode,
    pub active: Option<String>,
    #[serde(default)]
    pub subscriptions: Vec<SubscriptionEntry>,
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
    #[serde(default)]
    pub downloader_kind: Option<String>,
    #[serde(default)]
    pub input: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub search_query: Option<String>,
    #[serde(default)]
    pub filter: Option<String>,
    #[serde(default)]
    pub subject: Option<String>,
    #[serde(default)]
    pub paused: Option<bool>,
    #[serde(default)]
    pub auto_watch: Option<bool>,
    #[serde(default)]
    pub check_period: Option<i64>,
    #[serde(default)]
    pub last_check: Option<i64>,
    #[serde(default)]
    pub queue: Vec<DownloadItem>,
    #[serde(default)]
    pub results: Vec<String>,
    #[serde(default)]
    pub fixed_hashes: Vec<String>,
}

pub struct LoadedSession {
    pub root: Notebook,
    pub subscriptions: Vec<SubscriptionEntry>,
}

pub fn load_session(path: &Path) -> Option<LoadedSession> {
    let text = std::fs::read_to_string(path).ok()?;
    let session: SessionFile = serde_json::from_str(&text).ok()?;

    Some(LoadedSession {
        root: to_notebook(&session.root),
        subscriptions: session.subscriptions,
    })
}

pub fn load(path: &Path) -> Option<Notebook> {
    load_session(path).map(|session| session.root)
}

pub fn save(state: &super::model::AppState) -> std::io::Result<()> {
    let path = state.controller.db_dir().join("gui session.json");
    let session = SessionFile {
        version: 2,
        root: from_notebook(&state.root),
        active: state.active.clone(),
        subscriptions: state.subscriptions.clone(),
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
        downloader_kind: None,
        input: None,
        url: None,
        search_query: None,
        filter: None,
        subject: None,
        paused: None,
        auto_watch: None,
        check_period: None,
        last_check: None,
        queue: Vec::new(),
        results: Vec::new(),
        fixed_hashes: Vec::new(),
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
            fixed_hashes: page
                .fixed_hashes
                .as_ref()
                .map(|hashes| hashes.iter().map(|hash| hex(hash)).collect())
                .unwrap_or_default(),
            queue: Vec::new(),
            results: Vec::new(),
            downloader_kind: None,
            input: None,
            url: None,
            search_query: None,
            filter: None,
            subject: None,
            paused: None,
            auto_watch: None,
            check_period: None,
            last_check: None,
        },
        PageNode::Notebook(notebook) => from_notebook(notebook),
        PageNode::Downloader(page) => SessionNode {
            id: page.id.clone(),
            kind: "downloader".to_string(),
            title: page.title.clone(),
            query: None,
            location: None,
            message: None,
            current: None,
            children: Vec::new(),
            downloader_kind: Some(downloader_kind_name(page.kind).to_string()),
            input: Some(page.input.clone()),
            url: Some(page.url.clone()),
            search_query: Some(page.query.clone()),
            filter: Some(page.filter.clone()),
            subject: Some(page.subject.clone()),
            paused: Some(page.paused),
            auto_watch: Some(page.auto_watch),
            check_period: Some(page.check_period),
            last_check: Some(page.last_check),
            queue: page.queue.clone(),
            results: page.results.iter().map(|hash| hex(hash)).collect(),
            fixed_hashes: Vec::new(),
        },
        PageNode::Subscriptions(page) => SessionNode {
            id: page.id.clone(),
            kind: "subscriptions".to_string(),
            title: page.title.clone(),
            query: None,
            location: None,
            message: None,
            current: None,
            children: Vec::new(),
            downloader_kind: None,
            input: None,
            url: None,
            search_query: None,
            filter: None,
            subject: None,
            paused: None,
            auto_watch: None,
            check_period: None,
            last_check: None,
            queue: Vec::new(),
            results: Vec::new(),
            fixed_hashes: Vec::new(),
        },
        PageNode::Duplicates(page) => SessionNode {
            id: page.id.clone(),
            kind: "duplicates".to_string(),
            title: page.title.clone(),
            query: None,
            location: None,
            message: None,
            current: None,
            children: Vec::new(),
            downloader_kind: None,
            input: None,
            url: None,
            search_query: None,
            filter: None,
            subject: None,
            paused: None,
            auto_watch: None,
            check_period: None,
            last_check: None,
            queue: Vec::new(),
            results: Vec::new(),
            fixed_hashes: Vec::new(),
        },
        PageNode::Placeholder(page) => SessionNode {
            id: page.id.clone(),
            kind: "placeholder".to_string(),
            title: page.title.clone(),
            query: None,
            location: None,
            message: Some(page.message.clone()),
            current: None,
            children: Vec::new(),
            downloader_kind: None,
            input: None,
            url: None,
            search_query: None,
            filter: None,
            subject: None,
            paused: None,
            auto_watch: None,
            check_period: None,
            last_check: None,
            queue: Vec::new(),
            results: Vec::new(),
            fixed_hashes: Vec::new(),
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

            let fixed: Vec<Vec<u8>> = node
                .fixed_hashes
                .iter()
                .filter_map(|hash| unhex(hash))
                .collect();

            if !fixed.is_empty() {
                page.fixed_hashes = Some(fixed);
            }

            PageNode::Query(page)
        }
        "downloader" => {
            let kind = node
                .downloader_kind
                .as_deref()
                .and_then(DownloaderKind::parse)
                .unwrap_or(DownloaderKind::Urls);
            let mut page = DownloaderPage::new(node.id.clone(), kind);
            page.title = node.title.clone();
            page.input = node.input.clone().unwrap_or_default();
            page.url = node.url.clone().unwrap_or_default();
            page.query = node.search_query.clone().unwrap_or_default();
            page.filter = node.filter.clone().unwrap_or_default();
            page.subject = node.subject.clone().unwrap_or_default();
            page.paused = node.paused.unwrap_or(false);
            page.auto_watch = node.auto_watch.unwrap_or(false);
            page.check_period = node.check_period.unwrap_or(3600);
            page.last_check = node.last_check.unwrap_or(0);
            page.queue = node.queue.clone();
            page.results = node.results.iter().filter_map(|hash| unhex(hash)).collect();

            // Downloads are not running after a restart.
            page.running = false;
            for item in &mut page.queue {
                if matches!(item.status, super::model::DownloadItemStatus::Working) {
                    item.status = super::model::DownloadItemStatus::Pending;
                }
            }

            PageNode::Downloader(page)
        }
        "subscriptions" => {
            let mut page = SubscriptionsPage::new(node.id.clone());
            page.title = node.title.clone();
            PageNode::Subscriptions(page)
        }
        "duplicates" => {
            let mut page = DuplicatesPage::new(node.id.clone());
            page.title = node.title.clone();
            PageNode::Duplicates(page)
        }
        "placeholder" => {
            let title = node.title.to_ascii_lowercase();

            match title.as_str() {
                "urls" => {
                    PageNode::Downloader(DownloaderPage::new(node.id.clone(), DownloaderKind::Urls))
                }
                "gallery" => PageNode::Downloader(DownloaderPage::new(
                    node.id.clone(),
                    DownloaderKind::Gallery,
                )),
                "watcher" => PageNode::Downloader(DownloaderPage::new(
                    node.id.clone(),
                    DownloaderKind::Watcher,
                )),
                "simple downloader" => PageNode::Downloader(DownloaderPage::new(
                    node.id.clone(),
                    DownloaderKind::Simple,
                )),
                "subscriptions" => PageNode::Subscriptions(SubscriptionsPage::new(node.id.clone())),
                "duplicates processing" => {
                    PageNode::Duplicates(DuplicatesPage::new(node.id.clone()))
                }
                _ => PageNode::Placeholder(PlaceholderPage {
                    id: node.id.clone(),
                    title: node.title.clone(),
                    message: node
                        .message
                        .clone()
                        .unwrap_or_else(|| "Unknown page type from an older session.".to_string()),
                    status: String::new(),
                }),
            }
        }
        _ => PageNode::Placeholder(PlaceholderPage {
            id: node.id.clone(),
            title: node.title.clone(),
            message: node
                .message
                .clone()
                .unwrap_or_else(|| "Unknown page type from an older session.".to_string()),
            status: String::new(),
        }),
    }
}

fn downloader_kind_name(kind: DownloaderKind) -> &'static str {
    match kind {
        DownloaderKind::Urls => "urls",
        DownloaderKind::Gallery => "gallery",
        DownloaderKind::Watcher => "watcher",
        DownloaderKind::Simple => "simple",
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn unhex(value: &str) -> Option<Vec<u8>> {
    if value.len() % 2 != 0 {
        return None;
    }

    (0..value.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(&value[index..index + 2], 16).ok())
        .collect()
}

#[cfg(test)]
mod tests;
