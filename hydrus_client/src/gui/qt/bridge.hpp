#pragma once

#include "rust/cxx.h"

#include <QApplication>
#include <QCloseEvent>
#include <QContextMenuEvent>
#include <QCheckBox>
#include <QComboBox>
#include <QDialog>
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
#include <QPlainTextEdit>
#include <QPushButton>
#include <QResizeEvent>
#include <QSpinBox>
#include <QString>
#include <QStringList>
#include <QTabWidget>
#include <QTableWidget>
#include <QTimer>
#include <QWheelEvent>
#include <QWidget>
#include <memory>

class HydrusMainWindow;
class HydrusNotebook;
class HydrusQueryPage;
class HydrusThumbnailGrid;
class HydrusViewerWindow;
class HydrusDownloaderPage;
class HydrusSubscriptionsPage;
class HydrusDuplicatesPage;
class HydrusServicesDialog;
class HydrusNetworkDialog;
class HydrusMetadataPanel;
class HydrusImportFoldersPage;
class HydrusDuplicatesRulesDialog;
class HydrusTagRelationsDialog;
class QTabBar;
class QCompleter;
class QVBoxLayout;
class QDragEnterEvent;
class QDropEvent;
class QEvent;
class HydrusTabBar;
class QMovie;
class QSystemTrayIcon;

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
    void zoom_to(double factor);
    void rotate_by(int degrees);
    void set_recenter_on_resize(bool value);
    void set_background_mode(const QString &mode);
    void set_pan_mode(bool enabled);
    void toggle_animation();
    bool is_animated() const;

protected:
    void resizeEvent(QResizeEvent *event) override;
    void wheelEvent(QWheelEvent *event) override;

private:
    QGraphicsScene *scene = nullptr;
    QGraphicsPixmapItem *pixmap_item = nullptr;
    QMovie *movie = nullptr;
    bool fit_mode = true;
    bool recenter_on_resize = true;
    int rotation = 0;
};

// One media page, laid out like Hydrus: search/preview sidebar on the left and
// the thumbnail grid on the right.
class HydrusAutocompletePopup;

class HydrusQueryPage : public QWidget {
public:
    HydrusQueryPage(HydrusMainWindow *main, const QString &page_id, const QJsonObject &state);

    const QString &page_id() const;

    void update_state(const QJsonObject &state);
    void set_selection(
        const QString &hash,
        const QStringList &tags,
        const QString &preview_path,
        const QJsonObject &metadata);
    void set_suggestions(const QStringList &suggestions);
    void show_autocomplete(const QJsonObject &state);
    void insert_autocomplete(const QString &text);

    void focus_hash(const QString &hash);
    void open_focused();
    void archive_focused();
    void inbox_focused();
    void delete_focused();

    QString focused_hash() const;

protected:
    bool eventFilter(QObject *watched, QEvent *event) override;

private:
    void request_autocomplete();

    HydrusMainWindow *main = nullptr;
    QString id;
    QComboBox *location = nullptr;
    QComboBox *tag_service = nullptr;
    QComboBox *sort = nullptr;
    QCheckBox *sort_ascending = nullptr;
    QLineEdit *search = nullptr;
    QCompleter *completer = nullptr;
    HydrusAutocompletePopup *autocomplete = nullptr;
    QString text_before_completion;
    QPushButton *search_button = nullptr;
    QLabel *status_label = nullptr;
    QLabel *empty_label = nullptr;
    HydrusMetadataPanel *metadata = nullptr;
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
    QStringList selected_hashes() const;

protected:
    void contextMenuEvent(QContextMenuEvent *event) override;
    void wheelEvent(QWheelEvent *event) override;
    void startDrag(Qt::DropActions supported_actions) override;
    void dragEnterEvent(QDragEnterEvent *event) override;
    void dragMoveEvent(QDragMoveEvent *event) override;
    void dropEvent(QDropEvent *event) override;

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
    HydrusMetadataPanel *metadata = nullptr;
    QLabel *status = nullptr;
    QPushButton *animation_button = nullptr;
    QComboBox *background = nullptr;
    QPushButton *pan_button = nullptr;
    QString file_path;
    QString url;
    double zoom_step = 1.25;
    bool always_on_top = false;
    bool frameless = false;
};

// Full Hydrus-style metadata sidebar: tag services, ratings and notes for the
// focused file. Shared by query pages and the media viewer.
class HydrusMetadataPanel : public QWidget {
public:
    explicit HydrusMetadataPanel(HydrusMainWindow *main, QWidget *parent = nullptr);

