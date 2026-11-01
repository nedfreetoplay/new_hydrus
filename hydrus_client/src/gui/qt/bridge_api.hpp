#pragma once

// WS-15.1.4: free hydrus_*/qapp_* glue declarations, verbatim from bridge.hpp.
// WS-15.3.2c: show/update entry points + forward decls live in bridge_decls.hpp (R3).
#include "bridge_decls.hpp"

#include "rust/cxx.h"

// Implemented in Rust (see bridge.rs / gui::app).
void app_action(
    HydrusMainWindow &window,
    UiAction action,
    rust::Str target,
    rust::Str payload) noexcept;

void app_tick(HydrusMainWindow &window) noexcept;

// Reverse seams (WS-15.2.2a): tab drag & drop, manage-pages close. In Rust.
void app_move_page(
    HydrusMainWindow &window,
    rust::Str source,
    rust::Str target,
    rust::Str position) noexcept;

void app_close_pages(HydrusMainWindow &window, rust::Vec<rust::String> ids) noexcept;

// Reverse seams (WS-15.2.2b): selection, focus, viewer, page-to-page drag.
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

// Reverse seams (WS-15.2.2b): tag/rating/notes actions. In Rust.
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
void app_shortcut_set_activate(HydrusMainWindow &window, rust::Str name) noexcept;
rust::String hydrus_mouse_gesture_action(rust::Str context, rust::Str gesture) noexcept;
void app_shortcut_mouse_save(
    HydrusMainWindow &window,
    rust::Str set_name,
    rust::Str context,
    rust::Str action,
    rust::Vec<rust::String> gestures) noexcept;
void app_shortcut_context_open(HydrusMainWindow &window, rust::Str context) noexcept;

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
// `container_json`: arbitrary nested import options dict (R1 exception).
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
// Reverse seams (WS-15.2.4c, партия 4): admin + duplicate dialogs.
void app_account_type_save(
    HydrusMainWindow &window,
    rust::Str key,
    rust::Str title,
    rust::Vec<int32_t> content_types,
    rust::Vec<int32_t> actions,
    int64_t auto_create_accounts,
    int64_t auto_create_time_delta) noexcept;
void app_account_type_delete(
    HydrusMainWindow &window,
    rust::Str key,
    rust::Str replacement) noexcept;
void app_api_permissions_save(
    HydrusMainWindow &window,
    rust::Str name,
    rust::Str access_key,
    rust::Str original,
    bool permits_everything,
    rust::Vec<int32_t> basic,
    rust::Vec<rust::String> slices,
    rust::Vec<rust::String> rules) noexcept;
void app_api_permissions_delete(
    HydrusMainWindow &window,
    rust::Str access_key) noexcept;
void app_duplicates_merge_options_save(
    HydrusMainWindow &window,
    rust::Str kind,
    rust::Str archive,
    rust::Str modified,
    rust::Str urls,
    rust::Str notes,
    rust::Vec<rust::String> tag_services,
    rust::Vec<rust::String> tag_actions,
    rust::Vec<rust::String> rating_services,
    rust::Vec<rust::String> rating_actions) noexcept;
void app_duplicates_custom_apply(
    HydrusMainWindow &window,
    rust::Str page_id,
    rust::Str action,
    rust::Str better_side,
    rust::Str delete_mode,
    rust::Str better_archive,
    rust::Str better_modified,
    rust::Str better_urls,
    rust::Str better_notes,
    rust::Vec<rust::String> better_tag_services,
    rust::Vec<rust::String> better_tag_actions,
    rust::Vec<rust::String> better_rating_services,
    rust::Vec<rust::String> better_rating_actions,
    rust::Str same_archive,
    rust::Str same_modified,
    rust::Str same_urls,
    rust::Str same_notes,
    rust::Vec<rust::String> same_tag_services,
    rust::Vec<rust::String> same_tag_actions,
    rust::Vec<rust::String> same_rating_services,
    rust::Vec<rust::String> same_rating_actions) noexcept;
void app_duplicates_review_apply(
    HydrusMainWindow &window,
    rust::Vec<int64_t> indices,
    bool approve,
    bool all) noexcept;
void app_duplicates_show_pair(
    HydrusMainWindow &window,
    rust::Vec<rust::String> hashes) noexcept;
void app_orphan_files_delete(
    HydrusMainWindow &window,
    rust::Vec<rust::String> paths) noexcept;
void app_orphan_files_move(
    HydrusMainWindow &window,
    rust::Vec<rust::String> paths,
    rust::Str target) noexcept;
// Reverse seams (WS-15.2.4c): export/import folders, local import, options.
void app_export_folder_run(
    HydrusMainWindow &window,
    rust::Str page_id,
    rust::Str name,
    rust::Vec<rust::String> hashes) noexcept;
void app_export_folder_save(
    HydrusMainWindow &window,
    rust::Str page_id,
    rust::Str original,
    rust::Str name,
    rust::Str path,
    rust::Str query,
    rust::Str phrase,
    rust::Str export_type,
    rust::Str sidecar_format,
    bool overwrite_next,
    bool overwrite_always,
    bool delete_from_client,
    bool export_symlinks,
    bool run_regularly,
    bool show_working_popup,
    bool run_now_flag,
    int64_t period) noexcept;
void app_export_folder_test_examples(
    HydrusMainWindow &window,
    rust::Str query) noexcept;
void app_import_folder_save(
    HydrusMainWindow &window,
    rust::Str page_id,
    rust::Str original,
    rust::Str name,
    rust::Str path,
    int64_t period,
    bool check_regularly,
    bool search_subdirectories,
    bool paused,
    bool check_now,
    int64_t last_modified_skip,
    bool show_working_popup,
    bool publish_to_popup_button,
    bool publish_to_page,
    rust::Str action,
    rust::Str action_path,
    rust::Vec<FilenameTaggingEntry> tagging) noexcept;
void app_local_import_add_paths(
    HydrusMainWindow &window,
    rust::Vec<rust::String> paths) noexcept;
void app_local_import_remove_paths(
    HydrusMainWindow &window,
    rust::Vec<rust::String> paths) noexcept;
void app_local_import_search_subdirectories(
    HydrusMainWindow &window,
    bool value) noexcept;
void app_local_import_delete_after_success(
    HydrusMainWindow &window,
    bool value) noexcept;
void app_local_import_tagging_save(
    HydrusMainWindow &window,
    rust::Vec<FilenameTaggingEntry> tagging,
    bool import_after_save) noexcept;
// `options_json`: whole options tree (R1 exception: open-ended dict).
void app_options_apply(
    HydrusMainWindow &window,
    rust::Str options_json) noexcept;
// `options_json`: import options tree or one container (R1 exception).
void app_import_options_save(
    HydrusMainWindow &window,
    UiAction action,
    rust::Str target,
    rust::Str caller,
    rust::Str options_json) noexcept;
void app_notes_save(
    HydrusMainWindow &window,
    rust::Str hash,
    rust::Vec<rust::String> names,
    rust::Vec<rust::String> texts) noexcept;

// Implemented in Rust (WS-15.3.2b): scalar getters and
// availability/visibility predicates live in avail.hpp.
#include "avail.hpp"



