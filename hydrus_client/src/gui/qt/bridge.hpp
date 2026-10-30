#pragma once

#include "rust/cxx.h"

#include <QApplication>
#include <QCloseEvent>
#include <QContextMenuEvent>
#include <QCheckBox>
#include <QComboBox>
#include <QDialog>
#include <QFrame>
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
#include <QProgressBar>
#include <QPushButton>
#include <QResizeEvent>
#include <QScrollArea>
#include <QSpinBox>
#include <QSplashScreen>
#include <QString>
#include <QStringList>
#include <QTabWidget>
#include <QTableWidget>
#include <QTimer>
#include <QToolButton>
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
class HydrusApiPermissionsDialog;
class HydrusAccountTypesDialog;
class HydrusOrphanFilesDialog;
class HydrusDatabaseBackupDialog;
class HydrusNetworkDialog;
class HydrusMetadataPanel;
class HydrusImportFoldersPage;
class HydrusExportFoldersPage;
class HydrusDuplicatesRulesDialog;
class HydrusAutoResolutionRuleDialog;
class HydrusDuplicateReviewDialog;
class HydrusDuplicateMergeOptionsDialog;
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
    void update_progress(const QJsonObject &state);
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
    void resizeEvent(QResizeEvent *event) override;

private:
    void request_autocomplete();

    HydrusMainWindow *main = nullptr;
    QString id;
    QString location_value;
    QString tag_service_value;
    QStringList tag_services;
    QString sort_value;
    bool sort_ascending_value = true;
    QPushButton *sort_type = nullptr;
    QPushButton *sort_order = nullptr;
    QComboBox *collect = nullptr;
    QToolButton *collect_cog = nullptr;
    QScrollArea *sidebar_scroll = nullptr;
    QWidget *sidebar_content = nullptr;
    QPushButton *location = nullptr;
    QPushButton *tag_service = nullptr;
    QLineEdit *search = nullptr;
    QCompleter *completer = nullptr;
    HydrusAutocompletePopup *autocomplete = nullptr;
    QString text_before_completion;
    QListWidget *predicates = nullptr;
    QPushButton *include_current = nullptr;
    QPushButton *include_pending = nullptr;
    QPushButton *search_pause_play = nullptr;
    QPushButton *or_button = nullptr;
    QPushButton *search_expand = nullptr;
    QWidget *search_box = nullptr;
    QWidget *search_content = nullptr;
    QTabWidget *autocomplete_tabs = nullptr;
    QListWidget *results_list = nullptr;
    QListWidget *favourites_list = nullptr;
    QListWidget *children_list = nullptr;
    QPushButton *tag_sort_type = nullptr;
    QPushButton *tag_sort_order = nullptr;
    QPushButton *tag_sort_namespace = nullptr;
    QFrame *selection_tags_box = nullptr;
    QWidget *sidebar_filler = nullptr;
    QListWidget *selection_tags = nullptr;
    QFrame *preview_frame = nullptr;
    QWidget *results = nullptr;
    QWidget *loading_panel = nullptr;
    QLabel *loading_text = nullptr;
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
    void set_layout(bool masonry, bool horizontal, int scale_percent);
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
    bool masonry = false;
    bool horizontal = false;
    int scale_percent = 100;
};

// Media viewer frame: canvas plus a tag sidebar, as in Hydrus' canvas frame.
class HydrusViewerWindow : public QWidget {
public:
    HydrusViewerWindow(HydrusMainWindow *main, const QJsonObject &state);

    const QString &viewer_id() const;

    void update_state(const QJsonObject &state);

protected:
    bool eventFilter(QObject *watched, QEvent *event) override;
    void closeEvent(QCloseEvent *event) override;

private:
    void navigate(const QString &direction);
    void show_hover_frame();

    HydrusMainWindow *main = nullptr;
    QString id;
    QString page_id;
    QString hash;
    HydrusCanvas *canvas = nullptr;
    QWidget *hover_frame = nullptr;
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

// Export folders manager page.
class HydrusExportFoldersPage : public QWidget {
public:
    HydrusExportFoldersPage(HydrusMainWindow *main, const QString &page_id, const QJsonObject &state);

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
    void set_preview(const QJsonObject &preview);

private:
    void add_rule();
    void edit_selected();
    void duplicate_selected();
    void remove_selected();
    void open_review();
    void edit_rule(const QJsonObject &rule);

