
// WS-15.1.3c2: account types dialog moved verbatim from bridge.cpp (move-only split).

#include <hydrus_client/src/gui/qt/api.cxx.h>
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

void show_account_type_editor(HydrusMainWindow *main, QWidget *parent, const std::vector<AccountPermissionGroupInfo> &possible, const AccountTypeInfo *entry) {
    const bool adding = entry == nullptr;
    const QString key = adding ? QString() : entry->key;

    QDialog dialog(parent);
    dialog.setWindowTitle(adding ? QStringLiteral("add account type") : QStringLiteral("edit account type"));
    dialog.setObjectName(QStringLiteral("hydrus_account_type_editor"));
    dialog.resize(640, 620);

    auto *layout = new QVBoxLayout(&dialog);
    auto *form = new QFormLayout();

    auto *title = new QLineEdit(adding ? QString() : entry->title, &dialog);
    form->addRow(QStringLiteral("title"), title);

    auto *accounts = new QSpinBox(&dialog);
    accounts->setRange(0, 1000000);
    accounts->setValue(adding ? 0 : entry->auto_create_accounts);
    auto *time_delta = new QSpinBox(&dialog);
    time_delta->setRange(60, 365 * 86400);
    time_delta->setValue(adding ? 86400 : entry->auto_create_time_delta);
    time_delta->setSuffix(QStringLiteral(" s"));

    auto *velocity_row = new QHBoxLayout();
    velocity_row->addWidget(accounts);
    velocity_row->addWidget(new QLabel(QStringLiteral("account(s) per"), &dialog));
    velocity_row->addWidget(time_delta);
    form->addRow(QStringLiteral("auto-creation"), velocity_row);
    layout->addLayout(form);

    QHash<int, QComboBox *> permission_controls;
    QHash<int, int> current_actions;

    if (!adding) {
        for (const AccountPermissionInfo &pair : entry->permissions) {
            current_actions.insert(pair.content_type, pair.has_action ? pair.action : -1);
        }
    }

    auto *permissions_box = new QGroupBox(QStringLiteral("permissions"), &dialog);
    auto *permissions_form = new QFormLayout(permissions_box);

    for (const AccountPermissionGroupInfo &content_type : possible) {
        auto *combo = new QComboBox(permissions_box);
        combo->setObjectName(QStringLiteral("account_type_permission_%1").arg(content_type.content_type));

        for (const AccountPermissionActionInfo &action : content_type.actions) {
            combo->addItem(action.label, action.has_value ? action.value : -1);
        }

        const int wanted = current_actions.value(content_type.content_type, -1);
        const int index = combo->findData(wanted);
        combo->setCurrentIndex(index >= 0 ? index : 0);
        permissions_form->addRow(content_type.label, combo);
        permission_controls.insert(content_type.content_type, combo);
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

    rust::Vec<int32_t> content_types;
    rust::Vec<int32_t> actions;

    for (auto iterator = permission_controls.constBegin(); iterator != permission_controls.constEnd(); ++iterator) {
        const int action = iterator.value()->currentData().toInt();

        if (action < 0) {
            continue;
        }

        content_types.push_back(iterator.key());
        actions.push_back(action);
    }

    const QByteArray key_bytes = key.toUtf8();
    const QByteArray title_bytes = title->text().trimmed().toUtf8();
    app_account_type_save(
        *main,
        rust::Str(key_bytes.constData(), key_bytes.size()),
        rust::Str(title_bytes.constData(), title_bytes.size()),
        std::move(content_types),
        std::move(actions),
        static_cast<int64_t>(accounts->value()),
        static_cast<int64_t>(time_delta->value()));
}

HydrusAccountTypesDialog::HydrusAccountTypesDialog(HydrusMainWindow *main_window, const AccountTypesDialogState &state, QWidget *parent)
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
            const int index = table->item(row, 0) != nullptr
                ? table->item(row, 0)->data(Qt::UserRole).toInt()
                : -1;

            if (index >= 0 && index < static_cast<int>(type_infos.size())
                && type_infos[static_cast<size_t>(index)].key == edit_key) {
                table->selectRow(row);
                edit_selected();
                break;
            }
        }
    }
}

