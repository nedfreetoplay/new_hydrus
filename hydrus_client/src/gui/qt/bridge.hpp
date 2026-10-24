#pragma once

#include "rust/cxx.h"

#include <QApplication>
#include <QMainWindow>
#include <memory>

std::unique_ptr<QApplication> new_application();
int exec_application(QApplication &application);
void quit_application(QApplication &application);

std::unique_ptr<QMainWindow> new_main_window();
void show_main_window(QMainWindow &window);
void set_main_window_title(QMainWindow &window, rust::Str title);
void resize_main_window(QMainWindow &window, int width, int height);