    HydrusMainWindow *main = nullptr;
    QTableWidget *table = nullptr;
    QCheckBox *enabled = nullptr;
    QJsonObject data;
    HydrusAutoResolutionRuleDialog *rule_editor = nullptr;
};

// Edit one duplicate auto-resolution rule: search, comparison and action.
class HydrusAutoResolutionRuleDialog : public QDialog {
public:
    HydrusAutoResolutionRuleDialog(
        HydrusMainWindow *main,
        HydrusDuplicatesRulesDialog *rules_dialog,
        const QJsonObject &rule,
        const QJsonObject &catalog,
        QWidget *parent = nullptr);

    void set_preview(const QJsonObject &preview);

private:
    QJsonObject rule_json() const;
    void refresh_comparators();
    void add_comparator();
    void add_group();
    void edit_selected_comparator();
    void remove_selected_comparator();
    void run_preview();
    void save();

    HydrusMainWindow *main = nullptr;
    HydrusDuplicatesRulesDialog *rules_dialog = nullptr;
    QJsonObject catalog;
    QString original_name;
    QJsonArray selector;
    QLineEdit *name = nullptr;
    QCheckBox *paused = nullptr;
    QComboBox *operation = nullptr;
    QSpinBox *max_pending = nullptr;
    QLineEdit *search_query = nullptr;
    QComboBox *search_mode = nullptr;
    QSpinBox *max_distance = nullptr;
    QListWidget *comparators = nullptr;
    QComboBox *action = nullptr;
    QCheckBox *delete_a = nullptr;
    QCheckBox *delete_b = nullptr;
    QLabel *preview_label = nullptr;
    QTableWidget *preview_pass = nullptr;
    QTableWidget *preview_fail = nullptr;
};

// Pending, denied and actioned auto-resolution pairs.
class HydrusDuplicateReviewDialog : public QDialog {
public:
    explicit HydrusDuplicateReviewDialog(HydrusMainWindow *main, const QJsonObject &state, QWidget *parent = nullptr);

    void set_state(const QJsonObject &state);

private:
    void apply_selected(bool approve);
    void apply_all(bool approve);
    QList<int> selected_pending_rows() const;

    HydrusMainWindow *main = nullptr;
    QTabWidget *tabs = nullptr;
    QTableWidget *pending = nullptr;
    QTableWidget *denied = nullptr;
    QTableWidget *actioned = nullptr;
    QLabel *status = nullptr;
};

// One page of duplicate content merge options. Both the global merge options
// dialog and the per-decision custom action dialog share it.
struct DuplicateMergePage {
    QComboBox *archive = nullptr;
    QComboBox *modified_date = nullptr;
    QComboBox *urls = nullptr;
    QComboBox *notes = nullptr;
    QTableWidget *tags = nullptr;
    QTableWidget *ratings = nullptr;
};

void build_duplicate_merge_page(
    QWidget *parent,
    QTabWidget *tabs,
    const QJsonObject &data,
    const QString &title,
    DuplicateMergePage &page);
void fill_duplicate_merge_page(
    DuplicateMergePage &page,
    const QJsonObject &data,
    const QJsonObject &options);
QJsonObject read_duplicate_merge_page(const DuplicateMergePage &page);

// Which metadata a duplicate merge copies from the worse file, for each
// duplicate action ("this is better" and "same quality").
class HydrusDuplicateMergeOptionsDialog : public QDialog {
public:
    explicit HydrusDuplicateMergeOptionsDialog(HydrusMainWindow *main, const QJsonObject &state, QWidget *parent = nullptr);

    void set_state(const QJsonObject &state);

private:
    void save();

    HydrusMainWindow *main = nullptr;
    QJsonObject data;
    DuplicateMergePage better;
    DuplicateMergePage same;
};

// Per-decision version of the duplicate action menu: choose the action, which
// file is better, what to delete, and merge options only for this pair.
class HydrusDuplicateCustomActionDialog : public QDialog {
public:
    explicit HydrusDuplicateCustomActionDialog(HydrusMainWindow *main, const QJsonObject &state, QWidget *parent = nullptr);

private:
    void apply();

