
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

HydrusNetworkDialog::HydrusNetworkDialog(HydrusMainWindow *main_window, const NetworkDialogState &state, QWidget *parent)
    : QDialog(parent), main(main_window) {
    setWindowTitle(QStringLiteral("manage network"));
    setObjectName(QStringLiteral("hydrus_network_dialog"));
    setAttribute(Qt::WA_DeleteOnClose, true);
    resize(780, 620);

    auto *layout = new QVBoxLayout(this);
    auto *tabs = new QTabWidget(this);
    layout->addWidget(tabs, 1);

    auto *connection = new QWidget(tabs);
    auto *connection_layout = new QVBoxLayout(connection);
    auto *connection_form = new QFormLayout();

    http_proxy = new QLineEdit(connection);
    https_proxy = new QLineEdit(connection);
    no_proxy = new QLineEdit(connection);
    timeout = new QSpinBox(connection);
    timeout->setRange(3, 600);
    max_jobs = new QSpinBox(connection);
    max_jobs->setRange(1, 500);
    max_jobs_domain = new QSpinBox(connection);
    max_jobs_domain->setRange(1, 100);

    connection_form->addRow(QStringLiteral("http proxy"), http_proxy);
    connection_form->addRow(QStringLiteral("https proxy"), https_proxy);
    connection_form->addRow(QStringLiteral("no proxy"), no_proxy);
    connection_form->addRow(QStringLiteral("timeout (seconds)"), timeout);
    connection_form->addRow(QStringLiteral("max network jobs"), max_jobs);
    connection_form->addRow(QStringLiteral("max jobs per domain"), max_jobs_domain);
    connection_layout->addLayout(connection_form);

    paused = new QCheckBox(QStringLiteral("pause all new network traffic"), connection);
    connection_layout->addWidget(paused);
    connection_layout->addStretch(1);
    tabs->addTab(connection, QStringLiteral("connection"));

    auto *api = new QWidget(tabs);
    auto *api_layout = new QVBoxLayout(api);
    auto *api_form = new QFormLayout();
    api_enabled = new QCheckBox(QStringLiteral("run the local Client API"), api);
    api_port = new QSpinBox(api);
    api_port->setRange(1, 65535);
    api_key = new QLineEdit(api);
    api_key->setPlaceholderText(QStringLiteral("64 hex characters; generated when empty"));
    api_status = new QLabel(api);
    api_status->setTextInteractionFlags(Qt::TextSelectableByMouse);

    api_form->addRow(api_enabled);
    api_form->addRow(QStringLiteral("port"), api_port);
    api_form->addRow(QStringLiteral("access key"), api_key);
    api_form->addRow(QStringLiteral("status"), api_status);
    api_layout->addLayout(api_form);
    api_layout->addStretch(1);
    tabs->addTab(api, QStringLiteral("client api"));

    auto *headers_page = new QWidget(tabs);
    auto *headers_layout = new QVBoxLayout(headers_page);
    headers = new QTableWidget(headers_page);
    headers->setColumnCount(2);
    headers->setHorizontalHeaderLabels({QStringLiteral("name"), QStringLiteral("value")});
    headers->horizontalHeader()->setStretchLastSection(true);
    headers->horizontalHeader()->setSectionResizeMode(0, QHeaderView::ResizeToContents);
    headers->setSelectionBehavior(QAbstractItemView::SelectRows);
    headers->setEditTriggers(QAbstractItemView::NoEditTriggers);
    headers->verticalHeader()->setVisible(false);
    headers_layout->addWidget(headers, 1);

    auto *header_buttons = new QHBoxLayout();
    auto *add_header_button = new QPushButton(QStringLiteral("add header..."), headers_page);
    auto *remove_header_button = new QPushButton(QStringLiteral("remove"), headers_page);
    header_buttons->addWidget(add_header_button);
    header_buttons->addWidget(remove_header_button);
    header_buttons->addStretch(1);
    headers_layout->addLayout(header_buttons);
    tabs->addTab(headers_page, QStringLiteral("custom headers"));

    auto *classes_page = new QWidget(tabs);
    auto *classes_layout = new QVBoxLayout(classes_page);
    url_classes = new QTableWidget(classes_page);
    url_classes->setColumnCount(4);
    url_classes->setHorizontalHeaderLabels({QStringLiteral("name"), QStringLiteral("type"), QStringLiteral("domain"), QStringLiteral("path")});
    url_classes->horizontalHeader()->setStretchLastSection(true);
    url_classes->horizontalHeader()->setSectionResizeMode(0, QHeaderView::ResizeToContents);
    url_classes->horizontalHeader()->setSectionResizeMode(1, QHeaderView::ResizeToContents);
    url_classes->setSelectionBehavior(QAbstractItemView::SelectRows);
    url_classes->setEditTriggers(QAbstractItemView::NoEditTriggers);
    url_classes->verticalHeader()->setVisible(false);
    classes_layout->addWidget(url_classes, 1);

    auto *class_buttons = new QHBoxLayout();
    auto *add_class = new QPushButton(QStringLiteral("add url class..."), classes_page);
    auto *edit_class = new QPushButton(QStringLiteral("edit..."), classes_page);
    auto *remove_class = new QPushButton(QStringLiteral("remove"), classes_page);
    class_buttons->addWidget(add_class);
    class_buttons->addWidget(edit_class);
    class_buttons->addWidget(remove_class);
    class_buttons->addStretch(1);
    classes_layout->addLayout(class_buttons);
    tabs->addTab(classes_page, QStringLiteral("url classes"));

    auto *pools_page = new QWidget(tabs);
    auto *pools_layout = new QVBoxLayout(pools_page);
    pools = new QTableWidget(pools_page);
    pools->setColumnCount(2);
    pools->setHorizontalHeaderLabels({QStringLiteral("domain"), QStringLiteral("simultaneous downloads")});
    pools->horizontalHeader()->setStretchLastSection(true);
    pools->setSelectionBehavior(QAbstractItemView::SelectRows);
    pools->setEditTriggers(QAbstractItemView::NoEditTriggers);
    pools->verticalHeader()->setVisible(false);
    pools_layout->addWidget(pools, 1);

    auto *pool_buttons = new QHBoxLayout();
    auto *add_pool_button = new QPushButton(QStringLiteral("add domain limit..."), pools_page);
    auto *remove_pool_button = new QPushButton(QStringLiteral("remove"), pools_page);
    pool_buttons->addWidget(add_pool_button);
    pool_buttons->addWidget(remove_pool_button);
    pool_buttons->addStretch(1);
    pools_layout->addLayout(pool_buttons);
    tabs->addTab(pools_page, QStringLiteral("download pools"));

    auto *bandwidth_page = new QWidget(tabs);
    auto *bandwidth_layout = new QVBoxLayout(bandwidth_page);
    bandwidth_rules = new QTableWidget(bandwidth_page);
    bandwidth_rules->setColumnCount(5);
    bandwidth_rules->setHorizontalHeaderLabels({
        QStringLiteral("context"),
        QStringLiteral("domain"),
        QStringLiteral("type"),
        QStringLiteral("period"),
        QStringLiteral("maximum")
    });
    bandwidth_rules->horizontalHeader()->setStretchLastSection(true);
    bandwidth_rules->setSelectionBehavior(QAbstractItemView::SelectRows);
    bandwidth_rules->setEditTriggers(QAbstractItemView::NoEditTriggers);
    bandwidth_rules->verticalHeader()->setVisible(false);
    bandwidth_layout->addWidget(bandwidth_rules, 1);
    bandwidth_layout->addWidget(new QLabel(
        QStringLiteral("Data limits are in bytes. A zero period means the calendar month."),
        bandwidth_page));

    auto *bandwidth_buttons = new QHBoxLayout();
    auto *add_rule_button = new QPushButton(QStringLiteral("add rule..."), bandwidth_page);
    auto *remove_rule_button = new QPushButton(QStringLiteral("remove"), bandwidth_page);
    bandwidth_buttons->addWidget(add_rule_button);
    bandwidth_buttons->addWidget(remove_rule_button);
    bandwidth_buttons->addStretch(1);
    bandwidth_layout->addLayout(bandwidth_buttons);
    tabs->addTab(bandwidth_page, QStringLiteral("bandwidth"));

    auto *buttons = new QDialogButtonBox(QDialogButtonBox::Save | QDialogButtonBox::Close, this);
    connect(buttons, &QDialogButtonBox::accepted, this, [this]() { save(); });
    connect(buttons, &QDialogButtonBox::rejected, this, &QDialog::close);
    layout->addWidget(buttons);

    connect(paused, &QCheckBox::clicked, this, [this](bool) {
        main->notify(UiAction::NetworkPause);
    });
    connect(add_header_button, &QPushButton::clicked, this, [this]() { add_header(); });
    connect(remove_header_button, &QPushButton::clicked, this, [this]() { remove_header(); });
    connect(add_class, &QPushButton::clicked, this, [this]() { add_url_class(); });
    connect(edit_class, &QPushButton::clicked, this, [this]() { edit_url_class(); });
    connect(remove_class, &QPushButton::clicked, this, [this]() { remove_url_class(); });
    connect(add_pool_button, &QPushButton::clicked, this, [this]() { add_pool(); });
    connect(remove_pool_button, &QPushButton::clicked, this, [this]() { remove_pool(); });
    connect(add_rule_button, &QPushButton::clicked, this, [this]() { add_bandwidth_rule(); });
    connect(remove_rule_button, &QPushButton::clicked, this, [this]() { remove_bandwidth_rule(); });

    set_state(state);
}

