
// WS-15.1.3a: moved verbatim from bridge.cpp (move-only split).

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

QString service_type_label(const std::vector<ServiceTypeInfo> &types, int value) {
    for (const ServiceTypeInfo &entry : types) {
        if (entry.value == value) {
            return entry.label;
        }
    }

    return {};
}

QString service_family(const std::vector<ServiceTypeInfo> &types, int value) {
    for (const ServiceTypeInfo &entry : types) {
        if (entry.value == value) {
            return entry.family;
        }
    }

    return {};
}

void build_service_editor(HydrusMainWindow *main, QWidget *parent, const std::vector<ServiceTypeInfo> &types, const LocalServiceInfo &service, const QString &original) {
    QDialog dialog(parent);
    dialog.setWindowTitle(original.isEmpty() ? QStringLiteral("add service")
                                             : QStringLiteral("edit service"));
    dialog.resize(620, 520);

    auto *layout = new QVBoxLayout(&dialog);
    auto *form = new QFormLayout();

    auto *name = new QLineEdit(service.name, &dialog);
    auto *type = new QComboBox(&dialog);

    for (const ServiceTypeInfo &entry : types) {
        type->addItem(entry.label, entry.value);
    }

    const int type_index = type->findData(service.type_int);
    type->setCurrentIndex(type_index >= 0 ? type_index : 0);

    auto *host = new QLineEdit(service.host, &dialog);
    host->setPlaceholderText(QStringLiteral("127.0.0.1"));
    auto *port = new QSpinBox(&dialog);
    port->setRange(1, 65535);
    port->setValue(service.port > 0 ? service.port : 45871);
    auto *access_key = new QLineEdit(service.access_key, &dialog);
    access_key->setPlaceholderText(QStringLiteral("hex access key from the server administrator"));

    auto *update_period = new QSpinBox(&dialog);
    update_period->setRange(0, 365 * 86400);
    update_period->setValue(service.has_update_period ? service.update_period : 86400);
    update_period->setSuffix(QStringLiteral(" s"));
    auto *nullification_period = new QSpinBox(&dialog);
    nullification_period->setRange(0, 365 * 86400);
    nullification_period->setValue(service.has_nullify_period ? service.nullify_period : 90 * 86400);
    nullification_period->setSuffix(QStringLiteral(" s"));

    auto *max_storage = new QSpinBox(&dialog);
    max_storage->setRange(0, 1000000000);
    max_storage->setValue(service.max_storage);
    max_storage->setSuffix(QStringLiteral(" MB"));
    auto *log_uploader_ips = new QCheckBox(QStringLiteral("log uploader IPs"), &dialog);
    log_uploader_ips->setChecked(service.log_uploader_ips);

    form->addRow(QStringLiteral("name"), name);
    form->addRow(QStringLiteral("type"), type);
    form->addRow(QStringLiteral("host"), host);
    form->addRow(QStringLiteral("port"), port);
    form->addRow(QStringLiteral("access key"), access_key);
    form->addRow(QStringLiteral("update period"), update_period);
    form->addRow(QStringLiteral("nullification period"), nullification_period);
    form->addRow(QStringLiteral("max storage"), max_storage);
    form->addRow(log_uploader_ips);
    layout->addLayout(form);

    auto *note = new QLabel(
        QStringLiteral("Local services need only a name. Repository and administration "
                       "services also store the host, port and access key the client uses to "
                       "contact them."),
        &dialog);
    note->setWordWrap(true);
    layout->addWidget(note);

    auto sync_fields = [&]() {
        const int value = type->currentData().toInt();
        const QString family = service_family(types, value);
        const bool remote = family == QStringLiteral("remote");
        const bool repository = service_type_label(types, value).contains(QStringLiteral("repository"));

        host->setEnabled(remote);
        port->setEnabled(remote);
        access_key->setEnabled(remote);
        update_period->setEnabled(repository);
        nullification_period->setEnabled(repository);
        max_storage->setEnabled(value == 1);
        log_uploader_ips->setEnabled(value == 1);
    };

    QObject::connect(type, &QComboBox::currentIndexChanged, &dialog, [&](int) { sync_fields(); });
    sync_fields();

    auto *buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, &dialog);
    layout->addWidget(buttons);
    QObject::connect(buttons, &QDialogButtonBox::rejected, &dialog, &QDialog::reject);
    QObject::connect(buttons, &QDialogButtonBox::accepted, &dialog, [&]() {
        if (name->text().trimmed().isEmpty()) {
            QMessageBox::warning(&dialog, QStringLiteral("service"), QStringLiteral("Enter a name."));
            return;
        }

        const QString family = service_family(types, type->currentData().toInt());

        if (family == QStringLiteral("remote") && access_key->text().trimmed().isEmpty()) {
            QMessageBox::warning(&dialog, QStringLiteral("service"),
                                 QStringLiteral("Remote services need an access key."));
            return;
        }

        if (family == QStringLiteral("remote") && host->text().trimmed().isEmpty()) {
            QMessageBox::warning(&dialog, QStringLiteral("service"),
                                 QStringLiteral("Remote services need a host."));
            return;
        }

        dialog.accept();
    });

    if (dialog.exec() != QDialog::Accepted) {
        return;
    }

    QJsonObject payload;
    payload.insert(QStringLiteral("original"), original);
    payload.insert(QStringLiteral("name"), name->text().trimmed());
    payload.insert(QStringLiteral("type_int"), type->currentData().toInt());

    QJsonObject next_dictionary;
    const QString family = service_family(types, type->currentData().toInt());
    const bool repository = type->currentText().contains(QStringLiteral("repository"));

    if (family == QStringLiteral("remote")) {
        next_dictionary.insert(QStringLiteral("host"), host->text().trimmed());
        next_dictionary.insert(QStringLiteral("port"), port->value());
        next_dictionary.insert(QStringLiteral("access_key"),
                               access_key->text().trimmed().toLower());
    }

    if (repository) {
        next_dictionary.insert(QStringLiteral("update_period"), update_period->value());
        next_dictionary.insert(QStringLiteral("nullification_period"), nullification_period->value());
    }

    if (type->currentData().toInt() == 1) {
        next_dictionary.insert(QStringLiteral("max_storage"), max_storage->value());
        next_dictionary.insert(QStringLiteral("log_uploader_ips"), log_uploader_ips->isChecked());
    }

    payload.insert(QStringLiteral("dictionary"), next_dictionary);
    main->notify(UiAction::ServiceSave, QString(), json_payload(payload));
}

QString cell_value(QTableWidget *table, int row, int column) {
    QTableWidgetItem *item = table->item(row, column);

    return item == nullptr ? QString() : item->text();
}

QString cell_key(QTableWidget *table, int row) {
    QTableWidgetItem *item = table->item(row, 0);

    return item == nullptr ? QString() : item->data(Qt::UserRole + 1).toString();
}