    HydrusMainWindow *main = nullptr;
    QJsonObject data;
    QComboBox *action = nullptr;
    QComboBox *better_side = nullptr;
    QComboBox *delete_mode = nullptr;
    QTabWidget *tabs = nullptr;
    DuplicateMergePage better;
    DuplicateMergePage same;
};

// Editor for the import options containers: per-caller defaults and named
// favourites. One tab per option type, with a "use default" switch so a
// container only overrides what it sets.
class HydrusImportOptionsDialog : public QDialog {
public:
    explicit HydrusImportOptionsDialog(HydrusMainWindow *main, const QJsonObject &state, QWidget *parent = nullptr);

private:
    struct Editor {
        QTabWidget *tabs = nullptr;
        QHash<QString, QCheckBox *> use_default;
        QHash<QString, QWidget *> content;

        QComboBox *prefetch_hash = nullptr;
        QComboBox *prefetch_url = nullptr;
        QCheckBox *prefetch_spam = nullptr;
        QCheckBox *prefetch_meta_hash = nullptr;
        QCheckBox *prefetch_meta_url = nullptr;

        QSpinBox *min_size = nullptr;
        QSpinBox *max_size = nullptr;
        QSpinBox *min_width = nullptr;
        QSpinBox *min_height = nullptr;
        QSpinBox *max_width = nullptr;
        QSpinBox *max_height = nullptr;
        QSpinBox *max_gif_size = nullptr;
        QCheckBox *exclude_deleted = nullptr;
        QCheckBox *allow_bombs = nullptr;
        QListWidget *mimes = nullptr;

        QTableWidget *tag_rules = nullptr;
        QPlainTextEdit *tag_whitelist = nullptr;

        QListWidget *destinations = nullptr;
        QCheckBox *auto_archive = nullptr;
        QCheckBox *assoc_primary = nullptr;
        QCheckBox *assoc_source = nullptr;
        QCheckBox *archive_existing = nullptr;
        QCheckBox *destinations_existing = nullptr;

        QTableWidget *tag_services = nullptr;

        QCheckBox *get_notes = nullptr;
        QCheckBox *extend_notes = nullptr;
        QComboBox *note_conflict = nullptr;
        QPlainTextEdit *note_whitelist = nullptr;
        QLineEdit *note_all_override = nullptr;
        QPlainTextEdit *note_overrides = nullptr;

        QComboBox *presentation_status = nullptr;
        QComboBox *presentation_inbox = nullptr;

        QTableWidget *external_programs = nullptr;
    };

    void build_editor();
    void rebuild_list(const QString &select_key = QString());
    void select_key(const QString &key);
    QString current_key() const;
    void store_current();
    void fill_editor(const QJsonObject &container, const QJsonObject &fallback);
    QJsonObject read_editor() const;
    QJsonObject *container_for(const QString &key);
    void add_favourite();
    void rename_favourite();
    void delete_favourite();
    void save();

    HydrusMainWindow *main = nullptr;
    QJsonObject data;
    QWidget *left_panel = nullptr;
    QListWidget *list = nullptr;
    QLabel *description = nullptr;
    QPushButton *rename_button = nullptr;
    QPushButton *delete_button = nullptr;
    Editor editor;
    bool rebuilding = false;
    bool specific_mode = false;
    QString selected_key;
};

// File seed cache (urls/gallery/simple pages) and gallery seed log. Both share
// the same table; the gallery log reads the page's gallery seeds instead of the
// file queue.
class HydrusSeedCacheDialog : public QDialog {
public:
    HydrusSeedCacheDialog(HydrusMainWindow *main, const QJsonObject &state, bool gallery, QWidget *parent = nullptr);

    void set_state(const QJsonObject &state);

private:
    void rebuild();
    void send_action(const QString &action);
    QStringList selected_urls() const;

    HydrusMainWindow *main = nullptr;
    bool gallery = false;
    QJsonObject data;
    QTableWidget *table = nullptr;
    QComboBox *filter = nullptr;
    QLineEdit *search = nullptr;
    QLabel *subtitle = nullptr;
    QPushButton *retry_button = nullptr;
};

// Local import review panel: parse paths, show good/bad/sidecar rows, then
// import the good ones (optionally deleting the originals).
class HydrusLocalImportReviewDialog : public QDialog {
public:
    HydrusLocalImportReviewDialog(HydrusMainWindow *main, const QJsonObject &state, QWidget *parent = nullptr);

