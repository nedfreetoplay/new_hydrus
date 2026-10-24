//! Qt wiring for the networking, duplicates, import, parsing, options and
//! string panel models.
//!
//! The port deliberately reuses the generic list/panel widgets from
//! `bridge.hpp`; these helpers feed each model into a tab.

use std::pin::Pin;

use cxx_qt_lib as _;

use super::bridge::ffi;
use crate::duplicates::DuplicatePairDecision;
use crate::gui::import_panels::{DuplicatesPanelState, ImportPanelState, ParsingPanelState};
use crate::gui::networking_panels::{LoginState, NetworkingPanelState};
use crate::gui::options_panel::OptionsPanelState;
use crate::gui::popup_messages::PopupMessageManager;
use crate::gui::string_panels::{StringPanel, StringToStringDictPanel};
use crate::networking::NetworkContext;

type Window<'a> = Pin<&'a mut ffi::HydrusMainWindow>;

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

pub(crate) fn add_networking_pages(mut window: Window<'_>) {
    let mut state = NetworkingPanelState::new();
    state.add_context(&NetworkContext::for_domain("example.com"));
    state.add_context(&NetworkContext::for_domain("cdn.example.com"));
    state.set_header("User-Agent", "Hydrus Client (Rust)");

    let mut contexts = ffi::new_hydrus_list_panel();
    ffi::hydrus_list_set_columns(contexts.pin_mut(), "Network context");
    for context in &state.contexts {
        ffi::hydrus_list_add_row(contexts.pin_mut(), context);
    }
    ffi::hydrus_add_list_page(window.as_mut(), "Networking", contexts.pin_mut());

    let mut headers = ffi::new_hydrus_list_panel();
    ffi::hydrus_list_set_columns(headers.pin_mut(), "Header,Value");
    for (name, value) in &state.headers {
        ffi::hydrus_list_add_row(headers.pin_mut(), &format!("{name},{value}"));
    }
    ffi::hydrus_add_list_page(window.as_mut(), "Headers", headers.pin_mut());

    let mut login = LoginState::new("example.com");
    login.username = "user".to_string();
    state.set_login(login);

    let mut panel = ffi::new_hydrus_panel();
    ffi::hydrus_panel_add_label(panel.pin_mut(), "Login");
    if let Some(login) = &state.login {
        ffi::hydrus_panel_add_line_edit(panel.pin_mut(), &login.domain);
        ffi::hydrus_panel_add_line_edit(panel.pin_mut(), &login.username);
        ffi::hydrus_panel_add_line_edit(panel.pin_mut(), "password");
    }
    ffi::hydrus_panel_add_button(panel.pin_mut(), "Log in");
    ffi::hydrus_panel_add_stretch(panel.pin_mut());
    ffi::hydrus_add_page_widget(window.as_mut(), "Login", panel.pin_mut());
}

pub(crate) fn add_duplicates_page(mut window: Window<'_>) {
    let mut state = DuplicatesPanelState::new();
    state.load_pairs([
        (vec![0xab, 0xcd, 0x01], vec![0xab, 0xcd, 0x02]),
        (vec![0xab, 0xcd, 0x03], vec![0xab, 0xcd, 0x04]),
    ]);

    let mut panel = ffi::new_hydrus_list_panel();
    ffi::hydrus_list_set_columns(panel.pin_mut(), "File A,File B,Status");

    for (index, (a, b)) in state.pairs.iter().enumerate() {
        let status = if state.current_index == Some(index) {
            "current"
        } else {
            "pending"
        };
        ffi::hydrus_list_add_row(panel.pin_mut(), &format!("{},{},{status}", hex(a), hex(b)));
    }

    ffi::hydrus_add_list_page(window.as_mut(), "Duplicate Filter", panel.pin_mut());

    let (done, total) = state.progress();
    let decision = DuplicatePairDecision::Approve;
    state.decide(&decision);

    let mut controls = ffi::new_hydrus_panel();
    ffi::hydrus_panel_add_label(controls.pin_mut(), &format!("Progress: {done}/{total}"));
    ffi::hydrus_panel_add_button(controls.pin_mut(), "Approve");
    ffi::hydrus_panel_add_button(controls.pin_mut(), "Deny");
    ffi::hydrus_panel_add_button(controls.pin_mut(), "Skip");
    ffi::hydrus_panel_add_stretch(controls.pin_mut());
    ffi::hydrus_add_page_widget(window.as_mut(), "Duplicates", controls.pin_mut());
}

