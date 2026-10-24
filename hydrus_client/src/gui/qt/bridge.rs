#![allow(dead_code)]

#[cxx::bridge]
mod ffi {
    unsafe extern "C++" {
        include!("bridge.hpp");
        type QApplication;
        type HydrusMainWindow;
        type HydrusPanel;
        type HydrusMediaPanel;
        type HydrusCanvas;

        fn new_application() -> UniquePtr<QApplication>;
        fn exec_application(application: Pin<&mut QApplication>) -> i32;
        fn quit_application(application: Pin<&mut QApplication>);

        fn new_hydrus_main_window() -> UniquePtr<HydrusMainWindow>;
        fn show_hydrus_main_window(window: Pin<&mut HydrusMainWindow>);
        fn build_hydrus_shell(window: Pin<&mut HydrusMainWindow>);
        fn hydrus_add_page(window: Pin<&mut HydrusMainWindow>, title: &str) -> i32;
        fn hydrus_add_sidebar_item(window: Pin<&mut HydrusMainWindow>, title: &str);
        fn hydrus_add_menu_action(window: Pin<&mut HydrusMainWindow>, menu: &str, action: &str);
        fn hydrus_add_exit_action(window: Pin<&mut HydrusMainWindow>, menu: &str, action: &str);
        fn hydrus_set_status(window: Pin<&mut HydrusMainWindow>, message: &str);
        fn hydrus_set_title(window: Pin<&mut HydrusMainWindow>, title: &str);
        fn hydrus_resize_window(window: Pin<&mut HydrusMainWindow>, width: i32, height: i32);

        fn new_hydrus_panel() -> UniquePtr<HydrusPanel>;
        fn hydrus_panel_add_label(panel: Pin<&mut HydrusPanel>, text: &str);
        fn hydrus_panel_add_button(panel: Pin<&mut HydrusPanel>, text: &str);
        fn hydrus_panel_add_checkbox(panel: Pin<&mut HydrusPanel>, text: &str, checked: bool);
        fn hydrus_panel_add_line_edit(panel: Pin<&mut HydrusPanel>, placeholder: &str);
        fn hydrus_panel_add_stretch(panel: Pin<&mut HydrusPanel>);
        fn hydrus_add_page_widget(
            window: Pin<&mut HydrusMainWindow>,
            title: &str,
            panel: Pin<&mut HydrusPanel>,
        ) -> i32;

        fn new_hydrus_media_panel() -> UniquePtr<HydrusMediaPanel>;
        fn hydrus_media_panel_add_item(panel: Pin<&mut HydrusMediaPanel>, label: &str);
        fn hydrus_media_panel_clear(panel: Pin<&mut HydrusMediaPanel>);
        fn hydrus_media_panel_count(panel: &HydrusMediaPanel) -> i32;
        fn hydrus_add_media_page(
            window: Pin<&mut HydrusMainWindow>,
            title: &str,
            panel: Pin<&mut HydrusMediaPanel>,
        ) -> i32;

        fn new_hydrus_canvas() -> UniquePtr<HydrusCanvas>;
        fn hydrus_canvas_load_image(canvas: Pin<&mut HydrusCanvas>, path: &str) -> bool;
        fn hydrus_canvas_clear(canvas: Pin<&mut HydrusCanvas>);
        fn hydrus_canvas_fit(canvas: Pin<&mut HydrusCanvas>);
        fn hydrus_add_canvas_page(
            window: Pin<&mut HydrusMainWindow>,
            title: &str,
            canvas: Pin<&mut HydrusCanvas>,
        ) -> i32;

        fn qapp_set_application_name(name: &str);
        fn qapp_set_application_display_name(name: &str);
        fn qapp_set_organization_name(name: &str);
        fn qapp_set_style(name: &str);
        fn qapp_set_stylesheet(stylesheet: &str);
    }
}

use crate::gui::frame_gui;
use crate::gui::qt_init::QtInit;

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

pub fn run_application(title: &str, status: &str) {
    let model = frame_gui::default_frame_gui();
    let mut app = ffi::new_application();

    QtInit::default().apply().ok();

    let mut window = ffi::new_hydrus_main_window();

    ffi::hydrus_set_title(window.pin_mut(), title);
    ffi::hydrus_resize_window(window.pin_mut(), 1280, 800);
    ffi::build_hydrus_shell(window.pin_mut());

    for item in model.sidebar {
        ffi::hydrus_add_sidebar_item(window.pin_mut(), item);
    }

    let mut panel = ffi::new_hydrus_panel();
    ffi::hydrus_panel_add_label(
        panel.pin_mut(),
        "Welcome to Hydrus Client — Rust/Qt 6 port.",
    );
    ffi::hydrus_panel_add_line_edit(panel.pin_mut(), "Search tags…");
    ffi::hydrus_panel_add_button(panel.pin_mut(), "Search");
    ffi::hydrus_panel_add_checkbox(panel.pin_mut(), "Darkmode", false);
    ffi::hydrus_panel_add_stretch(panel.pin_mut());
    ffi::hydrus_add_page_widget(
        window.pin_mut(),
        model.pages.first().copied().unwrap_or("Welcome"),
        panel.pin_mut(),
    );

    let mut media_panel = ffi::new_hydrus_media_panel();
    ffi::hydrus_media_panel_add_item(media_panel.pin_mut(), "No files loaded yet");
    ffi::hydrus_media_panel_add_item(media_panel.pin_mut(), "Open a page to browse media");
    ffi::hydrus_add_media_page(window.pin_mut(), "Files", media_panel.pin_mut());

    let mut canvas = ffi::new_hydrus_canvas();
    ffi::hydrus_add_canvas_page(window.pin_mut(), "Canvas", canvas.pin_mut());

    for menu in model.menus {
        for action in menu.actions {
            ffi::hydrus_add_menu_action(window.pin_mut(), menu.title, action);
        }

        if menu.has_exit {
            ffi::hydrus_add_exit_action(window.pin_mut(), menu.title, "Exit");
        }
    }

    ffi::hydrus_set_status(window.pin_mut(), status);
    ffi::show_hydrus_main_window(window.pin_mut());

    ffi::exec_application(app.pin_mut());
}
