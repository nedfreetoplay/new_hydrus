#include "services.hpp"

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

QString HydrusServicesDialog::selected_key() const {
    const int row = table->currentRow();

    if (row < 0) {
        return {};
    }

    return cell_key(table, row);
}

QJsonObject HydrusServicesDialog::selected_remote_service() const {
    const int row = remote_services->currentRow();

    if (row < 0 || remote_services->item(row, 0) == nullptr) {
        return {};
    }

    return remote_services->item(row, 0)->data(Qt::UserRole).toJsonObject();
}

QString HydrusServicesDialog::selected_remote_key() const {
    const QJsonObject service = selected_remote_service();

    return service.value(QStringLiteral("key")).toString();
}

void HydrusServicesDialog::add_service() {
    QJsonObject service;
    service.insert(QStringLiteral("name"), QString());
    service.insert(QStringLiteral("type_int"), 2);
    service.insert(QStringLiteral("dictionary"), QJsonObject());

    build_service_editor(main, this, data, service);
}

void HydrusServicesDialog::edit_selected() {
    const int row = table->currentRow();

    if (row < 0 || table->item(row, 0) == nullptr) {
        return;
    }

    QJsonObject service = table->item(row, 0)->data(Qt::UserRole).toJsonObject();
    service.insert(QStringLiteral("original"), selected_key());
    build_service_editor(main, this, data, service);
}

void HydrusServicesDialog::delete_selected() {
    const QString key = selected_key();

    if (key.isEmpty()) {
        return;
    }

    if (QMessageBox::question(
            this,
            QStringLiteral("delete service"),
            QStringLiteral("Delete this service? Only empty custom services can be removed."))
        != QMessageBox::Yes) {
        return;
    }

    QJsonObject payload;
    payload.insert(QStringLiteral("key"), key);

    main->notify(QStringLiteral("service_delete"), QString(), json_payload(payload));
}

void HydrusServicesDialog::show_specifier() {
    QDialog dialog(this);
    dialog.setWindowTitle(QStringLiteral("service specifier"));
    dialog.resize(460, 360);

    auto *layout = new QVBoxLayout(&dialog);
    auto *type = new QComboBox(&dialog);
    type->addItem(QStringLiteral("file domains"), QStringLiteral("local file domain"));
    type->addItem(QStringLiteral("tag domains"), QStringLiteral("local tag domain"));
    type->addItem(QStringLiteral("notes"), QStringLiteral("notes"));
    type->addItem(QStringLiteral("ratings"), QStringLiteral("ratings"));
    type->addItem(QStringLiteral("tag repositories"), QStringLiteral("tag repository"));
    type->addItem(QStringLiteral("file repositories"), QStringLiteral("file repository"));
    layout->addWidget(type);

    auto *list = new QListWidget(&dialog);
    layout->addWidget(list, 1);

    auto *status = new QLabel(&dialog);
    status->setWordWrap(true);
    layout->addWidget(status);

    auto refresh = [this, list, type, status]() {
        list->clear();
        const QString wanted = type->currentData().toString();

        for (const QJsonValue &entry : data.value(QStringLiteral("services")).toArray()) {
            const QJsonObject service = entry.toObject();
            const QString short_type = service.value(QStringLiteral("type_short")).toString();

            const bool matches = wanted == QStringLiteral("ratings")
                ? short_type.contains(QStringLiteral("rating"))
                : short_type == wanted;

            if (matches) {
                auto *item = new QListWidgetItem(service.value(QStringLiteral("name")).toString(), list);
                item->setData(Qt::UserRole, service.value(QStringLiteral("key")).toString());
            }
        }

        status->setText(list->count() == 0 ? QStringLiteral("no services of that type")
                                           : QStringLiteral("%1 service(s) available").arg(list->count()));
    };

    connect(type, &QComboBox::currentIndexChanged, &dialog, [refresh](int) { refresh(); });
    refresh();

    auto *buttons = new QDialogButtonBox(QDialogButtonBox::Close, &dialog);
    auto *copy = new QPushButton(QStringLiteral("copy key"), &dialog);
    buttons->addButton(copy, QDialogButtonBox::ActionRole);
    layout->addWidget(buttons);
    connect(copy, &QPushButton::clicked, &dialog, [list, status]() {
        QListWidgetItem *item = list->currentItem();

        if (item == nullptr) {
            status->setText(QStringLiteral("select a service first"));
            return;
        }

        QApplication::clipboard()->setText(item->data(Qt::UserRole).toString());
        status->setText(QStringLiteral("service key copied to the clipboard"));
    });
    QObject::connect(buttons, &QDialogButtonBox::rejected, &dialog, &QDialog::reject);

    dialog.exec();
}

