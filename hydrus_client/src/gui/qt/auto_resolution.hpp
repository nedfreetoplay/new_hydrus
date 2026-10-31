#pragma once

// WS-15.1.3a: auto-resolution dialogs moved verbatim from bridge.hpp/bridge.cpp (move-only split).

#include <QDialog>
#include <QJsonArray>
#include <QJsonObject>
#include <QString>
#include <QWidget>

class QCheckBox;
class QComboBox;
class QLabel;
class QLineEdit;
class QListWidget;
class QPushButton;
class QSpinBox;
class QTabWidget;
class QTableWidget;
class HydrusMainWindow;
class HydrusAutoResolutionRuleDialog;

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

// Shared comparator helpers (auto_resolution_comparators.cpp).
QComboBox *catalog_combo(const QJsonObject &catalog, const QString &key, QWidget *parent);
void select_combo(QComboBox *combo, const QString &value);
QString comparator_pretty(const QJsonObject &comparator);
QJsonObject edit_condition_dialog(QWidget *parent, const QJsonObject &initial, const QJsonObject &catalog);
QJsonObject edit_group_dialog(QWidget *parent, const QJsonObject &initial, const QJsonObject &catalog);
