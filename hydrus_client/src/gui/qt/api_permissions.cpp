#include "api_permissions.hpp"

// WS-15.1.3c2: client api permissions dialog moved verbatim from bridge.cpp (move-only split).

#include "bridge.hpp"
#include "support/support.hpp"

#include <QCheckBox>
#include <QDialog>
#include <QDialogButtonBox>
#include <QFormLayout>
#include <QHBoxLayout>
#include <QHeaderView>
#include <QJsonArray>
#include <QLabel>
#include <QLineEdit>
#include <QListWidget>
#include <QMessageBox>
#include <QPlainTextEdit>
#include <QPushButton>
#include <QRandomGenerator>
#include <QSet>
#include <QTabWidget>
#include <QTableWidget>
#include <QVBoxLayout>

QString api_permission_slice_display(const QString &slice) {
    if (slice.isEmpty()) {
        return QStringLiteral("*");
    }

    if (slice == QStringLiteral(":")) {
        return QStringLiteral("*:*");
    }

    return slice;
}

void show_api_permission_editor(HydrusMainWindow *main, QWidget *parent, const QJsonObject &data, const QJsonObject &entry) {
    const QString original = entry.value(QStringLiteral("access_key")).toString();

    QDialog dialog(parent);
    dialog.setWindowTitle(original.isEmpty() ? QStringLiteral("add api permissions")
                                             : QStringLiteral("edit api permissions"));
    dialog.setObjectName(QStringLiteral("hydrus_api_permission_editor"));
    dialog.resize(680, 620);

    auto *layout = new QVBoxLayout(&dialog);
    auto *form = new QFormLayout();

    auto *name = new QLineEdit(entry.value(QStringLiteral("name")).toString(), &dialog);
    auto *access_key = new QLineEdit(original, &dialog);
    access_key->setReadOnly(true);
    auto *generate = new QPushButton(QStringLiteral("generate new key"), &dialog);

    auto *key_row = new QHBoxLayout();
    key_row->addWidget(access_key, 1);
    key_row->addWidget(generate);

    form->addRow(QStringLiteral("name"), name);
    form->addRow(QStringLiteral("access key"), key_row);
    layout->addLayout(form);

    QObject::connect(generate, &QPushButton::clicked, &dialog, [access_key]() {
        QString key;

        for (int index = 0; index < 4; ++index) {
            key += QStringLiteral("%1").arg(QRandomGenerator::system()->generate64(), 16, 16, QLatin1Char('0'));
        }

        access_key->setText(key);
    });

    auto *permits_everything = new QCheckBox(QStringLiteral("permits everything"), &dialog);
    permits_everything->setChecked(entry.value(QStringLiteral("permits_everything")).toBool(true));
    layout->addWidget(permits_everything);

    auto *permission_list = new QListWidget(&dialog);
    permission_list->setObjectName(QStringLiteral("api_permission_list"));

    QJsonArray existing_permissions = entry.value(QStringLiteral("basic_permissions")).toArray();
    QSet<int> checked_permissions;

    for (const QJsonValue &value : existing_permissions) {
        checked_permissions.insert(value.toInt());
    }

    for (const QJsonValue &value : data.value(QStringLiteral("available_permissions")).toArray()) {
        const QJsonObject permission = value.toObject();
        auto *item = new QListWidgetItem(permission.value(QStringLiteral("label")).toString(), permission_list);
        item->setData(Qt::UserRole, permission.value(QStringLiteral("value")).toInt());
        item->setFlags(item->flags() | Qt::ItemIsUserCheckable);
        item->setCheckState(checked_permissions.contains(permission.value(QStringLiteral("value")).toInt())
                                ? Qt::Checked
                                : Qt::Unchecked);
    }

    layout->addWidget(new QLabel(QStringLiteral("basic permissions"), &dialog));
    layout->addWidget(permission_list, 1);

    auto *tabs = new QTabWidget(&dialog);
    auto *whitelist = new QPlainTextEdit(tabs);
    whitelist->setPlaceholderText(QStringLiteral("character:"));
    auto *blacklist = new QPlainTextEdit(tabs);
    blacklist->setPlaceholderText(QStringLiteral("*\n*:*"));

    QStringList whitelist_lines;
    QStringList blacklist_lines;

    for (const QJsonValue &value : entry.value(QStringLiteral("search_tag_filter")).toArray()) {
        const QJsonArray pair = value.toArray();

        if (pair.size() != 2) {
            continue;
        }

        const QString display = api_permission_slice_display(pair.at(0).toString());

        if (pair.at(1).toString() == QStringLiteral("whitelist")) {
            whitelist_lines.append(display);
        } else {
            blacklist_lines.append(display);
        }
    }

    whitelist->setPlainText(whitelist_lines.join(QLatin1Char('\n')));
    blacklist->setPlainText(blacklist_lines.join(QLatin1Char('\n')));
    tabs->addTab(whitelist, QStringLiteral("allowed search tags"));
    tabs->addTab(blacklist, QStringLiteral("denied search tags"));
    layout->addWidget(tabs, 1);

    auto collect_rules = [](QPlainTextEdit *edit, const QString &rule) {
        QJsonArray rules;

        for (const QString &line : edit->toPlainText().split(QLatin1Char('\n'))) {
            const QString trimmed = line.trimmed();

            if (trimmed.isEmpty()) {
                continue;
            }

            QString slice = trimmed;

            if (trimmed == QStringLiteral("*")) {
                slice = QString();
            } else if (trimmed == QStringLiteral("*:*") || trimmed == QStringLiteral("*:")) {
                slice = QStringLiteral(":");
            }

            rules.append(QJsonArray{slice, rule});
        }

        return rules;
    };

    auto update_enabled = [permits_everything, permission_list]() {
        permission_list->setEnabled(!permits_everything->isChecked());
    };
    QObject::connect(permits_everything, &QCheckBox::clicked, &dialog, [update_enabled](bool) { update_enabled(); });
    update_enabled();

    auto *buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, &dialog);
    layout->addWidget(buttons);
    QObject::connect(buttons, &QDialogButtonBox::rejected, &dialog, &QDialog::reject);
    QObject::connect(buttons, &QDialogButtonBox::accepted, &dialog, [&]() {
        if (name->text().trimmed().isEmpty()) {
            QMessageBox::warning(&dialog, QStringLiteral("api permissions"),
                                 QStringLiteral("Enter a name for this access key."));
            return;
        }

        if (access_key->text().trimmed().size() != 64) {
            QMessageBox::warning(&dialog, QStringLiteral("api permissions"),
                                 QStringLiteral("The access key must be 64 hex characters."));
            return;
        }

        dialog.accept();
    });

    if (dialog.exec() != QDialog::Accepted) {
        return;
    }

    QJsonArray permissions;

    for (int index = 0; index < permission_list->count(); ++index) {
        QListWidgetItem *item = permission_list->item(index);

        if (item->checkState() == Qt::Checked) {
            permissions.append(item->data(Qt::UserRole).toInt());
        }
    }

    QJsonArray rules = collect_rules(whitelist, QStringLiteral("whitelist"));

    for (const QJsonValue &rule : collect_rules(blacklist, QStringLiteral("blacklist"))) {
        rules.append(rule);
    }

    QJsonObject payload;
    payload.insert(QStringLiteral("name"), name->text().trimmed());
    payload.insert(QStringLiteral("access_key"), access_key->text().trimmed().toLower());
    payload.insert(QStringLiteral("original_access_key"), original);
    payload.insert(QStringLiteral("permits_everything"), permits_everything->isChecked());
    payload.insert(QStringLiteral("basic_permissions"), permissions);
    payload.insert(QStringLiteral("rules"), rules);

    main->notify(QStringLiteral("api_permissions_save"), QString(), json_payload(payload));
}

