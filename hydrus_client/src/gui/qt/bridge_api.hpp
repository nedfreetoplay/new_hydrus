#pragma once

// WS-15.1.4: free hydrus_*/qapp_* glue declarations, verbatim from bridge.hpp.

#include "rust/cxx.h"

#include <QString>

class QApplication;
class QSplashScreen;
class HydrusMainWindow;

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
void hydrus_set_menus(HydrusMainWindow &window, rust::Str json);
void hydrus_set_tag_suggestions(HydrusMainWindow &window, rust::Str json);
void hydrus_show_autocomplete(HydrusMainWindow &window, rust::Str json);
void hydrus_set_status(HydrusMainWindow &window, rust::Str json);
void hydrus_build_pages(HydrusMainWindow &window, rust::Str json);
void hydrus_update_page(HydrusMainWindow &window, rust::Str page_id, rust::Str json);
void hydrus_update_page_progress(
    HydrusMainWindow &window,
    rust::Str page_id,
    rust::Str json);
void hydrus_update_downloader(HydrusMainWindow &window, rust::Str page_id, rust::Str json);
void hydrus_update_subscriptions(HydrusMainWindow &window, rust::Str page_id, rust::Str json);
void hydrus_show_subscriptions_dialog(HydrusMainWindow &window, rust::Str json);
void hydrus_update_subscriptions_dialog(HydrusMainWindow &window, rust::Str json);
void hydrus_show_subscription_editor(HydrusMainWindow &window, rust::Str json);
void hydrus_update_duplicates(HydrusMainWindow &window, rust::Str page_id, rust::Str json);
void hydrus_update_import_folders(HydrusMainWindow &window, rust::Str page_id, rust::Str json);
void hydrus_show_import_folders_dialog(HydrusMainWindow &window, rust::Str json);
void hydrus_update_import_folders_dialog(HydrusMainWindow &window, rust::Str json);
void hydrus_update_export_folders(HydrusMainWindow &window, rust::Str page_id, rust::Str json);
void hydrus_show_export_folders_dialog(HydrusMainWindow &window, rust::Str json);
void hydrus_update_export_folders_dialog(HydrusMainWindow &window, rust::Str json);
void hydrus_show_duplicates_rules(HydrusMainWindow &window, rust::Str json);
void hydrus_show_duplicate_merge_options(HydrusMainWindow &window, rust::Str json);
void hydrus_show_duplicate_custom_action(HydrusMainWindow &window, rust::Str json);
void hydrus_show_import_options(HydrusMainWindow &window, rust::Str json);
void hydrus_show_file_seed_cache(HydrusMainWindow &window, rust::Str json);
void hydrus_show_gallery_seed_log(HydrusMainWindow &window, rust::Str json);
void hydrus_update_auto_resolution_preview(HydrusMainWindow &window, rust::Str json);
void hydrus_show_duplicate_review(HydrusMainWindow &window, rust::Str json);
void hydrus_update_duplicate_review(HydrusMainWindow &window, rust::Str json);
void hydrus_show_tag_relations(HydrusMainWindow &window, rust::Str json);
void hydrus_show_import_folder(HydrusMainWindow &window, rust::Str json);
void hydrus_show_local_import_review(HydrusMainWindow &window, rust::Str json);
void hydrus_show_local_import_tagging(HydrusMainWindow &window, rust::Str json);
void hydrus_close_local_import_review(HydrusMainWindow &window);
void hydrus_show_repair_file_locations(HydrusMainWindow &window, rust::Str json);
void hydrus_close_repair_file_locations(HydrusMainWindow &window);
void hydrus_show_export_folder(HydrusMainWindow &window, rust::Str json);
void hydrus_show_export_review(HydrusMainWindow &window, rust::Str json);
void hydrus_show_notes(HydrusMainWindow &window, rust::Str json);
void hydrus_set_selection(
    HydrusMainWindow &window,
    rust::Str page_id,
    rust::Str hash,
    rust::Str tags_json,
    rust::Str preview_path,
    rust::Str metadata_json);
void hydrus_show_page_chooser(HydrusMainWindow &window, rust::Str notebook_id);
void hydrus_show_page_manager(HydrusMainWindow &window, rust::Str json);
void hydrus_show_command_palette(HydrusMainWindow &window);
void hydrus_prompt_text(
    HydrusMainWindow &window,
    rust::Str action,
    rust::Str title,
    rust::Str label,
    rust::Str initial);
void hydrus_confirm_action(
    HydrusMainWindow &window,
    rust::Str action,
    rust::Str target,
    rust::Str title,
    rust::Str text);
void hydrus_show_or_dialog(
    HydrusMainWindow &window,
    rust::Str page_id,
    rust::Str initial);
void hydrus_pick_files(HydrusMainWindow &window);
void hydrus_prompt_url(HydrusMainWindow &window);
void hydrus_open_viewer(HydrusMainWindow &window, rust::Str json);
void hydrus_update_viewer(HydrusMainWindow &window, rust::Str viewer_id, rust::Str json);
void hydrus_close_viewer(HydrusMainWindow &window, rust::Str viewer_id);
void hydrus_show_tag_dialog(HydrusMainWindow &window, rust::Str json);
void hydrus_show_notifications(HydrusMainWindow &window, rust::Str json);
void hydrus_show_viewing_stats(HydrusMainWindow &window, rust::Str json);
void hydrus_show_about(HydrusMainWindow &window, rust::Str json);
void hydrus_export_downloader(HydrusMainWindow &window, rust::Str page_id);
void hydrus_import_downloader(HydrusMainWindow &window, rust::Str target);
void hydrus_show_manage_tags(HydrusMainWindow &window, rust::Str json);
void hydrus_show_timestamps(HydrusMainWindow &window, rust::Str json);
void hydrus_show_options(HydrusMainWindow &window, rust::Str json);
void hydrus_show_services(HydrusMainWindow &window, rust::Str json);
void hydrus_update_services(HydrusMainWindow &window, rust::Str json);
void hydrus_show_tag_filter(HydrusMainWindow &window, rust::Str json);
void hydrus_show_api_permissions(HydrusMainWindow &window, rust::Str json);
void hydrus_update_api_permissions(HydrusMainWindow &window, rust::Str json);
void hydrus_show_account_types(HydrusMainWindow &window, rust::Str json);
void hydrus_update_account_types(HydrusMainWindow &window, rust::Str json);
void hydrus_show_orphan_files(HydrusMainWindow &window, rust::Str json);
void hydrus_update_orphan_files(HydrusMainWindow &window, rust::Str json);
void hydrus_show_database_backup(HydrusMainWindow &window, rust::Str json);
void hydrus_update_database_backup(HydrusMainWindow &window, rust::Str json);
void hydrus_show_network(HydrusMainWindow &window, rust::Str json);
void hydrus_update_network(HydrusMainWindow &window, rust::Str json);
void hydrus_start_tick(HydrusMainWindow &window);
void hydrus_defer_action(
    HydrusMainWindow &window,
    rust::Str action,
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
    rust::Str action,
    rust::Str target,
    rust::Str payload) noexcept;

void app_tick(HydrusMainWindow &window) noexcept;

// Implemented in Rust: the Python VetoException text for an invalid export
// filename phrase, or an empty string when the phrase parses.
rust::String hydrus_validate_export_phrase(rust::Str phrase) noexcept;

// Implemented in Rust: resolves the vendored static/ asset directory.
rust::String hydrus_static_dir() noexcept;
