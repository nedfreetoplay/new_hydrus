#![allow(dead_code)]

#[cxx::bridge]
pub(crate) mod ffi {
    unsafe extern "C++" {
        include!("bridge.hpp");

        type QApplication;
        type QSplashScreen;
        type HydrusMainWindow;

        fn new_application() -> UniquePtr<QApplication>;
        fn exec_application(application: Pin<&mut QApplication>) -> i32;
        fn quit_application(application: Pin<&mut QApplication>);
        fn qapp_process_events();

        fn new_hydrus_splash() -> UniquePtr<QSplashScreen>;
        fn hydrus_splash_show(splash: Pin<&mut QSplashScreen>);
        fn hydrus_splash_message(splash: Pin<&mut QSplashScreen>, message: &str);
        fn hydrus_splash_finish(splash: Pin<&mut QSplashScreen>);
        fn hydrus_splash_finish_later(splash: Pin<&mut QSplashScreen>, delay_ms: i32);
        fn hydrus_splash_screenshot(splash: Pin<&mut QSplashScreen>, path: &str, delay_ms: i32);

        fn new_hydrus_main_window() -> UniquePtr<HydrusMainWindow>;
        fn build_hydrus_shell(window: Pin<&mut HydrusMainWindow>);
        fn show_hydrus_main_window(window: Pin<&mut HydrusMainWindow>);

        fn hydrus_set_title(window: Pin<&mut HydrusMainWindow>, title: &str);
        fn hydrus_resize_window(window: Pin<&mut HydrusMainWindow>, width: i32, height: i32);
        fn hydrus_set_menus(window: Pin<&mut HydrusMainWindow>, json: &str);
        fn hydrus_set_tag_suggestions(window: Pin<&mut HydrusMainWindow>, json: &str);
        fn hydrus_show_autocomplete(window: Pin<&mut HydrusMainWindow>, json: &str);
        fn hydrus_set_status(window: Pin<&mut HydrusMainWindow>, json: &str);
        fn hydrus_build_pages(window: Pin<&mut HydrusMainWindow>, json: &str);
        fn hydrus_update_page(window: Pin<&mut HydrusMainWindow>, page_id: &str, json: &str);
        fn hydrus_update_page_progress(
            window: Pin<&mut HydrusMainWindow>,
            page_id: &str,
            json: &str,
        );
        fn hydrus_update_downloader(window: Pin<&mut HydrusMainWindow>, page_id: &str, json: &str);
        fn hydrus_update_subscriptions(
            window: Pin<&mut HydrusMainWindow>,
            page_id: &str,
            json: &str,
        );
        fn hydrus_update_duplicates(window: Pin<&mut HydrusMainWindow>, page_id: &str, json: &str);
        fn hydrus_update_import_folders(
            window: Pin<&mut HydrusMainWindow>,
            page_id: &str,
            json: &str,
        );
        fn hydrus_update_export_folders(
            window: Pin<&mut HydrusMainWindow>,
            page_id: &str,
            json: &str,
        );
        fn hydrus_show_duplicates_rules(window: Pin<&mut HydrusMainWindow>, json: &str);
        fn hydrus_show_tag_relations(window: Pin<&mut HydrusMainWindow>, json: &str);
        fn hydrus_show_import_folder(window: Pin<&mut HydrusMainWindow>, json: &str);
        fn hydrus_show_export_folder(window: Pin<&mut HydrusMainWindow>, json: &str);
        fn hydrus_show_export_review(window: Pin<&mut HydrusMainWindow>, json: &str);
        fn hydrus_show_notes(window: Pin<&mut HydrusMainWindow>, json: &str);
        fn hydrus_set_selection(
            window: Pin<&mut HydrusMainWindow>,
            page_id: &str,
            hash: &str,
            tags_json: &str,
            preview_path: &str,
            metadata_json: &str,
        );
        fn hydrus_show_page_chooser(window: Pin<&mut HydrusMainWindow>, notebook_id: &str);
        fn hydrus_show_page_manager(window: Pin<&mut HydrusMainWindow>, json: &str);
        fn hydrus_show_command_palette(window: Pin<&mut HydrusMainWindow>);
        fn hydrus_prompt_text(
            window: Pin<&mut HydrusMainWindow>,
            action: &str,
            title: &str,
            label: &str,
            initial: &str,
        );
        fn hydrus_confirm_action(
            window: Pin<&mut HydrusMainWindow>,
            action: &str,
            target: &str,
            title: &str,
            text: &str,
        );
        fn hydrus_show_or_dialog(window: Pin<&mut HydrusMainWindow>, page_id: &str, initial: &str);
        fn hydrus_pick_files(window: Pin<&mut HydrusMainWindow>);
        fn hydrus_prompt_url(window: Pin<&mut HydrusMainWindow>);
        fn hydrus_open_viewer(window: Pin<&mut HydrusMainWindow>, json: &str);
        fn hydrus_update_viewer(window: Pin<&mut HydrusMainWindow>, viewer_id: &str, json: &str);
        fn hydrus_close_viewer(window: Pin<&mut HydrusMainWindow>, viewer_id: &str);
        fn hydrus_show_tag_dialog(window: Pin<&mut HydrusMainWindow>, json: &str);
        fn hydrus_show_notifications(window: Pin<&mut HydrusMainWindow>, json: &str);
        fn hydrus_show_viewing_stats(window: Pin<&mut HydrusMainWindow>, json: &str);
        fn hydrus_show_about(window: Pin<&mut HydrusMainWindow>, json: &str);
        fn hydrus_export_downloader(window: Pin<&mut HydrusMainWindow>, page_id: &str);
        fn hydrus_import_downloader(window: Pin<&mut HydrusMainWindow>, target: &str);
        fn hydrus_show_manage_tags(window: Pin<&mut HydrusMainWindow>, json: &str);
        fn hydrus_show_timestamps(window: Pin<&mut HydrusMainWindow>, json: &str);
        fn hydrus_show_options(window: Pin<&mut HydrusMainWindow>, json: &str);
        fn hydrus_show_services(window: Pin<&mut HydrusMainWindow>, json: &str);
        fn hydrus_update_services(window: Pin<&mut HydrusMainWindow>, json: &str);
        fn hydrus_show_tag_filter(window: Pin<&mut HydrusMainWindow>, json: &str);
        fn hydrus_show_api_permissions(window: Pin<&mut HydrusMainWindow>, json: &str);
        fn hydrus_update_api_permissions(window: Pin<&mut HydrusMainWindow>, json: &str);
        fn hydrus_show_account_types(window: Pin<&mut HydrusMainWindow>, json: &str);
        fn hydrus_update_account_types(window: Pin<&mut HydrusMainWindow>, json: &str);
        fn hydrus_show_network(window: Pin<&mut HydrusMainWindow>, json: &str);
        fn hydrus_update_network(window: Pin<&mut HydrusMainWindow>, json: &str);
        fn hydrus_start_tick(window: Pin<&mut HydrusMainWindow>);
        fn hydrus_defer_action(
            window: Pin<&mut HydrusMainWindow>,
            action: &str,
            target: &str,
            payload: &str,
            delay_ms: i32,
        );
        fn hydrus_show_message(window: Pin<&mut HydrusMainWindow>, title: &str, text: &str);
        fn hydrus_notify_message(window: Pin<&mut HydrusMainWindow>, message: &str);
        fn hydrus_set_clipboard(window: Pin<&mut HydrusMainWindow>, text: &str);
        fn hydrus_open_path(path: &str);
        fn hydrus_open_url(url: &str);
        fn hydrus_schedule_screenshot(window: Pin<&mut HydrusMainWindow>, path: &str);

        fn qapp_set_application_name(name: &str);
        fn qapp_set_application_display_name(name: &str);
        fn qapp_set_organization_name(name: &str);
        fn qapp_set_style(name: &str);
        fn qapp_set_stylesheet(stylesheet: &str);
        fn qapp_quit();
    }