HydrusApiPermissionsDialog::HydrusApiPermissionsDialog(HydrusMainWindow *main_window, const QJsonObject &state, QWidget *parent)
    : QDialog(parent), main(main_window) {
    setWindowTitle(QStringLiteral("client api permissions"));
    setObjectName(QStringLiteral("hydrus_api_permissions_dialog"));
    setAttribute(Qt::WA_DeleteOnClose, true);
    resize(860, 480);

    auto *layout = new QVBoxLayout(this);

    status = new QLabel(this);
    status->setWordWrap(true);
    layout->addWidget(status);

    table = new QTableWidget(this);
    table->setColumnCount(3);
    table->setHorizontalHeaderLabels({
        QStringLiteral("name"),
        QStringLiteral("access key"),
        QStringLiteral("permissions")
    });
    table->horizontalHeader()->setSectionResizeMode(0, QHeaderView::ResizeToContents);
    table->horizontalHeader()->setSectionResizeMode(1, QHeaderView::Stretch);
    table->horizontalHeader()->setSectionResizeMode(2, QHeaderView::Stretch);
    table->setSelectionBehavior(QAbstractItemView::SelectRows);
    table->setSelectionMode(QAbstractItemView::SingleSelection);
    table->setEditTriggers(QAbstractItemView::NoEditTriggers);
    table->verticalHeader()->setVisible(false);
    layout->addWidget(table, 1);

    auto *buttons = new QHBoxLayout();
    auto *add = new QPushButton(QStringLiteral("add access key..."), this);
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

    const QString edit_key = qEnvironmentVariable("HYDRUS_QT_API_EDIT");

    if (edit_key == QStringLiteral("new")) {
        add_entry();
    } else if (!edit_key.isEmpty()) {
        for (int row = 0; row < table->rowCount(); ++row) {
            if (table->item(row, 1) != nullptr
                && table->item(row, 1)->text().compare(edit_key, Qt::CaseInsensitive) == 0) {
                table->selectRow(row);
                edit_selected();
                break;
            }
        }
    }
}

