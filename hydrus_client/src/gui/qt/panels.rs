//! Executes [`UiCommand`]s against the Qt widgets. The only place where the
//! application layer talks to concrete widgets.

use std::pin::Pin;

use super::api::ffi;
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
        UiCommand::Autocomplete(json) => {
            ffi::hydrus_show_autocomplete(window.as_mut(), &json);
        }
        UiCommand::BuildPages(json) => ffi::hydrus_build_pages(window.as_mut(), &json),
        UiCommand::UpdatePage(page_id, json) => {
            ffi::hydrus_update_page(window.as_mut(), &page_id, &json);
        }
        UiCommand::PageProgress(page_id, json) => {
            ffi::hydrus_update_page_progress(window.as_mut(), &page_id, &json);
        }
        UiCommand::Defer {
            action,
            target,
            payload,
            delay_ms,
        } => {
            ffi::hydrus_defer_action(window.as_mut(), &action, &target, &payload, delay_ms);
        }
        UiCommand::UpdateDownloader(page_id, json) => {
            ffi::hydrus_update_downloader(window.as_mut(), &page_id, &json);
        }
        UiCommand::UpdateSubscriptions(page_id, json) => {
            ffi::hydrus_update_subscriptions(window.as_mut(), &page_id, &json);
        }
        UiCommand::SubscriptionsDialog(json) => {
            ffi::hydrus_show_subscriptions_dialog(window.as_mut(), &json);
        }
        UiCommand::UpdateSubscriptionsDialog(json) => {
            ffi::hydrus_update_subscriptions_dialog(window.as_mut(), &json);
        }
        UiCommand::SubscriptionEditor(json) => {
            ffi::hydrus_show_subscription_editor(window.as_mut(), &json);
        }
        UiCommand::UpdateDuplicates(page_id, json) => {
            ffi::hydrus_update_duplicates(window.as_mut(), &page_id, &json);
        }
        UiCommand::UpdateImportFolders(page_id, json) => {
            ffi::hydrus_update_import_folders(window.as_mut(), &page_id, &json);
        }
        UiCommand::ImportFoldersDialog(json) => {
            ffi::hydrus_show_import_folders_dialog(window.as_mut(), &json);
        }
        UiCommand::UpdateImportFoldersDialog(json) => {
            ffi::hydrus_update_import_folders_dialog(window.as_mut(), &json);
        }
        UiCommand::UpdateExportFolders(page_id, json) => {
            ffi::hydrus_update_export_folders(window.as_mut(), &page_id, &json);
        }
        UiCommand::ExportFoldersDialog(json) => {
            ffi::hydrus_show_export_folders_dialog(window.as_mut(), &json);
        }
        UiCommand::UpdateExportFoldersDialog(json) => {
            ffi::hydrus_update_export_folders_dialog(window.as_mut(), &json);
        }
        UiCommand::DuplicatesRulesDialog(json) => {
            ffi::hydrus_show_duplicates_rules(window.as_mut(), &json);
        }
        UiCommand::DuplicatesMergeOptionsDialog(json) => {
            ffi::hydrus_show_duplicate_merge_options(window.as_mut(), &json);
        }
        UiCommand::ImportOptionsDialog(json) => {
            ffi::hydrus_show_import_options(window.as_mut(), &json);
        }
        UiCommand::DuplicateCustomActionDialog(json) => {
            ffi::hydrus_show_duplicate_custom_action(window.as_mut(), &json);
        }
        UiCommand::DuplicatesRulePreview(json) => {
            ffi::hydrus_update_auto_resolution_preview(window.as_mut(), &json);
        }
        UiCommand::DuplicateReviewDialog(json) => {
            ffi::hydrus_show_duplicate_review(window.as_mut(), &json);
        }
        UiCommand::UpdateDuplicateReview(json) => {
            ffi::hydrus_update_duplicate_review(window.as_mut(), &json);
        }
        UiCommand::TagRelationsDialog(json) => {
            ffi::hydrus_show_tag_relations(window.as_mut(), &json);
        }
        UiCommand::ImportFolderDialog(json) => {
            ffi::hydrus_show_import_folder(window.as_mut(), &json);
        }
        UiCommand::LocalImportReviewDialog(json) => {
            ffi::hydrus_show_local_import_review(window.as_mut(), &json);
        }
        UiCommand::LocalImportTaggingDialog(json) => {
            ffi::hydrus_show_local_import_tagging(window.as_mut(), &json);
        }
        UiCommand::CloseLocalImportReview => {
            ffi::hydrus_close_local_import_review(window.as_mut());
        }
        UiCommand::RepairFileLocationsDialog(json) => {
            ffi::hydrus_show_repair_file_locations(window.as_mut(), &json);
        }
        UiCommand::CloseRepairFileLocations => {
            ffi::hydrus_close_repair_file_locations(window.as_mut());
        }
        UiCommand::ExportFolderDialog(json) => {
            ffi::hydrus_show_export_folder(window.as_mut(), &json);
        }
        UiCommand::ExportReviewDialog(json) => {
            ffi::hydrus_show_export_review(window.as_mut(), &json);
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
        UiCommand::Status(status) => ffi::hydrus_set_status(window.as_mut(), &status),
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
        UiCommand::PromptOr { page_id, initial } => {
            ffi::hydrus_show_or_dialog(window.as_mut(), &page_id, &initial);
        }
        UiCommand::NotificationsDialog(json) => {
            ffi::hydrus_show_notifications(window.as_mut(), &json);
        }
        UiCommand::ViewingStatsDialog(state) => {
            ffi::hydrus_show_viewing_stats(window.as_mut(), &state);
        }
        UiCommand::AboutDialog(state) => {
            ffi::hydrus_show_about(window.as_mut(), &state);
        }
        UiCommand::ExportDownloader(page_id) => {
            ffi::hydrus_export_downloader(window.as_mut(), &page_id);
        }
        UiCommand::FileSeedCacheDialog(json) => {
            ffi::hydrus_show_file_seed_cache(window.as_mut(), &json);
        }
        UiCommand::GallerySeedLogDialog(json) => {
            ffi::hydrus_show_gallery_seed_log(window.as_mut(), &json);
        }
        UiCommand::ImportDownloader(notebook_id) => {
            ffi::hydrus_import_downloader(window.as_mut(), &notebook_id);
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
        UiCommand::ManageTagsDialog(json) => {
            ffi::hydrus_show_manage_tags(window.as_mut(), &json);
        }
        UiCommand::TimestampsDialog(json) => {
            ffi::hydrus_show_timestamps(window.as_mut(), &json);
        }
        UiCommand::OptionsDialog(json) => ffi::hydrus_show_options(window.as_mut(), &json),
        UiCommand::ServicesDialog(json) => ffi::hydrus_show_services(window.as_mut(), &json),
        UiCommand::UpdateServices(json) => ffi::hydrus_update_services(window.as_mut(), &json),
        UiCommand::TagFilterDialog(json) => ffi::hydrus_show_tag_filter(window.as_mut(), &json),
        UiCommand::ApiPermissionsDialog(json) => {
            ffi::hydrus_show_api_permissions(window.as_mut(), &json)
        }
        UiCommand::UpdateApiPermissions(json) => {
            ffi::hydrus_update_api_permissions(window.as_mut(), &json)
        }
        UiCommand::AccountTypesDialog(json) => {
            ffi::hydrus_show_account_types(window.as_mut(), &json)
        }
        UiCommand::UpdateAccountTypes(json) => {
            ffi::hydrus_update_account_types(window.as_mut(), &json)
        }
        UiCommand::OrphanFilesDialog(json) => ffi::hydrus_show_orphan_files(window.as_mut(), &json),
        UiCommand::UpdateOrphanFiles(json) => {
            ffi::hydrus_update_orphan_files(window.as_mut(), &json)
        }
        UiCommand::DatabaseBackupDialog(json) => {
            ffi::hydrus_show_database_backup(window.as_mut(), &json)
        }
        UiCommand::UpdateDatabaseBackup(json) => {
            ffi::hydrus_update_database_backup(window.as_mut(), &json)
        }
        UiCommand::NetworkDialog(json) => ffi::hydrus_show_network(window.as_mut(), &json),
        UiCommand::NotifyMessage(message) => {
            ffi::hydrus_notify_message(window.as_mut(), &message);
        }
        UiCommand::SetClipboard(text) => {
            ffi::hydrus_set_clipboard(window.as_mut(), &text);
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