    extern "Rust" {
        fn app_action(
            window: Pin<&mut HydrusMainWindow>,
            action: &str,
            target: &str,
            payload: &str,
        );
        fn app_tick(window: Pin<&mut HydrusMainWindow>);
    }
}

use std::pin::Pin;
use std::sync::Arc;

use crate::controller::Controller;
use crate::gui::app;
use crate::gui::qt_init::QtInit;

use super::panels;

fn app_action(
    mut window: Pin<&mut ffi::HydrusMainWindow>,
    action: &str,
    target: &str,
    payload: &str,
) {
    app::handle_action(window.as_mut(), action, target, payload);
}

fn app_tick(mut window: Pin<&mut ffi::HydrusMainWindow>) {
    app::handle_tick(window.as_mut());
}

pub(crate) fn apply_application_settings(
    app_name: &str,
    display_name: &str,
    organization_name: &str,
    style_name: &str,
    stylesheet: &str,
) {
    ffi::qapp_set_application_name(app_name);
    ffi::qapp_set_application_display_name(display_name);
    ffi::qapp_set_organization_name(organization_name);
    ffi::qapp_set_style(style_name);
    ffi::qapp_set_stylesheet(stylesheet);
}

pub(crate) fn quit_application() {
    ffi::qapp_quit();
}

