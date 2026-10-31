#pragma once

// WS-15.1.2c: HydrusDuplicatesPage moved verbatim from bridge.hpp/bridge.cpp (move-only split).

#include <QJsonObject>
#include <QString>
#include <QWidget>

class QComboBox;
class QLineEdit;
class QListWidget;
class QLabel;
class QPushButton;
class QScrollArea;
class QSpinBox;
class QSplitter;
class QTableWidget;
class QVBoxLayout;
class QHBoxLayout;
class QPoint;
class HydrusCanvas;
class HydrusMainWindow;
class HydrusThumbnailGrid;

// Side-by-side duplicate pair filter.
class HydrusDuplicatesPage : public QWidget {
public:
    HydrusDuplicatesPage(HydrusMainWindow *main, const QString &page_id, const QJsonObject &state);

    const QString &page_id() const;
    void update_state(const QJsonObject &state);
    void update_auto_resolution_rules(const QJsonObject &state);

private:
    void act(const QString &action);
    void notify_search_context();
    void update_search_controls();
    void show_group_actions(const QString &target, const QPoint &global_pos);

    // WS-15.1.2c: constructor split so each .cpp stays within R3.
    // build_media_panel runs at the end of the ctor; no logic changed.
    void build_media_panel(
        QSplitter *page_splitter,
        QScrollArea *sidebar_scroll,
        QWidget *sidebar,
        QVBoxLayout *sidebar_layout,
        QHBoxLayout *outer,
        const QJsonObject &state);

    HydrusMainWindow *main = nullptr;
    QString id;
    QLabel *status = nullptr;
    HydrusThumbnailGrid *grid = nullptr;
    QLabel *media_status = nullptr;
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
    QTableWidget *auto_rules_table = nullptr;
    QLabel *auto_rules_status = nullptr;
    QSpinBox *batch = nullptr;
    QPushButton *next_batch = nullptr;
};
