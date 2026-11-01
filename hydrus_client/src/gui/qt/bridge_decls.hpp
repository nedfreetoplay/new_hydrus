#pragma once

// WS-15.3.2c: forward decls + hydrus_* show/update entry points, split from
// bridge_api.hpp (R3). No semantics changed; bridge_api.hpp includes this.

#include "rust/cxx.h"

#include <QJsonObject>
#include <QString>

class QApplication;
class QSplashScreen;
class HydrusMainWindow;
enum class UiAction : std::int32_t;

// WS-15.2.1: typed seams; the structs are forward-declared here and owned by
// Rust (api.rs, generated <hydrus_client/src/gui/qt/api.cxx.h>).
struct StatusBar;
struct AboutInfo;
struct ViewingStats;
struct MenuBar;
struct Pages;
struct PageProgress;
struct PageTree;
// WS-15.2.2b: media/selection/viewer seams.
struct MediaItem;
struct QueryPageState;
struct SelectionState;
struct MetadataState;
struct ViewerState;
struct DownloaderPageState;
struct SubscriptionsPageState;
struct SubscriptionsDialogState;
struct SubscriptionEditorState;
struct SeedCacheState;
struct ImportOptionsDialogState;
struct OptionsDialogState;
struct ServicesDialogState;
struct NetworkDialogState;
struct AccountTypesDialogState;
struct ApiPermissionsDialogState;
struct TagFilterDialogState;
struct TagRelationsDialogState;
struct ManageTagsDialogState;
struct TimestampsDialogState;
struct NotificationsDialogState;
struct ExportReviewDialogState;
struct RepairFileLocationsDialogState;
struct OrphanFilesDialogState;
struct DatabaseBackupDialogState;
struct LocalImportReviewDialogState;
struct LocalImportTaggingDialogState;
struct DuplicatesRulesDialogState;
struct DuplicatesMergeOptionsDialogState;
struct DuplicateCustomActionDialogState;
struct DuplicatesRulePreviewState;
struct DuplicateReviewDialogState;
struct DirectoryTaggingRow;
struct QuickNamespaceRow;
struct FilenameTaggingEntry;
struct ShortcutSetsDialogState;

// Flattens the tagging editor's `filename_tagging` object (WS-15.2.4c).
rust::Vec<FilenameTaggingEntry> filename_tagging_entries(const QJsonObject &tagging);

std::unique_ptr<QApplication> new_application();
int exec_application(QApplication &application);
void quit_application(QApplication &application);
void qapp_process_events();

std::unique_ptr<QSplashScreen> new_hydrus_splash();
void hydrus_splash_show(QSplashScreen &splash);
void hydrus_splash_message(QSplashScreen &splash, rust::Str message);
void hydrus_splash_finish(QSplashScreen &splash);
void hydrus_splash_finish_later(QSplashScreen &splash, int delay_ms);
void hydrus_splash_screenshot(QSplashScreen &splash, rust::Str path, int delay_ms);

std::unique_ptr<HydrusMainWindow> new_hydrus_main_window();
void build_hydrus_shell(HydrusMainWindow &window);
void show_hydrus_main_window(HydrusMainWindow &window);

void hydrus_set_title(HydrusMainWindow &window, rust::Str title);
void hydrus_resize_window(HydrusMainWindow &window, int width, int height);
void hydrus_set_menus(HydrusMainWindow &window, const MenuBar &menus);
void hydrus_set_tag_suggestions(HydrusMainWindow &window, rust::Str json);
void hydrus_show_autocomplete(HydrusMainWindow &window, const AutocompleteState &state);
void hydrus_set_status(HydrusMainWindow &window, const StatusBar &status);
void hydrus_build_pages(HydrusMainWindow &window, const Pages &pages);
void hydrus_update_page(HydrusMainWindow &window, const QueryPageState &page);
void hydrus_update_page_progress(HydrusMainWindow &window, const PageProgress &progress);
void hydrus_update_downloader(HydrusMainWindow &window, const DownloaderPageState &state);
void hydrus_update_subscriptions(HydrusMainWindow &window, const SubscriptionsPageState &state);
void hydrus_show_subscriptions_dialog(HydrusMainWindow &window, const SubscriptionsDialogState &state);
void hydrus_update_subscriptions_dialog(HydrusMainWindow &window, const SubscriptionsDialogState &state);
void hydrus_show_subscription_editor(HydrusMainWindow &window, const SubscriptionEditorState &state);
void hydrus_update_duplicates(HydrusMainWindow &window, const DuplicatesPageState &state);
void hydrus_update_import_folders(HydrusMainWindow &window, rust::Str page_id, const FolderPageState &state);
void hydrus_show_import_folders_dialog(HydrusMainWindow &window, const FolderPageState &state);
void hydrus_update_import_folders_dialog(HydrusMainWindow &window, const FolderPageState &state);
void hydrus_update_export_folders(HydrusMainWindow &window, rust::Str page_id, const FolderPageState &state);
void hydrus_show_export_folders_dialog(HydrusMainWindow &window, const FolderPageState &state);
void hydrus_update_export_folders_dialog(HydrusMainWindow &window, const FolderPageState &state);
void hydrus_show_duplicates_rules(HydrusMainWindow &window, const DuplicatesRulesDialogState &state);
void hydrus_show_duplicate_merge_options(HydrusMainWindow &window, const DuplicatesMergeOptionsDialogState &state);
void hydrus_show_duplicate_custom_action(HydrusMainWindow &window, const DuplicateCustomActionDialogState &state);
void hydrus_show_import_options(HydrusMainWindow &window, const ImportOptionsDialogState &state);
void hydrus_show_file_seed_cache(HydrusMainWindow &window, const SeedCacheState &state);
void hydrus_show_gallery_seed_log(HydrusMainWindow &window, const SeedCacheState &state);
void hydrus_update_auto_resolution_preview(HydrusMainWindow &window, const DuplicatesRulePreviewState &state);
void hydrus_show_duplicate_review(HydrusMainWindow &window, const DuplicateReviewDialogState &state);
void hydrus_update_duplicate_review(HydrusMainWindow &window, const DuplicateReviewDialogState &state);
void hydrus_show_tag_relations(HydrusMainWindow &window, const TagRelationsDialogState &state);
void hydrus_show_import_folder(HydrusMainWindow &window, const ImportFolderEditorState &state);
void hydrus_show_local_import_review(HydrusMainWindow &window, const LocalImportReviewDialogState &state);
void hydrus_show_local_import_tagging(HydrusMainWindow &window, const LocalImportTaggingDialogState &state);
void hydrus_close_local_import_review(HydrusMainWindow &window);
void hydrus_show_repair_file_locations(HydrusMainWindow &window, const RepairFileLocationsDialogState &state);
void hydrus_close_repair_file_locations(HydrusMainWindow &window);
void hydrus_show_export_folder(HydrusMainWindow &window, const ExportFolderEditorState &state);
void hydrus_show_export_review(HydrusMainWindow &window, const ExportReviewDialogState &state);
void hydrus_show_notes(HydrusMainWindow &window, const NotesDialogState &state);
void hydrus_set_selection(HydrusMainWindow &window, const SelectionState &selection);
void hydrus_show_page_chooser(HydrusMainWindow &window, rust::Str notebook_id);
void hydrus_show_page_manager(HydrusMainWindow &window, const PageTree &tree);
void hydrus_show_command_palette(HydrusMainWindow &window);
void hydrus_show_shortcut_sets(HydrusMainWindow &window, const ShortcutSetsDialogState &state);
void hydrus_apply_shortcuts(HydrusMainWindow &window, const ShortcutSetsDialogState &state);
void hydrus_prompt_text(
    HydrusMainWindow &window,
    UiAction action,
    rust::Str title,
    rust::Str label,
    rust::Str initial);
