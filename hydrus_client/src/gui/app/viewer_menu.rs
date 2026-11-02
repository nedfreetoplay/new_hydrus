//! Media viewer right-click menu (WS-13.13).
//!
//! R2: the menu composition, order, labels, tooltips and enablement live here
//! with unit tests. C++ only builds a `QMenu` from the typed rows and fires
//! `notify(action, target, payload)` (notes go through the typed
//! `app_manage_notes`, like the hover button).
//!
//! Order mirrors `CanvasPanel.ShowMenuFromSignal`
//! (`ClientGUICanvas.py`, v692 — the port viewer reuses it): info submenu,
//! volume, archive/inbox, per-service delete (+ trashed items), manage
//! submenu, known-urls submenu, open submenu, share submenu, player.
//!
//! No-mechanism items stay in the menu but disabled, exactly as Python would
//! show them: manage ratings (no ratings service), force filetype, viewing
//! stats clear, similar-file search, duplicate-filter page open, external
//! `using ...` launchers, `in another program`, `export files`, `copy file`
//! (clipboard files), copy bitmap/thumbnail rows, known-urls manage /
//! page search / force refetch, `ratings`, audio mute / volume (no audio
//! backend), `delete physically now` (no purge seam), all player rows
//! (no media player).
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
    /// Internal hash_id (Python `GetHashId()`), when the file is known.
    pub hash_id: Option<i64>,
    pub blurhash: Option<String>,
    pub pixel_hash_hex: Option<String>,
    pub urls: Vec<String>,
    /// Local file domains the file is currently in (sorted, Python
    /// `GetCurrent() ∩ local file service keys`).
    pub local_file_services: Vec<String>,
    pub is_trashed: bool,
    /// Any ratings service exists (Python `i_can_post_ratings`).
    pub can_post_ratings: bool,
    /// The file has a local file path (Python `IsLocal()`).
    pub is_local: bool,
    /// Python `HasStaticImages()` — gate for the `similar files` submenu;
    /// `IsStaticImage()` also gates the bitmap copy rows in `share`.
    pub has_static_image: bool,
    /// `(width, height)` for static images; gates the source-lookup row.
    pub image_width: Option<u32>,
    pub image_height: Option<u32>,
    /// Python `mime in HC.MIMES_WITH_THUMBNAILS`.
    pub mime_has_thumbnails: bool,
    pub advanced_mode: bool,
    pub global_mute: bool,
    pub viewer_mute: bool,
}

fn entry(depth: i32, label: &str, is_submenu: bool, is_separator: bool) -> MenuEntry {
    MenuEntry {
        depth,
        label: label.to_string(),
        action: UiAction::None,
        tooltip: String::new(),
        enabled: true,
        is_separator,
        is_submenu,
        title: String::new(),
        target: String::new(),
        payload: String::new(),
        checkable: false,
        checked: false,
    }
}

fn separator(depth: i32) -> MenuEntry {
    entry(depth, "", false, true)
}

fn submenu(depth: i32, label: &str) -> MenuEntry {
    entry(depth, label, true, false)
}

/// A plain menu row. `hash`/`page` fill the default typed payload when the
/// action needs the focus file.
fn row(
    depth: i32,
    label: &str,
    tooltip: &str,
    action: UiAction,
    facts: &ViewerMenuFacts,
    use_hash: bool,
    use_page: bool,
    label_target: &str,
    payload: &str,
    enabled: bool,
) -> MenuEntry {
    let mut item = entry(depth, label, false, false);
    item.action = action;
    item.target = if use_page {
        facts.page_id.clone()
    } else if label_target.is_empty() {
        String::new()
    } else {
        label_target.to_string()
    };
    item.payload = if use_hash {
        facts.hash_hex.clone()
    } else {
        payload.to_string()
    };
    item.tooltip = tooltip.to_string();
    item.enabled = enabled;
    item
}

fn label_row(depth: i32, label: &str, tooltip: &str) -> MenuEntry {
    row(
        depth,
        label,
        tooltip,
        UiAction::None,
        &ViewerMenuFacts::default(),
        false,
        false,
        "",
        "",
        false,
    )
}

