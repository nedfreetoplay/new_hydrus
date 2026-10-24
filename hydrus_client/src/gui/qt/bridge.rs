#[cxx::bridge]
mod ffi {
    unsafe extern "C++" {
        include!("bridge.hpp");
        type QApplication;
        type HydrusMainWindow;

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
    }
}

use crate::gui::frame_gui;

pub fn run_application(title: &str, status: &str) {
    let model = frame_gui::default_frame_gui();
    let mut app = ffi::new_application();
    let mut window = ffi::new_hydrus_main_window();

    ffi::hydrus_set_title(window.pin_mut(), title);
    ffi::hydrus_resize_window(window.pin_mut(), 1280, 800);
    ffi::build_hydrus_shell(window.pin_mut());

    for item in model.sidebar {
        ffi::hydrus_add_sidebar_item(window.pin_mut(), item);
    }

    for page in model.pages {
        ffi::hydrus_add_page(window.pin_mut(), page);
    }

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
