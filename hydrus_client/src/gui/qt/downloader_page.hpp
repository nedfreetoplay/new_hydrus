#pragma once

// WS-15.1.2c: HydrusDownloaderPage moved verbatim from bridge.hpp/bridge.cpp (move-only split).

#include <QJsonArray>
#include <QJsonObject>
#include <QString>
#include <QWidget>

class QCheckBox;
class QComboBox;
class QFrame;
class QLabel;
class QLineEdit;
class QListWidget;
class QPlainTextEdit;
class QProgressBar;
class QPushButton;
class QScrollArea;
class QSpinBox;
class QTableWidget;
class QToolButton;
class QVBoxLayout;
class HydrusMainWindow;

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

    // WS-15.1.2c: constructor body split by responsibility so each .cpp stays
    // within R3. The helpers run in ctor order; no logic, text or order changed.
    QToolButton *make_row_icon(QWidget *parent, const QString &icon_kind, const QString &tooltip);
    QFrame *make_status_box(
        QWidget *container,
        QVBoxLayout *outer_layout,
        const QString &title,
        QLabel **status_label,
        QLabel **seed_label,
        QToolButton **pause_button,
        QToolButton **log_button,
        QProgressBar **progress_bar,
        QLabel **percent_label,
        QProgressBar **network_bar,
        QLabel **network_percent,
        QToolButton **cog_button,
        const QString &pause_tip,
        const QString &log_text,
        const QString &log_action);
    void build_gallery_panel(QVBoxLayout *main_layout);
    void build_watcher_panel(QVBoxLayout *main_layout);
    void build_urls_panels(QVBoxLayout *main_layout);
    void build_simple_panels(QVBoxLayout *main_layout);
    void build_highlight_panel(QVBoxLayout *sidebar_layout, const QJsonObject &state);
    void build_import_options(QVBoxLayout *main_layout, const QJsonObject &state);
    void build_footer(QVBoxLayout *sidebar_layout);

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
    QTableWidget *watcher_table = nullptr;
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
    QLineEdit *watcher_url = nullptr;
    QToolButton *watcher_paste = nullptr;
    QPushButton *checker_options = nullptr;
    QToolButton *watcher_highlight = nullptr;
    QToolButton *watcher_clear = nullptr;
    QToolButton *watcher_files_pause = nullptr;
    QToolButton *watcher_check_pause = nullptr;
    QToolButton *watcher_retry = nullptr;
    QToolButton *watcher_trash = nullptr;
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
    QLabel *highlight_subject = nullptr;
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