    void set_state(const QJsonObject &state);

private:
    void rebuild();
    void send(const QString &action, const QJsonObject &payload = QJsonObject());
    QStringList selected_paths() const;

    HydrusMainWindow *main = nullptr;
    QJsonObject data;
    QTableWidget *table = nullptr;
    QLabel *summary = nullptr;
    QLabel *warning = nullptr;
    QCheckBox *search = nullptr;
    QCheckBox *delete_after_success = nullptr;
    QPushButton *import_button = nullptr;
    QPushButton *tag_button = nullptr;
};

// Per-local-tag-service filename tagging rules for the local import review.
// Shares the rule editor with the import folder dialog.
class HydrusLocalImportTaggingDialog : public QDialog {
public:
    HydrusLocalImportTaggingDialog(HydrusMainWindow *main, const QJsonObject &state, QWidget *parent = nullptr);

    void set_state(const QJsonObject &state);

private:
    void rebuild();
    void edit_rules(const QString &key, const QString &name, const QJsonObject &current);
    void save();

    HydrusMainWindow *main = nullptr;
    QJsonObject data;
    QJsonObject filename_tagging;
    QHash<QString, QString> service_name_to_key;
    QListWidget *list = nullptr;
    QLabel *help = nullptr;
    QString example_path;
    bool import_after_save = false;
};

// Edit one subscription's site, query, filter and timing.
class HydrusSubscriptionEditorDialog : public QDialog {
public:
    HydrusSubscriptionEditorDialog(HydrusMainWindow *main, const QJsonObject &state, QWidget *parent = nullptr);

    void set_state(const QJsonObject &state);

private:
    void save();

    HydrusMainWindow *main = nullptr;
    QJsonObject data;
    QString page_id;
    QLineEdit *name = nullptr;
    QLineEdit *url = nullptr;
    QLineEdit *query = nullptr;
    QComboBox *site = nullptr;
    QLineEdit *filter = nullptr;
    QSpinBox *period = nullptr;
    QCheckBox *paused = nullptr;
};

// Remap missing media/thumbnail subfolder roots.
class HydrusRepairFileLocationsDialog : public QDialog {
public:
    HydrusRepairFileLocationsDialog(HydrusMainWindow *main, const QJsonObject &state, QWidget *parent = nullptr);

    void set_state(const QJsonObject &state);

private:
    void rebuild();
    void send(const QString &action, const QJsonObject &payload = QJsonObject());
    QStringList selected_prefixes() const;

    HydrusMainWindow *main = nullptr;
    QJsonObject data;
    QTableWidget *table = nullptr;
    QLabel *summary = nullptr;
    QLabel *message = nullptr;
    QCheckBox *create_thumbnails = nullptr;
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
    void fill_sites(const QJsonArray &sites);
    void apply_file_limit(int value, bool no_limit);

    HydrusMainWindow *main = nullptr;
    QString id;
    QString kind;
    QJsonArray sites;
    int file_limit_value = 2000;
    bool no_limit_value = false;
    bool site_loaded = false;

    QScrollArea *sidebar_scroll = nullptr;
    QWidget *sidebar_content = nullptr;

    QPushButton *sort_type = nullptr;
    QPushButton *sort_order = nullptr;
    QComboBox *collect = nullptr;
    QToolButton *collect_cog = nullptr;

