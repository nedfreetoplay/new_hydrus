#include "account_types.hpp"

// WS-15.1.3c2: account types dialog moved verbatim from bridge.cpp (move-only split).

#include "bridge.hpp"
#include "support/support.hpp"

#include <QComboBox>
#include <QDialog>
#include <QDialogButtonBox>
#include <QFormLayout>
#include <QGroupBox>
#include <QHBoxLayout>
#include <QHash>
#include <QHeaderView>
#include <QInputDialog>
#include <QJsonArray>
#include <QLabel>
#include <QMessageBox>
#include <QPushButton>
#include <QSpinBox>
#include <QTableWidget>
#include <QVBoxLayout>

void show_account_type_editor(HydrusMainWindow *main, QWidget *parent, const QJsonObject &data, const QJsonObject &entry) {
    const bool adding = entry.isEmpty();
    const QString key = entry.value(QStringLiteral("key")).toString();

    QDialog dialog(parent);
    dialog.setWindowTitle(adding ? QStringLiteral("add account type") : QStringLiteral("edit account type"));
    dialog.setObjectName(QStringLiteral("hydrus_account_type_editor"));
    dialog.resize(640, 620);

    auto *layout = new QVBoxLayout(&dialog);
    auto *form = new QFormLayout();

    auto *title = new QLineEdit(entry.value(QStringLiteral("title")).toString(), &dialog);
    form->addRow(QStringLiteral("title"), title);

    auto *accounts = new QSpinBox(&dialog);
    accounts->setRange(0, 1000000);
    accounts->setValue(entry.value(QStringLiteral("auto_create_accounts")).toInt(0));
    auto *time_delta = new QSpinBox(&dialog);
    time_delta->setRange(60, 365 * 86400);
    time_delta->setValue(entry.value(QStringLiteral("auto_create_time_delta")).toInt(86400));
    time_delta->setSuffix(QStringLiteral(" s"));

    auto *velocity_row = new QHBoxLayout();
    velocity_row->addWidget(accounts);
    velocity_row->addWidget(new QLabel(QStringLiteral("account(s) per"), &dialog));
    velocity_row->addWidget(time_delta);
    form->addRow(QStringLiteral("auto-creation"), velocity_row);
    layout->addLayout(form);

    QHash<int, QComboBox *> permission_controls;
    QJsonArray entry_permissions = entry.value(QStringLiteral("permissions")).toArray();
    QHash<int, int> current_actions;

    for (const QJsonValue &value : entry_permissions) {
        const QJsonArray pair = value.toArray();

        if (pair.size() == 2) {
            current_actions.insert(pair.at(0).toInt(), pair.at(1).isNull() ? -1 : pair.at(1).toInt());
        }
    }

    auto *permissions_box = new QGroupBox(QStringLiteral("permissions"), &dialog);
    auto *permissions_form = new QFormLayout(permissions_box);

    for (const QJsonValue &value : data.value(QStringLiteral("possible")).toArray()) {
        const QJsonObject content_type = value.toObject();
        auto *combo = new QComboBox(permissions_box);
        combo->setObjectName(QStringLiteral("account_type_permission_%1")
                                 .arg(content_type.value(QStringLiteral("content_type")).toInt()));

        for (const QJsonValue &action_value : content_type.value(QStringLiteral("actions")).toArray()) {
            const QJsonObject action = action_value.toObject();
            const QJsonValue action_number = action.value(QStringLiteral("value"));

            combo->addItem(action.value(QStringLiteral("label")).toString(),
                           action_number.isNull() ? -1 : action_number.toInt());
        }

        const int wanted = current_actions.value(content_type.value(QStringLiteral("content_type")).toInt(), -1);
        const int index = combo->findData(wanted);
        combo->setCurrentIndex(index >= 0 ? index : 0);
        permissions_form->addRow(content_type.value(QStringLiteral("label")).toString(), combo);
        permission_controls.insert(content_type.value(QStringLiteral("content_type")).toInt(), combo);
    }

    layout->addWidget(permissions_box, 1);

    auto *note = new QLabel(
        QStringLiteral("Bandwidth rules and auto-creation history are preserved. "
                       "Permissions without an entry mean no permission."),
        &dialog);
    note->setWordWrap(true);
    layout->addWidget(note);

    auto *buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, &dialog);
    layout->addWidget(buttons);
    QObject::connect(buttons, &QDialogButtonBox::rejected, &dialog, &QDialog::reject);
    QObject::connect(buttons, &QDialogButtonBox::accepted, &dialog, [&]() {
        if (title->text().trimmed().isEmpty()) {
            QMessageBox::warning(&dialog, QStringLiteral("account type"),
                                 QStringLiteral("Enter a title."));
            return;
        }

        dialog.accept();
    });

    if (dialog.exec() != QDialog::Accepted) {
        return;
    }

    QJsonArray permissions;

    for (auto iterator = permission_controls.constBegin(); iterator != permission_controls.constEnd(); ++iterator) {
        const int action = iterator.value()->currentData().toInt();

        if (action < 0) {
            continue;
        }

        permissions.append(QJsonArray{iterator.key(), action});
    }

    QJsonObject payload;
    payload.insert(QStringLiteral("account_type_key"), key);
    payload.insert(QStringLiteral("title"), title->text().trimmed());
    payload.insert(QStringLiteral("permissions"), permissions);
    payload.insert(QStringLiteral("auto_create_accounts"), accounts->value());
    payload.insert(QStringLiteral("auto_create_time_delta"), time_delta->value());

    main->notify(QStringLiteral("account_type_save"), QString(), json_payload(payload));
}

