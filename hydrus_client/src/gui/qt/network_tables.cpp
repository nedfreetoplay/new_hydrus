
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

namespace {

void send_url_class(HydrusMainWindow &main, const QJsonObject &payload, bool is_add) {
    const QByteArray original = payload.value(QStringLiteral("original")).toString().toUtf8();
    const QByteArray name = payload.value(QStringLiteral("name")).toString().toUtf8();
    const QByteArray scheme = payload.value(QStringLiteral("preferred_scheme")).toString().toUtf8();
    const QByteArray netloc = payload.value(QStringLiteral("netloc")).toString().toUtf8();
    const QByteArray path = payload.value(QStringLiteral("path")).toString().toUtf8();
    const QByteArray parameters = payload.value(QStringLiteral("parameters")).toString().toUtf8();
    const QByteArray example = payload.value(QStringLiteral("example_url")).toString().toUtf8();
    app_network_url_class_save(
        main,
        rust::Str(original.constData(), original.size()),
        rust::Str(name.constData(), name.size()),
        payload.value(QStringLiteral("url_type")).toInt(),
        rust::Str(scheme.constData(), scheme.size()),
        rust::Str(netloc.constData(), netloc.size()),
        rust::Str(path.constData(), path.size()),
        rust::Str(parameters.constData(), parameters.size()),
        rust::Str(example.constData(), example.size()),
        payload.value(QStringLiteral("match_subdomains")).toBool(),
        payload.value(QStringLiteral("keep_matched_subdomains")).toBool(),
        payload.value(QStringLiteral("can_produce_multiple_files")).toBool(),
        payload.value(QStringLiteral("should_be_associated_with_files")).toBool(),
        is_add);
}

}

void HydrusNetworkDialog::add_header() {
    QDialog dialog(this);    dialog.setWindowTitle(QStringLiteral("add custom header"));
    auto *layout = new QVBoxLayout(&dialog);
    auto *form = new QFormLayout();
    auto *name = new QLineEdit(&dialog);
    auto *value = new QLineEdit(&dialog);
    form->addRow(QStringLiteral("name"), name);
    form->addRow(QStringLiteral("value"), value);
    layout->addLayout(form);
    auto *buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, &dialog);
    connect(buttons, &QDialogButtonBox::accepted, &dialog, &QDialog::accept);
    QObject::connect(buttons, &QDialogButtonBox::rejected, &dialog, &QDialog::reject);
    layout->addWidget(buttons);

    if (dialog.exec() != QDialog::Accepted || name->text().trimmed().isEmpty()) {
        return;
    }

    const QByteArray header_name = name->text().trimmed().toUtf8();
    const QByteArray header_value = value->text().toUtf8();
    app_network_header(
        *main,
        rust::Str(header_name.constData(), header_name.size()),
        rust::Str(header_value.constData(), header_value.size()),
        false);
}

void HydrusNetworkDialog::remove_header() {
    const int row = headers->currentRow();

    if (row < 0 || headers->item(row, 0) == nullptr) {
        return;
    }

    const QByteArray remove_name = headers->item(row, 0)->text().toUtf8();
    app_network_header(
        *main,
        rust::Str(remove_name.constData(), remove_name.size()),
        rust::Str("", 0),
        true);
}

void HydrusNetworkDialog::add_url_class() {
    NetworkUrlClassInfo empty;
    HydrusUrlClassDialog dialog(empty, url_type_infos, this);

    if (dialog.exec() != QDialog::Accepted) {
        return;
    }

    const QJsonObject payload = dialog.payload();

    if (payload.value(QStringLiteral("netloc")).toString().isEmpty()) {
        return;
    }

    send_url_class(*main, payload, true);
}

void HydrusNetworkDialog::edit_url_class() {
    const int row = url_classes->currentRow();

    if (row < 0 || row >= static_cast<int>(url_class_infos.size())) {
        return;
    }

    HydrusUrlClassDialog dialog(url_class_infos[static_cast<size_t>(row)], url_type_infos, this);

    if (dialog.exec() != QDialog::Accepted) {
        return;
    }

    send_url_class(*main, dialog.payload(), false);
}

void HydrusNetworkDialog::remove_url_class() {
    const int row = url_classes->currentRow();

    if (row < 0 || url_classes->item(row, 0) == nullptr) {
        return;
    }

    const QByteArray class_name = url_classes->item(row, 0)->text().toUtf8();
    app_network_url_class_remove(
        *main, rust::Str(class_name.constData(), class_name.size()));
}

void HydrusNetworkDialog::add_pool() {
    QDialog dialog(this);
    dialog.setWindowTitle(QStringLiteral("domain download pool"));
    auto *layout = new QVBoxLayout(&dialog);
    auto *form = new QFormLayout();
    auto *domain = new QLineEdit(&dialog);
    domain->setPlaceholderText(QStringLiteral("example.com"));
    auto *limit = new QSpinBox(&dialog);
    limit->setRange(1, 64);
    limit->setValue(2);
    form->addRow(QStringLiteral("domain"), domain);
    form->addRow(QStringLiteral("simultaneous downloads"), limit);
    layout->addLayout(form);
    auto *buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, &dialog);
    connect(buttons, &QDialogButtonBox::accepted, &dialog, &QDialog::accept);
    QObject::connect(buttons, &QDialogButtonBox::rejected, &dialog, &QDialog::reject);
    layout->addWidget(buttons);

    if (dialog.exec() != QDialog::Accepted || domain->text().trimmed().isEmpty()) {
        return;
    }

    const QByteArray pool_domain = domain->text().trimmed().toUtf8();
    app_network_pool_set(
        *main, rust::Str(pool_domain.constData(), pool_domain.size()), limit->value());
}