/// Build the viewer context menu rows for the given facts.
pub fn viewer_menu(facts: &ViewerMenuFacts) -> MenuBar {
    let mut menu = Vec::new();
    if !facts.has_media {
        return MenuBar { entries: menu };
    }

    // info submenu (filename summary as the only pretty info line yet)
    menu.push(submenu(0, &facts.info_title));
    menu.push(label_row(1, &facts.info_title, ""));
    menu.push(separator(0));

    // volume submenu (Python AddAudioVolumeMenu, media viewer canvas type).
    // The port has no audio backend, so every row is disabled.
    menu.push(submenu(0, "volume"));
    menu.push(label_row(
        1,
        &volume_label(facts.global_mute, "global"),
        "Mute/unmute audio.",
    ));
    menu.push(separator(1));
    menu.push(label_row(
        1,
        &volume_label(facts.viewer_mute, "media viewer"),
        "Mute/unmute audio.",
    ));
    menu.push(separator(1));
    menu.push(label_row(1, "volume", "Set the volume directly"));
    menu.push(separator(0));

    if facts.inbox {
        menu.push(row(
            0,
            "archive",
            "Archive this file.",
            UiAction::ArchiveFiles,
            facts,
            true,
            true,
            "",
            "",
            true,
        ));
    }
    if facts.archived {
        menu.push(row(
            0,
            "inbox",
            "Send this files back to the inbox.",
            UiAction::InboxFiles,
            facts,
            true,
            true,
            "",
            "",
            true,
        ));
    }
    menu.push(separator(0));

    push_delete(&mut menu, facts);
    push_trashed(&mut menu, facts);
    menu.push(separator(0));
    push_manage(&mut menu, facts);
    push_urls(&mut menu, facts);
    push_open(&mut menu, facts);
    push_share(&mut menu, facts);
    menu.push(separator(0));
    push_player(&mut menu, facts);

    MenuBar { entries: menu }
}

fn volume_label(mute: bool, scope: &str) -> String {
    if mute {
        format!("unmute {scope}")
    } else {
        format!("mute {scope}")
    }
}

/// Python `CanvasPanel.ShowMenuFromSignal`: one item for a single local file
/// service, a `delete` submenu otherwise.
fn push_delete(menu: &mut Vec<MenuEntry>, facts: &ViewerMenuFacts) {
    match facts.local_file_services.len() {
        0 => {}
        1 => menu.push(row(
            0,
            &format!("delete from {}", facts.local_file_services[0]),
            "Delete this file.",
            UiAction::DeleteFilesFromService,
            facts,
            true,
            false,
            &facts.local_file_services[0],
            "",
            true,
        )),
        _ => {
            menu.push(submenu(0, "delete"));
            for service in &facts.local_file_services {
                menu.push(row(
                    1,
                    &format!("from {service}"),
                    "Delete this file.",
                    UiAction::DeleteFilesFromService,
                    facts,
                    true,
                    false,
                    service,
                    "",
                    true,
                ));
            }
        }
    }
}

/// Python: trashed files get `delete physically now` and `undelete`. The port
/// has no physical-purge seam, so `delete physically now` stays disabled.
fn push_trashed(menu: &mut Vec<MenuEntry>, facts: &ViewerMenuFacts) {
    if !facts.is_trashed {
        return;
    }

    menu.push(label_row(
        0,
        "delete physically now",
        "Physically delete this file from disk.",
    ));
    menu.push(row(
        0,
        "undelete",
        "Take this file out of the trash.",
        UiAction::UndeleteFiles,
        facts,
        true,
        true,
        "",
        "",
        true,
    ));
}

fn push_manage(menu: &mut Vec<MenuEntry>, facts: &ViewerMenuFacts) {
    menu.push(submenu(0, "manage"));
    menu.push(row(
        1,
        "tags",
        "Manage this file's tags.",
        UiAction::ManageTags,
        facts,
        true,
        true,
        "",
        "",
        true,
    ));
    // No manage-ratings dialog in the port yet: disabled like Python's row
    // would be when nothing can act on it.
    if facts.can_post_ratings {
        menu.push(label_row(1, "ratings", "Manage this file's ratings."));
    }
    menu.push(row(
        1,
        &notes_label(facts.note_count),
        "Manage this file's notes.",
        UiAction::NotesManage,
        facts,
        true,
        false,
        "",
        "",
        true,
    ));
    menu.push(row(
        1,
        "times",
        "Edit the timestamps for your files.",
        UiAction::EditTimestamps,
        facts,
        true,
        false,
        "",
        "",
        true,
    ));
    menu.push(label_row(
        1,
        "force filetype",
        "Force your files to appear as a different filetype.",
    ));
    // Python `AddManageFileViewingStatsMenu`: a `viewing stats` submenu with
    // one `clear` row. The port has no clear-stats seam, so it stays disabled.
    menu.push(submenu(1, "viewing stats"));
    menu.push(label_row(
        2,
        "clear",
        "Clear all the recorded file viewing stats for the selected files.",
    ));
}