void HydrusApiPermissionsDialog::set_state(const QJsonObject &state) {
    data = state;
    status->setText(QStringLiteral("api: %1").arg(state.value(QStringLiteral("api_status")).toString()));

    const QJsonArray permissions = state.value(QStringLiteral("permissions")).toArray();
    table->setRowCount(permissions.size());

    for (int index = 0; index < permissions.size(); ++index) {
        const QJsonObject entry = permissions.at(index).toObject();

        for (int column = 0; column < 3; ++column) {
            if (table->item(index, column) == nullptr) {
                table->setItem(index, column, new QTableWidgetItem());
            }
        }

        auto *name_item = table->item(index, 0);
        name_item->setText(entry.value(QStringLiteral("name")).toString());
        name_item->setData(Qt::UserRole, entry);
        table->item(index, 1)->setText(entry.value(QStringLiteral("access_key")).toString());
        table->item(index, 2)->setText(entry.value(QStringLiteral("summary")).toString());
    }

    table->resizeRowsToContents();
}

QJsonObject HydrusApiPermissionsDialog::selected_entry() const {
    const int row = table->currentRow();

    if (row < 0 || table->item(row, 0) == nullptr) {
        return {};
    }

    return table->item(row, 0)->data(Qt::UserRole).toJsonObject();
}

void HydrusApiPermissionsDialog::add_entry() {
    show_api_permission_editor(main, this, data, QJsonObject());
}

void HydrusApiPermissionsDialog::edit_selected() {
    const QJsonObject entry = selected_entry();

    if (!entry.isEmpty()) {
        show_api_permission_editor(main, this, data, entry);
    }
}

void HydrusApiPermissionsDialog::delete_selected() {
    const QJsonObject entry = selected_entry();

    if (entry.isEmpty()) {
        return;
    }

    if (QMessageBox::question(
            this,
            QStringLiteral("delete access key"),
            QStringLiteral("Delete '%1'? Anything using that key will lose access.")
                .arg(entry.value(QStringLiteral("name")).toString()))
        != QMessageBox::Yes) {
        return;
    }

    main->notify(QStringLiteral("api_permissions_delete"), QString(),
                 entry.value(QStringLiteral("access_key")).toString());
}

void HydrusMainWindow::show_api_permissions(const QJsonObject &state) {
    if (api_permissions_dialog == nullptr) {
        api_permissions_dialog = new HydrusApiPermissionsDialog(this, state, this);
        connect(api_permissions_dialog, &QObject::destroyed, this, [this]() { api_permissions_dialog = nullptr; });
        api_permissions_dialog->show();
    } else {
        api_permissions_dialog->set_state(state);
        api_permissions_dialog->raise();
        api_permissions_dialog->activateWindow();
    }
}

void HydrusMainWindow::update_api_permissions(const QJsonObject &state) {
    if (api_permissions_dialog != nullptr) {
        api_permissions_dialog->set_state(state);
    }
}