    QFrame *main_box = nullptr;
    QWidget *main_content = nullptr;
    QLabel *status_top = nullptr;
    QLabel *status_bottom = nullptr;
    QTableWidget *queue = nullptr;
    QWidget *query_row = nullptr;
    QLineEdit *query = nullptr;
    QWidget *url_row = nullptr;
    QLineEdit *url = nullptr;
    QWidget *subject_row = nullptr;
    QLineEdit *subject = nullptr;
    QWidget *period_row = nullptr;
    QSpinBox *period = nullptr;
    QWidget *site_row = nullptr;
    QComboBox *site = nullptr;
    QPushButton *gug_button = nullptr;
    QWidget *file_limit_row = nullptr;
    QSpinBox *file_limit = nullptr;
    QCheckBox *no_limit = nullptr;
    QPlainTextEdit *input = nullptr;
    QWidget *input_row = nullptr;
    QLineEdit *url_input = nullptr;
    QToolButton *url_paste = nullptr;
    QToolButton *file_cancel = nullptr;
    QToolButton *search_cancel = nullptr;
    QListWidget *simple_jobs = nullptr;
    QLineEdit *simple_url = nullptr;
    QToolButton *simple_paste = nullptr;
    QComboBox *simple_formula = nullptr;
    QToolButton *simple_formula_cog = nullptr;
    QPushButton *start_button = nullptr;
    QPushButton *auto_button = nullptr;
    QPushButton *import_options_button = nullptr;

    QFrame *highlight_box = nullptr;
    QWidget *highlight_content = nullptr;
    QLineEdit *highlight_query = nullptr;
    QWidget *highlight_limit_row = nullptr;
    QSpinBox *highlight_file_limit = nullptr;
    QCheckBox *highlight_no_limit = nullptr;
    QPushButton *highlight_import_options = nullptr;

    QFrame *imports_box = nullptr;
    QLabel *imports_status = nullptr;
    QLabel *file_seed_status = nullptr;
    QToolButton *files_pause = nullptr;
    QToolButton *file_log_button = nullptr;
    QProgressBar *file_progress = nullptr;
    QLabel *file_percent = nullptr;
    QProgressBar *file_network_progress = nullptr;
    QLabel *file_network_percent = nullptr;
    QToolButton *file_cog = nullptr;

    QFrame *search_box = nullptr;
    QLabel *search_status = nullptr;
    QLabel *search_seed_status = nullptr;
    QToolButton *search_pause = nullptr;
    QToolButton *search_log_button = nullptr;
    QProgressBar *search_progress = nullptr;
    QLabel *search_percent = nullptr;
    QProgressBar *search_network_progress = nullptr;
    QLabel *search_network_percent = nullptr;
    QToolButton *search_cog = nullptr;

    QFrame *selection_tags_box = nullptr;
    QListWidget *selection_tags = nullptr;
    QPushButton *tag_sort_type = nullptr;
    QPushButton *tag_sort_order = nullptr;
    QPushButton *tag_sort_namespace = nullptr;

    QWidget *results = nullptr;
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
    QPushButton *import_options_button = nullptr;
};

// Side-by-side duplicate pair filter.
class HydrusDuplicatesPage : public QWidget {
public:
    HydrusDuplicatesPage(HydrusMainWindow *main, const QString &page_id, const QJsonObject &state);

    const QString &page_id() const;
    void update_state(const QJsonObject &state);

private:
    void act(const QString &action);
    void notify_search_context();
    void update_search_controls();
    void show_group_actions(const QString &target, const QPoint &global_pos);

    HydrusMainWindow *main = nullptr;
    QString id;
    QLabel *status = nullptr;
    QLabel *distance = nullptr;
    QComboBox *dupe_search_type = nullptr;
    QLineEdit *search_query_1 = nullptr;
    QLineEdit *search_query_2 = nullptr;
    QComboBox *search_location_1 = nullptr;
    QComboBox *search_location_2 = nullptr;
    QLabel *search_row_2_label = nullptr;
    QComboBox *pixel_dupes = nullptr;
    QSpinBox *search_distance = nullptr;
    QComboBox *sort_type = nullptr;
    QComboBox *sort_ascending = nullptr;
    QLabel *search_summary = nullptr;
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
    QPushButton *keep_a_both = nullptr;
    QPushButton *keep_b = nullptr;
    QPushButton *keep_b_both = nullptr;
    QPushButton *same = nullptr;
    QPushButton *not_duplicates = nullptr;
    QPushButton *alternates = nullptr;
    QPushButton *custom_action = nullptr;
    QPushButton *group_actions = nullptr;
    QPushButton *skip = nullptr;
    QPushButton *refresh = nullptr;
    QPushButton *delete_both = nullptr;
    QPushButton *rules = nullptr;
    QPushButton *merge_options = nullptr;
    QSpinBox *batch = nullptr;
    QPushButton *next_batch = nullptr;
};

// Non-modal service manager. It is updated by `ServicesDialog` commands.
class HydrusServicesDialog : public QDialog {
public:
    explicit HydrusServicesDialog(HydrusMainWindow *main, const QJsonObject &state, QWidget *parent = nullptr);

