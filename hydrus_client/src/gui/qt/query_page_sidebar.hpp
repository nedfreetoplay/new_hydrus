#pragma once

// WS-15.1.2b: HydrusQueryPage moved verbatim from bridge.hpp/bridge.cpp (move-only split).

#include <QJsonObject>
#include <QString>
#include <QStringList>
#include <QWidget>

class QComboBox;
class QCompleter;
class QEvent;
class QFrame;
class QLabel;
class QLineEdit;
class QListWidget;
class QPushButton;
class QResizeEvent;
class QScrollArea;
class QTabWidget;
class QToolButton;
class HydrusAutocompletePopup;
class HydrusCanvas;
class HydrusMainWindow;
class HydrusThumbnailGrid;

// Page construction lives in query_page_sidebar.cpp, results and autocomplete
// handling in query_page_results.cpp, state and commands in query_page_state.cpp.
// One media page, laid out like Hydrus: search/preview sidebar on the left and
// the thumbnail grid on the right.
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
