#pragma once

// WS-15.1.4: free hydrus_*/qapp_* glue declarations, verbatim from bridge.hpp.

#include "rust/cxx.h"

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

// Implemented in Rust (see bridge.rs / gui::app).
void app_action(
    HydrusMainWindow &window,
    UiAction action,
    rust::Str target,
    rust::Str payload) noexcept;

void app_tick(HydrusMainWindow &window) noexcept;

// Typed reverse seams (WS-15.2.2a): tab drag & drop and manage-pages close
// actions. Implemented in Rust, same core as the notify actions.
void app_move_page(
    HydrusMainWindow &window,
    rust::Str source,
    rust::Str target,
    rust::Str position) noexcept;

void app_close_pages(HydrusMainWindow &window, rust::Vec<rust::String> ids) noexcept;

// Typed reverse seams (WS-15.2.2b): thumbnail grid selection, focus, viewer
// open and page-to-page drag. Implemented in Rust, same cores as the notify
// actions.
void app_set_selection(
    HydrusMainWindow &window,
    rust::Str page_id,
    rust::Vec<rust::String> hashes) noexcept;

void app_focus_hash(HydrusMainWindow &window, rust::Str page_id, rust::Str hash) noexcept;

void app_open_viewer(HydrusMainWindow &window, rust::Str page_id, rust::Str hash) noexcept;

void app_add_hashes(
    HydrusMainWindow &window,
    rust::Str page_id,
    rust::Vec<rust::String> hashes) noexcept;

// Typed reverse seams (WS-15.2.2b): metadata panel tag/rating/notes actions.
// Implemented in Rust, same cores as tag_add/tag_remove, rating_set and
// notes_manage.
void app_tag_edit(
    HydrusMainWindow &window,
    rust::Str hash,
    rust::Str service,
    rust::Str tag,
    bool add) noexcept;

void app_set_rating(
    HydrusMainWindow &window,
    rust::Str hash,
    rust::Str service,
    rust::Str action,
    double value) noexcept;

void app_manage_notes(HydrusMainWindow &window, rust::Str hash) noexcept;

// Typed reverse seams (WS-15.2.3a): page/tab/session/history events.
void app_page_selected(HydrusMainWindow &window, rust::Str page_id) noexcept;
void app_close_page(HydrusMainWindow &window, rust::Str page_id) noexcept;
void app_close_other_pages(HydrusMainWindow &window, rust::Str page_id) noexcept;
void app_rename_page(HydrusMainWindow &window, rust::Str page_id, rust::Str name) noexcept;
void app_refresh_page(HydrusMainWindow &window, rust::Str page_id) noexcept;
void app_page_manager_activate(HydrusMainWindow &window, rust::Str page_id) noexcept;
void app_page_sort(HydrusMainWindow &window, rust::Str page_id, rust::Str sort, bool ascending) noexcept;
void app_history_step(HydrusMainWindow &window, int step) noexcept;

// Typed reverse seams (WS-15.2.4c): services + network. Implemented in Rust.
void app_service_save(
    HydrusMainWindow &window,
    rust::Str original,
    rust::Str name,
    int type_int,
    bool remote,
    rust::Str host,
    int port,
    rust::Str access_key,
    bool has_update,
    int64_t update_period,
    bool has_nullify,
    int64_t nullify_period,
    int64_t max_storage,
    bool log_uploader_ips) noexcept;
void app_service_delete(HydrusMainWindow &window, rust::Str key) noexcept;
void app_login_save(
    HydrusMainWindow &window,
    rust::Str domain,
    rust::Str username,
    rust::Str password,
    rust::Str access_key,
    int64_t expires) noexcept;
void app_login_remove(HydrusMainWindow &window, rust::Str domain) noexcept;
void app_admin_connect(HydrusMainWindow &window, rust::Str address, rust::Str access_key) noexcept;
void app_admin_select_service(
    HydrusMainWindow &window,
    rust::Str service_key,
    rust::Str access_key) noexcept;
void app_admin_service_action(HydrusMainWindow &window, rust::Str action, int64_t period) noexcept;
void app_admin_add_service(
    HydrusMainWindow &window,
    rust::Str name,
    int type_int,
    int port) noexcept;
void app_admin_edit_service(
    HydrusMainWindow &window,
    rust::Str service_key,
    rust::Str name,
    int port) noexcept;
void app_admin_account_action(
    HydrusMainWindow &window,
    rust::Str action,
    rust::Str subject_account_key,
    rust::Str text,
    int64_t expires) noexcept;
void app_network_save(
    HydrusMainWindow &window,
    bool api_enabled,
    int network_timeout,
    int max_network_jobs,
    int max_network_jobs_per_domain,
    int api_port,
    rust::Str api_access_key,
    rust::Str http_proxy,
    rust::Str https_proxy,
    rust::Str no_proxy) noexcept;
void app_network_header(
    HydrusMainWindow &window,
    rust::Str name,
    rust::Str value,
    bool remove) noexcept;
void app_network_url_class_save(
    HydrusMainWindow &window,
    rust::Str original,
    rust::Str name,
    int url_type,
    rust::Str preferred_scheme,
    rust::Str netloc,
    rust::Str path,
    rust::Str parameters,
    rust::Str example_url,
    bool match_subdomains,
    bool keep_matched_subdomains,
    bool can_produce_multiple_files,
    bool should_be_associated_with_files,
    bool is_add) noexcept;