    void set_state(const QJsonObject &state);

private:
    void add_service();
    void edit_selected();
    void delete_selected();
    void show_specifier();
    void edit_login();
    void remove_login();
    void connect_admin();
    void select_remote_service();
    void add_remote_service();
    void edit_remote_service();
    void remove_remote_service();
    void remote_service_action();
    void account_action();
    QString selected_key() const;
    QJsonObject selected_remote_service() const;
    QString selected_remote_key() const;

    HydrusMainWindow *main = nullptr;
    QJsonObject data;
    QTabWidget *tabs = nullptr;
    QTableWidget *table = nullptr;
    QTableWidget *remote_services = nullptr;
    QTableWidget *accounts = nullptr;
    QTableWidget *logins = nullptr;
    QLabel *admin_status = nullptr;
    QLineEdit *admin_address = nullptr;
    QLineEdit *admin_key = nullptr;
    QLineEdit *remote_key = nullptr;
    bool opened_account_types = false;
};

// Non-modal network manager.
// Non-modal Client API permission manager.
class HydrusApiPermissionsDialog : public QDialog {
public:
    explicit HydrusApiPermissionsDialog(HydrusMainWindow *main, const QJsonObject &state, QWidget *parent = nullptr);

    void set_state(const QJsonObject &state);

private:
    void add_entry();
    void edit_selected();
    void delete_selected();
    QJsonObject selected_entry() const;

    HydrusMainWindow *main = nullptr;
    QJsonObject data;
    QTableWidget *table = nullptr;
    QLabel *status = nullptr;
};

// Non-modal account type manager for the selected server service.
class HydrusAccountTypesDialog : public QDialog {
public:
    explicit HydrusAccountTypesDialog(HydrusMainWindow *main, const QJsonObject &state, QWidget *parent = nullptr);

    void set_state(const QJsonObject &state);

private:
    void add_entry();
    void edit_selected();
    void delete_selected();
    QJsonObject selected_entry() const;

    HydrusMainWindow *main = nullptr;
    QJsonObject data;
    QTableWidget *table = nullptr;
};

// Non-modal surplus file cleaner for the media storage.
class HydrusOrphanFilesDialog : public QDialog {
public:
    explicit HydrusOrphanFilesDialog(HydrusMainWindow *main, const QJsonObject &state, QWidget *parent = nullptr);

    void set_state(const QJsonObject &state);

private:
    void delete_selected(bool all);
    void move_selected(bool all);
    QStringList selected_paths(bool all) const;

    HydrusMainWindow *main = nullptr;
    QJsonObject data;
    QTableWidget *table = nullptr;
    QLabel *status = nullptr;
};

// Non-modal database backup dialog.
class HydrusDatabaseBackupDialog : public QDialog {
public:
    explicit HydrusDatabaseBackupDialog(HydrusMainWindow *main, const QJsonObject &state, QWidget *parent = nullptr);

    void set_state(const QJsonObject &state);

private:
    void choose_path();