void HydrusNetworkDialog::remove_pool() {
    const int row = pools->currentRow();

    if (row < 0 || pools->item(row, 0) == nullptr) {
        return;
    }

    const QByteArray pool_name = pools->item(row, 0)->text().toUtf8();
    app_network_pool_remove(*main, rust::Str(pool_name.constData(), pool_name.size()));
}

void HydrusNetworkDialog::add_bandwidth_rule() {
    QDialog dialog(this);
    dialog.setWindowTitle(QStringLiteral("bandwidth rule"));
    auto *layout = new QVBoxLayout(&dialog);
    auto *form = new QFormLayout();
    auto *context_type = new QComboBox(&dialog);
    context_type->addItem(QStringLiteral("global"), QStringLiteral("global"));
    context_type->addItem(QStringLiteral("domain"), QStringLiteral("domain"));
    context_type->addItem(QStringLiteral("hydrus"), QStringLiteral("hydrus"));
    auto *context = new QLineEdit(&dialog);
    context->setPlaceholderText(QStringLiteral("domain name (domain rules only)"));
    auto *bandwidth_type = new QComboBox(&dialog);
    bandwidth_type->addItem(QStringLiteral("requests"), QStringLiteral("requests"));
    bandwidth_type->addItem(QStringLiteral("data"), QStringLiteral("data"));
    auto *period = new QSpinBox(&dialog);
    period->setRange(0, 30 * 86400);
    period->setValue(1);
    period->setSpecialValueText(QStringLiteral("calendar month"));
    auto *maximum = new QSpinBox(&dialog);
    maximum->setRange(0, 2'000'000'000);
    maximum->setValue(1);
    auto *hint = new QLabel(QStringLiteral("Data maximums are in MB; request maximums are counts."), &dialog);
    hint->setWordWrap(true);
    form->addRow(QStringLiteral("context"), context_type);
    form->addRow(QStringLiteral("domain"), context);
    form->addRow(QStringLiteral("type"), bandwidth_type);
    form->addRow(QStringLiteral("period (seconds)"), period);
    form->addRow(QStringLiteral("maximum"), maximum);
    layout->addLayout(form);
    layout->addWidget(hint);
    auto *buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, &dialog);
    connect(buttons, &QDialogButtonBox::accepted, &dialog, &QDialog::accept);
    QObject::connect(buttons, &QDialogButtonBox::rejected, &dialog, &QDialog::reject);
    layout->addWidget(buttons);

    connect(bandwidth_type, QOverload<int>::of(&QComboBox::currentIndexChanged), &dialog, [maximum, bandwidth_type](int) {
        if (bandwidth_type->currentData().toString() == QStringLiteral("data")) {
            maximum->setRange(0, 2'000'000);
            maximum->setValue(1024);
        } else {
            maximum->setRange(0, 2'000'000'000);
            maximum->setValue(1);
        }
    });

    if (dialog.exec() != QDialog::Accepted) {
        return;
    }

    const QByteArray rule_context_type = context_type->currentData().toString().toUtf8();
    const QByteArray rule_context = context->text().trimmed().toUtf8();
    const QByteArray rule_bandwidth = bandwidth_type->currentData().toString().toUtf8();
    const int64_t time_delta = period->value() <= 0 ? -1 : static_cast<int64_t>(period->value());
    const int64_t max_allowed = bandwidth_type->currentData().toString() == QStringLiteral("data")
        ? static_cast<int64_t>(maximum->value()) * 1024 * 1024
        : static_cast<int64_t>(maximum->value());
    app_network_rule_set(
        *main,
        rust::Str(rule_context_type.constData(), rule_context_type.size()),
        rust::Str(rule_context.constData(), rule_context.size()),
        rust::Str(rule_bandwidth.constData(), rule_bandwidth.size()),
        time_delta,
        max_allowed);
}

void HydrusNetworkDialog::remove_bandwidth_rule() {
    const int row = bandwidth_rules->currentRow();

    if (row < 0 || bandwidth_rules->item(row, 0) == nullptr) {
        return;
    }

    const QByteArray rm_context_type = bandwidth_rules->item(row, 0)->text().toUtf8();
    const QByteArray rm_context = bandwidth_rules->item(row, 1)->text().toUtf8();
    const QByteArray rm_bandwidth = bandwidth_rules->item(row, 2)->text().toUtf8();
    app_network_rule_remove(
        *main,
        rust::Str(rm_context_type.constData(), rm_context_type.size()),
        rust::Str(rm_context.constData(), rm_context.size()),
        rust::Str(rm_bandwidth.constData(), rm_bandwidth.size()));
}
