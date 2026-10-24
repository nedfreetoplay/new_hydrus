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

pub(crate) fn quit_application() {
    ffi::qapp_quit();
}

pub fn run_application(
    title: &str,
    status: &str,
    session: Option<&crate::gui::session::GuiSession>,
) {
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

    if let Some(session) = session {
        for page in &session.pages {
            ffi::hydrus_add_sidebar_item(window.pin_mut(), &page.name);
            ffi::hydrus_add_page(window.pin_mut(), &page.name);
        }
    } else {
        for page in model.pages {
            ffi::hydrus_add_page(window.pin_mut(), page);
        }
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

    let mut options_panel = ffi::new_hydrus_scrolled_panel();
    ffi::hydrus_scrolled_add_label(options_panel.pin_mut(), "Client options");
    ffi::hydrus_scrolled_add_line_edit(options_panel.pin_mut(), "Search options…");
    ffi::hydrus_scrolled_add_checkbox(options_panel.pin_mut(), "Darkmode", false);
    ffi::hydrus_scrolled_add_button(options_panel.pin_mut(), "Save");
    ffi::hydrus_scrolled_add_stretch(options_panel.pin_mut());
    ffi::hydrus_add_scrolled_page(window.pin_mut(), "Options", options_panel.pin_mut());

    let mut list_panel = ffi::new_hydrus_list_panel();
    ffi::hydrus_list_set_columns(list_panel.pin_mut(), "Name,Size");
    ffi::hydrus_list_add_row(list_panel.pin_mut(), "example.png,1024");
    ffi::hydrus_add_list_page(window.pin_mut(), "List", list_panel.pin_mut());

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
