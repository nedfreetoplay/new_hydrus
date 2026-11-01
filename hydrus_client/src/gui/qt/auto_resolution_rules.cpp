
// WS-15.2.4b: typed DuplicatesRulesDialogState (no JSON parsing).

#include <hydrus_client/src/gui/qt/api.cxx.h>
#include "support/support.hpp"

#include <QApplication>
#include <QCheckBox>
#include <QClipboard>
#include <QComboBox>
#include <QDateTime>
#include <QDialog>
#include <QDialogButtonBox>
#include <QDoubleSpinBox>
#include <QFormLayout>
#include <QHBoxLayout>
#include <QHeaderView>
#include <QInputDialog>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QJsonValue>
#include <QLabel>
#include <QLineEdit>
#include <QListWidget>
#include <QMessageBox>
#include <QPlainTextEdit>
#include <QPushButton>
#include <QSpinBox>
#include <QTableWidget>
#include <QTabWidget>
#include <QTextEdit>
#include <QTimer>
#include <QVBoxLayout>
#include <algorithm>

namespace {

AutoRuleInfo to_rule_info(const AutoRuleRow &rule) {
    AutoRuleInfo info;
    info.name = to_qstring(rule.name);
    info.paused = rule.paused;
    info.operation_mode = to_qstring(rule.operation_mode);
    info.operation_label = to_qstring(rule.operation_label);
    info.operation_display = to_qstring(rule.operation_display);
    info.status_text = to_qstring(rule.status_text);
    info.max_pending_pairs = static_cast<int>(rule.max_pending_pairs);
    info.search_query = to_qstring(rule.search_query);
    info.search_mode = to_qstring(rule.search_mode);
    info.max_distance = static_cast<int>(rule.max_distance);
    info.action = to_qstring(rule.action);
    info.delete_a = rule.delete_a;
    info.delete_b = rule.delete_b;
    info.search_summary = to_qstring(rule.search_summary);
    info.selector_summary = to_qstring(rule.selector_summary);
    info.action_summary = to_qstring(rule.action_summary);
    for (const RuleComparatorRow &row : rule.selectors) {
        ComparatorInfo comparator;
        comparator.kind = to_qstring(row.kind);
        comparator.looking_at = to_qstring(row.looking_at);
        comparator.property = to_qstring(row.property);
        comparator.op = to_qstring(row.op);
        comparator.num_value = row.num_value;
        comparator.text_value = to_qstring(row.text_value);
        comparator.multiplier = row.multiplier;
        comparator.delta = row.delta;
        comparator.same = row.same;
        comparator.group_mode = to_qstring(row.group_mode);
        comparator.depth = static_cast<int>(row.depth);
        comparator.pretty = to_qstring(row.pretty);
        info.selectors.push_back(std::move(comparator));
    }
    return info;
}

std::vector<ChoiceInfo> to_choices(const rust::Vec<DuplicateChoiceRow> &rows) {
    std::vector<ChoiceInfo> out;
    for (const DuplicateChoiceRow &row : rows) {
        out.push_back({to_qstring(row.value), to_qstring(row.label)});
    }
    return out;
}

}  // namespace

