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

HydrusAutoResolutionRuleDialog::HydrusAutoResolutionRuleDialog(
    HydrusMainWindow *main_window,
    HydrusDuplicatesRulesDialog *rules,
    const QJsonObject &rule,
    const QJsonObject &rule_catalog,
    QWidget *parent)
    : QDialog(parent), main(main_window), rules_dialog(rules), catalog(rule_catalog) {
    setWindowTitle(QStringLiteral("edit auto-resolution rule"));
    setObjectName(QStringLiteral("hydrus_auto_resolution_rule_dialog"));
    setAttribute(Qt::WA_DeleteOnClose, true);
    resize(820, 680);

    original_name = rule.value(QStringLiteral("name")).toString();
    selector = rule.value(QStringLiteral("pair_selector")).toArray();

    auto *layout = new QVBoxLayout(this);

    auto *warning = new QLabel(
        QStringLiteral("Rules test both ways around. Every comparator in the comparison list must match "
                       "(AND); add a group to mix AND/OR conditions. Preview before enabling automatic actions."),
        this);
    warning->setWordWrap(true);
    layout->addWidget(warning);

    auto *form = new QFormLayout();
    name = new QLineEdit(rule.value(QStringLiteral("name")).toString(), this);
    paused = new QCheckBox(QStringLiteral("paused"), this);
    paused->setChecked(rule.value(QStringLiteral("paused")).toBool());
    operation = catalog_combo(catalog, QStringLiteral("operation_modes"), this);
    select_combo(operation, rule.value(QStringLiteral("operation_mode")).toString(QStringLiteral("semi-automatic")));
    max_pending = new QSpinBox(this);
    max_pending->setRange(1, 10000);
    max_pending->setValue(rule.value(QStringLiteral("max_pending_pairs")).toInt(500));
    max_pending->setToolTip(QStringLiteral("semi-automatic queue cap for this rule"));
    form->addRow(QStringLiteral("name"), name);
    form->addRow(QStringLiteral("paused"), paused);
    form->addRow(QStringLiteral("operation"), operation);
    form->addRow(QStringLiteral("max pending pairs"), max_pending);
    layout->addLayout(form);

    auto *tabs = new QTabWidget(this);

    auto *search_tab = new QWidget(tabs);
    auto *search_form = new QFormLayout(search_tab);
    search_query = new QLineEdit(rule.value(QStringLiteral("search_query")).toString(), search_tab);
    search_query->setPlaceholderText(QStringLiteral("e.g. system:filetype = image, or leave empty for every pair"));
    search_mode = catalog_combo(catalog, QStringLiteral("search_modes"), search_tab);
    if (search_mode->count() == 0) {
        search_mode->addItem(QStringLiteral("both files match"), QStringLiteral("both"));
        search_mode->addItem(QStringLiteral("either file matches"), QStringLiteral("either"));
    }
    select_combo(search_mode, rule.value(QStringLiteral("search_mode")).toString(QStringLiteral("both")));
    max_distance = new QSpinBox(search_tab);
    max_distance->setRange(0, 1000);
    max_distance->setValue(rule.value(QStringLiteral("max_distance")).toInt(0));
    max_distance->setSpecialValueText(QStringLiteral("use the page/global distance"));
    search_form->addRow(QStringLiteral("search predicate"), search_query);
    search_form->addRow(QStringLiteral("files must"), search_mode);
    search_form->addRow(QStringLiteral("maximum distance"), max_distance);
    tabs->addTab(search_tab, QStringLiteral("search"));

    auto *comparison_tab = new QWidget(tabs);
    auto *comparison_layout = new QVBoxLayout(comparison_tab);
    auto *comparison_help = new QLabel(
        QStringLiteral("The pair must satisfy every row. A single row may be a one-file condition, "
                       "an A/B comparison or an AND/OR group."),
        comparison_tab);
    comparison_help->setWordWrap(true);
    comparison_layout->addWidget(comparison_help);
    comparators = new QListWidget(comparison_tab);
    comparison_layout->addWidget(comparators, 1);
    auto *comparison_buttons = new QHBoxLayout();
    auto *add_condition = new QPushButton(QStringLiteral("add condition..."), comparison_tab);
    auto *add_group = new QPushButton(QStringLiteral("add group..."), comparison_tab);
    auto *edit_condition = new QPushButton(QStringLiteral("edit"), comparison_tab);
    auto *remove_condition = new QPushButton(QStringLiteral("remove"), comparison_tab);
    comparison_buttons->addWidget(add_condition);
    comparison_buttons->addWidget(add_group);
    comparison_buttons->addWidget(edit_condition);
    comparison_buttons->addWidget(remove_condition);
    comparison_buttons->addStretch(1);
    comparison_layout->addLayout(comparison_buttons);
    tabs->addTab(comparison_tab, QStringLiteral("comparison"));

    auto *action_tab = new QWidget(tabs);
    auto *action_form = new QFormLayout(action_tab);
    action = catalog_combo(catalog, QStringLiteral("actions"), action_tab);
    select_combo(action, rule.value(QStringLiteral("action")).toString(QStringLiteral("better")));
    delete_a = new QCheckBox(QStringLiteral("also delete A after the action"), action_tab);
    delete_b = new QCheckBox(QStringLiteral("also delete B after the action"), action_tab);
    delete_a->setChecked(rule.value(QStringLiteral("delete_a")).toBool());
    delete_b->setChecked(rule.value(QStringLiteral("delete_b")).toBool());
    action_form->addRow(QStringLiteral("action"), action);
    action_form->addRow(delete_a);
    action_form->addRow(delete_b);
    tabs->addTab(action_tab, QStringLiteral("action"));

    auto *preview_tab = new QWidget(tabs);
    auto *preview_layout = new QVBoxLayout(preview_tab);
    auto *preview_help = new QLabel(
        QStringLiteral("Test this rule against the current duplicate queue. Nothing is changed."),
        preview_tab);
    preview_help->setWordWrap(true);
    preview_layout->addWidget(preview_help);
    preview_label = new QLabel(QStringLiteral("not tested yet"), preview_tab);
    preview_label->setWordWrap(true);
    preview_label->setTextInteractionFlags(Qt::TextSelectableByMouse);
    preview_layout->addWidget(preview_label);

    preview_layout->addWidget(new QLabel(QStringLiteral("pairs that will be actioned"), preview_tab));
    preview_pass = new QTableWidget(preview_tab);
    preview_pass->setColumnCount(4);
    preview_pass->setHorizontalHeaderLabels({
        QStringLiteral("A"),
        QStringLiteral("B"),
        QStringLiteral("distance"),
        QStringLiteral("action")
    });
    preview_pass->horizontalHeader()->setSectionResizeMode(0, QHeaderView::Fixed);
    preview_pass->horizontalHeader()->setSectionResizeMode(1, QHeaderView::Fixed);
    preview_pass->horizontalHeader()->setSectionResizeMode(2, QHeaderView::ResizeToContents);
    preview_pass->horizontalHeader()->setSectionResizeMode(3, QHeaderView::Stretch);
    preview_pass->setColumnWidth(0, 116);
    preview_pass->setColumnWidth(1, 116);
    preview_pass->setSelectionBehavior(QAbstractItemView::SelectRows);
    preview_pass->setSelectionMode(QAbstractItemView::ExtendedSelection);
    preview_pass->setEditTriggers(QAbstractItemView::NoEditTriggers);
    preview_pass->verticalHeader()->setVisible(false);
    preview_pass->setIconSize(QSize(110, 90));
    preview_layout->addWidget(preview_pass, 1);

    preview_layout->addWidget(new QLabel(QStringLiteral("pairs that will be skipped"), preview_tab));
    preview_fail = new QTableWidget(preview_tab);
    preview_fail->setColumnCount(3);
    preview_fail->setHorizontalHeaderLabels({
        QStringLiteral("A"),
        QStringLiteral("B"),
        QStringLiteral("distance")
    });
    preview_fail->horizontalHeader()->setSectionResizeMode(0, QHeaderView::Fixed);
    preview_fail->horizontalHeader()->setSectionResizeMode(1, QHeaderView::Fixed);
    preview_fail->horizontalHeader()->setSectionResizeMode(2, QHeaderView::Stretch);
    preview_fail->setColumnWidth(0, 116);
    preview_fail->setColumnWidth(1, 116);
    preview_fail->setSelectionBehavior(QAbstractItemView::SelectRows);
    preview_fail->setSelectionMode(QAbstractItemView::ExtendedSelection);
    preview_fail->setEditTriggers(QAbstractItemView::NoEditTriggers);
    preview_fail->verticalHeader()->setVisible(false);
    preview_fail->setIconSize(QSize(110, 90));
    preview_layout->addWidget(preview_fail, 1);

    install_pair_list_actions(preview_pass, main);
    install_pair_list_actions(preview_fail, main);

    auto *run_preview = new QPushButton(QStringLiteral("test against the queue"), preview_tab);
    preview_layout->addWidget(run_preview, 0, Qt::AlignLeft);
    tabs->addTab(preview_tab, QStringLiteral("preview"));

    layout->addWidget(tabs, 1);

    auto *footer = new QHBoxLayout();
    auto *cancel = new QPushButton(QStringLiteral("cancel"), this);
    auto *save = new QPushButton(QStringLiteral("save rule"), this);
    footer->addStretch(1);
    footer->addWidget(cancel);
    footer->addWidget(save);
    layout->addLayout(footer);

    QObject::connect(add_condition, &QPushButton::clicked, this, [this]() { add_comparator(); });
    QObject::connect(add_group, &QPushButton::clicked, this, [this]() { this->add_group(); });
    QObject::connect(edit_condition, &QPushButton::clicked, this, [this]() { edit_selected_comparator(); });
    QObject::connect(remove_condition, &QPushButton::clicked, this, [this]() { remove_selected_comparator(); });
    QObject::connect(comparators, &QListWidget::itemDoubleClicked, this, [this](QListWidgetItem *) {
        edit_selected_comparator();
    });
    QObject::connect(run_preview, &QPushButton::clicked, this, [this]() { this->run_preview(); });
    QObject::connect(save, &QPushButton::clicked, this, [this]() { this->save(); });
    QObject::connect(cancel, &QPushButton::clicked, this, &QDialog::close);
    QObject::connect(operation, &QComboBox::currentIndexChanged, this, [this](int) {
        max_pending->setEnabled(operation->currentData().toString() != QLatin1String("fully automatic"));
    });

    max_pending->setEnabled(operation->currentData().toString() != QLatin1String("fully automatic"));
    refresh_comparators();

    if (qEnvironmentVariableIsSet("HYDRUS_QT_DUPLICATES_RULE_PREVIEW")) {
        tabs->setCurrentIndex(3);
        QTimer::singleShot(250, this, [this]() { this->run_preview(); });
    }

    const QString tab = qEnvironmentVariable("HYDRUS_QT_DUPLICATES_RULE_TAB");
    if (tab == QStringLiteral("comparison")) {
        tabs->setCurrentIndex(1);
    } else if (tab == QStringLiteral("action")) {
        tabs->setCurrentIndex(2);
    } else if (tab == QStringLiteral("preview")) {
        tabs->setCurrentIndex(3);
    }
}

