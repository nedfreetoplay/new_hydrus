
// WS-15.1.3c2: client api permissions dialog moved verbatim from bridge.cpp (move-only split).

#include <hydrus_client/src/gui/qt/api.cxx.h>
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

void show_api_permission_editor(HydrusMainWindow *main, QWidget *parent, const std::vector<ApiAvailablePermissionInfo> &available, const ApiPermissionInfo *entry) {
    const QString original = entry == nullptr ? QString() : entry->access_key;

    QDialog dialog(parent);
    dialog.setWindowTitle(original.isEmpty() ? QStringLiteral("add api permissions")
                                             : QStringLiteral("edit api permissions"));
    dialog.setObjectName(QStringLiteral("hydrus_api_permission_editor"));
    dialog.resize(680, 620);

    auto *layout = new QVBoxLayout(&dialog);
    auto *form = new QFormLayout();

    auto *name = new QLineEdit(entry == nullptr ? QString() : entry->name, &dialog);
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
    permits_everything->setChecked(entry == nullptr ? true : entry->permits_everything);
    layout->addWidget(permits_everything);

    auto *permission_list = new QListWidget(&dialog);
    permission_list->setObjectName(QStringLiteral("api_permission_list"));

    QSet<int> checked_permissions;

    if (entry != nullptr) {
        for (int value : entry->basic_permissions) {
            checked_permissions.insert(value);
        }
    }

    for (const ApiAvailablePermissionInfo &permission : available) {
        auto *item = new QListWidgetItem(permission.label, permission_list);
        item->setData(Qt::UserRole, permission.value);
        item->setFlags(item->flags() | Qt::ItemIsUserCheckable);
        item->setCheckState(checked_permissions.contains(permission.value) ? Qt::Checked : Qt::Unchecked);
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

    if (entry != nullptr) {
        for (const ApiPermissionRuleInfo &rule : entry->rules) {
            if (rule.rule == QStringLiteral("whitelist")) {
                whitelist_lines.append(rule.display);
            } else {
                blacklist_lines.append(rule.display);
            }
        }
    }

    whitelist->setPlainText(whitelist_lines.join(QLatin1Char('\n')));
    blacklist->setPlainText(blacklist_lines.join(QLatin1Char('\n')));
    tabs->addTab(whitelist, QStringLiteral("allowed search tags"));
    tabs->addTab(blacklist, QStringLiteral("denied search tags"));
    layout->addWidget(tabs, 1);

    auto update_enabled = [permits_everything, permission_list]() {
        permission_list->setEnabled(hydrus_avail_permission_list(permits_everything->isChecked()));
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

    rust::Vec<int32_t> basic;

    for (int index = 0; index < permission_list->count(); ++index) {
        QListWidgetItem *item = permission_list->item(index);

        if (item->checkState() == Qt::Checked) {
            basic.push_back(item->data(Qt::UserRole).toInt());
        }
    }

    QStringList rule_slices;
    QStringList rule_kinds;

    auto collect_rules = [](QPlainTextEdit *edit, const QString &rule, QStringList *slices, QStringList *kinds) {
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

            slices->append(slice);
            kinds->append(rule);
        }
    };

    collect_rules(whitelist, QStringLiteral("whitelist"), &rule_slices, &rule_kinds);
    collect_rules(blacklist, QStringLiteral("blacklist"), &rule_slices, &rule_kinds);

    const QByteArray name_bytes = name->text().trimmed().toUtf8();
    const QByteArray key_bytes = access_key->text().trimmed().toLower().toUtf8();
    const QByteArray original_bytes = original.toUtf8();
    app_api_permissions_save(
        *main,
        rust::Str(name_bytes.constData(), name_bytes.size()),
        rust::Str(key_bytes.constData(), key_bytes.size()),
        rust::Str(original_bytes.constData(), original_bytes.size()),
        permits_everything->isChecked(),
        std::move(basic),
        to_rust_vec(rule_slices),
        to_rust_vec(rule_kinds));
}

HydrusApiPermissionsDialog::HydrusApiPermissionsDialog(HydrusMainWindow *main_window, const ApiPermissionsDialogState &state, QWidget *parent)
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

void HydrusApiPermissionsDialog::set_state(const ApiPermissionsDialogState &state) {
    permission_infos.clear();
    available_infos.clear();

    for (const ApiAvailablePermission &entry : state.available) {
        available_infos.push_back({entry.value, to_qstring(entry.label)});
    }

    for (const ApiPermissionRow &entry : state.rows) {
        ApiPermissionInfo info;
        info.name = to_qstring(entry.name);
        info.access_key = to_qstring(entry.access_key);
        info.summary = to_qstring(entry.summary);
        info.permits_everything = entry.permits_everything;

        for (int value : entry.basic_permissions) {
            info.basic_permissions.push_back(value);
        }

        for (const ApiPermissionRule &rule : entry.rules) {
            info.rules.push_back(
                {to_qstring(rule.slice), to_qstring(rule.display), to_qstring(rule.rule)});
        }

        permission_infos.push_back(std::move(info));
    }

    status->setText(to_qstring(state.status));

    table->setRowCount(static_cast<int>(permission_infos.size()));

    for (size_t index = 0; index < permission_infos.size(); ++index) {
        const ApiPermissionInfo &entry = permission_infos[index];
        const int row = static_cast<int>(index);

        for (int column = 0; column < 3; ++column) {
            if (table->item(row, column) == nullptr) {
                table->setItem(row, column, new QTableWidgetItem());
            }
        }

        auto *name_item = table->item(row, 0);
        name_item->setText(entry.name);
        name_item->setData(Qt::UserRole, row);
        table->item(row, 1)->setText(entry.access_key);
        table->item(row, 2)->setText(entry.summary);
    }

    table->resizeRowsToContents();
}

int HydrusApiPermissionsDialog::selected_index() const {
    const int row = table->currentRow();

    if (row < 0 || table->item(row, 0) == nullptr) {
        return -1;
    }

    const int index = table->item(row, 0)->data(Qt::UserRole).toInt();

    if (index < 0 || index >= static_cast<int>(permission_infos.size())) {
        return -1;
    }

    return index;
}

void HydrusApiPermissionsDialog::add_entry() {
    show_api_permission_editor(main, this, available_infos, nullptr);
}

void HydrusApiPermissionsDialog::edit_selected() {
    const int index = selected_index();

    if (index >= 0) {
        show_api_permission_editor(main, this, available_infos, &permission_infos[static_cast<size_t>(index)]);
    }
}

void HydrusApiPermissionsDialog::delete_selected() {
    const int index = selected_index();

    if (index < 0) {
        return;
    }

    const ApiPermissionInfo &entry = permission_infos[static_cast<size_t>(index)];

    if (QMessageBox::question(
            this,
            QStringLiteral("delete access key"),
            QStringLiteral("Delete '%1'? Anything using that key will lose access.")
                .arg(entry.name))
        != QMessageBox::Yes) {
        return;
    }

    const QByteArray key_bytes = entry.access_key.toUtf8();
    app_api_permissions_delete(
        *main, rust::Str(key_bytes.constData(), key_bytes.size()));
}

void HydrusMainWindow::show_api_permissions(const ApiPermissionsDialogState &typed) {
    if (api_permissions_dialog == nullptr) {
        api_permissions_dialog = new HydrusApiPermissionsDialog(this, typed, this);
        connect(api_permissions_dialog, &QObject::destroyed, this, [this]() { api_permissions_dialog = nullptr; });
        api_permissions_dialog->show();
    } else {
        api_permissions_dialog->set_state(typed);
        api_permissions_dialog->raise();
        api_permissions_dialog->activateWindow();
    }
}

void HydrusMainWindow::update_api_permissions(const ApiPermissionsDialogState &typed) {
    if (api_permissions_dialog != nullptr) {
        api_permissions_dialog->set_state(typed);
    }
}
