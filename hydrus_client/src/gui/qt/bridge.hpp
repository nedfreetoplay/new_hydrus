#pragma once

#include "rust/cxx.h"

#include <QApplication>
#include <QMainWindow>
#include <QMenu>
#include <QSplitter>
#include <QStatusBar>
#include <QTabWidget>
#include <QTreeWidget>
#include <memory>

class HydrusMainWindow : public QMainWindow {
public:
    explicit HydrusMainWindow(QWidget *parent = nullptr);
    void build_shell();
    int add_page(rust::Str title);
    void add_sidebar_item(rust::Str title);
    void add_menu_action(rust::Str menu_title, rust::Str action_title);
    void add_exit_action(rust::Str menu_title, rust::Str action_title);
    void set_status(rust::Str message);
    void set_title(rust::Str title);
    void resize_window(int width, int height);

private:
    QMenu *menu_for(const QString &title);

    QTabWidget *tabs = nullptr;
    QTreeWidget *sidebar = nullptr;
    QSplitter *splitter = nullptr;
};

std::unique_ptr<QApplication> new_application();
int exec_application(QApplication &application);
void quit_application(QApplication &application);

std::unique_ptr<HydrusMainWindow> new_hydrus_main_window();
void show_hydrus_main_window(HydrusMainWindow &window);
void build_hydrus_shell(HydrusMainWindow &window);
int hydrus_add_page(HydrusMainWindow &window, rust::Str title);
void hydrus_add_sidebar_item(HydrusMainWindow &window, rust::Str title);
void hydrus_add_menu_action(HydrusMainWindow &window, rust::Str menu, rust::Str action);
void hydrus_add_exit_action(HydrusMainWindow &window, rust::Str menu, rust::Str action);
void hydrus_set_status(HydrusMainWindow &window, rust::Str message);
void hydrus_set_title(HydrusMainWindow &window, rust::Str title);
void hydrus_resize_window(HydrusMainWindow &window, int width, int height);

void qapp_set_application_name(rust::Str name);
void qapp_set_application_display_name(rust::Str name);
void qapp_set_organization_name(rust::Str name);
void qapp_set_style(rust::Str name);
void qapp_set_stylesheet(rust::Str stylesheet);