void HydrusAutoResolutionRuleDialog::refresh_comparators() {
    comparators->clear();

    for (const QJsonValue &value : selector) {
        comparators->addItem(comparator_pretty(value.toObject()));
    }
}

void HydrusAutoResolutionRuleDialog::add_comparator() {
    QJsonObject initial;
    initial.insert(QStringLiteral("kind"), QStringLiteral("one_file"));
    initial.insert(QStringLiteral("looking_at"), QStringLiteral("a"));
    initial.insert(QStringLiteral("property"), QStringLiteral("size"));
    initial.insert(QStringLiteral("operator"), QStringLiteral(">"));
    initial.insert(QStringLiteral("value"), 0.0);

    const QJsonObject result = edit_condition_dialog(this, initial, catalog);

    if (!result.isEmpty()) {
        selector.append(result);
        refresh_comparators();
    }
}

void HydrusAutoResolutionRuleDialog::add_group() {
    const QJsonObject result = edit_group_dialog(this, QJsonObject(), catalog);

    if (!result.isEmpty()) {
        selector.append(result);
        refresh_comparators();
    }
}

void HydrusAutoResolutionRuleDialog::edit_selected_comparator() {
    const int row = comparators->currentRow();

    if (row < 0 || row >= selector.size()) {
        return;
    }

    const QJsonObject current = selector.at(row).toObject();
    const QJsonObject result =
        current.value(QStringLiteral("kind")).toString() == QLatin1String("group")
            ? edit_group_dialog(this, current, catalog)
            : edit_condition_dialog(this, current, catalog);

    if (!result.isEmpty()) {
        selector.replace(row, result);
        refresh_comparators();
    }
}