/// Python `AddKnownURLsViewCopyMenu` for one focused file. The port has no
/// url classes, so `manage` and the page-search submenu stay disabled.
fn push_urls(menu: &mut Vec<MenuEntry>, facts: &ViewerMenuFacts) {
    menu.push(submenu(0, "urls"));
    menu.push(label_row(1, "manage", "Edit which URLs this file has."));

    if !facts.urls.is_empty() {
        menu.push(submenu(1, "open in browser"));
        for url in &facts.urls {
            menu.push(row(
                2,
                url,
                "Open this url in your web browser.",
                UiAction::OpenKnownUrl,
                facts,
                false,
                false,
                "",
                url,
                true,
            ));
        }

        menu.push(submenu(1, "open in a new page"));
        for url in &facts.urls {
            menu.push(label_row(2, url, "Open a new page with the files that have this url."));
        }
        if facts.urls.len() > 1 {
            menu.push(label_row(
                2,
                "files with any of the above",
                "Open a new page with the files that share any of this file's urls.",
            ));
        }

        menu.push(submenu(1, "copy"));
        if facts.urls.len() > 1 {
            menu.push(row(
                2,
                &format!("this file's {} urls", facts.urls.len()),
                "Copy this url to your clipboard.",
                UiAction::CopyKnownUrl,
                facts,
                false,
                false,
                "",
                &facts.urls.join("\n"),
                true,
            ));
        }
        for url in &facts.urls {
            menu.push(label_row(2, url, "Copy this url to your clipboard."));
        }

        menu.push(separator(1));
        menu.push(submenu(1, "force metadata refetch"));
    }
}

/// Python `AddOpenMenu` for one focused file.
fn push_open(menu: &mut Vec<MenuEntry>, facts: &ViewerMenuFacts) {
    menu.push(submenu(0, "open"));
    menu.push(label_row(
        1,
        "in a new page",
        "Copy your current selection into a simple new page.",
    ));
    menu.push(label_row(
        1,
        "in a new duplicate filter page",
        "Make a new duplicate filter page that searches for these files specifically.",
    ));

    if facts.has_static_image {
        menu.push(submenu(1, "similar files in a new page"));
        let speculative_tooltip = "Search the database for files that probably look like those selected. This is sometimes useful for symbols with sharp edges or lines.";
        menu.push(separator(2));
        menu.push(label_row(
            2,
            "exact match",
            "Search the database for files that look precisely like those selected.",
        ));
        menu.push(label_row(
            2,
            "very similar",
            "Search the database for files that look just like those selected.",
        ));
        menu.push(label_row(
            2,
            "similar",
            "Search the database for files that look generally like those selected.",
        ));
        menu.push(label_row(2, "speculative", speculative_tooltip));
        menu.push(separator(2));
        menu.push(label_row(2, "custom", speculative_tooltip));
    }

    menu.push(separator(1));
    menu.push(row(
        1,
        "in web browser",
        "Show this file in your OS's web browser.",
        UiAction::OpenFileInWebBrowser,
        facts,
        true,
        false,
        "",
        "",
        true,
    ));

    if facts.is_local {
        menu.push(label_row(
            1,
            "in another program",
            "Choose which program to open this file with.",
        ));
        if facts.advanced_mode && crate::client_paths::CAN_OPEN_FILE_LOCATION {
            menu.push(row(
                1,
                "in file browser",
                "Show this file in your OS's file browser.",
                UiAction::OpenFileInFileBrowser,
                facts,
                true,
                false,
                "",
                "",
                true,
            ));
        }
        if crate::client_paths::CAN_OPEN_FILE_LOCATION {
            menu.push(row(
                1,
                "properties",
                "Open your OS's properties window for this file.",
                UiAction::OpenFileInNativeProperties,
                facts,
                true,
                false,
                "",
                "",
                true,
            ));
        }
    }
}

