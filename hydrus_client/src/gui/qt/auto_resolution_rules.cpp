#include "auto_resolution.hpp"

// WS-15.1.3a: moved verbatim from bridge.cpp (move-only split).

#include "bridge.hpp"
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

HydrusDuplicatesRulesDialog::HydrusDuplicatesRulesDialog(HydrusMainWindow *main_window, const QJsonObject &state, QWidget *parent)
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
        main->notify(QStringLiteral("duplicates_auto_toggle"));
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

void HydrusDuplicatesRulesDialog::set_state(const QJsonObject &state) {
    data = state;

    QJsonArray rules = state.value(QStringLiteral("auto_rules")).toArray();
    if (rules.isEmpty()) {
        rules = state.value(QStringLiteral("rules")).toArray();
    }
    const int previous_row = table->currentRow();
    table->setRowCount(rules.size());

    for (int index = 0; index < rules.size(); ++index) {
        const QJsonObject rule = rules.at(index).toObject();

        for (int column = 0; column < 5; ++column) {
            if (table->item(index, column) == nullptr) {
                table->setItem(index, column, new QTableWidgetItem());
            }
        }

        QString operation_label = rule.value(QStringLiteral("operation_label")).toString();
        if (rule.value(QStringLiteral("paused")).toBool()) {
            operation_label += QStringLiteral(" - paused");
        }

        table->item(index, 0)->setText(rule.value(QStringLiteral("name")).toString());
        table->item(index, 0)->setData(
            Qt::UserRole,
            QString::fromUtf8(QJsonDocument(rule).toJson(QJsonDocument::Compact)));
        table->item(index, 1)->setText(operation_label);
        table->item(index, 2)->setText(rule.value(QStringLiteral("search_summary")).toString());
        table->item(index, 3)->setText(rule.value(QStringLiteral("selector_summary")).toString());
        table->item(index, 4)->setText(rule.value(QStringLiteral("action_summary")).toString());
    }

    if (previous_row >= 0 && previous_row < table->rowCount()) {
        table->selectRow(previous_row);
    }

    QSignalBlocker blocker(enabled);
    enabled->setChecked(state.value(QStringLiteral("enabled")).toBool());

    const int pending = state.value(QStringLiteral("pending")).toInt(0);
    setToolTip(pending > 0 ? QStringLiteral("%1 pair(s) are waiting for review").arg(pending) : QString());

    table->resizeRowsToContents();
}

void HydrusDuplicatesRulesDialog::set_preview(const QJsonObject &preview) {
    if (rule_editor != nullptr) {
        rule_editor->set_preview(preview);
    }
}

void HydrusDuplicatesRulesDialog::add_rule() {
    QJsonObject rule;
    rule.insert(QStringLiteral("name"), QStringLiteral("new rule"));
    rule.insert(QStringLiteral("paused"), false);
    rule.insert(QStringLiteral("operation_mode"), QStringLiteral("semi-automatic"));
    rule.insert(QStringLiteral("max_pending_pairs"), 500);
    rule.insert(QStringLiteral("search_query"), QString());
    rule.insert(QStringLiteral("search_mode"), QStringLiteral("both"));
    rule.insert(QStringLiteral("max_distance"), 0);
    rule.insert(QStringLiteral("action"), QStringLiteral("better"));
    rule.insert(QStringLiteral("delete_a"), false);
    rule.insert(QStringLiteral("delete_b"), false);

    QJsonObject relative;
    relative.insert(QStringLiteral("kind"), QStringLiteral("relative"));
    relative.insert(QStringLiteral("property"), QStringLiteral("num_pixels"));
    relative.insert(QStringLiteral("operator"), QStringLiteral(">"));
    relative.insert(QStringLiteral("multiplier"), 1.0);
    relative.insert(QStringLiteral("delta"), 0.0);

    QJsonArray selector;
    selector.append(relative);
    rule.insert(QStringLiteral("pair_selector"), selector);

    edit_rule(rule);
}

void HydrusDuplicatesRulesDialog::edit_selected() {
    const int row = table->currentRow();

    if (row < 0 || table->item(row, 0) == nullptr) {
        return;
    }

    QJsonParseError error{};
    const QJsonDocument document = QJsonDocument::fromJson(
        table->item(row, 0)->data(Qt::UserRole).toString().toUtf8(), &error);

    if (error.error == QJsonParseError::NoError && document.isObject()) {
        edit_rule(document.object());
    }
}

void HydrusDuplicatesRulesDialog::duplicate_selected() {
    const int row = table->currentRow();

    if (row < 0 || table->item(row, 0) == nullptr) {
        return;
    }

    QJsonParseError error{};
    const QJsonDocument document = QJsonDocument::fromJson(
        table->item(row, 0)->data(Qt::UserRole).toString().toUtf8(), &error);

    if (error.error != QJsonParseError::NoError || !document.isObject()) {
        return;
    }

    QJsonObject copy = document.object();
    copy.insert(QStringLiteral("name"), copy.value(QStringLiteral("name")).toString() + QStringLiteral(" copy"));
    copy.insert(QStringLiteral("paused"), true);
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

    main->notify(QStringLiteral("duplicates_rule_remove"), QString(), name);
}

void HydrusDuplicatesRulesDialog::open_review() {
    main->notify(QStringLiteral("duplicates_review"));
}

void HydrusDuplicatesRulesDialog::edit_rule(const QJsonObject &rule) {
    if (rule_editor != nullptr) {
        rule_editor->close();
        rule_editor = nullptr;
    }

    rule_editor = new HydrusAutoResolutionRuleDialog(main, this, rule, data, this);
    QObject::connect(rule_editor, &QObject::destroyed, this, [this]() { rule_editor = nullptr; });
    rule_editor->show();
    rule_editor->raise();
    rule_editor->activateWindow();
}
