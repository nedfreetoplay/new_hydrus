#![allow(dead_code)]

#[cxx::bridge]
pub(crate) mod ffi {
    unsafe extern "C++" {
        include!("bridge.hpp");
        type QApplication;
        type HydrusMainWindow;
        type HydrusPanel;
        type HydrusMediaPanel;
        type HydrusCanvas;
        type HydrusScrolledPanel;
        type HydrusDialog;
        type HydrusListPanel;

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
        fn hydrus_media_panel_add_item_with_icon(
            panel: Pin<&mut HydrusMediaPanel>,
            path: &str,
            label: &str,
        );
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
        fn hydrus_canvas_zoom(canvas: Pin<&mut HydrusCanvas>, factor: f64);
        fn hydrus_canvas_rotate(canvas: Pin<&mut HydrusCanvas>, degrees: i32);
        fn hydrus_canvas_reset(canvas: Pin<&mut HydrusCanvas>);
        fn hydrus_add_canvas_page(
            window: Pin<&mut HydrusMainWindow>,
            title: &str,
            canvas: Pin<&mut HydrusCanvas>,
        ) -> i32;

        fn new_hydrus_scrolled_panel() -> UniquePtr<HydrusScrolledPanel>;
        fn hydrus_scrolled_add_label(panel: Pin<&mut HydrusScrolledPanel>, text: &str);
        fn hydrus_scrolled_add_button(panel: Pin<&mut HydrusScrolledPanel>, text: &str);
        fn hydrus_scrolled_add_checkbox(
            panel: Pin<&mut HydrusScrolledPanel>,
            text: &str,
            checked: bool,
        );
        fn hydrus_scrolled_add_line_edit(panel: Pin<&mut HydrusScrolledPanel>, placeholder: &str);
        fn hydrus_scrolled_add_stretch(panel: Pin<&mut HydrusScrolledPanel>);
        fn hydrus_add_scrolled_page(
            window: Pin<&mut HydrusMainWindow>,
            title: &str,
            panel: Pin<&mut HydrusScrolledPanel>,
        ) -> i32;

        fn new_hydrus_dialog(
            content: Pin<&mut HydrusPanel>,
            title: &str,
            width: i32,
            height: i32,
        ) -> UniquePtr<HydrusDialog>;
        fn hydrus_dialog_exec(dialog: Pin<&mut HydrusDialog>) -> i32;

        fn new_hydrus_list_panel() -> UniquePtr<HydrusListPanel>;
        fn hydrus_list_set_columns(panel: Pin<&mut HydrusListPanel>, csv: &str);
        fn hydrus_list_add_row(panel: Pin<&mut HydrusListPanel>, csv: &str);
        fn hydrus_list_clear(panel: Pin<&mut HydrusListPanel>);
        fn hydrus_list_row_count(panel: &HydrusListPanel) -> i32;
        fn hydrus_add_list_page(
            window: Pin<&mut HydrusMainWindow>,
            title: &str,
            panel: Pin<&mut HydrusListPanel>,
        ) -> i32;

        fn qapp_set_application_name(name: &str);
        fn qapp_set_application_display_name(name: &str);
        fn qapp_set_organization_name(name: &str);
        fn qapp_set_style(name: &str);
        fn qapp_set_stylesheet(stylesheet: &str);
        fn qapp_quit();
    }
}

use crate::gui::qt_init::QtInit;
use crate::gui::view_model::MainViewModel;

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

pub fn run_application(model: &MainViewModel) {
    let mut app = ffi::new_application();

    QtInit::default().apply().ok();

    let mut window = ffi::new_hydrus_main_window();

    ffi::hydrus_set_title(window.pin_mut(), &model.title);
    ffi::hydrus_resize_window(window.pin_mut(), 1280, 800);
    ffi::build_hydrus_shell(window.pin_mut());

    for item in &model.sidebar {
        ffi::hydrus_add_sidebar_item(window.pin_mut(), item);
    }

    for page in &model.pages {
        ffi::hydrus_add_page(window.pin_mut(), page);
    }

    panels::add_panels(window.pin_mut(), &model.panels);

    for menu in &model.menus {
        for action in menu.actions {
            ffi::hydrus_add_menu_action(window.pin_mut(), menu.title, action);
        }

        if menu.has_exit {
            ffi::hydrus_add_exit_action(window.pin_mut(), menu.title, "Exit");
        }
    }

    ffi::hydrus_set_status(window.pin_mut(), &model.status);
    ffi::show_hydrus_main_window(window.pin_mut());

    ffi::exec_application(app.pin_mut());
}
