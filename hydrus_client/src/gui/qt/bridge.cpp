#include "bridge.hpp"

#include <QString>

std::unique_ptr<QApplication> new_application() {
    static int argc = 1;
    static char name[] = "hydrus_client";
    static char *argv[] = {name, nullptr};
    return std::make_unique<QApplication>(argc, argv);
}

int exec_application(QApplication &application) {
    return application.exec();
}

void quit_application(QApplication &application) {
    application.quit();
}

std::unique_ptr<QMainWindow> new_main_window() {
    return std::make_unique<QMainWindow>();
}

void show_main_window(QMainWindow &window) {
    window.show();
}

void set_main_window_title(QMainWindow &window, rust::Str title) {
    window.setWindowTitle(QString::fromUtf8(title.data(), static_cast<int>(title.size())));
}

void resize_main_window(QMainWindow &window, int width, int height) {
    window.resize(width, height);
}