/// Starts the real client GUI.
///
/// The app state owns the controller and the page tree; the Qt layer is only a
/// renderer. Every widget event comes back through [`app_action`] and every
/// state change is rendered with an explicit [`app::UiCommand`].
pub fn run_application(controller: Arc<Controller>) {
    let mut app = ffi::new_application();

    let _ = QtInit::default().apply();

    let mut splash = ffi::new_hydrus_splash();
    let mut splash_model = crate::gui::splash::SplashModel::new("Hydrus Client");
    splash_model.set_status(format!(
        "software v{} · network v{}",
        hydrus_core::hydrus_constants::SOFTWARE_VERSION,
        hydrus_core::hydrus_constants::NETWORK_VERSION
    ));
    ffi::hydrus_splash_show(splash.pin_mut());
    ffi::hydrus_splash_message(
        splash.pin_mut(),
        splash_model.get_status().unwrap_or_default(),
    );

    let mut window = ffi::new_hydrus_main_window();
    ffi::build_hydrus_shell(window.pin_mut());

    app::install(controller);

    splash_model.set_status("loading pages...");
    ffi::hydrus_splash_message(
        splash.pin_mut(),
        splash_model.get_status().unwrap_or_default(),
    );

    if let Ok(paths) = std::env::var("HYDRUS_QT_IMPORT") {
        let paths: Vec<String> = paths
            .split(';')
            .filter(|path| !path.is_empty())
            .map(str::to_string)
            .collect();

        app::import_for_testing(&paths);
    }

    if let Ok(tag) = std::env::var("HYDRUS_QT_ADD_TAG") {
        app::add_tag_for_testing(&tag);
    }

    if let Ok(tags) = std::env::var("HYDRUS_QT_ADD_TAGS") {
        let tags: Vec<String> = tags
            .split(';')
            .filter(|tag| !tag.is_empty())
            .map(str::to_string)
            .collect();

        app::add_tags_for_testing(&tags);
    }

    if let Ok(value) = std::env::var("HYDRUS_QT_TAG_DISPLAY") {
        let parts: Vec<&str> = value.split(',').collect();
        let sort = parts.first().copied().unwrap_or("lexicographic");
        let show_namespaces = !parts.iter().any(|part| *part == "no-namespaces");
        let replace_underscores = parts.iter().any(|part| *part == "spaces");

        app::set_tag_display_options_for_testing(sort, show_namespaces, replace_underscores);
    }

    if let Ok(views) = std::env::var("HYDRUS_QT_VIEWS") {
        if let Ok(views) = views.parse::<i64>() {
            app::add_viewing_stats_for_testing(views);
        }
    }

    if let Ok(url) = std::env::var("HYDRUS_QT_URL") {
        app::add_url_for_testing(&url);
    }

    if let Ok(value) = std::env::var("HYDRUS_QT_THUMBNAILS") {
        let parts: Vec<&str> = value.split(',').collect();
        let scale = parts
            .first()
            .and_then(|part| part.parse().ok())
            .unwrap_or(100);
        let masonry = parts.iter().any(|part| *part == "masonry");
        let horizontal = parts.iter().any(|part| *part == "horizontal");

        app::set_thumbnail_options_for_testing(scale, masonry, horizontal);
    }

    if let Ok(query) = std::env::var("HYDRUS_QT_QUERY") {
        app::set_query_for_testing(&query);
    }

    if std::env::var_os("HYDRUS_QT_PAGE_LOADING").is_some() {
        app::show_page_loading_for_testing();
    }

    let mut commands = app::initial_commands();

    if std::env::var_os("HYDRUS_QT_FOCUS_FIRST").is_some() {
        commands.extend(app::focus_first_for_testing());
    }

    if std::env::var_os("HYDRUS_QT_OPEN_FIRST").is_some() {
        commands.extend(app::open_first_for_testing());
    }

    if let Ok(script) = std::env::var("HYDRUS_QT_SCRIPT") {
        commands.extend(app::run_script_for_testing(&script));
    }

    panels::execute_commands(window.pin_mut(), commands);

    ffi::hydrus_set_title(window.pin_mut(), "Hydrus Client");
    ffi::hydrus_resize_window(window.pin_mut(), 1280, 800);
    ffi::show_hydrus_main_window(window.pin_mut());
    ffi::hydrus_start_tick(window.pin_mut());

    splash_model.set_status("ready");
    ffi::hydrus_splash_message(
        splash.pin_mut(),
        splash_model.get_status().unwrap_or_default(),
    );

    if let Ok(path) = std::env::var("HYDRUS_QT_SPLASH_SCREENSHOT") {
        if !path.is_empty() {
            ffi::hydrus_splash_screenshot(splash.pin_mut(), &path, 500);
            ffi::hydrus_splash_finish_later(splash.pin_mut(), 1500);
        }
    } else if std::env::var_os("HYDRUS_QT_HOLD_SPLASH").is_some() {
        ffi::hydrus_splash_finish_later(splash.pin_mut(), 2000);
    } else {
        ffi::hydrus_splash_finish_later(splash.pin_mut(), 600);
    }

    if let Ok(path) = std::env::var("HYDRUS_QT_SCREENSHOT") {
        if !path.is_empty() {
            ffi::hydrus_schedule_screenshot(window.pin_mut(), &path);
        }
    }

    if let Ok(script) = std::env::var("HYDRUS_QT_DEFER") {
        for (index, step) in script.split(';').enumerate() {
            let mut parts = step.split('|');
            let action = parts.next().unwrap_or_default();
            let target = parts.next().unwrap_or_default();
            let payload = parts.next().unwrap_or_default();

            if !action.is_empty() {
                ffi::hydrus_defer_action(
                    window.pin_mut(),
                    action,
                    target,
                    payload,
                    400 + index as i32 * 250,
                );
            }
        }
    }

    ffi::exec_application(app.pin_mut());

    app::shutdown();
}

pub(crate) fn open_path(path: &str) {
    ffi::hydrus_open_path(path);
}
