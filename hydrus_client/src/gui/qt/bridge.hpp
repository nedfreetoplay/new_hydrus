#pragma once

#include "rust/cxx.h"

#include <QApplication>
#include <QCloseEvent>
#include <QContextMenuEvent>
#include <QGraphicsPixmapItem>
#include <QGraphicsScene>
#include <QGraphicsView>
#include <QHash>
#include <QJsonArray>
#include <QJsonObject>
#include <QLabel>
#include <QLineEdit>
#include <QListWidget>
#include <QMainWindow>
#include <QPushButton>
#include <QResizeEvent>
#include <QString>
#include <QStringList>
#include <QTabWidget>
#include <QTimer>
#include <QWheelEvent>
#include <QWidget>
#include <memory>

class HydrusMainWindow;
class HydrusNotebook;
class HydrusQueryPage;
class HydrusThumbnailGrid;
class HydrusViewerWindow;
class QTabBar;
class QCompleter;
class QDragEnterEvent;
class QDropEvent;

// A zoomable, pannable view for one image. Used both by the small preview in a
// page sidebar and by the media viewer window.
class HydrusCanvas : public QGraphicsView {
public:
    explicit HydrusCanvas(QWidget *parent = nullptr);

    bool load_image(const QString &path);
    void clear_image();
    void fit_image();
    void reset_zoom();
    void zoom_by(double factor);
    void rotate_by(int degrees);

protected:
    void resizeEvent(QResizeEvent *event) override;
    void wheelEvent(QWheelEvent *event) override;

private:
    QGraphicsScene *scene = nullptr;
    QGraphicsPixmapItem *pixmap_item = nullptr;
    bool fit_mode = true;
    int rotation = 0;
};

// One media page, laid out like Hydrus: search/preview sidebar on the left and
// the thumbnail grid on the right.
class HydrusQueryPage : public QWidget {
public:
    HydrusQueryPage(HydrusMainWindow *main, const QString &page_id, const QJsonObject &state);

    const QString &page_id() const;

    void update_state(const QJsonObject &state);
    void set_selection(const QString &hash, const QStringList &tags, const QString &preview_path);
    void set_suggestions(const QStringList &suggestions);

    void focus_hash(const QString &hash);
    void open_focused();
    void archive_focused();
    void inbox_focused();
    void delete_focused();

    QString focused_hash() const;

private:
    HydrusMainWindow *main = nullptr;
    QString id;
    QLineEdit *search = nullptr;
    QCompleter *completer = nullptr;
    QPushButton *search_button = nullptr;
    QLabel *status_label = nullptr;
    QLabel *empty_label = nullptr;
    QListWidget *selection_tags = nullptr;
    HydrusCanvas *preview = nullptr;
    HydrusThumbnailGrid *grid = nullptr;
};

// The thumbnail grid. Hydrus uses a custom graphics scene; a uniform-icon
// QListWidget gives the same grid behaviour with much less code.
class HydrusThumbnailGrid : public QListWidget {
public:
    HydrusThumbnailGrid(HydrusMainWindow *main, HydrusQueryPage *page);

    void set_media(const QJsonArray &media);
    void set_focused(const QString &hash);
    QString focused_hash() const;

protected:
    void contextMenuEvent(QContextMenuEvent *event) override;
    void wheelEvent(QWheelEvent *event) override;

private:
    HydrusMainWindow *main = nullptr;
    HydrusQueryPage *page = nullptr;
};

// Media viewer frame: canvas plus a tag sidebar, as in Hydrus' canvas frame.
class HydrusViewerWindow : public QWidget {
public:
    HydrusViewerWindow(HydrusMainWindow *main, const QJsonObject &state);

    const QString &viewer_id() const;

    void update_state(const QJsonObject &state);

protected:
    void closeEvent(QCloseEvent *event) override;

private:
    void navigate(const QString &direction);

    HydrusMainWindow *main = nullptr;
    QString id;
    QString page_id;
    QString hash;
    HydrusCanvas *canvas = nullptr;
    QListWidget *tags = nullptr;
    QLabel *status = nullptr;
};

// A notebook is a page of pages: tabs, a trailing "+" tab, and the usual
// close/rename/context-menu behaviour.
class HydrusNotebook : public QTabWidget {
public:
    HydrusNotebook(HydrusMainWindow *main, const QString &notebook_id, QWidget *parent = nullptr);

    const QString &notebook_id() const;

    void add_page_widget(const QString &page_id, const QString &title, QWidget *page);
    void finalize_notebook(const QString &current_page_id);
    void clear_pages();