void HydrusServicesDialog::edit_login() {
    QJsonObject existing;
    const int row = logins->currentRow();

    if (row >= 0 && logins->item(row, 0) != nullptr) {
        existing = logins->item(row, 0)->data(Qt::UserRole).toJsonObject();
    }

    QDialog dialog(this);
    dialog.setWindowTitle(existing.isEmpty() ? QStringLiteral("add credentials")
                                             : QStringLiteral("edit credentials"));
    auto *layout = new QVBoxLayout(&dialog);
    auto *form = new QFormLayout();

    auto *domain = new QLineEdit(existing.value(QStringLiteral("domain")).toString(), &dialog);
    auto *username = new QLineEdit(existing.value(QStringLiteral("username")).toString(), &dialog);
    auto *password = new QLineEdit(existing.value(QStringLiteral("password")).toString(), &dialog);
    password->setEchoMode(QLineEdit::Password);
    auto *access_key = new QLineEdit(existing.value(QStringLiteral("access_key")).toString(), &dialog);
    auto *expires = new QSpinBox(&dialog);
    expires->setRange(0, 3650);
    expires->setSpecialValueText(QStringLiteral("never"));
    expires->setSuffix(QStringLiteral(" days"));

    form->addRow(QStringLiteral("domain"), domain);
    form->addRow(QStringLiteral("username"), username);
    form->addRow(QStringLiteral("password"), password);
    form->addRow(QStringLiteral("access key"), access_key);
    form->addRow(QStringLiteral("expires"), expires);
    layout->addLayout(form);

    auto *buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, &dialog);
    layout->addWidget(buttons);
    connect(buttons, &QDialogButtonBox::accepted, &dialog, [&]() {
        if (domain->text().trimmed().isEmpty()) {
            QMessageBox::warning(&dialog, QStringLiteral("credentials"),
                                 QStringLiteral("Enter a domain."));
            return;
        }

        dialog.accept();
    });
    QObject::connect(buttons, &QDialogButtonBox::rejected, &dialog, &QDialog::reject);

    if (dialog.exec() != QDialog::Accepted) {
        return;
    }

    QJsonObject payload;
    payload.insert(QStringLiteral("domain"), domain->text().trimmed());
    payload.insert(QStringLiteral("username"), username->text());
    payload.insert(QStringLiteral("password"), password->text());
    payload.insert(QStringLiteral("access_key"), access_key->text().trimmed());

    if (expires->value() > 0) {
        payload.insert(QStringLiteral("expires"),
                       QDateTime::currentSecsSinceEpoch() + static_cast<qint64>(expires->value()) * 86400);
    }

    main->notify(QStringLiteral("login_save"), QString(), json_payload(payload));
}

void HydrusServicesDialog::remove_login() {
    const int row = logins->currentRow();

    if (row < 0) {
        return;
    }

    const QString domain = cell_value(logins, row, 0);

    if (domain.isEmpty()) {
        return;
    }

    main->notify(QStringLiteral("login_remove"), QString(), domain);
}

void HydrusServicesDialog::connect_admin() {
    QJsonObject payload;
    payload.insert(QStringLiteral("address"), admin_address->text().trimmed());
    payload.insert(QStringLiteral("access_key"), admin_key->text().trimmed());

    main->notify(QStringLiteral("admin_connect"), QString(), json_payload(payload));
}

void HydrusServicesDialog::select_remote_service() {
    const QString key = selected_remote_key();

    if (key.isEmpty()) {
        return;
    }

    QJsonObject payload;
    payload.insert(QStringLiteral("service_key"), key);
    payload.insert(QStringLiteral("access_key"), remote_key->text().trimmed());

    main->notify(QStringLiteral("admin_service_action"), QString(),
                 json_payload([&]() {
                     QJsonObject with_action = payload;
                     with_action.insert(QStringLiteral("action"), QStringLiteral("select"));
                     return with_action;
                 }()));
}