void app_network_url_class_remove(HydrusMainWindow &window, rust::Str name) noexcept;
void app_network_pool_set(HydrusMainWindow &window, rust::Str domain, int limit) noexcept;
void app_network_pool_remove(HydrusMainWindow &window, rust::Str domain) noexcept;
void app_network_rule_set(
    HydrusMainWindow &window,
    rust::Str context_type,
    rust::Str context,
    rust::Str bandwidth_type,
    int64_t time_delta,
    int64_t max_allowed) noexcept;
void app_network_rule_remove(
    HydrusMainWindow &window,
    rust::Str context_type,
    rust::Str context,
    rust::Str bandwidth_type) noexcept;

// Typed reverse seams (WS-15.2.4c): tags, duplicates, subscriptions.
void app_tag_filter_test(
    HydrusMainWindow &window,
    rust::Vec<rust::String> slices,
    rust::Vec<rust::String> rules,
    rust::Str tag) noexcept;
void app_tag_filter_save(
    HydrusMainWindow &window,
    rust::Str service_key,
    rust::Vec<rust::String> slices,
    rust::Vec<rust::String> rules) noexcept;
void app_purge_tags(
    HydrusMainWindow &window,
    rust::Str service_key,
    rust::Vec<rust::String> tags,
    rust::Str reason) noexcept;
void app_tag_relation_add(
    HydrusMainWindow &window,
    rust::Str relation,
    rust::Str service,
    rust::Str first,
    rust::Str second) noexcept;
void app_tag_relation_remove(
    HydrusMainWindow &window,
    rust::Str relation,
    rust::Str service,
    rust::Str first,
    rust::Str second) noexcept;
void app_duplicates_batch_size(
    HydrusMainWindow &window,
    rust::Str page_id,
    int64_t value) noexcept;
void app_duplicates_search_context(
    HydrusMainWindow &window,
    rust::Str page_id,
    rust::Str query_1,
    rust::Str location_1,
    rust::Str query_2,
    rust::Str location_2,
    rust::Str dupe_search_type,
    rust::Str pixel_dupes,
    int64_t max_distance,
    rust::Str sort_type,
    bool sort_ascending) noexcept;
void app_duplicates_group_action(
    HydrusMainWindow &window,
    rust::Str page_id,
    rust::Str kind,
    rust::Str target) noexcept;
void app_subscription_add(
    HydrusMainWindow &window,
    rust::Str page_id,
    rust::Str name,
    rust::Str url,
    rust::Str query,
    rust::Str site,
    rust::Str filter,
    int64_t period) noexcept;
void app_subscription_edit_save(
    HydrusMainWindow &window,
    rust::Str page_id,
    rust::Str original,
    rust::Str name,
    rust::Str url,
    rust::Str query,
    rust::Str site,
    rust::Str filter,
    int64_t period,
    bool paused) noexcept;
// `container_json` holds the arbitrary nested import options dict (same R1
// exception as `SubscriptionDialogRow.import_options_json`).
void app_subscription_import_options_save(
    HydrusMainWindow &window,
    rust::Str name,
    rust::Str container_json) noexcept;
void app_file_seed_action(
    HydrusMainWindow &window,
    rust::Str page,
    rust::Str action,
    rust::Vec<rust::String> urls) noexcept;
void app_gallery_seed_action(
    HydrusMainWindow &window,
    rust::Str page,
    rust::Str action,
    rust::Vec<rust::String> urls) noexcept;
void app_downloader_start(
    HydrusMainWindow &window,
    rust::Str page_id,
    rust::Str input,
    rust::Str url,
    rust::Str query,
    rust::Str site,
    rust::Str formula,
    rust::Str subject,
    bool auto_watch,
    int64_t period,
    bool no_limit,
    int64_t file_limit,
    int64_t present) noexcept;
void app_downloader_jobs_set(
    HydrusMainWindow &window,
    rust::Str page_id,
    rust::Vec<rust::String> urls,
    bool remove_missing) noexcept;
void app_duplicates_rule_save(
    HydrusMainWindow &window,
    rust::Str name,
    bool paused,
    rust::Str operation_mode,
    int64_t max_pending_pairs,
    rust::Str search_query,
    rust::Str search_mode,
    int64_t max_distance,
    rust::Vec<RuleComparatorRow> selectors,
    rust::Str action,
    bool delete_a,
    bool delete_b,
    rust::Str original_name) noexcept;
void app_duplicates_rule_preview(
    HydrusMainWindow &window,
    rust::Str name,
    bool paused,
    rust::Str operation_mode,
    int64_t max_pending_pairs,
    rust::Str search_query,
    rust::Str search_mode,
    int64_t max_distance,
    rust::Vec<RuleComparatorRow> selectors,
    rust::Str action,
    bool delete_a,
    bool delete_b) noexcept;

// Implemented in Rust: the Python VetoException text for an invalid export
// filename phrase, or an empty string when the phrase parses.
rust::String hydrus_validate_export_phrase(rust::Str phrase) noexcept;

// Implemented in Rust: resolves the vendored static/ asset directory.
rust::String hydrus_static_dir() noexcept;

// Implemented in Rust: maps an action string to UiAction (data-driven seams).
UiAction action_from_str(rust::Str action) noexcept;