    void set_state(const QJsonObject &metadata);
    void set_suggestions(const QStringList &suggestions);

private:
    void rebuild_tags();
    void rebuild_ratings(const QJsonArray &ratings);
    void add_tag();
    void remove_selected_tags();
    void edit_notes();
    void send_tag(const QString &tag, bool add);
    void send_rating(const QString &service, const QString &action, double value = 0.0);

    HydrusMainWindow *main = nullptr;
    QString hash;
    QJsonObject data;
    QComboBox *services = nullptr;
    QLineEdit *tag_filter = nullptr;
    QListWidget *tags = nullptr;
    QCheckBox *display_tags = nullptr;
    QLineEdit *tag_input = nullptr;
    QCompleter *completer = nullptr;
    QWidget *ratings = nullptr;
    QVBoxLayout *ratings_layout = nullptr;
    QListWidget *notes = nullptr;
};

// Tag sibling/parent editor. One service at a time, like Hydrus' dialog.
class HydrusTagRelationsDialog : public QDialog {
public:
    HydrusTagRelationsDialog(HydrusMainWindow *main, const QJsonObject &state, QWidget *parent = nullptr);

    void set_state(const QJsonObject &state);

private:
    void rebuild();
    void add_relation();
    void remove_selected();
    QTableWidget *current_table() const;
    QString current_relation() const;

    HydrusMainWindow *main = nullptr;
    QJsonObject data;
    QComboBox *services = nullptr;
    QTabWidget *tabs = nullptr;
    QTableWidget *siblings = nullptr;
    QTableWidget *parents = nullptr;
};

// Import folders manager page.
class HydrusImportFoldersPage : public QWidget {
public:
    HydrusImportFoldersPage(HydrusMainWindow *main, const QString &page_id, const QJsonObject &state);

    const QString &page_id() const;
    void update_state(const QJsonObject &state);

private:
    void with_selected(const QString &action);

    HydrusMainWindow *main = nullptr;
    QString id;
    QTableWidget *table = nullptr;
    QLabel *status = nullptr;
};

// Editor for duplicate auto-resolution rules.
class HydrusDuplicatesRulesDialog : public QDialog {
public:
    explicit HydrusDuplicatesRulesDialog(HydrusMainWindow *main, const QJsonObject &state, QWidget *parent = nullptr);

    void set_state(const QJsonObject &state);

private:
    void add_rule();
    void remove_selected();

    HydrusMainWindow *main = nullptr;
    QTableWidget *table = nullptr;
    QCheckBox *enabled = nullptr;
    QJsonObject data;
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

protected:
    bool eventFilter(QObject *watched, QEvent *event) override;

private:
    void install_plus_tab();
    void show_tab_menu(const QPoint &position);
    void request_close(int index);

    HydrusMainWindow *main = nullptr;
    QString id;
    int plus_index = -1;
    int last_normal_index = 0;
};

// URL/gallery/watcher/simple downloader page.
class HydrusDownloaderPage : public QWidget {
public:
    HydrusDownloaderPage(HydrusMainWindow *main, const QString &page_id, const QJsonObject &state);

    const QString &page_id() const;
    void update_state(const QJsonObject &state);

private:
    void start();
    void retry_selected();

    HydrusMainWindow *main = nullptr;
    QString id;
    QString kind;
    QPlainTextEdit *input = nullptr;
    QWidget *input_row = nullptr;
    QLineEdit *url = nullptr;
    QLineEdit *query = nullptr;
    QComboBox *site = nullptr;
    QLineEdit *filter = nullptr;
    QLineEdit *subject = nullptr;
    QSpinBox *period = nullptr;
    QPushButton *start_button = nullptr;
    QPushButton *pause_button = nullptr;
    QPushButton *clear_button = nullptr;
    QPushButton *results_button = nullptr;
    QPushButton *auto_button = nullptr;
    QLabel *status_label = nullptr;
    QTableWidget *queue = nullptr;
    QPlainTextEdit *log = nullptr;
    QWidget *url_row = nullptr;
    QWidget *query_row = nullptr;
    QWidget *site_row = nullptr;
    QWidget *subject_row = nullptr;
    QWidget *period_row = nullptr;
};

// Subscription manager page.
class HydrusSubscriptionsPage : public QWidget {
public:
    HydrusSubscriptionsPage(HydrusMainWindow *main, const QString &page_id, const QJsonObject &state);

    const QString &page_id() const;
    void update_state(const QJsonObject &state);

private:
    void add_subscription();
    void with_selected(const QString &action);

