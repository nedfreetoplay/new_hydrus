#include "bridge.hpp"

#include "support/support.hpp"

#include <QCheckBox>
#include <QComboBox>
#include <QDateTime>
#include <QDialog>
#include <QDialogButtonBox>
#include <QFormLayout>
#include <QHBoxLayout>
#include <QHeaderView>
#include <QLabel>
#include <QLineEdit>
#include <QPushButton>
#include <QSpinBox>
#include <QTableWidget>
#include <QVBoxLayout>

// WS-15.1.5: HydrusSubscriptionsPage + editor moved verbatim from bridge.cpp (move-only split).

HydrusSubscriptionsPage::HydrusSubscriptionsPage(HydrusMainWindow *main_window, const QString &page_id, const QJsonObject &state)
    : QWidget(), main(main_window), id(page_id) {
    auto *layout = new QVBoxLayout(this);
    layout->setContentsMargins(6, 6, 6, 6);

    status = new QLabel(this);
    layout->addWidget(status);

    table = new QTableWidget(this);
    table->setColumnCount(7);
    table->setHorizontalHeaderLabels({
        QStringLiteral("name"),
        QStringLiteral("period"),
        QStringLiteral("last check"),
        QStringLiteral("next check"),
        QStringLiteral("imported"),
        QStringLiteral("failed"),
        QStringLiteral("status")
    });
    table->horizontalHeader()->setStretchLastSection(true);
    table->horizontalHeader()->setSectionResizeMode(0, QHeaderView::ResizeToContents);
    table->setSelectionBehavior(QAbstractItemView::SelectRows);
    table->setSelectionMode(QAbstractItemView::SingleSelection);
    table->setEditTriggers(QAbstractItemView::NoEditTriggers);
    table->verticalHeader()->setVisible(false);
    layout->addWidget(table, 1);

    auto *buttons = new QHBoxLayout();
    auto *add = new QPushButton(QStringLiteral("add subscription..."), this);
    auto *check = new QPushButton(QStringLiteral("check now"), this);
    auto *edit_button = new QPushButton(QStringLiteral("edit..."), this);
    auto *toggle = new QPushButton(QStringLiteral("pause / resume"), this);
    auto *remove = new QPushButton(QStringLiteral("delete"), this);
    auto *results = new QPushButton(QStringLiteral("open files"), this);
    import_options_button = new QPushButton(QStringLiteral("import options..."), this);
    buttons->addWidget(add);
    buttons->addWidget(check);
    buttons->addWidget(edit_button);
    buttons->addWidget(toggle);
    buttons->addWidget(remove);
    buttons->addWidget(results);
    buttons->addWidget(import_options_button);
    buttons->addStretch(1);
    layout->addLayout(buttons);

    connect(add, &QPushButton::clicked, this, [this]() { add_subscription(); });
    connect(check, &QPushButton::clicked, this, [this]() { with_selected(QStringLiteral("subscription_check")); });
    connect(edit_button, &QPushButton::clicked, this, [this]() { with_selected(QStringLiteral("subscription_edit")); });
    connect(toggle, &QPushButton::clicked, this, [this]() { with_selected(QStringLiteral("subscription_toggle")); });
    connect(remove, &QPushButton::clicked, this, [this]() { with_selected(QStringLiteral("subscription_delete")); });
    connect(results, &QPushButton::clicked, this, [this]() { with_selected(QStringLiteral("subscription_open_results")); });
    connect(import_options_button, &QPushButton::clicked, this, [this]() {
        with_selected(QStringLiteral("subscription_import_options"));
    });
    connect(table, &QTableWidget::cellDoubleClicked, this, [this](int, int) {
        with_selected(QStringLiteral("subscription_edit"));
    });

    update_state(state);
}

const QString &HydrusSubscriptionsPage::page_id() const {
    return id;
}

void HydrusSubscriptionsPage::with_selected(const QString &action) {
    const int row = table->currentRow();

    if (row < 0) {
        return;
    }

    QTableWidgetItem *item = table->item(row, 0);

    if (item != nullptr) {
        main->notify(action, id, item->data(Qt::UserRole).toString());
    }
}

