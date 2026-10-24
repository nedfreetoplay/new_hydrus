#[cxx::bridge]
mod ffi {
    unsafe extern "C++" {
        include!("bridge.hpp");
        type QApplication;
        type QMainWindow;

        fn new_application() -> UniquePtr<QApplication>;
        fn exec_application(application: Pin<&mut QApplication>) -> i32;
        fn quit_application(application: Pin<&mut QApplication>);

        fn new_main_window() -> UniquePtr<QMainWindow>;
        fn show_main_window(window: Pin<&mut QMainWindow>);
        fn set_main_window_title(window: Pin<&mut QMainWindow>, title: &str);
        fn resize_main_window(window: Pin<&mut QMainWindow>, width: i32, height: i32);
    }
}

pub fn run_application() {
    let mut app = ffi::new_application();
    {
        let mut window = ffi::new_main_window();
        ffi::set_main_window_title(window.pin_mut(), "Hydrus Client");
        ffi::resize_main_window(window.pin_mut(), 1280, 800);
        ffi::show_main_window(window.pin_mut());
    }
    ffi::exec_application(app.pin_mut());
}
