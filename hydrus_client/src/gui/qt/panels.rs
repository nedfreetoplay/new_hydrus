//! Executes [`UiCommand`]s against the Qt widgets. The only place where the
//! application layer talks to concrete widgets.

use std::pin::Pin;

use super::bridge::ffi;
use crate::gui::app::UiCommand;

type Window<'a> = Pin<&'a mut ffi::HydrusMainWindow>;

pub(crate) fn execute_commands(mut window: Window<'_>, commands: Vec<UiCommand>) {
    for command in commands {
        execute(window.as_mut(), command);
    }
}

pub(crate) fn execute(mut window: Window<'_>, command: UiCommand) {
    match command {
        UiCommand::Menus(json) => ffi::hydrus_set_menus(window.as_mut(), &json),
        UiCommand::TagSuggestions(json) => {
            ffi::hydrus_set_tag_suggestions(window.as_mut(), &json);
        }
        UiCommand::BuildPages(json) => ffi::hydrus_build_pages(window.as_mut(), &json),
        UiCommand::UpdatePage(page_id, json) => {
            ffi::hydrus_update_page(window.as_mut(), &page_id, &json);
        }
        UiCommand::UpdateDownloader(page_id, json) => {
            ffi::hydrus_update_downloader(window.as_mut(), &page_id, &json);
        }
        UiCommand::UpdateSubscriptions(page_id, json) => {
            ffi::hydrus_update_subscriptions(window.as_mut(), &page_id, &json);
        }
        UiCommand::UpdateDuplicates(page_id, json) => {
            ffi::hydrus_update_duplicates(window.as_mut(), &page_id, &json);
        }
        UiCommand::UpdateImportFolders(page_id, json) => {
            ffi::hydrus_update_import_folders(window.as_mut(), &page_id, &json);
        }
        UiCommand::DuplicatesRulesDialog(json) => {
            ffi::hydrus_show_duplicates_rules(window.as_mut(), &json);
        }
        UiCommand::TagRelationsDialog(json) => {
            ffi::hydrus_show_tag_relations(window.as_mut(), &json);
        }
        UiCommand::ImportFolderDialog(json) => {
            ffi::hydrus_show_import_folder(window.as_mut(), &json);
        }
        UiCommand::NotesDialog(json) => ffi::hydrus_show_notes(window.as_mut(), &json),
        UiCommand::SetSelection {
            page_id,
            hash,
            tags_json,
            preview_path,
            metadata_json,
        } => {
            ffi::hydrus_set_selection(
                window.as_mut(),
                &page_id,
                &hash,
                &tags_json,
                &preview_path,
                &metadata_json,
            );
        }
        UiCommand::Status(json) => ffi::hydrus_set_status(window.as_mut(), &json),
        UiCommand::ShowPageChooser(notebook_id) => {
            ffi::hydrus_show_page_chooser(window.as_mut(), &notebook_id);
        }
        UiCommand::PageManagerDialog(json) => {
            ffi::hydrus_show_page_manager(window.as_mut(), &json);
        }
        UiCommand::CommandPalette => {
            ffi::hydrus_show_command_palette(window.as_mut());
        }
        UiCommand::PromptText {
            action,
            title,
            label,
            initial,
        } => {
            ffi::hydrus_prompt_text(window.as_mut(), &action, &title, &label, &initial);
        }
        UiCommand::ConfirmAction {
            action,
            target,
            title,
            text,
        } => {
            ffi::hydrus_confirm_action(window.as_mut(), &action, &target, &title, &text);
        }
        UiCommand::PickFiles => ffi::hydrus_pick_files(window.as_mut()),
        UiCommand::PromptUrl => ffi::hydrus_prompt_url(window.as_mut()),
        UiCommand::OpenViewer(json) => ffi::hydrus_open_viewer(window.as_mut(), &json),
        UiCommand::UpdateViewer(viewer_id, json) => {
            ffi::hydrus_update_viewer(window.as_mut(), &viewer_id, &json);
        }
        UiCommand::CloseViewer(viewer_id) => {
            ffi::hydrus_close_viewer(window.as_mut(), &viewer_id);
        }
        UiCommand::TagDialog(json) => ffi::hydrus_show_tag_dialog(window.as_mut(), &json),
        UiCommand::OptionsDialog(json) => ffi::hydrus_show_options(window.as_mut(), &json),
        UiCommand::ServicesDialog(json) => ffi::hydrus_show_services(window.as_mut(), &json),
        UiCommand::NetworkDialog(json) => ffi::hydrus_show_network(window.as_mut(), &json),
        UiCommand::NotifyMessage(message) => {
            ffi::hydrus_notify_message(window.as_mut(), &message);
        }
        UiCommand::Message(title, text) => {
            ffi::hydrus_show_message(window.as_mut(), &title, &text);
        }
        UiCommand::OpenPath(path) => ffi::hydrus_open_path(&path),
        UiCommand::OpenUrl(url) => ffi::hydrus_open_url(&url),
        UiCommand::Screenshot(path) => ffi::hydrus_schedule_screenshot(window.as_mut(), &path),
        UiCommand::Quit => ffi::qapp_quit(),
    }
}
