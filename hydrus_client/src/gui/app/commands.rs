//! UI commands produced by the application layer and executed by the Qt layer.
//!
//! Commands are plain data and never hold a borrow of `AppState`, which keeps
//! dialogs and other re-entrant Qt callbacks safe.

#[derive(Debug, Clone)]
pub enum UiCommand {
    Menus(String),
    TagSuggestions(String),
    BuildPages(String),
    UpdatePage(String, String),
    SetSelection {
        page_id: String,
        hash: String,
        tags_json: String,
        preview_path: String,
    },
    Status(String),
    ShowPageChooser(String),
    PickFiles,
    PromptUrl,
    OpenViewer(String),
    UpdateViewer(String, String),
    CloseViewer(String),
    TagDialog(String),
    NotifyMessage(String),
    Message(String, String),
    OpenPath(String),
    OpenUrl(String),
    Screenshot(String),
    Quit,
}