void HydrusSubscriptionsPage::add_subscription() {
    QDialog dialog(this);
    dialog.setWindowTitle(QStringLiteral("add subscription"));
    auto *layout = new QVBoxLayout(&dialog);
    auto *form = new QFormLayout();

    auto *name = new QLineEdit(&dialog);
    auto *url = new QLineEdit(&dialog);
    auto *query = new QLineEdit(&dialog);
    auto *site = new QComboBox(&dialog);
    auto *filter = new QLineEdit(&dialog);
    auto *period = new QSpinBox(&dialog);
    period->setRange(60, 31536000);
    period->setValue(86400);

    site->addItem(QStringLiteral("auto detect"), QString());

    for (const QJsonValue &value : sites) {
        const QJsonObject parser = value.toObject();
        site->addItem(parser.value(QStringLiteral("title")).toString(), parser.value(QStringLiteral("id")).toString());
    }

    url->setPlaceholderText(QStringLiteral("https://example.com/search?q={query}"));
    query->setPlaceholderText(QStringLiteral("search terms for {query}"));
    form->addRow(QStringLiteral("name"), name);
    form->addRow(QStringLiteral("gallery url"), url);
    form->addRow(QStringLiteral("search terms"), query);
    form->addRow(QStringLiteral("site parser"), site);
    form->addRow(QStringLiteral("link filter"), filter);
    form->addRow(QStringLiteral("period (seconds)"), period);

    layout->addLayout(form);
    auto *buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, &dialog);
    connect(buttons, &QDialogButtonBox::accepted, &dialog, &QDialog::accept);
    QObject::connect(buttons, &QDialogButtonBox::rejected, &dialog, &QDialog::reject);
    layout->addWidget(buttons);

    if (dialog.exec() != QDialog::Accepted) {
        return;
    }

    QJsonObject payload;
    payload.insert(QStringLiteral("name"), name->text());
    payload.insert(QStringLiteral("url"), url->text());
    payload.insert(QStringLiteral("query"), query->text());
    payload.insert(QStringLiteral("site"), site->currentData().toString());
    payload.insert(QStringLiteral("filter"), filter->text());
    payload.insert(QStringLiteral("period"), period->value());

    main->notify(QStringLiteral("subscription_add"), id, json_payload(payload));
}

void HydrusSubscriptionsPage::update_state(const QJsonObject &state) {
    sites = state.value(QStringLiteral("sites")).toArray();
    const QJsonArray subscriptions = state.value(QStringLiteral("subscriptions")).toArray();

    status->setText(QStringLiteral("%1 subscriptions").arg(subscriptions.size()));
    table->setRowCount(subscriptions.size());

    for (int index = 0; index < subscriptions.size(); ++index) {
        const QJsonObject subscription = subscriptions.at(index).toObject();
        const QString name = subscription.value(QStringLiteral("name")).toString();

        for (int column = 0; column < table->columnCount(); ++column) {
            if (table->item(index, column) == nullptr) {
                table->setItem(index, column, new QTableWidgetItem());
            }
        }

        auto *name_item = table->item(index, 0);
        name_item->setText(name);
        name_item->setData(Qt::UserRole, name);
        name_item->setToolTip(subscription.value(QStringLiteral("url")).toString());

        table->item(index, 1)->setText(QStringLiteral("%1 s").arg(subscription.value(QStringLiteral("period")).toInt()));
        table->item(index, 2)->setText(subscription.value(QStringLiteral("last_check")).toInt() > 0
            ? QDateTime::fromSecsSinceEpoch(subscription.value(QStringLiteral("last_check")).toInt()).toString(QStringLiteral("yyyy-MM-dd hh:mm"))
            : QStringLiteral("never"));
        table->item(index, 3)->setText(subscription.value(QStringLiteral("running")).toBool()
            ? QStringLiteral("now")
            : QStringLiteral("in %1 s").arg(subscription.value(QStringLiteral("due_in")).toInt()));
        table->item(index, 4)->setText(QString::number(subscription.value(QStringLiteral("imported")).toInt()));
        table->item(index, 5)->setText(QString::number(subscription.value(QStringLiteral("failed")).toInt()));
        table->item(index, 6)->setText(subscription.value(QStringLiteral("status")).toString());

        if (subscription.value(QStringLiteral("paused")).toBool()) {
            table->item(index, 6)->setText(QStringLiteral("paused - ") + table->item(index, 6)->text());
        }
    }

    table->resizeRowsToContents();
}

