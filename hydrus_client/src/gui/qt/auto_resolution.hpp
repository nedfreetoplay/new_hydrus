#pragma once

// WS-15.2.4b: auto-resolution dialogs take typed cxx structs (no QJsonObject
// in signatures). Rule selectors cross the seam as flat comparator rows with
// depth; C++ rebuilds the nested array without parsing JSON.

#include <QDialog>
#include <QJsonArray>
#include <QJsonObject>
#include <QString>
#include <QWidget>

#include <vector>

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

struct DuplicatesRulesDialogState;
struct DuplicatesRulePreviewState;
struct DuplicateReviewDialogState;
struct DuplicateCustomActionDialogState;
struct DuplicatesMergeOptionsDialogState;
struct MergeOptionsData;
struct AutoRuleRow;
struct RuleComparatorRow;
struct DuplicateChoiceRow;
struct MergeOptionsPage;
struct MergeServiceActionRow;
struct ReviewPairRow;
struct PreviewPairRow;

struct ChoiceInfo {
    QString value;
    QString label;
};

struct ComparatorInfo {
    QString kind;
    QString looking_at;
    QString property;
    QString op;
    double num_value = 0.0;
    QString text_value;
    double multiplier = 1.0;
    double delta = 0.0;
    bool same = false;
    QString group_mode;
    int depth = 0;
    QString pretty;
};

struct AutoRuleInfo {
    QString name;
    bool paused = false;
    QString operation_mode;
    QString operation_label;
    int max_pending_pairs = 500;
    QString search_query;
    QString search_mode;
    int max_distance = 0;
    QString action;
    bool delete_a = false;
    bool delete_b = false;
    QString search_summary;
    QString selector_summary;
    QString action_summary;
    std::vector<ComparatorInfo> selectors;
};

// Editor for duplicate auto-resolution rules.
class HydrusDuplicatesRulesDialog : public QDialog {
public:
    explicit HydrusDuplicatesRulesDialog(HydrusMainWindow *main, const DuplicatesRulesDialogState &state, QWidget *parent = nullptr);

    void set_state(const DuplicatesRulesDialogState &state);
    void set_preview(const DuplicatesRulePreviewState &preview);

private:
    void add_rule();
    void edit_selected();
    void duplicate_selected();
    void remove_selected();
    void open_review();
    void edit_rule(const AutoRuleInfo &rule);

    HydrusMainWindow *main = nullptr;
    QTableWidget *table = nullptr;
    QCheckBox *enabled = nullptr;
    std::vector<AutoRuleInfo> rules;
    std::vector<ChoiceInfo> catalog_properties;
    std::vector<ChoiceInfo> catalog_operators;
    std::vector<ChoiceInfo> catalog_looking_at;
    std::vector<ChoiceInfo> catalog_search_modes;
    std::vector<ChoiceInfo> catalog_group_modes;
    std::vector<ChoiceInfo> catalog_actions;
    std::vector<ChoiceInfo> catalog_operation_modes;
    HydrusAutoResolutionRuleDialog *rule_editor = nullptr;
};

// Edit one duplicate auto-resolution rule: search, comparison and action.
class HydrusAutoResolutionRuleDialog : public QDialog {
public:
    HydrusAutoResolutionRuleDialog(
        HydrusMainWindow *main,
        HydrusDuplicatesRulesDialog *rules_dialog,
        const AutoRuleInfo &rule,
        const QJsonObject &catalog,
        QWidget *parent = nullptr);

    void set_preview(const DuplicatesRulePreviewState &preview);

private:
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
    explicit HydrusDuplicateReviewDialog(HydrusMainWindow *main, const DuplicateReviewDialogState &state, QWidget *parent = nullptr);

    void set_state(const DuplicateReviewDialogState &state);

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

struct MergeServiceActionInfo {
    QString service;
    QString action;
};

struct MergeOptionsPageInfo {
    QString archive_action;
    QString modified_date_action;
    QString urls_action;
    QString notes_action;
    std::vector<MergeServiceActionInfo> tag_actions;
    std::vector<MergeServiceActionInfo> rating_actions;
};

struct MergeOptionsInfo {
    MergeOptionsPageInfo better;
    MergeOptionsPageInfo same;
    QStringList tag_services;
    QStringList rating_services;
    std::vector<ChoiceInfo> actions;
    std::vector<ChoiceInfo> archive_actions;
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

// One page of duplicate content merge options as plain Qt values (no JSON).
struct DuplicateMergePageValues {
    QString archive;
    QString modified_date;
    QString urls;
    QString notes;
    QStringList tag_services;
    QStringList tag_actions;
    QStringList rating_services;
    QStringList rating_actions;
};

void build_duplicate_merge_page(
    QWidget *parent,
    QTabWidget *tabs,
    const MergeOptionsInfo &data,
    const QString &title,
    DuplicateMergePage &page);
void fill_duplicate_merge_page(
    DuplicateMergePage &page,
    const MergeOptionsInfo &data,
    const MergeOptionsPageInfo &options);
DuplicateMergePageValues read_duplicate_merge_page(const DuplicateMergePage &page);

// Which metadata a duplicate merge copies from the worse file, for each
// duplicate action ("this is better" and "same quality").
class HydrusDuplicateMergeOptionsDialog : public QDialog {
public:
    explicit HydrusDuplicateMergeOptionsDialog(HydrusMainWindow *main, const DuplicatesMergeOptionsDialogState &state, QWidget *parent = nullptr);

    void set_state(const DuplicatesMergeOptionsDialogState &state);

private:
    void save();

    HydrusMainWindow *main = nullptr;
    MergeOptionsInfo data;
    DuplicateMergePage better;
    DuplicateMergePage same;
};

// Per-decision version of the duplicate action menu: choose the action, which
// file is better, what to delete, and merge options only for this pair.
class HydrusDuplicateCustomActionDialog : public QDialog {
public:
    explicit HydrusDuplicateCustomActionDialog(HydrusMainWindow *main, const DuplicateCustomActionDialogState &state, QWidget *parent = nullptr);

private:
    void apply();

    HydrusMainWindow *main = nullptr;
    QString page_id;
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