    QString current_page_id() const;

private:
    void install_plus_tab();
    void show_tab_menu(const QPoint &position);
    void request_close(int index);

    HydrusMainWindow *main = nullptr;
    QString id;
    int plus_index = -1;
    int last_normal_index = 0;
};

class HydrusMainWindow : public QMainWindow {
public:
    explicit HydrusMainWindow(QWidget *parent = nullptr);

    void build_shell();
    void set_status(const QJsonObject &status);
    void set_menus(const QJsonArray &menus);
    void build_pages(const QJsonObject &pages);
    void update_page(const QString &page_id, const QJsonObject &state);
    void set_selection(
        const QString &page_id,
        const QString &hash,
        const QStringList &tags,
        const QString &preview_path);
    void set_tag_suggestions(const QStringList &suggestions);
    const QStringList &suggestions() const;
    void show_page_chooser(const QString &notebook_id);
    void prompt_url();
    void open_viewer(const QJsonObject &state);
    void update_viewer(const QString &viewer_id, const QJsonObject &state);
    void close_viewer(const QString &viewer_id);
    void show_tag_dialog(const QJsonObject &state);
    void show_message(const QString &title, const QString &text);
    void notify_message(const QString &message);
    void schedule_screenshot(const QString &path);

    HydrusQueryPage *find_page(const QString &page_id) const;

    void notify(const QString &action, const QString &target = {}, const QString &payload = {});

    bool building = true;
    QString active_page;
    QString active_notebook;

protected:
    void dragEnterEvent(QDragEnterEvent *event) override;
    void dropEvent(QDropEvent *event) override;

public:

private:
    void populate_notebook(HydrusNotebook *notebook, const QJsonObject &object);
    void clear_notebook(HydrusNotebook *notebook);
    void build_shortcuts();

    HydrusNotebook *root = nullptr;
    QLabel *status_message = nullptr;
    QLabel *status_media = nullptr;
    QLabel *status_idle = nullptr;
    QLabel *status_db = nullptr;

    QHash<QString, HydrusQueryPage *> pages;
    QHash<QString, HydrusNotebook *> notebooks;
    QHash<QString, HydrusViewerWindow *> viewers;
    QStringList tag_suggestions;
};

std::unique_ptr<QApplication> new_application();
int exec_application(QApplication &application);
void quit_application(QApplication &application);

std::unique_ptr<HydrusMainWindow> new_hydrus_main_window();
void build_hydrus_shell(HydrusMainWindow &window);
void show_hydrus_main_window(HydrusMainWindow &window);

void hydrus_set_title(HydrusMainWindow &window, rust::Str title);
void hydrus_resize_window(HydrusMainWindow &window, int width, int height);
void hydrus_set_menus(HydrusMainWindow &window, rust::Str json);
void hydrus_set_tag_suggestions(HydrusMainWindow &window, rust::Str json);
void hydrus_set_status(HydrusMainWindow &window, rust::Str json);
void hydrus_build_pages(HydrusMainWindow &window, rust::Str json);
void hydrus_update_page(HydrusMainWindow &window, rust::Str page_id, rust::Str json);
void hydrus_set_selection(
    HydrusMainWindow &window,
    rust::Str page_id,
    rust::Str hash,
    rust::Str tags_json,
    rust::Str preview_path);
void hydrus_show_page_chooser(HydrusMainWindow &window, rust::Str notebook_id);
void hydrus_pick_files(HydrusMainWindow &window);
void hydrus_prompt_url(HydrusMainWindow &window);
void hydrus_open_viewer(HydrusMainWindow &window, rust::Str json);
void hydrus_update_viewer(HydrusMainWindow &window, rust::Str viewer_id, rust::Str json);
void hydrus_close_viewer(HydrusMainWindow &window, rust::Str viewer_id);
void hydrus_show_tag_dialog(HydrusMainWindow &window, rust::Str json);
void hydrus_show_message(HydrusMainWindow &window, rust::Str title, rust::Str text);
void hydrus_notify_message(HydrusMainWindow &window, rust::Str message);
void hydrus_open_path(rust::Str path);
void hydrus_open_url(rust::Str url);
void hydrus_schedule_screenshot(HydrusMainWindow &window, rust::Str path);
void qapp_set_application_name(rust::Str name);
void qapp_set_application_display_name(rust::Str name);
void qapp_set_organization_name(rust::Str name);
void qapp_set_style(rust::Str name);
void qapp_set_stylesheet(rust::Str stylesheet);
void qapp_quit();

// Implemented in Rust (see bridge.rs / gui::app).
void app_action(
    HydrusMainWindow &window,
    rust::Str action,
    rust::Str target,
    rust::Str payload) noexcept;
