//! Live page tree and media-viewer context of the client GUI.
//!
//! This mirrors the object graph the Python client keeps around its
//! `PagesNotebook`/`Page` widgets. Rendering lives in `render.rs`; this module
//! only owns state.

use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::controller::Controller;
use crate::gui::app::worker::AsyncRuntime;

#[derive(Debug, Clone)]
pub struct QueryPage {
    pub id: String,
    pub title: String,
    pub query: String,
    /// Name of the local file domain used by this page ("my files", "trash", ...).
    pub location: String,
    /// Local tag service name; empty means the first local tag service.
    pub tag_service: String,
    /// Sort key used by the results panel, e.g. `hash id`, `size`, `mime`.
    pub sort: String,
    pub sort_ascending: bool,
    pub hashes: Vec<Vec<u8>>,
    pub selected: Vec<Vec<u8>>,
    pub focused: Option<Vec<u8>>,
    pub status: String,
    pub empty_text: String,
    /// Set for file pages opened from a downloader result set.
    pub fixed_hashes: Option<Vec<Vec<u8>>>,
    pub query_undo: Vec<String>,
    pub query_redo: Vec<String>,
    /// True between the search action and the deferred `run_query` that fills
    /// the page, so the results panel can show its loading state.
    pub loading: bool,
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
            tag_service: String::new(),
            sort: "hash id".to_string(),
            sort_ascending: true,
            hashes: Vec::new(),
            selected: Vec::new(),
            focused: None,
            status: String::new(),
            empty_text: String::new(),
            fixed_hashes: None,
            query_undo: Vec::new(),
            query_redo: Vec::new(),
            loading: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DownloaderKind {
    Urls,
    Gallery,
    Watcher,
    Simple,
}

impl DownloaderKind {
    pub fn title(self) -> &'static str {
        match self {
            Self::Urls => "urls",
            Self::Gallery => "gallery",
            Self::Watcher => "watcher",
            Self::Simple => "simple downloader",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "urls" => Some(Self::Urls),
            "gallery" => Some(Self::Gallery),
            "watcher" => Some(Self::Watcher),
            "simple" | "simple downloader" => Some(Self::Simple),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DownloadItemStatus {
    Pending,
    Working,
    Done,
    Error,
}

impl DownloadItemStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Working => "downloading",
            Self::Done => "done",
            Self::Error => "error",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadItem {
    pub url: String,
    pub status: DownloadItemStatus,
    #[serde(default)]
    pub note: String,
}

impl DownloadItem {
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            status: DownloadItemStatus::Pending,
            note: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloaderPage {
    pub id: String,
    pub title: String,
    pub kind: DownloaderKind,
    /// One URL per line for the urls and simple downloader pages.
    pub input: String,
    /// The page or gallery URL used by gallery and watcher pages.
    pub url: String,
    /// Gallery search terms, substituted into `{query}` in the URL.
    pub query: String,
    /// Built-in site parser id, or empty/auto to detect it from the URL.
    #[serde(default)]
    pub site: String,
    /// Optional case-insensitive link filter for gallery/watcher parsing.
    pub filter: String,
    pub subject: String,
    pub paused: bool,
    pub running: bool,
    pub auto_watch: bool,
    pub last_check: i64,
    pub check_period: i64,
    pub queue: Vec<DownloadItem>,
    pub imported: usize,
    pub failed: usize,
    pub status: String,
    pub log: Vec<String>,
    pub results: Vec<Vec<u8>>,
}

impl DownloaderPage {
    pub fn new(id: impl Into<String>, kind: DownloaderKind) -> Self {
        let title = kind.title().to_string();

        Self {
            id: id.into(),
            title,
            kind,
            input: String::new(),
            url: String::new(),
            query: String::new(),
            site: String::new(),
            filter: String::new(),
            subject: String::new(),
            paused: false,
            running: false,
            auto_watch: false,
            last_check: 0,
            check_period: 3600,
            queue: Vec::new(),
            imported: 0,
            failed: 0,
            status: "ready".to_string(),
            log: Vec::new(),
            results: Vec::new(),
        }
    }

    pub fn note(&mut self, message: impl Into<String>) {
        let message = message.into();
        self.status = message.clone();
        self.log.push(message);

        if self.log.len() > 500 {
            self.log.drain(0..self.log.len() - 500);
        }
    }

    pub fn pending_or_working(&self) -> usize {
        self.queue
            .iter()
            .filter(|item| {
                matches!(
                    item.status,
                    DownloadItemStatus::Pending | DownloadItemStatus::Working
                )
            })
            .count()
    }

    pub fn clear_finished(&mut self) {
        self.queue.retain(|item| {
            !matches!(
                item.status,
                DownloadItemStatus::Done | DownloadItemStatus::Error
            )
        });
    }

    pub fn item_mut(&mut self, url: &str) -> Option<&mut DownloadItem> {
        self.queue.iter_mut().find(|item| item.url == url)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionEntry {
    pub name: String,
    pub url: String,
    pub query: String,
    #[serde(default)]
    pub site: String,
    pub filter: String,
    #[serde(default = "default_subscription_period")]
    pub period: i64,
    pub paused: bool,
    #[serde(default)]
    pub running: bool,
    #[serde(default)]
    pub last_check: i64,
    #[serde(default)]
    pub imported: usize,
    #[serde(default)]
    pub failed: usize,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub results: Vec<Vec<u8>>,
}

fn default_subscription_period() -> i64 {
    86_400
}

impl SubscriptionEntry {
    pub fn new(name: impl Into<String>, url: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            url: url.into(),
            query: String::new(),
            site: String::new(),
            filter: String::new(),
            period: default_subscription_period(),
            paused: false,
            running: false,
            last_check: 0,
            imported: 0,
            failed: 0,
            status: "ready".to_string(),
            results: Vec::new(),
        }
    }

    pub fn is_due(&self, now: i64) -> bool {
        !self.paused && !self.running && now >= self.last_check + self.period.max(60)
    }
}

#[derive(Debug, Clone)]
pub struct SubscriptionsPage {
    pub id: String,
    pub title: String,
    pub status: String,
}

impl SubscriptionsPage {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            title: "subscriptions".to_string(),
            status: String::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DupeSearchType {
    OneFile,
    BothFiles,
    DifferentSearches,
}

impl DupeSearchType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::OneFile => "one_file",
            Self::BothFiles => "both_files",
            Self::DifferentSearches => "different_searches",
        }
    }

    pub fn from_str(value: &str) -> Self {
        match value {
            "both_files" => Self::BothFiles,
            "different_searches" => Self::DifferentSearches,
            _ => Self::OneFile,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PixelDupesPreference {
    Required,
    Allowed,
    Excluded,
}

impl PixelDupesPreference {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Required => "required",
            Self::Allowed => "allowed",
            Self::Excluded => "excluded",
        }
    }

    pub fn from_str(value: &str) -> Self {
        match value {
            "required" => Self::Required,
            "excluded" => Self::Excluded,
            _ => Self::Allowed,
        }
    }
}

fn default_duplicate_location() -> String {
    "all local files".to_string()
}

/// The pair search scope of a duplicate filter page, mirroring
/// `ClientPotentialDuplicatesSearchContext.PotentialDuplicatesSearchContext`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DuplicateSearchContext {
    #[serde(default)]
    pub query_1: String,
    #[serde(default = "default_duplicate_location")]
    pub location_1: String,
    #[serde(default)]
    pub query_2: String,
    #[serde(default = "default_duplicate_location")]
    pub location_2: String,
    #[serde(default = "default_dupe_search_type")]
    pub dupe_search_type: String,
    #[serde(default = "default_pixel_dupes_preference")]
    pub pixel_dupes: String,
    #[serde(default = "default_duplicate_distance")]
    pub max_distance: i64,
}

fn default_dupe_search_type() -> String {
    DupeSearchType::OneFile.as_str().to_string()
}

fn default_pixel_dupes_preference() -> String {
    PixelDupesPreference::Allowed.as_str().to_string()
}

fn default_duplicate_distance() -> i64 {
    4
}

impl Default for DuplicateSearchContext {
    fn default() -> Self {
        Self {
            query_1: String::new(),
            location_1: default_duplicate_location(),
            query_2: String::new(),
            location_2: default_duplicate_location(),
            dupe_search_type: default_dupe_search_type(),
            pixel_dupes: default_pixel_dupes_preference(),
            max_distance: default_duplicate_distance(),
        }
    }
}

impl DuplicateSearchContext {
    /// Keeps a context loaded from a session or dialog inside the bounds the
    /// Python controls enforce.
    pub fn normalise(&mut self) {
        self.max_distance = self.max_distance.clamp(0, 64);

        let search_type = DupeSearchType::from_str(&self.dupe_search_type);
        self.dupe_search_type = search_type.as_str().to_string();

        let pixel_dupes = PixelDupesPreference::from_str(&self.pixel_dupes);
        self.pixel_dupes = pixel_dupes.as_str().to_string();

        if self.location_1.is_empty() {
            self.location_1 = default_duplicate_location();
        }

        if search_type != DupeSearchType::DifferentSearches || self.location_2.is_empty() {
            self.location_2 = self.location_1.clone();
        }
    }

    pub fn search_type(&self) -> DupeSearchType {
        DupeSearchType::from_str(&self.dupe_search_type)
    }

    pub fn pixel_preference(&self) -> PixelDupesPreference {
        PixelDupesPreference::from_str(&self.pixel_dupes)
    }

    fn context_summary(location: &str, query: &str) -> String {
        let query = query.trim();

        if query.is_empty() {
            location.to_string()
        } else {
            format!("{location}: {query}")
        }
    }

    /// Mirrors `PotentialDuplicatesSearchContext.GetSummary`.
    pub fn summary(&self) -> String {
        let mut components = Vec::new();

        match self.pixel_preference() {
            PixelDupesPreference::Required => components.push("pixel duplicates".to_string()),
            PixelDupesPreference::Excluded => components.push("not pixel duplicates".to_string()),
            PixelDupesPreference::Allowed => {}
        }

        match self.search_type() {
            DupeSearchType::DifferentSearches => components.push(format!(
                "files matching [{}] and [{}]",
                Self::context_summary(&self.location_1, &self.query_1),
                Self::context_summary(&self.location_2, &self.query_2)
            )),
            DupeSearchType::BothFiles => components.push(format!(
                "both files matching [{}]",
                Self::context_summary(&self.location_1, &self.query_1)
            )),
            DupeSearchType::OneFile => components.push(format!(
                "one file matching [{}]",
                Self::context_summary(&self.location_1, &self.query_1)
            )),
        }

        if self.pixel_preference() != PixelDupesPreference::Required {
            components.push(format!("max search distance: {}", self.max_distance));
        }

        components.join(", ")
    }
}

#[derive(Debug, Clone)]
pub struct DuplicatePairView {
    pub a: Vec<u8>,
    pub b: Vec<u8>,
    pub distance: i64,
}

#[derive(Debug, Clone)]
pub struct DuplicatesPage {
    pub id: String,
    pub title: String,
    pub pairs: Vec<DuplicatePairView>,
    pub index: usize,
    pub status: String,
    pub searched: i64,
    pub matched: i64,
    pub search_context: DuplicateSearchContext,
}

impl DuplicatesPage {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            title: "duplicates processing".to_string(),
            pairs: Vec::new(),
            index: 0,
            status: String::new(),
            searched: 0,
            matched: 0,
            search_context: DuplicateSearchContext::default(),
        }
    }

