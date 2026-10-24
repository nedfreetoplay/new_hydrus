#pragma once

#include "rust/cxx.h"

#include <QApplication>
#include <QCheckBox>
#include <QGraphicsScene>
#include <QGraphicsView>
#include <QLineEdit>
#include <QListWidget>
#include <QMainWindow>
#include <QMenu>
#include <QPushButton>
#include <QSplitter>
#include <QStatusBar>
#include <QTabWidget>
#include <QTreeWidget>
#include <QVBoxLayout>
#include <memory>

class HydrusPanel;
class HydrusMediaPanel;
class HydrusCanvas;

class HydrusMainWindow : public QMainWindow {
public:
    explicit HydrusMainWindow(QWidget *parent = nullptr);
    void build_shell();
    int add_page(rust::Str title);
    int add_page_widget(rust::Str title, HydrusPanel *panel);
    int add_media_page(rust::Str title, HydrusMediaPanel *panel);
    int add_canvas_page(rust::Str title, HydrusCanvas *canvas);
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

class HydrusPanel : public QWidget {
public:
    explicit HydrusPanel(QWidget *parent = nullptr);
    void add_label(rust::Str text);
    void add_button(rust::Str text);
    void add_checkbox(rust::Str text, bool checked);
    void add_line_edit(rust::Str placeholder);
    void add_stretch();

private:
    QVBoxLayout *layout = nullptr;
};

class HydrusMediaPanel : public QWidget {
public:
    explicit HydrusMediaPanel(QWidget *parent = nullptr);
    void add_item(rust::Str label);
    void clear_items();
    int item_count() const;

private:
    QListWidget *list = nullptr;
};

class HydrusCanvas : public QWidget {
public:
    explicit HydrusCanvas(QWidget *parent = nullptr);
    bool load_image(rust::Str path);
    void clear();
    void fit_in_view();

private:
    QGraphicsView *view = nullptr;
    QGraphicsScene *scene = nullptr;
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

std::unique_ptr<HydrusPanel> new_hydrus_panel();
void hydrus_panel_add_label(HydrusPanel &panel, rust::Str text);
void hydrus_panel_add_button(HydrusPanel &panel, rust::Str text);
void hydrus_panel_add_checkbox(HydrusPanel &panel, rust::Str text, bool checked);
void hydrus_panel_add_line_edit(HydrusPanel &panel, rust::Str placeholder);
void hydrus_panel_add_stretch(HydrusPanel &panel);
int hydrus_add_page_widget(HydrusMainWindow &window, rust::Str title, HydrusPanel &panel);

std::unique_ptr<HydrusMediaPanel> new_hydrus_media_panel();
void hydrus_media_panel_add_item(HydrusMediaPanel &panel, rust::Str label);
void hydrus_media_panel_clear(HydrusMediaPanel &panel);
int hydrus_media_panel_count(const HydrusMediaPanel &panel);
int hydrus_add_media_page(HydrusMainWindow &window, rust::Str title, HydrusMediaPanel &panel);

std::unique_ptr<HydrusCanvas> new_hydrus_canvas();
bool hydrus_canvas_load_image(HydrusCanvas &canvas, rust::Str path);
void hydrus_canvas_clear(HydrusCanvas &canvas);
void hydrus_canvas_fit(HydrusCanvas &canvas);
int hydrus_add_canvas_page(HydrusMainWindow &window, rust::Str title, HydrusCanvas &canvas);

void qapp_set_application_name(rust::Str name);
void qapp_set_application_display_name(rust::Str name);
void qapp_set_organization_name(rust::Str name);
void qapp_set_style(rust::Str name);
void qapp_set_stylesheet(rust::Str stylesheet);