    HydrusMainWindow *main = nullptr;
    QString id;
    QTableWidget *table = nullptr;
    QLabel *status = nullptr;
    QJsonArray sites;
};

// Side-by-side duplicate pair filter.
class HydrusDuplicatesPage : public QWidget {
public:
    HydrusDuplicatesPage(HydrusMainWindow *main, const QString &page_id, const QJsonObject &state);

    const QString &page_id() const;
    void update_state(const QJsonObject &state);

private:
    void act(const QString &action);

    HydrusMainWindow *main = nullptr;
    QString id;
    QLabel *status = nullptr;
    QLabel *distance = nullptr;
    QLabel *hash_a = nullptr;
    QLabel *hash_b = nullptr;
    QLabel *info_a = nullptr;
    QLabel *info_b = nullptr;
    HydrusCanvas *canvas_a = nullptr;
    HydrusCanvas *canvas_b = nullptr;
    QListWidget *tags_a = nullptr;
    QListWidget *tags_b = nullptr;
    QListWidget *urls_a = nullptr;
    QListWidget *urls_b = nullptr;
    QPushButton *keep_a = nullptr;
    QPushButton *keep_b = nullptr;
    QPushButton *same = nullptr;
    QPushButton *not_duplicates = nullptr;
    QPushButton *skip = nullptr;
    QPushButton *refresh = nullptr;
    QPushButton *delete_both = nullptr;
    QPushButton *rules = nullptr;
};

// Non-modal service manager. It is updated by `ServicesDialog` commands.
class HydrusServicesDialog : public QDialog {
public:
    explicit HydrusServicesDialog(HydrusMainWindow *main, const QJsonObject &state, QWidget *parent = nullptr);

    void set_state(const QJsonObject &state);

private:
    void add_service();
    void rename_selected();
    void delete_selected();
    QString selected_key() const;

    HydrusMainWindow *main = nullptr;
    QJsonObject data;
    QTableWidget *table = nullptr;
    QComboBox *type_combo = nullptr;
};

// Non-modal network manager.
class HydrusNetworkDialog : public QDialog {
public:
    explicit HydrusNetworkDialog(HydrusMainWindow *main, const QJsonObject &state, QWidget *parent = nullptr);

    void set_state(const QJsonObject &state);

private:
    void save();
    void add_header();
    void remove_header();
    void add_url_class();
    void edit_url_class();
    void remove_url_class();
    void add_pool();
    void remove_pool();
    void add_bandwidth_rule();
    void remove_bandwidth_rule();

    HydrusMainWindow *main = nullptr;
    QJsonObject data;
    QLineEdit *http_proxy = nullptr;
    QLineEdit *https_proxy = nullptr;
    QLineEdit *no_proxy = nullptr;
    QSpinBox *timeout = nullptr;
    QSpinBox *max_jobs = nullptr;
    QSpinBox *max_jobs_domain = nullptr;
    QCheckBox *api_enabled = nullptr;
    QSpinBox *api_port = nullptr;
    QLineEdit *api_key = nullptr;
    QLabel *api_status = nullptr;
    QCheckBox *paused = nullptr;
    QTableWidget *headers = nullptr;
    QTableWidget *url_classes = nullptr;
    QTableWidget *pools = nullptr;
    QTableWidget *bandwidth_rules = nullptr;
};

class HydrusMainWindow : public QMainWindow {
public:
    explicit HydrusMainWindow(QWidget *parent = nullptr);