pub(crate) fn add_import_page(mut window: Window<'_>) {
    let mut state = ImportPanelState::new();
    state.gallery_count = 2;
    state.watcher_count = 1;
    state.subscription_count = 3;
    state.local_count = 1;

    let mut panel = ffi::new_hydrus_list_panel();
    ffi::hydrus_list_set_columns(panel.pin_mut(), "Importer,Count");
    for (name, count) in [
        ("Gallery", state.gallery_count),
        ("Watchers", state.watcher_count),
        ("Subscriptions", state.subscription_count),
        ("Local files", state.local_count),
    ] {
        ffi::hydrus_list_add_row(panel.pin_mut(), &format!("{name},{count}"));
    }

    ffi::hydrus_add_list_page(window.as_mut(), "Downloads", panel.pin_mut());

    let mut subscriptions = ffi::new_hydrus_list_panel();
    ffi::hydrus_list_set_columns(subscriptions.pin_mut(), "Subscription,Files");
    ffi::hydrus_list_add_row(
        subscriptions.pin_mut(),
        &format!("subscriptions,{}", state.subscription_count),
    );
    ffi::hydrus_add_list_page(window.as_mut(), "Subscriptions", subscriptions.pin_mut());

    let mut gallery = ffi::new_hydrus_list_panel();
    ffi::hydrus_list_set_columns(gallery.pin_mut(), "Gallery,Files");
    ffi::hydrus_list_add_row(
        gallery.pin_mut(),
        &format!("gallery,{}", state.gallery_count),
    );
    ffi::hydrus_add_list_page(window.as_mut(), "Gallery", gallery.pin_mut());

    let mut watchers = ffi::new_hydrus_list_panel();
    ffi::hydrus_list_set_columns(watchers.pin_mut(), "Watcher,Files");
    ffi::hydrus_list_add_row(
        watchers.pin_mut(),
        &format!("watchers,{}", state.watcher_count),
    );
    ffi::hydrus_add_list_page(window.as_mut(), "Watchers", watchers.pin_mut());
}

pub(crate) fn add_parsing_page(mut window: Window<'_>) {
    let mut state = ParsingPanelState::new();
    state.set_formulas(
        ["gallery url", "thread json", "tag html"]
            .into_iter()
            .map(str::to_string),
    );
    state.set_test_text("<div>example</div>");
    state.set_last_result("https://example.com/file.png");

    let mut panel = ffi::new_hydrus_list_panel();
    ffi::hydrus_list_set_columns(panel.pin_mut(), "Formula,Result");
    for (index, name) in state.formula_names.iter().enumerate() {
        let selected = state.selected_formula == Some(index);
        ffi::hydrus_list_add_row(
            panel.pin_mut(),
            &format!(
                "{name},{}",
                if selected {
                    state.last_result.as_deref().unwrap_or("")
                } else {
                    ""
                }
            ),
        );
    }

    ffi::hydrus_add_list_page(window.as_mut(), "Parsing", panel.pin_mut());
}

pub(crate) fn add_options_page(mut window: Window<'_>) {
    let mut state = OptionsPanelState::new();
    state.set_search_text("");

    let mut panel = ffi::new_hydrus_list_panel();
    ffi::hydrus_list_set_columns(panel.pin_mut(), "Options page,Keywords");
    for entry in state.filtered_panels() {
        ffi::hydrus_list_add_row(
            panel.pin_mut(),
            &format!("{},{}", entry.title, entry.keywords.join(";")),
        );
    }

    ffi::hydrus_add_list_page(window.as_mut(), "Options panels", panel.pin_mut());
}

pub(crate) fn add_string_pages(mut window: Window<'_>) {
    let panel = StringPanel::new("Rename tag").with_text("blue eyes");
    let mut string_panel = ffi::new_hydrus_panel();
    ffi::hydrus_panel_add_label(
        string_panel.pin_mut(),
        &format!("{}: {}", panel.title, panel.get_text()),
    );
    ffi::hydrus_panel_add_line_edit(string_panel.pin_mut(), panel.get_text());
    ffi::hydrus_panel_add_button(string_panel.pin_mut(), "OK");
    ffi::hydrus_panel_add_button(string_panel.pin_mut(), "Cancel");
    ffi::hydrus_panel_add_stretch(string_panel.pin_mut());
    ffi::hydrus_add_page_widget(window.as_mut(), "String editor", string_panel.pin_mut());

    let dict = StringToStringDictPanel::from_entries([
        ("User-Agent".to_string(), "Hydrus".to_string()),
        ("Referer".to_string(), "https://example.com".to_string()),
    ]);

    let mut dict_panel = ffi::new_hydrus_list_panel();
    ffi::hydrus_list_set_columns(dict_panel.pin_mut(), "Key,Value");
    for (key, value) in dict.entries() {
        ffi::hydrus_list_add_row(dict_panel.pin_mut(), &format!("{key},{value}"));
    }

    ffi::hydrus_add_list_page(window.as_mut(), "String dictionary", dict_panel.pin_mut());
}

pub(crate) fn add_popup_page(mut window: Window<'_>) {
    let manager = PopupMessageManager::new();

    let mut panel = ffi::new_hydrus_list_panel();
    ffi::hydrus_list_set_columns(panel.pin_mut(), "Popup,Status");
    for message in manager.get_active_messages() {
        ffi::hydrus_list_add_row(
            panel.pin_mut(),
            &format!("{},{:?}", message.message_key(), message.is_done()),
        );
    }

    if manager.is_empty() {
        ffi::hydrus_list_add_row(panel.pin_mut(), "No active popups,idle");
    }

    ffi::hydrus_add_list_page(window.as_mut(), "Popups", panel.pin_mut());
}