void HydrusAccountTypesDialog::set_state(const AccountTypesDialogState &state) {
    type_infos.clear();
    possible_infos.clear();

    for (const AccountTypeRow &entry : state.rows) {
        AccountTypeInfo info;
        info.key = to_qstring(entry.key);
        info.title = to_qstring(entry.title);
        info.summary = to_qstring(entry.summary);
        info.auto_creation_text = to_qstring(entry.auto_creation_text);
        info.is_null = entry.is_null;
        info.auto_create_accounts = entry.auto_create_accounts;
        info.auto_create_time_delta = entry.auto_create_time_delta;

        for (const AccountPermissionPair &pair : entry.permissions) {
            info.permissions.push_back({pair.content_type, pair.action, pair.has_action});
        }

        type_infos.push_back(std::move(info));
    }

    for (const AccountPermissionGroup &group : state.possible) {
        AccountPermissionGroupInfo info;
        info.content_type = group.content_type;
        info.label = to_qstring(group.label);

        for (const AccountPermissionAction &action : group.actions) {
            info.actions.push_back({action.value, action.has_value, to_qstring(action.label)});
        }

        possible_infos.push_back(std::move(info));
    }

    table->setRowCount(static_cast<int>(type_infos.size()));

    for (size_t type_index = 0; type_index < type_infos.size(); ++type_index) {
        const AccountTypeInfo &entry = type_infos[type_index];
        const int index = static_cast<int>(type_index);

        for (int column = 0; column < 3; ++column) {
            if (table->item(index, column) == nullptr) {
                table->setItem(index, column, new QTableWidgetItem());
            }
        }

        auto *title_item = table->item(index, 0);
        title_item->setText(entry.title);
        title_item->setData(Qt::UserRole, index);

        if (entry.is_null) {
            title_item->setForeground(QColor(120, 150, 190));
        }

        table->item(index, 1)->setText(entry.summary);

        table->item(index, 2)->setText(entry.auto_creation_text);
    }

    table->resizeRowsToContents();
}

int HydrusAccountTypesDialog::selected_index() const {
    const int row = table->currentRow();

    if (row < 0 || table->item(row, 0) == nullptr) {
        return -1;
    }

    const int index = table->item(row, 0)->data(Qt::UserRole).toInt();

    if (index < 0 || index >= static_cast<int>(type_infos.size())) {
        return -1;
    }

    return index;
}

void HydrusAccountTypesDialog::add_entry() {
    show_account_type_editor(main, this, possible_infos, nullptr);
}

void HydrusAccountTypesDialog::edit_selected() {
    const int index = selected_index();

    if (index < 0) {
        return;
    }

    const AccountTypeInfo &entry = type_infos[static_cast<size_t>(index)];

    if (entry.is_null) {
        QMessageBox::information(this, QStringLiteral("account type"),
                                 QStringLiteral("The null account type cannot be edited."));
        return;
    }

    show_account_type_editor(main, this, possible_infos, &entry);
}

void HydrusAccountTypesDialog::delete_selected() {
    const int index = selected_index();

    if (index < 0) {
        return;
    }

    const AccountTypeInfo &entry = type_infos[static_cast<size_t>(index)];

    if (entry.is_null) {
        QMessageBox::information(this, QStringLiteral("account type"),
                                 QStringLiteral("The null account type cannot be deleted."));
        return;
    }

    QStringList titles;
    QStringList keys;

    for (const AccountTypeInfo &other : type_infos) {
        if (other.key == entry.key || other.is_null) {
            continue;
        }

        titles.append(other.title);
        keys.append(other.key);
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

    const QByteArray key_bytes = entry.key.toUtf8();
    const QByteArray replacement_bytes = keys.at(titles.indexOf(title)).toUtf8();
    app_account_type_delete(
        *main,
        rust::Str(key_bytes.constData(), key_bytes.size()),
        rust::Str(replacement_bytes.constData(), replacement_bytes.size()));
}

void HydrusMainWindow::show_account_types(const AccountTypesDialogState &typed) {
    if (account_types_dialog == nullptr) {
        account_types_dialog = new HydrusAccountTypesDialog(this, typed, this);
        connect(account_types_dialog, &QObject::destroyed, this, [this]() { account_types_dialog = nullptr; });
        account_types_dialog->show();
    } else {
        account_types_dialog->set_state(typed);
        account_types_dialog->raise();
        account_types_dialog->activateWindow();
    }
}

void HydrusMainWindow::update_account_types(const AccountTypesDialogState &typed) {
    if (account_types_dialog != nullptr) {
        account_types_dialog->set_state(typed);
    }
}