HydrusAccountTypesDialog::HydrusAccountTypesDialog(HydrusMainWindow *main_window, const QJsonObject &state, QWidget *parent)
    : QDialog(parent), main(main_window) {
    setWindowTitle(QStringLiteral("account types"));
    setObjectName(QStringLiteral("hydrus_account_types_dialog"));
    setAttribute(Qt::WA_DeleteOnClose, true);
    resize(820, 480);

    auto *layout = new QVBoxLayout(this);

    table = new QTableWidget(this);
    table->setColumnCount(3);
    table->setHorizontalHeaderLabels({
        QStringLiteral("title"),
        QStringLiteral("permissions"),
        QStringLiteral("auto-creation")
    });
    table->horizontalHeader()->setSectionResizeMode(0, QHeaderView::ResizeToContents);
    table->horizontalHeader()->setSectionResizeMode(1, QHeaderView::Stretch);
    table->horizontalHeader()->setSectionResizeMode(2, QHeaderView::ResizeToContents);
    table->setSelectionBehavior(QAbstractItemView::SelectRows);
    table->setSelectionMode(QAbstractItemView::SingleSelection);
    table->setEditTriggers(QAbstractItemView::NoEditTriggers);
    table->verticalHeader()->setVisible(false);
    layout->addWidget(table, 1);

    auto *buttons = new QHBoxLayout();
    auto *add = new QPushButton(QStringLiteral("add account type..."), this);
    auto *edit = new QPushButton(QStringLiteral("edit..."), this);
    auto *remove = new QPushButton(QStringLiteral("delete"), this);
    buttons->addWidget(add);
    buttons->addWidget(edit);
    buttons->addWidget(remove);
    buttons->addStretch(1);
    layout->addLayout(buttons);

    connect(add, &QPushButton::clicked, this, [this]() { add_entry(); });
    connect(edit, &QPushButton::clicked, this, [this]() { edit_selected(); });
    connect(remove, &QPushButton::clicked, this, [this]() { delete_selected(); });
    connect(table, &QTableWidget::cellDoubleClicked, this, [this](int, int) { edit_selected(); });

    auto *close = new QPushButton(QStringLiteral("close"), this);
    auto *footer = new QHBoxLayout();
    footer->addStretch(1);
    footer->addWidget(close);
    layout->addLayout(footer);
    connect(close, &QPushButton::clicked, this, &QDialog::close);

    set_state(state);

    const QString edit_key = qEnvironmentVariable("HYDRUS_QT_ACCOUNT_TYPE_EDIT");

    if (edit_key == QStringLiteral("new")) {
        add_entry();
    } else if (!edit_key.isEmpty()) {
        for (int row = 0; row < table->rowCount(); ++row) {
            if (table->item(row, 0) != nullptr
                && table->item(row, 0)->data(Qt::UserRole).toJsonObject().value(QStringLiteral("key")).toString()
                       == edit_key) {
                table->selectRow(row);
                edit_selected();
                break;
            }
        }
    }
}