/// Python `AddShareMenu` for one focused local file.
fn push_share(menu: &mut Vec<MenuEntry>, facts: &ViewerMenuFacts) {
    menu.push(submenu(0, "share"));
    menu.push(label_row(
        1,
        "export files",
        "Export the selected files to an external folder.",
    ));
    menu.push(separator(1));
    menu.push(label_row(1, "copy file", "Copy this file to your clipboard."));
    menu.push(row(
        1,
        "copy path",
        "Copy this file's path to your clipboard, just as raw text.",
        UiAction::CopyFilePath,
        facts,
        true,
        false,
        "",
        "",
        true,
    ));
    menu.push(submenu(1, "copy hash"));
    menu.push(row(
        2,
        &format!("sha256 ({})", facts.hash_hex),
        "Copy this file's SHA256 hash to your clipboard.",
        UiAction::CopyFileHash,
        facts,
        true,
        false,
        "sha256",
        "",
        true,
    ));
    menu.push(label_row(
        2,
        "md5",
        "Copy this file's MD5 hash to your clipboard. Your client may not know this.",
    ));
    menu.push(label_row(
        2,
        "sha1",
        "Copy this file's SHA1 hash to your clipboard. Your client may not know this.",
    ));
    menu.push(label_row(
        2,
        "sha512",
        "Copy this file's SHA512 hash to your clipboard. Your client may not know this.",
    ));
    if let Some(blurhash) = &facts.blurhash {
        menu.push(row(
            2,
            &format!("blurhash ({blurhash})"),
            "Copy this file's blurhash.",
            UiAction::CopyFileHash,
            facts,
            true,
            false,
            "blurhash",
            "",
            true,
        ));
    }
    if let Some(pixel_hash) = &facts.pixel_hash_hex {
        menu.push(row(
            2,
            &format!("pixel hash ({pixel_hash})"),
            "Copy this file's pixel hash.",
            UiAction::CopyFileHash,
            facts,
            true,
            false,
            "pixel_hash",
            "",
            true,
        ));
    }
    menu.push(row(
        1,
        &copy_file_id_label(facts.hash_id),
        "Copy this file's internal file/hash_id.",
        UiAction::CopyFileId,
        facts,
        true,
        false,
        "",
        "",
        true,
    ));

    // Python `AddShareMenu`: bitmap rows for a focused local file. The port
    // has no clipboard-bitmap seam, so they stay disabled.
    if facts.is_local && facts.has_static_image {
        menu.push(label_row(
            1,
            "copy bitmap",
            "Copy this file's bitmap.",
        ));
        if let Some((label_w, label_h)) = source_lookup_resolution(facts) {
            menu.push(label_row(
                1,
                &format!("copy source lookup bitmap ({label_w}x{label_h})"),
                "Copy a smaller bitmap of this file, for quicker lookup on source-finding websites.",
            ));
        }
    }
    if facts.is_local && facts.mime_has_thumbnails {
        menu.push(label_row(
            1,
            "copy thumbnail bitmap",
            "Copy this file's thumbnail's bitmap.",
        ));
    }
}

/// Python `get_thumbnail_resolution(res, (1024, 1024), SCALE_TO_FIT, 100)`:
/// fit inside 1024x1024 preserving aspect, only when larger than the box.
/// Returns `None` when Python would not add the row at all.
pub fn source_lookup_resolution(facts: &ViewerMenuFacts) -> Option<(u32, u32)> {
    let (width, height) = (facts.image_width?, facts.image_height?);
    if width <= 1024 && height <= 1024 {
        return None;
    }
    if width == 0 || height == 0 {
        return None;
    }
    let scale = 1024.0 / width.max(height) as f64;
    Some((
        (width as f64 * scale).round() as u32,
        (height as f64 * scale).round() as u32,
    ))
}

fn copy_file_id_label(hash_id: Option<i64>) -> String {
    match hash_id {
        Some(hash_id) => format!("copy file id ({hash_id})"),
        // Python always has an id; the port falls back to a disabled plain
        // row when the file is unknown.
        None => "copy file id".to_string(),
    }
}

/// Python `CanvasPanel` always appends the `player` submenu with a label row.
/// The port has no media player, so the label says exactly what Python would.
fn push_player(menu: &mut Vec<MenuEntry>, facts: &ViewerMenuFacts) {
    let _ = facts;
    menu.push(submenu(0, "player"));
    menu.push(label_row(
        1,
        "This is a Unknown Media Player - let hydev know please.",
        "",
    ));
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