void HydrusAutoResolutionRuleDialog::remove_selected_comparator() {
    const int row = comparators->currentRow();

    if (row >= 0 && row < selector.size()) {
        selector.removeAt(row);
        refresh_comparators();
    }
}

QJsonObject HydrusAutoResolutionRuleDialog::rule_json() const {
    QJsonObject result;
    result.insert(QStringLiteral("name"), name->text().trimmed());
    result.insert(QStringLiteral("paused"), paused->isChecked());
    result.insert(QStringLiteral("operation_mode"), operation->currentData().toString());
    result.insert(QStringLiteral("max_pending_pairs"), max_pending->value());
    result.insert(QStringLiteral("search_query"), search_query->text().trimmed());
    result.insert(QStringLiteral("search_mode"), search_mode->currentData().toString());
    result.insert(QStringLiteral("max_distance"), max_distance->value());
    result.insert(QStringLiteral("pair_selector"), selector);
    result.insert(QStringLiteral("action"), action->currentData().toString());
    result.insert(QStringLiteral("delete_a"), delete_a->isChecked());
    result.insert(QStringLiteral("delete_b"), delete_b->isChecked());

    return result;
}

void HydrusAutoResolutionRuleDialog::run_preview() {
    if (name->text().trimmed().isEmpty()) {
        QMessageBox::warning(this, QStringLiteral("auto-resolution"), QStringLiteral("Give the rule a name first."));
        return;
    }

    preview_label->setText(QStringLiteral("testing..."));

    QJsonObject payload = rule_json();
    payload.insert(QStringLiteral("original_name"), original_name);
    main->notify(QStringLiteral("duplicates_rule_preview"), QString(), json_payload(payload));
}