void HydrusServicesDialog::add_remote_service() {
    QDialog dialog(this);
    dialog.setWindowTitle(QStringLiteral("add server service"));
    auto *layout = new QVBoxLayout(&dialog);
    auto *form = new QFormLayout();

    auto *name = new QLineEdit(&dialog);
    auto *type = new QComboBox(&dialog);
    type->addItem(QStringLiteral("tag repository"), 0);
    type->addItem(QStringLiteral("file repository"), 1);
    auto *port = new QSpinBox(&dialog);
    port->setRange(1, 65535);
    port->setValue(45871);

    form->addRow(QStringLiteral("name"), name);
    form->addRow(QStringLiteral("type"), type);
    form->addRow(QStringLiteral("port"), port);
    layout->addLayout(form);

    auto *note = new QLabel(
        QStringLiteral("The server returns a new access key for every service it creates; "
                       "it is stored here and used for account and option management."),
        &dialog);
    note->setWordWrap(true);
    layout->addWidget(note);

    auto *buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, &dialog);
    layout->addWidget(buttons);
    connect(buttons, &QDialogButtonBox::accepted, &dialog, [&]() {
        if (name->text().trimmed().isEmpty()) {
            QMessageBox::warning(&dialog, QStringLiteral("service"),
                                 QStringLiteral("Enter a service name."));
            return;
        }

        dialog.accept();
    });
    QObject::connect(buttons, &QDialogButtonBox::rejected, &dialog, &QDialog::reject);

    if (dialog.exec() != QDialog::Accepted) {
        return;
    }

    QJsonObject payload;
    payload.insert(QStringLiteral("name"), name->text().trimmed());
    payload.insert(QStringLiteral("type_int"), type->currentData().toInt());
    payload.insert(QStringLiteral("port"), port->value());

    main->notify(QStringLiteral("admin_add_service"), QString(), json_payload(payload));
}

void HydrusServicesDialog::edit_remote_service() {
    const QJsonObject service = selected_remote_service();

    if (service.isEmpty()) {
        return;
    }

    QDialog dialog(this);
    dialog.setWindowTitle(QStringLiteral("edit server service"));
    auto *layout = new QVBoxLayout(&dialog);
    auto *form = new QFormLayout();
    auto *name = new QLineEdit(service.value(QStringLiteral("name")).toString(), &dialog);
    auto *port = new QSpinBox(&dialog);
    port->setRange(1, 65535);
    port->setValue(service.value(QStringLiteral("port")).toInt());
    form->addRow(QStringLiteral("name"), name);
    form->addRow(QStringLiteral("port"), port);
    layout->addLayout(form);

    auto *buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, &dialog);
    layout->addWidget(buttons);
    connect(buttons, &QDialogButtonBox::accepted, &dialog, [&]() {
        if (name->text().trimmed().isEmpty()) {
            QMessageBox::warning(&dialog, QStringLiteral("service"),
                                 QStringLiteral("Enter a service name."));
            return;
        }

        dialog.accept();
    });
    connect(buttons, &QDialogButtonBox::rejected, &dialog, &QDialog::reject);

    if (dialog.exec() != QDialog::Accepted) {
        return;
    }

    QJsonObject payload;
    payload.insert(QStringLiteral("service_key"), service.value(QStringLiteral("key")).toString());
    payload.insert(QStringLiteral("name"), name->text().trimmed());
    payload.insert(QStringLiteral("port"), port->value());

    main->notify(QStringLiteral("admin_edit_service"), QString(), json_payload(payload));
}

void HydrusServicesDialog::remove_remote_service() {
    const QString key = selected_remote_key();

    if (key.isEmpty()) {
        return;
    }

    if (QMessageBox::question(this, QStringLiteral("delete service"),
                              QStringLiteral("Delete this service from the server? All of its "
                                             "content will be removed."))
        != QMessageBox::Yes) {
        return;
    }

    main->notify(QStringLiteral("admin_remove_service"), QString(), key);
}

void HydrusServicesDialog::remote_service_action() {
    const QString key = selected_remote_key();

    if (key.isEmpty()) {
        return;
    }

    auto *button = qobject_cast<QPushButton *>(sender());

    if (button == nullptr) {
        return;
    }

    QString action;
    const QString text = button->text();

    if (text.startsWith(QStringLiteral("lock"))) {
        action = QStringLiteral("lock_on");
    } else if (text.startsWith(QStringLiteral("unlock"))) {
        action = QStringLiteral("lock_off");
    } else if (text.startsWith(QStringLiteral("regen"))) {
        action = QStringLiteral("regen_info");
    } else if (text.startsWith(QStringLiteral("update"))) {
        action = QStringLiteral("update_period");
    } else if (text.startsWith(QStringLiteral("nullification"))) {
        action = QStringLiteral("nullification_period");
    }

    if (action.isEmpty()) {
        return;
    }

    QJsonObject payload;
    payload.insert(QStringLiteral("action"), action);
    payload.insert(QStringLiteral("service_key"), key);
    payload.insert(QStringLiteral("access_key"), remote_key->text().trimmed());

    if (action.endsWith(QStringLiteral("period"))) {
        bool ok = false;
        const int seconds = QInputDialog::getInt(
            this, QStringLiteral("service period"), QStringLiteral("seconds"),
            action.startsWith(QStringLiteral("update")) ? 86400 : 90 * 86400, 0,
            365 * 86400, 60, &ok);

        if (!ok) {
            return;
        }

        payload.insert(QStringLiteral("period"), seconds);
    }

    main->notify(QStringLiteral("admin_service_action"), QString(), json_payload(payload));
}