HydrusSubscriptionEditorDialog::HydrusSubscriptionEditorDialog(
    HydrusMainWindow *main_window, const QJsonObject &state, QWidget *parent)
    : QDialog(parent), main(main_window) {
    setWindowTitle(QStringLiteral("subscription editor"));
    setObjectName(QStringLiteral("hydrus_subscription_editor"));
    setAttribute(Qt::WA_DeleteOnClose, true);
    resize(620, 380);

    auto *layout = new QVBoxLayout(this);
    auto *form = new QFormLayout();

    name = new QLineEdit(this);
    url = new QLineEdit(this);
    url->setPlaceholderText(QStringLiteral("https://example.com/search?q={query}"));
    query = new QLineEdit(this);
    query->setPlaceholderText(QStringLiteral("search terms for {query}"));
    site = new QComboBox(this);
    filter = new QLineEdit(this);
    filter->setPlaceholderText(QStringLiteral("optional link filter"));
    period = new QSpinBox(this);
    period->setRange(60, 31536000);
    period->setSingleStep(300);
    paused = new QCheckBox(QStringLiteral("paused"), this);

    form->addRow(QStringLiteral("name"), name);
    form->addRow(QStringLiteral("gallery url"), url);
    form->addRow(QStringLiteral("search terms"), query);
    form->addRow(QStringLiteral("site parser"), site);
    form->addRow(QStringLiteral("link filter"), filter);
    form->addRow(QStringLiteral("period (seconds)"), period);
    form->addRow(QString(), paused);
    layout->addLayout(form);

    auto *help = new QLabel(
        QStringLiteral("Subscriptions use the page URL as a template; {query} is replaced with the "
                       "search terms. Import options are edited from the subscriptions page."),
        this);
    help->setWordWrap(true);
    layout->addWidget(help);
    layout->addStretch(1);

    auto *buttons = new QDialogButtonBox(QDialogButtonBox::Save | QDialogButtonBox::Cancel, this);
    connect(buttons, &QDialogButtonBox::accepted, this, [this]() { save(); });
    connect(buttons, &QDialogButtonBox::rejected, this, &QDialog::close);
    layout->addWidget(buttons);

    set_state(state);
}

void HydrusSubscriptionEditorDialog::set_state(const QJsonObject &state) {
    data = state;
    page_id = state.value(QStringLiteral("page_id")).toString();

    name->setText(state.value(QStringLiteral("name")).toString());
    url->setText(state.value(QStringLiteral("url")).toString());
    query->setText(state.value(QStringLiteral("query")).toString());
    filter->setText(state.value(QStringLiteral("filter")).toString());
    period->setValue(state.value(QStringLiteral("period")).toInt(86400));
    paused->setChecked(state.value(QStringLiteral("paused")).toBool());

    site->clear();
    site->addItem(QStringLiteral("auto detect"), QString());

    for (const QJsonValue &value : state.value(QStringLiteral("sites")).toArray()) {
        const QJsonObject parser = value.toObject();
        site->addItem(
            parser.value(QStringLiteral("title")).toString(),
            parser.value(QStringLiteral("id")).toString());
    }

    const int index = site->findData(state.value(QStringLiteral("site")).toString());

    if (index >= 0) {
        site->setCurrentIndex(index);
    }
}

void HydrusSubscriptionEditorDialog::save() {
    QJsonObject payload;
    payload.insert(QStringLiteral("original"), data.value(QStringLiteral("original")));
    payload.insert(QStringLiteral("name"), name->text().trimmed());
    payload.insert(QStringLiteral("url"), url->text().trimmed());
    payload.insert(QStringLiteral("query"), query->text());
    payload.insert(QStringLiteral("site"), site->currentData().toString());
    payload.insert(QStringLiteral("filter"), filter->text());
    payload.insert(QStringLiteral("period"), period->value());
    payload.insert(QStringLiteral("paused"), paused->isChecked());

    main->notify(
        QStringLiteral("subscription_edit_save"),
        page_id,
        json_payload(payload));

    close();
}

void HydrusMainWindow::show_subscription_editor(const QJsonObject &state) {
    if (subscription_editor_dialog != nullptr) {
        subscription_editor_dialog->close();
    }

    subscription_editor_dialog = new HydrusSubscriptionEditorDialog(this, state, this);
    connect(subscription_editor_dialog, &QObject::destroyed, this, [this]() {
        subscription_editor_dialog = nullptr;
    });
    subscription_editor_dialog->show();
    subscription_editor_dialog->raise();
    subscription_editor_dialog->activateWindow();
}