    pub fn current(&self) -> Option<&DuplicatePairView> {
        self.pairs.get(self.index)
    }

    pub fn remove_current(&mut self) {
        if self.index < self.pairs.len() {
            self.pairs.remove(self.index);
        }

        if self.index >= self.pairs.len() {
            self.index = 0;
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct ImportQueueState {
    pub running: bool,
    pub done: usize,
    pub total: usize,
    pub imported: usize,
    pub failed: usize,
    pub message: String,
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
pub struct ImportFoldersPage {
    pub id: String,
    pub title: String,
    pub status: String,
}

impl ImportFoldersPage {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            title: "import folders".to_string(),
            status: String::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ExportFoldersPage {
    pub id: String,
    pub title: String,
    pub status: String,
}

impl ExportFoldersPage {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            title: "export folders".to_string(),
            status: String::new(),
        }
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
    Downloader(DownloaderPage),
    Subscriptions(SubscriptionsPage),
    Duplicates(DuplicatesPage),
    ImportFolders(ImportFoldersPage),
    ExportFolders(ExportFoldersPage),
    Placeholder(PlaceholderPage),
}

impl PageNode {
    pub fn id(&self) -> &str {
        match self {
            Self::Query(page) => &page.id,
            Self::Notebook(notebook) => &notebook.id,
            Self::Downloader(page) => &page.id,
            Self::Subscriptions(page) => &page.id,
            Self::Duplicates(page) => &page.id,
            Self::ImportFolders(page) => &page.id,
            Self::ExportFolders(page) => &page.id,
            Self::Placeholder(page) => &page.id,
        }
    }

    pub fn title(&self) -> &str {
        match self {
            Self::Query(page) => &page.title,
            Self::Notebook(notebook) => &notebook.title,
            Self::Downloader(page) => &page.title,
            Self::Subscriptions(page) => &page.title,
            Self::Duplicates(page) => &page.title,
            Self::ImportFolders(page) => &page.title,
            Self::ExportFolders(page) => &page.title,
            Self::Placeholder(page) => &page.title,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ViewerContext {
    pub page_id: String,
    pub hash: Vec<u8>,
    pub opened_at_ms: i64,
}

/// A page removed from the tree, kept so the user can restore it with undo.
#[derive(Debug, Clone)]
pub struct ClosedPage {
    pub parent_id: String,
    pub index: usize,
    pub node: PageNode,
}

/// Back/forward list of pages the user has visited, mirroring the navigation
/// history of `PagesNotebook` in the Python client.
#[derive(Debug, Clone, Default)]
pub struct PageHistory {
    entries: Vec<String>,
    index: usize,
}

impl PageHistory {
    pub const LIMIT: usize = 200;

    pub fn record(&mut self, page_id: &str) {
        if page_id.is_empty() || self.entries.get(self.index).is_some_and(|id| id == page_id) {
            return;
        }

        self.entries.truncate(self.index + 1);
        self.entries.push(page_id.to_string());

        if self.entries.len() > Self::LIMIT {
            let excess = self.entries.len() - Self::LIMIT;
            self.entries.drain(0..excess);
        }

        self.index = self.entries.len() - 1;
    }

    pub fn back(&mut self) -> Option<String> {
        if self.index == 0 {
            return None;
        }

        self.index -= 1;
        self.entries.get(self.index).cloned()
    }

    pub fn forward(&mut self) -> Option<String> {
        if self.index + 1 >= self.entries.len() {
            return None;
        }

        self.index += 1;
        self.entries.get(self.index).cloned()
    }

    pub fn can_back(&self) -> bool {
        self.index > 0
    }

    pub fn can_forward(&self) -> bool {
        self.index + 1 < self.entries.len()
    }

    pub fn entries(&self) -> &[String] {
        &self.entries
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.index = 0;
    }

    /// Drops ids that no longer exist. Keeps the cursor on the same page when
    /// possible and otherwise parks it at the newest surviving entry.
    pub fn retain(&mut self, mut keep: impl FnMut(&str) -> bool) {
        let current = self.entries.get(self.index).cloned();
        let mut kept = Vec::with_capacity(self.entries.len());

        for id in self.entries.drain(..) {
            if keep(&id) {
                kept.push(id);
            }
        }

        self.entries = kept;
        self.index = current
            .and_then(|id| self.entries.iter().position(|entry| *entry == id))
            .unwrap_or_else(|| self.entries.len().saturating_sub(1));
    }
}

pub struct AppState {
    pub controller: Arc<Controller>,
    pub root: Notebook,
    pub active: Option<String>,
    pub message: String,
    pub viewers: std::collections::BTreeMap<String, ViewerContext>,
    pub subscriptions: Vec<SubscriptionEntry>,
    pub import_folders: Vec<super::import_folders::ImportFolderEntry>,
    pub export_folders: Vec<super::export::ExportFolderEntry>,
    pub auto_rules: Vec<super::auto_resolution::AutoResolutionRule>,
    pub duplicate_review: super::auto_resolution::DuplicateReviewState,
    pub favourites: Vec<super::favourites::FavouriteSearch>,
    pub search_history: Vec<String>,
    pub notifications: super::notifications::NotificationLog,
    pub admin: super::services::AdminState,
    pub import_queue: ImportQueueState,
    pub history: PageHistory,
    pub closed_pages: Vec<ClosedPage>,
    pub session_name: Option<String>,
    pub(crate) runtime: AsyncRuntime,
    next_id: u64,
}

impl AppState {
    pub fn new(controller: Arc<Controller>) -> Self {
        let session_path = controller.db_dir().join("gui session.json");
        let db_dir = controller.db_dir().to_path_buf();
        let runtime = AsyncRuntime::new(controller.clone());

        let mut state = Self {
            controller,
            root: Notebook::new("root", "top page notebook"),
            active: None,
            message: "ready".to_string(),
            viewers: Default::default(),
            subscriptions: Vec::new(),
            import_folders: Vec::new(),
            export_folders: Vec::new(),
            auto_rules: Vec::new(),
            duplicate_review: Default::default(),
            favourites: Vec::new(),
            search_history: Vec::new(),
            notifications: super::notifications::NotificationLog::default(),
            admin: super::services::AdminState::default(),
            import_queue: ImportQueueState::default(),
            history: PageHistory::default(),
            closed_pages: Vec::new(),
            session_name: None,
            runtime,
            next_id: 1,
        };

        let mut restored_active = None;

        if let Some(loaded) = crate::gui::app::session::load_session(&session_path) {
            if let Some(legacy) = loaded.legacy {
                state.controller.set_session(legacy);
            }

            restored_active = loaded.active;
            state.session_name = loaded.session_name;
            state.root = loaded.root;
            state.subscriptions = loaded.subscriptions;
        } else {
            state.root = default_root();
        }

        state.import_folders = super::import_folders::load(&db_dir.join("import folders.json"));
        state.export_folders = super::export::load(&db_dir.join("export folders.json"));
        state.auto_rules = super::auto_resolution::load_rules(
            &db_dir.join("duplicate auto-resolution rules.json"),
        );
        state.duplicate_review = super::auto_resolution::load_review(
            &db_dir.join("duplicate auto-resolution pending.json"),
        );
        state.favourites = super::favourites::load(&db_dir.join("favourite searches.json"));
        state.search_history = super::search_history::load(&db_dir.join("search history.json"));
        state.admin = super::services::load_admin(&db_dir.join("server admin.json"));

        state.reindex();
        state.active = restored_active
            .filter(|id| find_node(&state.root, id).is_some())
            .or_else(|| current_leaf(&state.root))
            .or_else(|| first_page_id(&state.root));

        state
    }

    pub fn new_id(&mut self, prefix: &str) -> String {
        loop {
            let id = format!("{prefix}{}", self.next_id);
            self.next_id += 1;

            if find_node(&self.root, &id).is_none() {
                return id;
            }
        }
    }

    /// Makes the internal counter larger than every id in the loaded session
    /// and repairs duplicate ids written by early builds that started their
    /// counter at 1 while the default page was already `p1`.
    fn reindex(&mut self) {
        fn visit(
            node: &mut PageNode,
            seen: &mut std::collections::BTreeSet<String>,
            next: &mut u64,
        ) {
            let id = node.id().to_string();

            if !seen.insert(id.clone()) {
                let replacement = format!("p{next}");
                *next += 1;
                seen.insert(replacement.clone());

                match node {
                    PageNode::Query(page) => page.id = replacement,
                    PageNode::Notebook(notebook) => notebook.id = replacement,
                    PageNode::Downloader(page) => page.id = replacement,
                    PageNode::Subscriptions(page) => page.id = replacement,
                    PageNode::Duplicates(page) => page.id = replacement,
                    PageNode::ImportFolders(page) => page.id = replacement,
                    PageNode::ExportFolders(page) => page.id = replacement,
                    PageNode::Placeholder(page) => page.id = replacement,
                }
            }

            if let PageNode::Notebook(notebook) = node {
                for child in &mut notebook.children {
                    visit(child, seen, next);
                }
            }
        }

        let mut seen = std::collections::BTreeSet::new();
        let mut next = self.next_id.max(2);

        for child in &mut self.root.children {
            visit(child, &mut seen, &mut next);
        }

        fn max_id(node: &PageNode, max: &mut u64) {
            let digits = node
                .id()
                .trim_start_matches(|character: char| !character.is_ascii_digit());

            if let Ok(number) = digits.parse::<u64>() {
                *max = (*max).max(number);
            }

            if let PageNode::Notebook(notebook) = node {
                for child in &notebook.children {
                    max_id(child, max);
                }
            }
        }

        let mut max = next.saturating_sub(1);

        for child in &self.root.children {
            max_id(child, &mut max);
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

    pub fn find_downloader(&self, id: &str) -> Option<&DownloaderPage> {
        find_node(&self.root, id).and_then(|node| match node {
            PageNode::Downloader(page) => Some(page),
            _ => None,
        })
    }

    pub fn find_downloader_mut(&mut self, id: &str) -> Option<&mut DownloaderPage> {
        find_node_mut(&mut self.root, id).and_then(|node| match node {
            PageNode::Downloader(page) => Some(page),
            _ => None,
        })
    }

    pub fn find_duplicates(&self, id: &str) -> Option<&DuplicatesPage> {
        find_node(&self.root, id).and_then(|node| match node {
            PageNode::Duplicates(page) => Some(page),
            _ => None,
        })
    }

    pub fn find_duplicates_mut(&mut self, id: &str) -> Option<&mut DuplicatesPage> {
        find_node_mut(&mut self.root, id).and_then(|node| match node {
            PageNode::Duplicates(page) => Some(page),
            _ => None,
        })
    }

    pub fn find_subscriptions(&self, id: &str) -> Option<&SubscriptionsPage> {
        find_node(&self.root, id).and_then(|node| match node {
            PageNode::Subscriptions(page) => Some(page),
            _ => None,
        })
    }

    pub fn find_subscriptions_mut(&mut self, id: &str) -> Option<&mut SubscriptionsPage> {
        find_node_mut(&mut self.root, id).and_then(|node| match node {
            PageNode::Subscriptions(page) => Some(page),
            _ => None,
        })
    }

    pub fn find_import_folders(&self, id: &str) -> Option<&ImportFoldersPage> {
        find_node(&self.root, id).and_then(|node| match node {
            PageNode::ImportFolders(page) => Some(page),
            _ => None,
        })
    }

    pub fn find_import_folders_mut(&mut self, id: &str) -> Option<&mut ImportFoldersPage> {
        find_node_mut(&mut self.root, id).and_then(|node| match node {
            PageNode::ImportFolders(page) => Some(page),
            _ => None,
        })
    }

    pub fn import_folder_page_ids(&self) -> Vec<String> {
        let mut ids = Vec::new();
        super::collect_import_folder_ids(&self.root, &mut ids);
        ids
    }

    pub fn find_export_folders(&self, id: &str) -> Option<&ExportFoldersPage> {
        find_node(&self.root, id).and_then(|node| match node {
            PageNode::ExportFolders(page) => Some(page),
            _ => None,
        })
    }

    pub fn find_export_folders_mut(&mut self, id: &str) -> Option<&mut ExportFoldersPage> {
        find_node_mut(&mut self.root, id).and_then(|node| match node {
            PageNode::ExportFolders(page) => Some(page),
            _ => None,
        })
    }

    pub fn export_folder_page_ids(&self) -> Vec<String> {
        let mut ids = Vec::new();
        super::collect_export_folder_ids(&self.root, &mut ids);
        ids
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

    /// Makes `page_id` the current page all the way down to the root notebook.
    pub fn select_page(&mut self, page_id: &str) -> bool {
        if find_node(&self.root, page_id).is_none() {
            return false;
        }

        select_in_notebook(&mut self.root, page_id);
        self.active = Some(page_id.to_string());
        true
    }

    /// Records a page the user navigated to. Repeated selections are ignored.
    pub fn record_history(&mut self, page_id: &str) {
        if find_node(&self.root, page_id).is_some() {
            self.history.record(page_id);
        }
    }

    /// Remembers a closed page so `undo` can put it back where it was.
    pub fn push_closed_page(&mut self, parent_id: impl Into<String>, index: usize, node: PageNode) {
        self.closed_pages.push(ClosedPage {
            parent_id: parent_id.into(),
            index,
            node,
        });

        if self.closed_pages.len() > 25 {
            self.closed_pages.remove(0);
        }
    }

    /// Restores the newest closed page and selects it.
    pub fn undo_closed_page(&mut self) -> Option<String> {
        while let Some(closed) = self.closed_pages.pop() {
            let Some(parent) = self.find_notebook_mut(&closed.parent_id) else {
                continue;
            };

            let id = closed.node.id().to_string();
            let index = closed.index.min(parent.children.len());
            parent.children.insert(index, closed.node);
            parent.current = Some(id.clone());

            if self.select_page(&id) {
                self.record_history(&id);
                return Some(id);
            }
        }

        None
    }

    pub fn history_back(&mut self) -> Option<String> {
        let mut candidate = self.history.back();

        while let Some(id) = candidate {
            if self.select_page(&id) {
                return Some(id);
            }

            candidate = self.history.back();
        }

        None
    }

    pub fn history_forward(&mut self) -> Option<String> {
        let mut candidate = self.history.forward();

        while let Some(id) = candidate {
            if self.select_page(&id) {
                return Some(id);
            }

            candidate = self.history.forward();
        }

        None
    }

    /// Moves a page relative to another page or into another notebook.
    ///
    /// `position` is `inside`, `before` or `after`; a non-notebook target falls
    /// back to `after`.
    pub fn move_page(
        &mut self,
        source_id: &str,
        target_id: &str,
        position: &str,
    ) -> Result<(), String> {
        if source_id == target_id {
            return Err("a page cannot be moved onto itself".to_string());
        }

        let source_is_notebook = match find_node(&self.root, source_id) {
            Some(PageNode::Notebook(_)) => true,
            Some(_) => false,
            None => return Err("page is missing".to_string()),
        };

        if source_is_notebook {
            let contains_target = match find_node(&self.root, source_id) {
                Some(PageNode::Notebook(notebook)) => find_node(notebook, target_id).is_some(),
                _ => false,
            };

            if contains_target {
                return Err("a page of pages cannot be moved into itself".to_string());
            }
        }

        if find_node(&self.root, target_id).is_none() {
            return Err("target page is missing".to_string());
        }

        let mut node = Some(
            take_node(&mut self.root, source_id).ok_or_else(|| "page is missing".to_string())?,
        );

        let inserted_inside = if position == "inside" {
            match find_node_mut(&mut self.root, target_id) {
                Some(PageNode::Notebook(notebook)) => {
                    if let Some(page) = node.take() {
                        notebook.add(page);
                    }
                    true
                }
                _ => false,
            }
        } else {
            false
        };

        if !inserted_inside {
            let parent_id = parent_notebook_id(&self.root, target_id)
                .ok_or_else(|| "target notebook is missing".to_string())?;
            let parent = self
                .find_notebook_mut(&parent_id)
                .ok_or_else(|| "target notebook is missing".to_string())?;
            let index = parent
                .children
                .iter()
                .position(|child| child.id() == target_id)
                .ok_or_else(|| "target page is missing".to_string())?;
            let insert_at = if position == "before" {
                index
            } else {
                index + 1
            };

            let page = node.ok_or_else(|| "page is missing".to_string())?;
            parent.children.insert(insert_at, page);
        }

        if !self.select_page(source_id) {
            return Err("moved page is missing".to_string());
        }

        Ok(())
    }

    /// Deep-copies a page and puts the copy right after the original.
    pub fn duplicate_page(&mut self, page_id: &str) -> Result<String, String> {
        let parent_id =
            parent_notebook_id(&self.root, page_id).ok_or_else(|| "page is missing".to_string())?;
        let mut node = find_node(&self.root, page_id)
            .cloned()
            .ok_or_else(|| "page is missing".to_string())?;

        self.reassign_ids(&mut node);

        let new_id = node.id().to_string();
        let parent = self
            .find_notebook_mut(&parent_id)
            .ok_or_else(|| "notebook is missing".to_string())?;
        let index = parent
            .children
            .iter()
            .position(|child| child.id() == page_id)
            .ok_or_else(|| "page is missing".to_string())?;

        parent.children.insert(index + 1, node);
        parent.current = Some(new_id.clone());
        self.select_page(&new_id);

        Ok(new_id)
    }

    /// Replaces the live tree with a saved session.
    pub fn replace_session(&mut self, loaded: crate::gui::app::session::LoadedSession) {
        self.session_name = loaded.session_name;
        self.root = loaded.root;
        self.subscriptions = loaded.subscriptions;
        self.history.clear();
        self.closed_pages.clear();
        self.viewers.clear();
        self.active = None;
        self.reindex();
        self.active = loaded
            .active
            .filter(|id| find_node(&self.root, id).is_some())
            .or_else(|| current_leaf(&self.root))
            .or_else(|| first_page_id(&self.root));

        if let Some(id) = self.active.clone() {
            self.select_page(&id);
            self.record_history(&id);
        }
    }

    /// Appends another session's pages and subscriptions to the live tree.
    pub fn append_session(&mut self, loaded: crate::gui::app::session::LoadedSession) -> usize {
        let mut added = 0;

        for mut child in loaded.root.children {
            self.reassign_ids(&mut child);
            let first = first_page_id_of(&child);
            self.root.children.push(child);
            added += 1;

            if let Some(id) = first {
                self.root.current = Some(id.clone());
                self.select_page(&id);
                self.record_history(&id);
            }
        }

        for subscription in loaded.subscriptions {
            if !self
                .subscriptions
                .iter()
                .any(|existing| existing.name == subscription.name)
            {
                self.subscriptions.push(subscription);
            }
        }

        added
    }

    /// Gives every node a fresh id and keeps notebook selections consistent.
    fn reassign_ids(&mut self, node: &mut PageNode) {
        let old_current = match node {
            PageNode::Notebook(notebook) => notebook.current.clone(),
            _ => None,
        };

        let prefix = match node {
            PageNode::Notebook(_) => "n",
            _ => "p",
        };
        let new_id = self.new_id(prefix);

        match node {
            PageNode::Query(page) => page.id = new_id,
            PageNode::Notebook(notebook) => notebook.id = new_id,
            PageNode::Downloader(page) => page.id = new_id,
            PageNode::Subscriptions(page) => page.id = new_id,
            PageNode::Duplicates(page) => page.id = new_id,
            PageNode::ImportFolders(page) => page.id = new_id,
            PageNode::ExportFolders(page) => page.id = new_id,
            PageNode::Placeholder(page) => page.id = new_id,
        }

        if let PageNode::Notebook(notebook) = node {
            let current_position = old_current
                .as_deref()
                .and_then(|old| notebook.children.iter().position(|child| child.id() == old));

            for child in &mut notebook.children {
                self.reassign_ids(child);
            }

            notebook.current = current_position
                .and_then(|index| notebook.children.get(index))
                .or_else(|| notebook.children.first())
                .map(|child| child.id().to_string());
        }
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

fn select_in_notebook(notebook: &mut Notebook, id: &str) -> bool {
    let mut found = None;

    for (index, child) in notebook.children.iter().enumerate() {
        if child.id() == id {
            found = Some((index, false));
            break;
        }

        if let PageNode::Notebook(inner) = child {
            if find_node(inner, id).is_some() {
                found = Some((index, true));
                break;
            }
        }
    }

    let Some((index, nested)) = found else {
        return false;
    };

    let child_id = notebook.children[index].id().to_string();
    notebook.current = Some(child_id);

    if nested {
        if let PageNode::Notebook(inner) = &mut notebook.children[index] {
            return select_in_notebook(inner, id);
        }
    }

    true
}

fn take_node(notebook: &mut Notebook, id: &str) -> Option<PageNode> {
    if let Some(position) = notebook.children.iter().position(|child| child.id() == id) {
        let node = notebook.children.remove(position);

        if notebook.current.as_deref() == Some(id) {
            notebook.current = notebook
                .children
                .get(position.min(notebook.children.len().saturating_sub(1)))
                .map(|child| child.id().to_string());
        }

        return Some(node);
    }

    for child in &mut notebook.children {
        if let PageNode::Notebook(inner) = child {
            if let Some(node) = take_node(inner, id) {
                return Some(node);
            }
        }
    }

    None
}

fn first_page_id_of(node: &PageNode) -> Option<String> {
    match node {
        PageNode::Notebook(notebook) => first_page_id(notebook),
        _ => Some(node.id().to_string()),
    }
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

#[cfg(test)]
mod tests;