void HydrusAccountTypesDialog::set_state(const QJsonObject &state) {
    data = state;

    const QJsonArray types = state.value(QStringLiteral("types")).toArray();
    table->setRowCount(types.size());

    for (int index = 0; index < types.size(); ++index) {
        const QJsonObject entry = types.at(index).toObject();

        for (int column = 0; column < 3; ++column) {
            if (table->item(index, column) == nullptr) {
                table->setItem(index, column, new QTableWidgetItem());
            }
        }

        auto *title_item = table->item(index, 0);
        title_item->setText(entry.value(QStringLiteral("title")).toString());
        title_item->setData(Qt::UserRole, entry);

        if (entry.value(QStringLiteral("is_null")).toBool()) {
            title_item->setForeground(QColor(120, 150, 190));
        }

        table->item(index, 1)->setText(entry.value(QStringLiteral("summary")).toString());

        const int accounts = entry.value(QStringLiteral("auto_create_accounts")).toInt();
        table->item(index, 2)->setText(accounts == 0
                                           ? QStringLiteral("disabled")
                                           : QStringLiteral("%1 / %2 s")
                                                 .arg(accounts)
                                                 .arg(entry.value(QStringLiteral("auto_create_time_delta")).toInt()));
    }

    table->resizeRowsToContents();
}

QJsonObject HydrusAccountTypesDialog::selected_entry() const {
    const int row = table->currentRow();

    if (row < 0 || table->item(row, 0) == nullptr) {
        return {};
    }

    return table->item(row, 0)->data(Qt::UserRole).toJsonObject();
}

void HydrusAccountTypesDialog::add_entry() {
    show_account_type_editor(main, this, data, QJsonObject());
}

void HydrusAccountTypesDialog::edit_selected() {
    const QJsonObject entry = selected_entry();

    if (!entry.isEmpty()) {
        if (entry.value(QStringLiteral("is_null")).toBool()) {
            QMessageBox::information(this, QStringLiteral("account type"),
                                     QStringLiteral("The null account type cannot be edited."));
            return;
        }

        show_account_type_editor(main, this, data, entry);
    }
}

void HydrusAccountTypesDialog::delete_selected() {
    const QJsonObject entry = selected_entry();

    if (entry.isEmpty()) {
        return;
    }

    if (entry.value(QStringLiteral("is_null")).toBool()) {
        QMessageBox::information(this, QStringLiteral("account type"),
                                 QStringLiteral("The null account type cannot be deleted."));
        return;
    }

    QStringList titles;
    QStringList keys;

    for (const QJsonValue &value : data.value(QStringLiteral("types")).toArray()) {
        const QJsonObject other = value.toObject();

        if (other.value(QStringLiteral("key")).toString() == entry.value(QStringLiteral("key")).toString()
            || other.value(QStringLiteral("is_null")).toBool()) {
            continue;
        }

        titles.append(other.value(QStringLiteral("title")).toString());
        keys.append(other.value(QStringLiteral("key")).toString());
    }

    if (titles.isEmpty()) {
        QMessageBox::information(this, QStringLiteral("account type"),
                                 QStringLiteral("There is no other account type to move accounts to."));
        return;
    }

    bool ok = false;
    const QString title = QInputDialog::getItem(
        this, QStringLiteral("delete account type"),
        QStringLiteral("Move its accounts to:"), titles, 0, false, &ok);

    if (!ok) {
        return;
    }

    QJsonObject payload;
    payload.insert(QStringLiteral("account_type_key"), entry.value(QStringLiteral("key")));
    payload.insert(QStringLiteral("replacement_account_type_key"), keys.at(titles.indexOf(title)));

    main->notify(QStringLiteral("account_type_delete"), QString(), json_payload(payload));
}

void HydrusMainWindow::show_account_types(const QJsonObject &state) {
    if (account_types_dialog == nullptr) {
        account_types_dialog = new HydrusAccountTypesDialog(this, state, this);
        connect(account_types_dialog, &QObject::destroyed, this, [this]() { account_types_dialog = nullptr; });
        account_types_dialog->show();
    } else {
        account_types_dialog->set_state(state);
        account_types_dialog->raise();
        account_types_dialog->activateWindow();
    }
}

void HydrusMainWindow::update_account_types(const QJsonObject &state) {
    if (account_types_dialog != nullptr) {
        account_types_dialog->set_state(state);
    }
}
