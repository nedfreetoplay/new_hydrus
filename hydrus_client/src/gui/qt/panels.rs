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
        UiCommand::Menus(menus) => ffi::hydrus_set_menus(window.as_mut(), &menus),
        UiCommand::TagSuggestions(json) => {
            ffi::hydrus_set_tag_suggestions(window.as_mut(), &json);
        }
        UiCommand::Autocomplete(state) => {
            ffi::hydrus_show_autocomplete(window.as_mut(), &state);
        }
        UiCommand::BuildPages(pages) => ffi::hydrus_build_pages(window.as_mut(), &pages),
        UiCommand::UpdatePage(state) => ffi::hydrus_update_page(window.as_mut(), &state),
        UiCommand::PageProgress(progress) => {
            ffi::hydrus_update_page_progress(window.as_mut(), &progress);
        }
        UiCommand::Defer {
            action,
            target,
            payload,
            delay_ms,
        } => {
            ffi::hydrus_defer_action(window.as_mut(), action, &target, &payload, delay_ms);
        }
        UiCommand::UpdateDownloader(state) => {
            ffi::hydrus_update_downloader(window.as_mut(), &state);
        }
        UiCommand::UpdateSubscriptions(state) => {
            ffi::hydrus_update_subscriptions(window.as_mut(), &state);
        }
        UiCommand::SubscriptionsDialog(state) => {
            ffi::hydrus_show_subscriptions_dialog(window.as_mut(), &state);
        }
        UiCommand::UpdateSubscriptionsDialog(state) => {
            ffi::hydrus_update_subscriptions_dialog(window.as_mut(), &state);
        }
        UiCommand::SubscriptionEditor(state) => {
            ffi::hydrus_show_subscription_editor(window.as_mut(), &state);
        }
        UiCommand::UpdateDuplicates(state) => {
            ffi::hydrus_update_duplicates(window.as_mut(), &state);
        }
        UiCommand::UpdateImportFolders(page_id, state) => {
            ffi::hydrus_update_import_folders(window.as_mut(), &page_id, &state);
        }
        UiCommand::ImportFoldersDialog(state) => {
            ffi::hydrus_show_import_folders_dialog(window.as_mut(), &state);
        }
        UiCommand::UpdateImportFoldersDialog(state) => {
            ffi::hydrus_update_import_folders_dialog(window.as_mut(), &state);
        }
        UiCommand::UpdateExportFolders(page_id, state) => {
            ffi::hydrus_update_export_folders(window.as_mut(), &page_id, &state);
        }
        UiCommand::ExportFoldersDialog(state) => {
            ffi::hydrus_show_export_folders_dialog(window.as_mut(), &state);
        }
        UiCommand::UpdateExportFoldersDialog(state) => {
            ffi::hydrus_update_export_folders_dialog(window.as_mut(), &state);
        }
        UiCommand::DuplicatesRulesDialog(state) => {
            ffi::hydrus_show_duplicates_rules(window.as_mut(), &state);
        }
        UiCommand::DuplicatesMergeOptionsDialog(state) => {
            ffi::hydrus_show_duplicate_merge_options(window.as_mut(), &state);
        }
        UiCommand::ImportOptionsDialog(state) => {
            ffi::hydrus_show_import_options(window.as_mut(), &state);
        }
        UiCommand::DuplicateCustomActionDialog(state) => {
            ffi::hydrus_show_duplicate_custom_action(window.as_mut(), &state);
        }
        UiCommand::DuplicatesRulePreview(state) => {
            ffi::hydrus_update_auto_resolution_preview(window.as_mut(), &state);
        }
        UiCommand::DuplicateReviewDialog(state) => {
            ffi::hydrus_show_duplicate_review(window.as_mut(), &state);
        }
        UiCommand::UpdateDuplicateReview(state) => {
            ffi::hydrus_update_duplicate_review(window.as_mut(), &state);
        }
        UiCommand::TagRelationsDialog(state) => {
            ffi::hydrus_show_tag_relations(window.as_mut(), &state);
        }
        UiCommand::ImportFolderDialog(state) => {
            ffi::hydrus_show_import_folder(window.as_mut(), &state);
        }
        UiCommand::LocalImportReviewDialog(state) => {
            ffi::hydrus_show_local_import_review(window.as_mut(), &state);
        }
        UiCommand::LocalImportTaggingDialog(state) => {
            ffi::hydrus_show_local_import_tagging(window.as_mut(), &state);
        }
        UiCommand::CloseLocalImportReview => {
            ffi::hydrus_close_local_import_review(window.as_mut());
        }
        UiCommand::RepairFileLocationsDialog(state) => {
            ffi::hydrus_show_repair_file_locations(window.as_mut(), &state);
        }
        UiCommand::CloseRepairFileLocations => {
            ffi::hydrus_close_repair_file_locations(window.as_mut());
        }
        UiCommand::ExportFolderDialog(state) => {
            ffi::hydrus_show_export_folder(window.as_mut(), &state);
        }
        UiCommand::ExportReviewDialog(state) => {
            ffi::hydrus_show_export_review(window.as_mut(), &state);
        }
        UiCommand::NotesDialog(state) => ffi::hydrus_show_notes(window.as_mut(), &state),
        UiCommand::SetSelection(selection) => {
            ffi::hydrus_set_selection(window.as_mut(), &selection);
        }
        UiCommand::Status(status) => ffi::hydrus_set_status(window.as_mut(), &status),
        UiCommand::ShowPageChooser(notebook_id) => {
            ffi::hydrus_show_page_chooser(window.as_mut(), &notebook_id);
        }
        UiCommand::PageManagerDialog(tree) => {
            ffi::hydrus_show_page_manager(window.as_mut(), &tree);
        }
        UiCommand::CommandPalette => {
            ffi::hydrus_show_command_palette(window.as_mut());
        }
        UiCommand::ShortcutSetsDialog(state) => {
            ffi::hydrus_show_shortcut_sets(window.as_mut(), &state);
        }
        UiCommand::ApplyShortcuts(state) => {
            ffi::hydrus_apply_shortcuts(window.as_mut(), &state);
        }
        UiCommand::PromptText {
            action,
            title,
            label,
            initial,
        } => {
            ffi::hydrus_prompt_text(window.as_mut(), action, &title, &label, &initial);
        }
        UiCommand::ConfirmAction {
            action,
            target,
            title,
            text,
        } => {
            ffi::hydrus_confirm_action(window.as_mut(), action, &target, &title, &text);
        }
        UiCommand::PromptOr { page_id, initial } => {
            ffi::hydrus_show_or_dialog(window.as_mut(), &page_id, &initial);
        }
        UiCommand::NotificationsDialog(state) => {
            ffi::hydrus_show_notifications(window.as_mut(), &state);
        }
        UiCommand::ExceptionReport(state) => {
            ffi::hydrus_show_exception(window.as_mut(), &state);
        }
        UiCommand::FileHistoryDialog(state) => {
            ffi::hydrus_show_file_history(window.as_mut(), &state);
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
        UiCommand::FileSeedCacheDialog(state) => {
            ffi::hydrus_show_file_seed_cache(window.as_mut(), &state);
        }
        UiCommand::GallerySeedLogDialog(state) => {
            ffi::hydrus_show_gallery_seed_log(window.as_mut(), &state);
        }
        UiCommand::ImportDownloader(notebook_id) => {
            ffi::hydrus_import_downloader(window.as_mut(), &notebook_id);
        }
        UiCommand::PickFiles => ffi::hydrus_pick_files(window.as_mut()),
        UiCommand::PromptUrl => ffi::hydrus_prompt_url(window.as_mut()),
        UiCommand::OpenViewer(state) => ffi::hydrus_open_viewer(window.as_mut(), &state),
        UiCommand::UpdateViewer(state) => {
            ffi::hydrus_update_viewer(window.as_mut(), &state);
        }
        UiCommand::CloseViewer(viewer_id) => {
            ffi::hydrus_close_viewer(window.as_mut(), &viewer_id);
        }
        UiCommand::TagDialog(state) => ffi::hydrus_show_tag_dialog(window.as_mut(), &state),
        UiCommand::ManageTagsDialog(state) => {
            ffi::hydrus_show_manage_tags(window.as_mut(), &state);
        }
        UiCommand::TimestampsDialog(state) => {
            ffi::hydrus_show_timestamps(window.as_mut(), &state);
        }
        UiCommand::OptionsDialog(state) => ffi::hydrus_show_options(window.as_mut(), &state),
        UiCommand::ServicesDialog(state) => ffi::hydrus_show_services(window.as_mut(), &state),
        UiCommand::UpdateServices(state) => ffi::hydrus_update_services(window.as_mut(), &state),
        UiCommand::TagFilterDialog(state) => ffi::hydrus_show_tag_filter(window.as_mut(), &state),
        UiCommand::ApiPermissionsDialog(state) => {
            ffi::hydrus_show_api_permissions(window.as_mut(), &state)
        }
        UiCommand::UpdateApiPermissions(state) => {
            ffi::hydrus_update_api_permissions(window.as_mut(), &state)
        }
        UiCommand::AccountTypesDialog(state) => {
            ffi::hydrus_show_account_types(window.as_mut(), &state)
        }
        UiCommand::UpdateAccountTypes(state) => {
            ffi::hydrus_update_account_types(window.as_mut(), &state)
        }
        UiCommand::OrphanFilesDialog(state) => {
            ffi::hydrus_show_orphan_files(window.as_mut(), &state)
        }
        UiCommand::UpdateOrphanFiles(state) => {
            ffi::hydrus_update_orphan_files(window.as_mut(), &state)
        }
        UiCommand::DatabaseBackupDialog(state) => {
            ffi::hydrus_show_database_backup(window.as_mut(), &state)
        }
        UiCommand::UpdateDatabaseBackup(state) => {
            ffi::hydrus_update_database_backup(window.as_mut(), &state)
        }
        UiCommand::NetworkDialog(state) => ffi::hydrus_show_network(window.as_mut(), &state),
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