HydrusDuplicatesRulesDialog::HydrusDuplicatesRulesDialog(HydrusMainWindow *main_window, const DuplicatesRulesDialogState &state, QWidget *parent)
    : QDialog(parent), main(main_window) {
    setWindowTitle(QStringLiteral("duplicate auto-resolution"));
    setObjectName(QStringLiteral("hydrus_duplicates_rules_dialog"));
    setAttribute(Qt::WA_DeleteOnClose, true);
    resize(980, 540);

    auto *layout = new QVBoxLayout(this);

    enabled = new QCheckBox(QStringLiteral("apply these rules automatically when a duplicates page refreshes"), this);
    layout->addWidget(enabled);

    auto *help = new QLabel(
        QStringLiteral("Rules are tested in alphabetical order; the first matching rule decides the pair. "
                       "Semi-automatic rules queue matches for review. Fully automatic rules apply immediately. "
                       "Run a preview before trusting a new rule."),
        this);
    help->setWordWrap(true);
    layout->addWidget(help);

    table = new QTableWidget(this);
    table->setColumnCount(5);
    table->setHorizontalHeaderLabels({
        QStringLiteral("name"),
        QStringLiteral("operation"),
        QStringLiteral("search"),
        QStringLiteral("comparison"),
        QStringLiteral("action")
    });
    table->horizontalHeader()->setSectionResizeMode(0, QHeaderView::ResizeToContents);
    table->horizontalHeader()->setSectionResizeMode(1, QHeaderView::ResizeToContents);
    table->horizontalHeader()->setSectionResizeMode(2, QHeaderView::ResizeToContents);
    table->horizontalHeader()->setSectionResizeMode(3, QHeaderView::Stretch);
    table->horizontalHeader()->setSectionResizeMode(4, QHeaderView::ResizeToContents);
    table->setSelectionBehavior(QAbstractItemView::SelectRows);
    table->setSelectionMode(QAbstractItemView::SingleSelection);
    table->setEditTriggers(QAbstractItemView::NoEditTriggers);
    table->verticalHeader()->setVisible(false);
    layout->addWidget(table, 1);

    auto *buttons = new QHBoxLayout();
    auto *add = new QPushButton(QStringLiteral("add rule..."), this);
    auto *edit = new QPushButton(QStringLiteral("edit..."), this);
    auto *duplicate = new QPushButton(QStringLiteral("duplicate"), this);
    auto *remove = new QPushButton(QStringLiteral("remove"), this);
    auto *review = new QPushButton(QStringLiteral("review pending..."), this);
    auto *close = new QPushButton(QStringLiteral("close"), this);
    buttons->addWidget(add);
    buttons->addWidget(edit);
    buttons->addWidget(duplicate);
    buttons->addWidget(remove);
    buttons->addWidget(review);
    buttons->addStretch(1);
    buttons->addWidget(close);
    layout->addLayout(buttons);

    QObject::connect(enabled, &QCheckBox::clicked, this, [this](bool) {
        main->notify(UiAction::DuplicatesAutoToggle);
    });
    QObject::connect(add, &QPushButton::clicked, this, [this]() { add_rule(); });
    QObject::connect(edit, &QPushButton::clicked, this, [this]() { edit_selected(); });
    QObject::connect(duplicate, &QPushButton::clicked, this, [this]() { duplicate_selected(); });
    QObject::connect(remove, &QPushButton::clicked, this, [this]() { remove_selected(); });
    QObject::connect(review, &QPushButton::clicked, this, [this]() { open_review(); });
    QObject::connect(close, &QPushButton::clicked, this, &QDialog::close);
    QObject::connect(table, &QTableWidget::cellDoubleClicked, this, [this](int, int) { edit_selected(); });

    set_state(state);

    const QString edit_key = qEnvironmentVariable("HYDRUS_QT_DUPLICATES_RULE_EDIT");

    if (!edit_key.isEmpty()) {
        const QString key = edit_key;
        QTimer::singleShot(150, this, [this, key]() {
            if (key == QStringLiteral("new")) {
                add_rule();
                return;
            }

            for (int row = 0; row < table->rowCount(); ++row) {
                if (table->item(row, 0) != nullptr
                    && table->item(row, 0)->text().compare(key, Qt::CaseInsensitive) == 0) {
                    table->selectRow(row);
                    edit_selected();
                    break;
                }
            }
        });
    }
}

void HydrusDuplicatesRulesDialog::set_state(const DuplicatesRulesDialogState &state) {
    rules.clear();
    for (const AutoRuleRow &rule : state.rules) {
        rules.push_back(to_rule_info(rule));
    }
    catalog_properties = to_choices(state.properties);
    catalog_operators = to_choices(state.operators);
    catalog_looking_at = to_choices(state.looking_at);
    catalog_search_modes = to_choices(state.search_modes);
    catalog_group_modes = to_choices(state.group_modes);
    catalog_actions = to_choices(state.actions_list);
    catalog_operation_modes = to_choices(state.operation_modes);

    const int previous_row = table->currentRow();
    table->setRowCount(static_cast<int>(rules.size()));

    for (size_t index = 0; index < rules.size(); ++index) {
        const AutoRuleInfo &rule = rules[index];
        const int row = static_cast<int>(index);

        for (int column = 0; column < 5; ++column) {
            if (table->item(row, column) == nullptr) {
                table->setItem(row, column, new QTableWidgetItem());
            }
        }

        const QString operation_label = rule.operation_display;

        table->item(row, 0)->setText(rule.name);
        table->item(row, 0)->setData(Qt::UserRole, row);
        table->item(row, 1)->setText(operation_label);
        table->item(row, 2)->setText(rule.search_summary);
        table->item(row, 3)->setText(rule.selector_summary);
        table->item(row, 4)->setText(rule.action_summary);
    }

    if (previous_row >= 0 && previous_row < table->rowCount()) {
        table->selectRow(previous_row);
    }

    QSignalBlocker blocker(enabled);
    enabled->setChecked(state.enabled);

    setToolTip(to_qstring(state.pending_tooltip));

    table->resizeRowsToContents();
}

