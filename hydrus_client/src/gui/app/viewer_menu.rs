//! Media viewer right-click menu (WS-13.13).
//!
//! R2: the menu composition, order, labels, tooltips and enablement live here
//! with unit tests. C++ only builds a `QMenu` from the typed rows and fires
//! `notify(action, target, payload)` (notes go through the typed
//! `app_manage_notes`, like the hover button).
//!
//! Order mirrors `CanvasPanel.ShowMenuFromSignal`
//! (`ClientGUICanvas.py`, preview/media canvas — the viewer canvas reuses it):
//! info submenu, archive/inbox, delete, manage submenu (tags, notes, times,
//! viewing stats). Audio-volume, known-URL, open/share and player submenus
//! have no typed reverse seam yet and stay a documented remainder.

use crate::gui::qt::api::ffi::{MenuBar, MenuEntry, UiAction};

/// Facts about the viewer file the menu is built for.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ViewerMenuFacts {
    pub has_media: bool,
    pub inbox: bool,
    pub archived: bool,
    pub note_count: i64,
    pub info_title: String,
    pub page_id: String,
    pub hash_hex: String,
}

fn separator() -> MenuEntry {
    MenuEntry {
        depth: 0,
        title: String::new(),
        label: String::new(),
        action: UiAction::None,
        target: String::new(),
        payload: String::new(),
        tooltip: String::new(),
        enabled: true,
        checkable: false,
        checked: false,
        is_separator: true,
        is_submenu: false,
    }
}

fn submenu(label: &str) -> MenuEntry {
    MenuEntry {
        depth: 0,
        title: String::new(),
        label: label.to_string(),
        action: UiAction::None,
        target: String::new(),
        payload: String::new(),
        tooltip: String::new(),
        enabled: true,
        checkable: false,
        checked: false,
        is_separator: false,
        is_submenu: true,
    }
}

fn item(
    depth: i32,
    label: &str,
    tooltip: &str,
    action: UiAction,
    facts: &ViewerMenuFacts,
    enabled: bool,
) -> MenuEntry {
    MenuEntry {
        depth,
        title: String::new(),
        label: label.to_string(),
        action,
        target: facts.page_id.clone(),
        payload: facts.hash_hex.clone(),
        tooltip: tooltip.to_string(),
        enabled,
        checkable: false,
        checked: false,
        is_separator: false,
        is_submenu: false,
    }
}

/// Build the viewer context menu rows for the given facts.
pub fn viewer_menu(facts: &ViewerMenuFacts) -> MenuBar {
    let mut entries: Vec<MenuEntry> = Vec::new();
    if !facts.has_media {
        return MenuBar { entries };
    }
    entries.push(submenu(&facts.info_title));
    entries.push(MenuEntry {
        depth: 1,
        enabled: false,
        ..item(
            1,
            &facts.info_title,
            "",
            UiAction::None,
            facts,
            false,
        )
    });
    entries.push(separator());
    if facts.inbox {
        entries.push(item(
            0,
            "archive",
            "Archive this file.",
            UiAction::ArchiveFiles,
            facts,
            true,
        ));
    }
    if facts.archived {
        entries.push(item(
            0,
            "inbox",
            "Send this files back to the inbox.",
            UiAction::InboxFiles,
            facts,
            true,
        ));
    }
    entries.push(separator());
    entries.push(item(
        0,
        "delete",
        "Delete this file.",
        UiAction::DeleteFiles,
        facts,
        true,
    ));
    entries.push(separator());
    entries.push(submenu("manage"));
    entries.push(item(
        1,
        "tags",
        "Manage this file's tags.",
        UiAction::ManageTags,
        facts,
        true,
    ));
    let notes_label = if facts.note_count > 0 {
        format!("notes ({})", facts.note_count)
    } else {
        "notes".to_string()
    };
    entries.push(item(
        1,
        &notes_label,
        "Manage this file's notes.",
        UiAction::NotesManage,
        facts,
        true,
    ));
    entries.push(item(
        1,
        "times",
        "Edit the timestamps for your files.",
        UiAction::EditTimestamps,
        facts,
        true,
    ));
    entries.push(item(
        1,
        "viewing stats",
        "Show file viewing statistics.",
        UiAction::ViewingStats,
        facts,
        true,
    ));
    MenuBar { entries }
}

/// Notes label helper (also used by the dialog enablement text).
pub fn notes_label(note_count: i64) -> String {
    if note_count > 0 {
        format!("notes ({note_count})")
    } else {
        "notes".to_string()
    }
}

#[cfg(test)]
mod tests;