void HydrusServicesDialog::account_action() {
    const int row = accounts->currentRow();

    if (row < 0 || accounts->item(row, 0) == nullptr) {
        return;
    }

    auto *button = qobject_cast<QPushButton *>(sender());

    if (button == nullptr) {
        return;
    }

    const QJsonObject account = accounts->item(row, 0)->data(Qt::UserRole).toJsonObject();
    const QString account_key = account.value(QStringLiteral("key")).toString();
    const QString text = button->text();
    QString action;

    if (text.startsWith(QStringLiteral("ban"))) {
        action = QStringLiteral("ban");
    } else if (text.startsWith(QStringLiteral("unban"))) {
        action = QStringLiteral("unban");
    } else if (text.startsWith(QStringLiteral("expires"))) {
        action = QStringLiteral("expires");
    } else if (text.startsWith(QStringLiteral("message"))) {
        action = QStringLiteral("message");
    } else if (text.startsWith(QStringLiteral("delete"))) {
        if (QMessageBox::question(this, QStringLiteral("delete content"),
                                  QStringLiteral("Delete all content this account uploaded?"))
            != QMessageBox::Yes) {
            return;
        }

        action = QStringLiteral("delete_all_content");
    } else if (text.startsWith(QStringLiteral("account type"))) {
        action = QStringLiteral("account_type");
    }

    if (action.isEmpty()) {
        return;
    }

    QJsonObject payload;
    payload.insert(QStringLiteral("action"), action);
    payload.insert(QStringLiteral("subject_account_key"), account_key);

    if (action == QStringLiteral("ban")) {
        bool ok = false;
        const QString reason = QInputDialog::getText(
            this, QStringLiteral("ban account"), QStringLiteral("reason"),
            QLineEdit::Normal, QStringLiteral("banned by admin"), &ok);

        if (!ok) {
            return;
        }

        payload.insert(QStringLiteral("reason"), reason);
    } else if (action == QStringLiteral("expires")) {
        bool ok = false;
        const int days = QInputDialog::getInt(
            this, QStringLiteral("account expiry"),
            QStringLiteral("days from now (0 = never)"), 30, 0, 36500, 1, &ok);

        if (!ok) {
            return;
        }

        if (days > 0) {
            payload.insert(QStringLiteral("expires"),
                           QDateTime::currentSecsSinceEpoch() + static_cast<qint64>(days) * 86400);
        } else {
            payload.insert(QStringLiteral("expires"), QJsonValue::Null);
        }
    } else if (action == QStringLiteral("message")) {
        bool ok = false;
        const QString message = QInputDialog::getText(
            this, QStringLiteral("account message"), QStringLiteral("message"),
            QLineEdit::Normal, account.value(QStringLiteral("message")).toString(), &ok);

        if (!ok) {
            return;
        }

        payload.insert(QStringLiteral("message"), message);
    } else if (action == QStringLiteral("account_type")) {
        const QJsonArray types = data.value(QStringLiteral("admin"))
                                     .toObject()
                                     .value(QStringLiteral("account_types"))
                                     .toArray();
        QStringList titles;

        for (const QJsonValue &entry : types) {
            titles.append(entry.toObject().value(QStringLiteral("title")).toString());
        }

        if (titles.isEmpty()) {
            QMessageBox::information(this, QStringLiteral("account type"),
                                     QStringLiteral("Review the service first."));
            return;
        }

        bool ok = false;
        const QString title = QInputDialog::getItem(
            this, QStringLiteral("account type"), QStringLiteral("type"), titles,
            titles.indexOf(account.value(QStringLiteral("type")).toString()), false, &ok);

        if (!ok) {
            return;
        }

        for (const QJsonValue &entry : types) {
            const QJsonObject object = entry.toObject();

            if (object.value(QStringLiteral("title")).toString() == title) {
                payload.insert(QStringLiteral("account_type_key"),
                               object.value(QStringLiteral("key")).toString());
                break;
            }
        }
    }

    main->notify(QStringLiteral("admin_account_action"), QString(), json_payload(payload));
}