void HydrusDuplicatesRulesDialog::set_preview(const DuplicatesRulePreviewState &preview) {
    if (rule_editor != nullptr) {
        rule_editor->set_preview(preview);
    }
}

void HydrusDuplicatesRulesDialog::add_rule() {
    AutoRuleInfo rule;
    rule.name = QStringLiteral("new rule");
    rule.paused = false;
    rule.operation_mode = QStringLiteral("semi-automatic");
    rule.max_pending_pairs = 500;
    rule.search_mode = QStringLiteral("both");
    rule.max_distance = 0;
    rule.action = QStringLiteral("better");
    ComparatorInfo relative;
    relative.kind = QStringLiteral("relative");
    relative.property = QStringLiteral("num_pixels");
    relative.op = QStringLiteral(">");
    relative.multiplier = 1.0;
    relative.delta = 0.0;
    relative.pretty = QStringLiteral("A num_pixels > B");
    rule.selectors.push_back(relative);

    edit_rule(rule);
}

void HydrusDuplicatesRulesDialog::edit_selected() {
    const int row = table->currentRow();

    if (row < 0 || row >= static_cast<int>(rules.size())) {
        return;
    }

    edit_rule(rules[static_cast<size_t>(row)]);
}

void HydrusDuplicatesRulesDialog::duplicate_selected() {
    const int row = table->currentRow();

    if (row < 0 || row >= static_cast<int>(rules.size())) {
        return;
    }

    AutoRuleInfo copy = rules[static_cast<size_t>(row)];
    copy.name += QStringLiteral(" copy");
    copy.paused = true;
    edit_rule(copy);
}

void HydrusDuplicatesRulesDialog::remove_selected() {
    const int row = table->currentRow();

    if (row < 0 || table->item(row, 0) == nullptr) {
        return;
    }

    const QString name = table->item(row, 0)->text();

    if (QMessageBox::question(this, QStringLiteral("delete auto-resolution rule"),
                              QStringLiteral("Delete rule \"%1\" and its pending review pairs?").arg(name))
        != QMessageBox::Yes) {
        return;
    }

    main->notify(UiAction::DuplicatesRuleRemove, QString(), name);
}

void HydrusDuplicatesRulesDialog::open_review() {
    main->notify(UiAction::DuplicatesReview);
}

void HydrusDuplicatesRulesDialog::edit_rule(const AutoRuleInfo &rule) {
    if (rule_editor != nullptr) {
        rule_editor->close();
        rule_editor = nullptr;
    }

    QJsonObject catalog;
    auto fill = [&catalog](const char *key, const std::vector<ChoiceInfo> &choices) {
        QJsonArray array;
        for (const ChoiceInfo &choice : choices) {
            QJsonObject entry;
            entry.insert(QStringLiteral("label"), choice.label);
            entry.insert(QStringLiteral("value"), choice.value);
            array.append(entry);
        }
        catalog.insert(QString::fromUtf8(key), array);
    };
    fill("properties", catalog_properties);
    fill("operators", catalog_operators);
    fill("looking_at", catalog_looking_at);
    fill("search_modes", catalog_search_modes);
    fill("group_modes", catalog_group_modes);
    fill("actions", catalog_actions);
    fill("operation_modes", catalog_operation_modes);

    rule_editor = new HydrusAutoResolutionRuleDialog(main, this, rule, catalog, this);
    QObject::connect(rule_editor, &QObject::destroyed, this, [this]() { rule_editor = nullptr; });
    rule_editor->show();
    rule_editor->raise();
    rule_editor->activateWindow();
}