void hydrus_confirm_action(
    HydrusMainWindow &window,
    UiAction action,
    rust::Str target,
    rust::Str title,
    rust::Str text);
void hydrus_show_or_dialog(
    HydrusMainWindow &window,
    rust::Str page_id,
    rust::Str initial);
void hydrus_pick_files(HydrusMainWindow &window);
void hydrus_prompt_url(HydrusMainWindow &window);
void hydrus_open_viewer(HydrusMainWindow &window, const ViewerState &state);
void hydrus_update_viewer(HydrusMainWindow &window, const ViewerState &state);
void hydrus_close_viewer(HydrusMainWindow &window, rust::Str viewer_id);
void hydrus_show_tag_dialog(HydrusMainWindow &window, const TagDialogState &state);
void hydrus_show_notifications(HydrusMainWindow &window, const NotificationsDialogState &state);
void hydrus_show_viewing_stats(HydrusMainWindow &window, const ViewingStats &state);
void hydrus_show_about(HydrusMainWindow &window, const AboutInfo &state);
void hydrus_export_downloader(HydrusMainWindow &window, rust::Str page_id);
void hydrus_import_downloader(HydrusMainWindow &window, rust::Str target);
void hydrus_show_manage_tags(HydrusMainWindow &window, const ManageTagsDialogState &state);
void hydrus_show_timestamps(HydrusMainWindow &window, const TimestampsDialogState &state);
void hydrus_show_options(HydrusMainWindow &window, const OptionsDialogState &state);
void hydrus_show_services(HydrusMainWindow &window, const ServicesDialogState &state);
void hydrus_update_services(HydrusMainWindow &window, const ServicesDialogState &state);
void hydrus_show_tag_filter(HydrusMainWindow &window, const TagFilterDialogState &state);
void hydrus_show_api_permissions(HydrusMainWindow &window, const ApiPermissionsDialogState &state);
void hydrus_update_api_permissions(HydrusMainWindow &window, const ApiPermissionsDialogState &state);
void hydrus_show_account_types(HydrusMainWindow &window, const AccountTypesDialogState &state);
void hydrus_update_account_types(HydrusMainWindow &window, const AccountTypesDialogState &state);
void hydrus_show_orphan_files(HydrusMainWindow &window, const OrphanFilesDialogState &state);
void hydrus_update_orphan_files(HydrusMainWindow &window, const OrphanFilesDialogState &state);
void hydrus_show_database_backup(HydrusMainWindow &window, const DatabaseBackupDialogState &state);
void hydrus_update_database_backup(HydrusMainWindow &window, const DatabaseBackupDialogState &state);
void hydrus_show_network(HydrusMainWindow &window, const NetworkDialogState &state);
void hydrus_update_network(HydrusMainWindow &window, const NetworkDialogState &state);
void hydrus_start_tick(HydrusMainWindow &window);
void hydrus_defer_action(
    HydrusMainWindow &window,
    UiAction action,
    rust::Str target,
    rust::Str payload,
    int delay_ms);
void hydrus_show_message(HydrusMainWindow &window, rust::Str title, rust::Str text);
void hydrus_notify_message(HydrusMainWindow &window, rust::Str message);
void hydrus_set_clipboard(HydrusMainWindow &window, rust::Str text);
void hydrus_open_path(rust::Str path);
void hydrus_open_url(rust::Str url);
void hydrus_schedule_screenshot(HydrusMainWindow &window, rust::Str path);
void qapp_set_application_name(rust::Str name);
void qapp_set_application_display_name(rust::Str name);
void qapp_set_organization_name(rust::Str name);
void qapp_set_style(rust::Str name);
void qapp_set_stylesheet(rust::Str stylesheet);
void qapp_quit();
