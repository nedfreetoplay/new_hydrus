// WS-15.2.1: the cxx bridge (`mod ffi`) moved to api.rs, which now owns the
// single bridge module plus the shared POD structs. This file keeps only the
// application bootstrap (run_application and helpers).
#![allow(dead_code)]

use std::sync::Arc;

use super::api::ffi;
use crate::controller::Controller;
use crate::gui::app;
use crate::gui::qt_init::QtInit;

use super::panels;

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

    ffi::hydrus_set_title(window.pin_mut(), "main");
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