    void build_shell();
    void set_status(const QJsonObject &status);
    void set_menus(const QJsonArray &menus);
    void build_pages(const QJsonObject &pages);
    void update_page(const QString &page_id, const QJsonObject &state);
    void update_downloader(const QString &page_id, const QJsonObject &state);
    void update_subscriptions(const QString &page_id, const QJsonObject &state);
    void update_duplicates(const QString &page_id, const QJsonObject &state);
    void update_import_folders(const QString &page_id, const QJsonObject &state);
    void set_selection(
        const QString &page_id,
        const QString &hash,
        const QStringList &tags,
        const QString &preview_path,
        const QJsonObject &metadata);
    void set_tag_suggestions(const QStringList &suggestions);
    const QStringList &suggestions() const;
    void show_autocomplete(const QJsonObject &state);
    void show_page_chooser(const QString &notebook_id);
    void prompt_url();
    void open_viewer(const QJsonObject &state);
    void update_viewer(const QString &viewer_id, const QJsonObject &state);
    void close_viewer(const QString &viewer_id);
    void show_tag_dialog(const QJsonObject &state);
    void show_notes(const QJsonObject &state);
    void show_duplicates_rules(const QJsonObject &state);
    void show_tag_relations(const QJsonObject &state);
    void show_import_folder(const QJsonObject &state);
    void show_options(const QJsonObject &state);
    void show_services(const QJsonObject &state);
    void update_services(const QJsonObject &state);
    void show_network(const QJsonObject &state);
    void update_network(const QJsonObject &state);
    void start_tick();
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
    void closeEvent(QCloseEvent *event) override;

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
    QHash<QString, HydrusDownloaderPage *> downloaders;
    QHash<QString, HydrusSubscriptionsPage *> subscriptions;
    QHash<QString, HydrusDuplicatesPage *> duplicates;
    QHash<QString, HydrusImportFoldersPage *> import_folders;
    QHash<QString, HydrusNotebook *> notebooks;
    QHash<QString, HydrusViewerWindow *> viewers;
    HydrusServicesDialog *services_dialog = nullptr;
    HydrusNetworkDialog *network_dialog = nullptr;
    HydrusDuplicatesRulesDialog *duplicates_rules_dialog = nullptr;
    HydrusTagRelationsDialog *tag_relations_dialog = nullptr;
    QTimer *tick_timer = nullptr;
    QSystemTrayIcon *tray = nullptr;
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
void hydrus_show_autocomplete(HydrusMainWindow &window, rust::Str json);
void hydrus_set_status(HydrusMainWindow &window, rust::Str json);
void hydrus_build_pages(HydrusMainWindow &window, rust::Str json);
void hydrus_update_page(HydrusMainWindow &window, rust::Str page_id, rust::Str json);
void hydrus_update_downloader(HydrusMainWindow &window, rust::Str page_id, rust::Str json);
void hydrus_update_subscriptions(HydrusMainWindow &window, rust::Str page_id, rust::Str json);
void hydrus_update_duplicates(HydrusMainWindow &window, rust::Str page_id, rust::Str json);
void hydrus_update_import_folders(HydrusMainWindow &window, rust::Str page_id, rust::Str json);
void hydrus_show_duplicates_rules(HydrusMainWindow &window, rust::Str json);
void hydrus_show_tag_relations(HydrusMainWindow &window, rust::Str json);
void hydrus_show_import_folder(HydrusMainWindow &window, rust::Str json);
void hydrus_show_notes(HydrusMainWindow &window, rust::Str json);
void hydrus_set_selection(
    HydrusMainWindow &window,
    rust::Str page_id,
    rust::Str hash,
    rust::Str tags_json,
    rust::Str preview_path,
    rust::Str metadata_json);
void hydrus_show_page_chooser(HydrusMainWindow &window, rust::Str notebook_id);
void hydrus_show_page_manager(HydrusMainWindow &window, rust::Str json);
void hydrus_show_command_palette(HydrusMainWindow &window);
void hydrus_prompt_text(
    HydrusMainWindow &window,
    rust::Str action,
    rust::Str title,
    rust::Str label,
    rust::Str initial);
void hydrus_confirm_action(
    HydrusMainWindow &window,
    rust::Str action,
    rust::Str target,
    rust::Str title,
    rust::Str text);
void hydrus_show_or_dialog(
    HydrusMainWindow &window,
    rust::Str page_id,
    rust::Str initial);
void hydrus_pick_files(HydrusMainWindow &window);
void hydrus_prompt_url(HydrusMainWindow &window);
void hydrus_open_viewer(HydrusMainWindow &window, rust::Str json);
void hydrus_update_viewer(HydrusMainWindow &window, rust::Str viewer_id, rust::Str json);
void hydrus_close_viewer(HydrusMainWindow &window, rust::Str viewer_id);
void hydrus_show_tag_dialog(HydrusMainWindow &window, rust::Str json);
void hydrus_show_manage_tags(HydrusMainWindow &window, rust::Str json);
void hydrus_show_timestamps(HydrusMainWindow &window, rust::Str json);
void hydrus_show_options(HydrusMainWindow &window, rust::Str json);
void hydrus_show_services(HydrusMainWindow &window, rust::Str json);
void hydrus_update_services(HydrusMainWindow &window, rust::Str json);
void hydrus_show_network(HydrusMainWindow &window, rust::Str json);
void hydrus_update_network(HydrusMainWindow &window, rust::Str json);
void hydrus_start_tick(HydrusMainWindow &window);
void hydrus_defer_action(
    HydrusMainWindow &window,
    rust::Str action,
    rust::Str target,
    rust::Str payload,
    int delay_ms);
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

void app_tick(HydrusMainWindow &window) noexcept;