    HydrusMainWindow *main = nullptr;
    QLineEdit *path = nullptr;
    QLabel *last_backup = nullptr;
    QLabel *status = nullptr;
};

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
    void update_page_progress(const QString &page_id, const QJsonObject &state);
    void update_downloader(const QString &page_id, const QJsonObject &state);
    void update_subscriptions(const QString &page_id, const QJsonObject &state);
    void show_subscription_editor(const QJsonObject &state);
    void update_duplicates(const QString &page_id, const QJsonObject &state);
    void update_import_folders(const QString &page_id, const QJsonObject &state);
    void update_export_folders(const QString &page_id, const QJsonObject &state);
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
    void show_notifications(const QJsonObject &state);
    void show_viewing_stats(const QJsonObject &state);
    void show_about(const QJsonObject &state);
    void show_notes(const QJsonObject &state);
    void show_duplicates_rules(const QJsonObject &state);
    void show_duplicate_merge_options(const QJsonObject &state);
    void show_duplicate_custom_action(const QJsonObject &state);
    void show_import_options(const QJsonObject &state);
    void show_file_seed_cache(const QJsonObject &state);
    void show_gallery_seed_log(const QJsonObject &state);
    void update_auto_resolution_preview(const QJsonObject &state);
    void show_duplicate_review(const QJsonObject &state);
    void update_duplicate_review(const QJsonObject &state);
    void show_tag_relations(const QJsonObject &state);
    void show_import_folder(const QJsonObject &state);
    void show_local_import_review(const QJsonObject &state);
    void show_local_import_tagging(const QJsonObject &state);
    void close_local_import_review();
    void show_repair_file_locations(const QJsonObject &state);
    void close_repair_file_locations();
    void show_export_folder(const QJsonObject &state);
    void show_export_review(const QJsonObject &state);
    void show_options(const QJsonObject &state);
    void show_services(const QJsonObject &state);
    void update_services(const QJsonObject &state);
    void show_tag_filter(const QJsonObject &state);
    void show_api_permissions(const QJsonObject &state);
    void update_api_permissions(const QJsonObject &state);
    void show_account_types(const QJsonObject &state);
    void update_account_types(const QJsonObject &state);
    void show_orphan_files(const QJsonObject &state);
    void update_orphan_files(const QJsonObject &state);
    void show_database_backup(const QJsonObject &state);
    void update_database_backup(const QJsonObject &state);
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
    QLabel *status_media = nullptr;
    QLabel *status_bandwidth = nullptr;
    QLabel *status_idle = nullptr;
    QLabel *status_hydrus_busy = nullptr;
    QLabel *status_cpu_busy = nullptr;
    QLabel *status_db = nullptr;

    QHash<QString, HydrusQueryPage *> pages;
    QHash<QString, HydrusDownloaderPage *> downloaders;
    QHash<QString, HydrusSubscriptionsPage *> subscriptions;
    QHash<QString, HydrusDuplicatesPage *> duplicates;
    QHash<QString, HydrusImportFoldersPage *> import_folders;
    QHash<QString, HydrusExportFoldersPage *> export_folders;
    QHash<QString, HydrusNotebook *> notebooks;
    QHash<QString, HydrusViewerWindow *> viewers;
    HydrusServicesDialog *services_dialog = nullptr;
    HydrusApiPermissionsDialog *api_permissions_dialog = nullptr;
    HydrusAccountTypesDialog *account_types_dialog = nullptr;
    HydrusOrphanFilesDialog *orphan_files_dialog = nullptr;
    HydrusDatabaseBackupDialog *database_backup_dialog = nullptr;
    HydrusNetworkDialog *network_dialog = nullptr;
    HydrusDuplicatesRulesDialog *duplicates_rules_dialog = nullptr;
    HydrusDuplicateMergeOptionsDialog *merge_options_dialog = nullptr;
    HydrusDuplicateCustomActionDialog *duplicate_custom_action_dialog = nullptr;
    HydrusImportOptionsDialog *import_options_dialog = nullptr;
    HydrusSeedCacheDialog *file_seed_cache_dialog = nullptr;
    HydrusSeedCacheDialog *gallery_seed_log_dialog = nullptr;
    HydrusLocalImportReviewDialog *local_import_review_dialog = nullptr;
    HydrusLocalImportTaggingDialog *local_import_tagging_dialog = nullptr;
    HydrusRepairFileLocationsDialog *repair_file_locations_dialog = nullptr;
    HydrusSubscriptionEditorDialog *subscription_editor_dialog = nullptr;
    HydrusDuplicateReviewDialog *duplicate_review_dialog = nullptr;
    HydrusTagRelationsDialog *tag_relations_dialog = nullptr;
    QTimer *tick_timer = nullptr;
    QSystemTrayIcon *tray = nullptr;
    QStringList tag_suggestions;
};

std::unique_ptr<QApplication> new_application();
int exec_application(QApplication &application);
void quit_application(QApplication &application);
void qapp_process_events();