void HydrusAutoResolutionRuleDialog::set_preview(const QJsonObject &preview) {
    const QString status = preview.value(QStringLiteral("status")).toString();

    if (status.isEmpty()) {
        preview_label->setText(QStringLiteral("no preview result"));
        preview_pass->setRowCount(0);
        preview_fail->setRowCount(0);
        return;
    }

    QStringList lines;
    lines.append(status);

    const QString selector_text = preview.value(QStringLiteral("selector")).toString();
    if (!selector_text.isEmpty()) {
        lines.append(QStringLiteral("selector: %1").arg(selector_text));
    }

    const QString search = preview.value(QStringLiteral("search")).toString();
    if (!search.isEmpty()) {
        lines.append(QStringLiteral("search: %1").arg(search));
    }

    preview_label->setText(lines.join(QLatin1Char('\n')));

    fill_pair_table(
        preview_pass,
        preview.value(QStringLiteral("samples")).toArray(),
        2,
        {QStringLiteral("distance"), QStringLiteral("decision")});
    fill_pair_table(
        preview_fail,
        preview.value(QStringLiteral("failed_samples")).toArray(),
        2,
        {QStringLiteral("distance")});
}

void HydrusAutoResolutionRuleDialog::save() {
    if (name->text().trimmed().isEmpty()) {
        QMessageBox::warning(this, QStringLiteral("auto-resolution"), QStringLiteral("Give the rule a name first."));
        return;
    }

    QJsonObject payload = rule_json();
    payload.insert(QStringLiteral("original_name"), original_name);
    main->notify(QStringLiteral("duplicates_rule_save"), QString(), json_payload(payload));
    close();
}