void HydrusNetworkDialog::set_state(const NetworkDialogState &state) {
    url_class_infos.clear();
    url_type_infos.clear();

    for (const NetworkUrlClassRow &entry : state.url_classes) {
        url_class_infos.push_back({to_qstring(entry.name),
                                   entry.url_type,
                                   to_qstring(entry.preferred_scheme),
                                   to_qstring(entry.netloc),
                                   to_qstring(entry.components_text),
                                   to_qstring(entry.parameters_text),
                                   entry.match_subdomains,
                                   entry.keep_matched_subdomains,
                                   entry.can_produce_multiple_files,
                                   entry.should_be_associated_with_files,
                                   to_qstring(entry.example_url)});
    }

    for (const UrlTypeRow &entry : state.url_types) {
        url_type_infos.push_back({entry.value, to_qstring(entry.label)});
    }

    http_proxy->setText(to_qstring(state.http_proxy));
    https_proxy->setText(to_qstring(state.https_proxy));
    no_proxy->setText(to_qstring(state.no_proxy));
    timeout->setValue(state.network_timeout);
    max_jobs->setValue(state.max_network_jobs);
    max_jobs_domain->setValue(state.max_network_jobs_per_domain);

    api_enabled->setChecked(state.api_enabled);
    api_port->setValue(state.api_port);
    api_key->setText(to_qstring(state.api_access_key));
    api_status->setText(to_qstring(state.api_status));

    QSignalBlocker blocker(paused);
    paused->setChecked(state.paused);

    headers->setRowCount(static_cast<int>(state.headers.size()));

    for (size_t index = 0; index < state.headers.size(); ++index) {
        const NetworkHeaderRow &header = state.headers[index];
        const int row = static_cast<int>(index);

        if (headers->item(row, 0) == nullptr) {
            headers->setItem(row, 0, new QTableWidgetItem());
            headers->setItem(row, 1, new QTableWidgetItem());
        }

        headers->item(row, 0)->setText(to_qstring(header.name));
        headers->item(row, 1)->setText(to_qstring(header.value));
    }

    url_classes->setRowCount(static_cast<int>(state.url_classes.size()));

    for (size_t index = 0; index < state.url_classes.size(); ++index) {
        const NetworkUrlClassRow &entry = state.url_classes[index];
        const int row = static_cast<int>(index);

        for (int column = 0; column < 4; ++column) {
            if (url_classes->item(row, column) == nullptr) {
                url_classes->setItem(row, column, new QTableWidgetItem());
            }
        }

        url_classes->item(row, 0)->setText(to_qstring(entry.name));
        url_classes->item(row, 0)->setData(Qt::UserRole, row);
        url_classes->item(row, 1)->setText(to_qstring(entry.url_type_name));
        url_classes->item(row, 2)->setText(to_qstring(entry.netloc));
        url_classes->item(row, 3)->setText(to_qstring(entry.components_text));
    }

    pools->setRowCount(static_cast<int>(state.download_pools.size()));

    for (size_t index = 0; index < state.download_pools.size(); ++index) {
        const NetworkPoolRow &entry = state.download_pools[index];
        const int row = static_cast<int>(index);

        for (int column = 0; column < 2; ++column) {
            if (pools->item(row, column) == nullptr) {
                pools->setItem(row, column, new QTableWidgetItem());
            }
        }

        pools->item(row, 0)->setText(to_qstring(entry.domain));
        pools->item(row, 0)->setData(Qt::UserRole, row);
        pools->item(row, 1)->setText(to_qstring(entry.limit_text));
    }

    bandwidth_rules->setRowCount(static_cast<int>(state.bandwidth_rules.size()));

    for (size_t index = 0; index < state.bandwidth_rules.size(); ++index) {
        const NetworkBandwidthRow &entry = state.bandwidth_rules[index];
        const int row = static_cast<int>(index);

        for (int column = 0; column < 5; ++column) {
            if (bandwidth_rules->item(row, column) == nullptr) {
                bandwidth_rules->setItem(row, column, new QTableWidgetItem());
            }
        }

        bandwidth_rules->item(row, 0)->setText(to_qstring(entry.context_type));
        bandwidth_rules->item(row, 0)->setData(Qt::UserRole, row);
        bandwidth_rules->item(row, 1)->setText(to_qstring(entry.context));
        bandwidth_rules->item(row, 1)->setData(Qt::UserRole, row);
        bandwidth_rules->item(row, 2)->setText(to_qstring(entry.bandwidth_type));
        bandwidth_rules->item(row, 2)->setData(Qt::UserRole, row);
        bandwidth_rules->item(row, 3)->setText(to_qstring(entry.period_text));
        bandwidth_rules->item(row, 4)->setText(to_qstring(entry.maximum_text));
    }

    headers->resizeRowsToContents();
    url_classes->resizeRowsToContents();
    pools->resizeRowsToContents();
    bandwidth_rules->resizeRowsToContents();
}

void HydrusNetworkDialog::save() {
    const QByteArray api_key_bytes = api_key->text().toUtf8();
    const QByteArray http_bytes = http_proxy->text().toUtf8();
    const QByteArray https_bytes = https_proxy->text().toUtf8();
    const QByteArray no_proxy_bytes = no_proxy->text().toUtf8();
    app_network_save(
        *main,
        api_enabled->isChecked(),
        timeout->value(),
        max_jobs->value(),
        max_jobs_domain->value(),
        api_port->value(),
        rust::Str(api_key_bytes.constData(), api_key_bytes.size()),
        rust::Str(http_bytes.constData(), http_bytes.size()),
        rust::Str(https_bytes.constData(), https_bytes.size()),
        rust::Str(no_proxy_bytes.constData(), no_proxy_bytes.size()));
}