std::unique_ptr<QSplashScreen> new_hydrus_splash();
void hydrus_splash_show(QSplashScreen &splash);
void hydrus_splash_message(QSplashScreen &splash, rust::Str message);
void hydrus_splash_finish(QSplashScreen &splash);
void hydrus_splash_finish_later(QSplashScreen &splash, int delay_ms);
void hydrus_splash_screenshot(QSplashScreen &splash, rust::Str path, int delay_ms);

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
void hydrus_update_page_progress(
    HydrusMainWindow &window,
    rust::Str page_id,
    rust::Str json);
void hydrus_update_downloader(HydrusMainWindow &window, rust::Str page_id, rust::Str json);
void hydrus_update_subscriptions(HydrusMainWindow &window, rust::Str page_id, rust::Str json);
void hydrus_show_subscription_editor(HydrusMainWindow &window, rust::Str json);
void hydrus_update_duplicates(HydrusMainWindow &window, rust::Str page_id, rust::Str json);
void hydrus_update_import_folders(HydrusMainWindow &window, rust::Str page_id, rust::Str json);
void hydrus_update_export_folders(HydrusMainWindow &window, rust::Str page_id, rust::Str json);
void hydrus_show_duplicates_rules(HydrusMainWindow &window, rust::Str json);
void hydrus_show_duplicate_merge_options(HydrusMainWindow &window, rust::Str json);
void hydrus_show_duplicate_custom_action(HydrusMainWindow &window, rust::Str json);
void hydrus_show_import_options(HydrusMainWindow &window, rust::Str json);
void hydrus_show_file_seed_cache(HydrusMainWindow &window, rust::Str json);
void hydrus_show_gallery_seed_log(HydrusMainWindow &window, rust::Str json);
void hydrus_update_auto_resolution_preview(HydrusMainWindow &window, rust::Str json);
void hydrus_show_duplicate_review(HydrusMainWindow &window, rust::Str json);
void hydrus_update_duplicate_review(HydrusMainWindow &window, rust::Str json);
void hydrus_show_tag_relations(HydrusMainWindow &window, rust::Str json);
void hydrus_show_import_folder(HydrusMainWindow &window, rust::Str json);
void hydrus_show_local_import_review(HydrusMainWindow &window, rust::Str json);
void hydrus_show_local_import_tagging(HydrusMainWindow &window, rust::Str json);
void hydrus_close_local_import_review(HydrusMainWindow &window);
void hydrus_show_repair_file_locations(HydrusMainWindow &window, rust::Str json);
void hydrus_close_repair_file_locations(HydrusMainWindow &window);
void hydrus_show_export_folder(HydrusMainWindow &window, rust::Str json);
void hydrus_show_export_review(HydrusMainWindow &window, rust::Str json);
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
void hydrus_show_notifications(HydrusMainWindow &window, rust::Str json);
void hydrus_show_viewing_stats(HydrusMainWindow &window, rust::Str json);
void hydrus_show_about(HydrusMainWindow &window, rust::Str json);
void hydrus_export_downloader(HydrusMainWindow &window, rust::Str page_id);
void hydrus_import_downloader(HydrusMainWindow &window, rust::Str target);
void hydrus_show_manage_tags(HydrusMainWindow &window, rust::Str json);
void hydrus_show_timestamps(HydrusMainWindow &window, rust::Str json);
void hydrus_show_options(HydrusMainWindow &window, rust::Str json);
void hydrus_show_services(HydrusMainWindow &window, rust::Str json);
void hydrus_update_services(HydrusMainWindow &window, rust::Str json);
void hydrus_show_tag_filter(HydrusMainWindow &window, rust::Str json);
void hydrus_show_api_permissions(HydrusMainWindow &window, rust::Str json);
void hydrus_update_api_permissions(HydrusMainWindow &window, rust::Str json);
void hydrus_show_account_types(HydrusMainWindow &window, rust::Str json);
void hydrus_update_account_types(HydrusMainWindow &window, rust::Str json);
void hydrus_show_orphan_files(HydrusMainWindow &window, rust::Str json);
void hydrus_update_orphan_files(HydrusMainWindow &window, rust::Str json);
void hydrus_show_database_backup(HydrusMainWindow &window, rust::Str json);
void hydrus_update_database_backup(HydrusMainWindow &window, rust::Str json);
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
void hydrus_set_clipboard(HydrusMainWindow &window, rust::Str text);
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
