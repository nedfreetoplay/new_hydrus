//! Qt-independent view models for the main window.
//!
//! Application logic (controller, session and the panel state modules) is
//! turned into these plain values before the GUI starts. The Qt layer in
//! [`super::qt`] only renders them; it does not build state or decide what a
//! page contains.
//!
//! shortcut: the view model is a snapshot built at startup; add change
//! notifications when pages become interactive.

use crate::gui::frame_gui::{MenuModel, default_frame_gui};
use crate::gui::import_panels::{DuplicatesPanelState, ImportPanelState, ParsingPanelState};
use crate::gui::networking_panels::{LoginState, NetworkingPanelState};
use crate::gui::options_panel::OptionsPanelState;
use crate::gui::popup_messages::PopupMessageManager;
use crate::gui::session::GuiSession;
use crate::gui::string_panels::{StringPanel, StringToStringDictPanel};
use crate::networking::NetworkContext;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WidgetViewModel {
    Label(String),
    LineEdit(String),
    Button(String),
    Checkbox { text: String, checked: bool },
    Stretch,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormViewModel {
    pub title: String,
    pub widgets: Vec<WidgetViewModel>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListViewModel {
    pub title: String,
    pub columns: String,
    pub rows: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaViewModel {
    pub title: String,
    pub items: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanvasViewModel {
    pub title: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PanelViewModel {
    Form(FormViewModel),
    ScrolledForm(FormViewModel),
    List(ListViewModel),
    Media(MediaViewModel),
    Canvas(CanvasViewModel),
}

/// Everything the main window needs for one render.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MainViewModel {
    pub title: String,
    pub status: String,
    pub sidebar: Vec<String>,
    pub pages: Vec<String>,
    pub menus: Vec<MenuModel>,
    pub panels: Vec<PanelViewModel>,
}

impl MainViewModel {
    pub fn new(
        title: impl Into<String>,
        status: impl Into<String>,
        session: Option<&GuiSession>,
    ) -> Self {
        let frame = default_frame_gui();
        let mut sidebar: Vec<String> = frame.sidebar.iter().map(|item| item.to_string()).collect();
        let mut pages: Vec<String> = frame.pages.iter().map(|page| page.to_string()).collect();

        if let Some(session) = session {
            pages.clear();

            for page in &session.pages {
                sidebar.push(page.name.clone());
                pages.push(page.name.clone());
            }
        }

        let welcome_title = frame
            .pages
            .first()
            .copied()
            .unwrap_or("Welcome")
            .to_string();

        let mut panels = vec![
            welcome_panel(welcome_title),
            media_panel(),
            canvas_panel(),
            options_panel(),
            example_list_panel(),
        ];
        panels.extend(networking_panels());
        panels.extend(duplicates_panels());
        panels.extend(import_panels());
        panels.push(parsing_panel());
        panels.push(options_panels_panel());
        panels.extend(string_panels());
        panels.push(popups_panel());

        Self {
            title: title.into(),
            status: status.into(),
            sidebar,
            pages,
            menus: frame.menus.to_vec(),
            panels,
        }
    }
}

fn label(text: impl Into<String>) -> WidgetViewModel {
    WidgetViewModel::Label(text.into())
}

fn line_edit(text: impl Into<String>) -> WidgetViewModel {
    WidgetViewModel::LineEdit(text.into())
}

fn button(text: impl Into<String>) -> WidgetViewModel {
    WidgetViewModel::Button(text.into())
}

fn checkbox(text: impl Into<String>, checked: bool) -> WidgetViewModel {
    WidgetViewModel::Checkbox {
        text: text.into(),
        checked,
    }
}

fn welcome_panel(title: String) -> PanelViewModel {
    PanelViewModel::Form(FormViewModel {
        title,
        widgets: vec![
            label("Welcome to Hydrus Client — Rust/Qt 6 port."),
            line_edit("Search tags…"),
            button("Search"),
            checkbox("Darkmode", false),
            WidgetViewModel::Stretch,
        ],
    })
}

fn media_panel() -> PanelViewModel {
    PanelViewModel::Media(MediaViewModel {
        title: "Files".to_string(),
        items: vec![
            "No files loaded yet".to_string(),
            "Open a page to browse media".to_string(),
        ],
    })
}

fn canvas_panel() -> PanelViewModel {
    PanelViewModel::Canvas(CanvasViewModel {
        title: "Canvas".to_string(),
    })
}

fn options_panel() -> PanelViewModel {
    PanelViewModel::ScrolledForm(FormViewModel {
        title: "Options".to_string(),
        widgets: vec![
            label("Client options"),
            line_edit("Search options…"),
            checkbox("Darkmode", false),
            button("Save"),
            WidgetViewModel::Stretch,
        ],
    })
}

fn example_list_panel() -> PanelViewModel {
    PanelViewModel::List(ListViewModel {
        title: "List".to_string(),
        columns: "Name,Size".to_string(),
        rows: vec!["example.png,1024".to_string()],
    })
}

fn networking_panels() -> Vec<PanelViewModel> {
    let mut state = NetworkingPanelState::new();
    state.add_context(&NetworkContext::for_domain("example.com"));
    state.add_context(&NetworkContext::for_domain("cdn.example.com"));
    state.set_header("User-Agent", "Hydrus Client (Rust)");

    let mut login = LoginState::new("example.com");
    login.username = "user".to_string();
    state.set_login(login);

    let contexts = PanelViewModel::List(ListViewModel {
        title: "Networking".to_string(),
        columns: "Network context".to_string(),
        rows: state.contexts.clone(),
    });

    let headers = PanelViewModel::List(ListViewModel {
        title: "Headers".to_string(),
        columns: "Header,Value".to_string(),
        rows: state
            .headers
            .iter()
            .map(|(name, value)| format!("{name},{value}"))
            .collect(),
    });

    let mut widgets = vec![label("Login")];

    if let Some(login) = &state.login {
        widgets.push(line_edit(login.domain.clone()));
        widgets.push(line_edit(login.username.clone()));
        widgets.push(line_edit("password"));
    }

    widgets.push(button("Log in"));
    widgets.push(WidgetViewModel::Stretch);

    vec![
        contexts,
        headers,
        PanelViewModel::Form(FormViewModel {
            title: "Login".to_string(),
            widgets,
        }),
    ]
}

fn duplicates_panels() -> Vec<PanelViewModel> {
    let mut state = DuplicatesPanelState::new();
    state.load_pairs([
        (vec![0xab, 0xcd, 0x01], vec![0xab, 0xcd, 0x02]),
        (vec![0xab, 0xcd, 0x03], vec![0xab, 0xcd, 0x04]),
    ]);

    let rows = state
        .pairs
        .iter()
        .enumerate()
        .map(|(index, (a, b))| {
            let status = if state.current_index == Some(index) {
                "current"
            } else {
                "pending"
            };

            format!("{},{},{status}", hex(a), hex(b))
        })
        .collect();

    let (done, total) = state.progress();

    vec![
        PanelViewModel::List(ListViewModel {
            title: "Duplicate Filter".to_string(),
            columns: "File A,File B,Status".to_string(),
            rows,
        }),
        PanelViewModel::Form(FormViewModel {
            title: "Duplicates".to_string(),
            widgets: vec![
                label(format!("Progress: {done}/{total}")),
                button("Approve"),
                button("Deny"),
                button("Skip"),
                WidgetViewModel::Stretch,
            ],
        }),
    ]
}

fn import_panels() -> Vec<PanelViewModel> {
    let mut state = ImportPanelState::new();
    state.gallery_count = 2;
    state.watcher_count = 1;
    state.subscription_count = 3;
    state.local_count = 1;

    vec![
        PanelViewModel::List(ListViewModel {
            title: "Downloads".to_string(),
            columns: "Importer,Count".to_string(),
            rows: vec![
                format!("Gallery,{}", state.gallery_count),
                format!("Watchers,{}", state.watcher_count),
                format!("Subscriptions,{}", state.subscription_count),
                format!("Local files,{}", state.local_count),
            ],
        }),
        PanelViewModel::List(ListViewModel {
            title: "Subscriptions".to_string(),
            columns: "Subscription,Files".to_string(),
            rows: vec![format!("subscriptions,{}", state.subscription_count)],
        }),
        PanelViewModel::List(ListViewModel {
            title: "Gallery".to_string(),
            columns: "Gallery,Files".to_string(),
            rows: vec![format!("gallery,{}", state.gallery_count)],
        }),
        PanelViewModel::List(ListViewModel {
            title: "Watchers".to_string(),
            columns: "Watcher,Files".to_string(),
            rows: vec![format!("watchers,{}", state.watcher_count)],
        }),
    ]
}

fn parsing_panel() -> PanelViewModel {
    let mut state = ParsingPanelState::new();
    state.set_formulas(
        ["gallery url", "thread json", "tag html"]
            .into_iter()
            .map(str::to_string),
    );
    state.set_test_text("<div>example</div>");
    state.set_last_result("https://example.com/file.png");

    let rows = state
        .formula_names
        .iter()
        .enumerate()
        .map(|(index, name)| {
            let result = if state.selected_formula == Some(index) {
                state.last_result.as_deref().unwrap_or("")
            } else {
                ""
            };

            format!("{name},{result}")
        })
        .collect();

    PanelViewModel::List(ListViewModel {
        title: "Parsing".to_string(),
        columns: "Formula,Result".to_string(),
        rows,
    })
}

fn options_panels_panel() -> PanelViewModel {
    let mut state = OptionsPanelState::new();
    state.set_search_text("");

    let rows = state
        .filtered_panels()
        .into_iter()
        .map(|entry| format!("{},{}", entry.title, entry.keywords.join(";")))
        .collect();

    PanelViewModel::List(ListViewModel {
        title: "Options panels".to_string(),
        columns: "Options page,Keywords".to_string(),
        rows,
    })
}

fn string_panels() -> Vec<PanelViewModel> {
    let panel = StringPanel::new("Rename tag").with_text("blue eyes");
    let editor = PanelViewModel::Form(FormViewModel {
        title: "String editor".to_string(),
        widgets: vec![
            label(format!("{}: {}", panel.title, panel.get_text())),
            line_edit(panel.get_text()),
            button("OK"),
            button("Cancel"),
            WidgetViewModel::Stretch,
        ],
    });

    let dict = StringToStringDictPanel::from_entries([
        ("User-Agent".to_string(), "Hydrus".to_string()),
        ("Referer".to_string(), "https://example.com".to_string()),
    ]);

    let dictionary = PanelViewModel::List(ListViewModel {
        title: "String dictionary".to_string(),
        columns: "Key,Value".to_string(),
        rows: dict
            .entries()
            .iter()
            .map(|(key, value)| format!("{key},{value}"))
            .collect(),
    });

    vec![editor, dictionary]
}

fn popups_panel() -> PanelViewModel {
    let manager = PopupMessageManager::new();
    let mut rows: Vec<String> = manager
        .get_active_messages()
        .iter()
        .map(|message| format!("{},{:?}", message.message_key(), message.is_done()))
        .collect();

    if manager.is_empty() {
        rows.push("No active popups,idle".to_string());
    }

    PanelViewModel::List(ListViewModel {
        title: "Popups".to_string(),
        columns: "Popup,Status".to_string(),
        rows,
    })
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests;
