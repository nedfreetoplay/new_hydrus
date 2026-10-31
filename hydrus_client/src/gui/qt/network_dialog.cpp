#include "network.hpp"

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

HydrusNetworkDialog::HydrusNetworkDialog(HydrusMainWindow *main_window, const QJsonObject &state, QWidget *parent)
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
        main->notify(QStringLiteral("network_pause"));
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

void HydrusNetworkDialog::set_state(const QJsonObject &state) {
    data = state;
    const QJsonObject options = state.value(QStringLiteral("options")).toObject();

    http_proxy->setText(options.value(QStringLiteral("http_proxy")).toString());
    https_proxy->setText(options.value(QStringLiteral("https_proxy")).toString());
    no_proxy->setText(options.value(QStringLiteral("no_proxy")).toString());
    timeout->setValue(options.value(QStringLiteral("network_timeout")).toInt(10));
    max_jobs->setValue(options.value(QStringLiteral("max_network_jobs")).toInt(15));
    max_jobs_domain->setValue(options.value(QStringLiteral("max_network_jobs_per_domain")).toInt(3));

    api_enabled->setChecked(options.value(QStringLiteral("api_enabled")).toBool());
    api_port->setValue(options.value(QStringLiteral("api_port")).toInt(45869));
    api_key->setText(options.value(QStringLiteral("api_access_key")).toString());
    api_status->setText(state.value(QStringLiteral("api_status")).toString());

    QSignalBlocker blocker(paused);
    paused->setChecked(state.value(QStringLiteral("paused")).toBool());

    const QJsonArray header_values = state.value(QStringLiteral("headers")).toArray();
    headers->setRowCount(header_values.size());

    for (int index = 0; index < header_values.size(); ++index) {
        const QJsonObject header = header_values.at(index).toObject();

        if (headers->item(index, 0) == nullptr) {
            headers->setItem(index, 0, new QTableWidgetItem());
            headers->setItem(index, 1, new QTableWidgetItem());
        }

        headers->item(index, 0)->setText(header.value(QStringLiteral("name")).toString());
        headers->item(index, 1)->setText(header.value(QStringLiteral("value")).toString());
    }

    const QJsonArray classes = state.value(QStringLiteral("url_classes")).toArray();
    url_classes->setRowCount(classes.size());

    for (int index = 0; index < classes.size(); ++index) {
        const QJsonObject entry = classes.at(index).toObject();

        for (int column = 0; column < 4; ++column) {
            if (url_classes->item(index, column) == nullptr) {
                url_classes->setItem(index, column, new QTableWidgetItem());
            }
        }

        url_classes->item(index, 0)->setText(entry.value(QStringLiteral("name")).toString());
        url_classes->item(index, 0)->setData(Qt::UserRole, entry.value(QStringLiteral("name")).toString());
        url_classes->item(index, 1)->setText(entry.value(QStringLiteral("url_type_name")).toString());
        url_classes->item(index, 2)->setText(entry.value(QStringLiteral("netloc")).toString());
        url_classes->item(index, 3)->setText(json_to_strings(entry.value(QStringLiteral("components")).toArray()).join(QStringLiteral("/")));
    }

    const QJsonArray pool_values = state.value(QStringLiteral("download_pools")).toArray();
    pools->setRowCount(pool_values.size());

    for (int index = 0; index < pool_values.size(); ++index) {
        const QJsonObject entry = pool_values.at(index).toObject();

        for (int column = 0; column < 2; ++column) {
            if (pools->item(index, column) == nullptr) {
                pools->setItem(index, column, new QTableWidgetItem());
            }
        }

        pools->item(index, 0)->setText(entry.value(QStringLiteral("domain")).toString());
        pools->item(index, 0)->setData(Qt::UserRole, entry.value(QStringLiteral("domain")).toString());
        pools->item(index, 1)->setText(QString::number(entry.value(QStringLiteral("limit")).toInt()));
    }

    const QJsonArray rules = state.value(QStringLiteral("bandwidth_rules")).toArray();
    bandwidth_rules->setRowCount(rules.size());

    for (int index = 0; index < rules.size(); ++index) {
        const QJsonObject entry = rules.at(index).toObject();

        for (int column = 0; column < 5; ++column) {
            if (bandwidth_rules->item(index, column) == nullptr) {
                bandwidth_rules->setItem(index, column, new QTableWidgetItem());
            }
        }

        const QString context_type = entry.value(QStringLiteral("context_type")).toString();
        const QString context = entry.value(QStringLiteral("context")).toString();
        const QString bandwidth_type = entry.value(QStringLiteral("bandwidth_type")).toString();
        const QJsonValue period_value = entry.value(QStringLiteral("time_delta"));
        const qint64 max_value = entry.value(QStringLiteral("max_allowed")).toVariant().toLongLong();

        bandwidth_rules->item(index, 0)->setText(context_type);
        bandwidth_rules->item(index, 0)->setData(Qt::UserRole, context_type);
        bandwidth_rules->item(index, 1)->setText(context);
        bandwidth_rules->item(index, 1)->setData(Qt::UserRole, context);
        bandwidth_rules->item(index, 2)->setText(bandwidth_type);
        bandwidth_rules->item(index, 2)->setData(Qt::UserRole, bandwidth_type);
        bandwidth_rules->item(index, 3)->setText(
            period_value.isNull() || period_value.toInt() <= 0
                ? QStringLiteral("month")
                : QStringLiteral("%1 s").arg(period_value.toInt()));
        bandwidth_rules->item(index, 4)->setText(
            bandwidth_type == QStringLiteral("data")
                ? QStringLiteral("%1 MB").arg(max_value / (1024 * 1024))
                : QString::number(max_value));
    }

    headers->resizeRowsToContents();
    url_classes->resizeRowsToContents();
    pools->resizeRowsToContents();
    bandwidth_rules->resizeRowsToContents();
}

void HydrusNetworkDialog::save() {
    QJsonObject booleans;
    booleans.insert(QStringLiteral("api_enabled"), api_enabled->isChecked());

    QJsonObject integers;
    integers.insert(QStringLiteral("network_timeout"), timeout->value());
    integers.insert(QStringLiteral("max_network_jobs"), max_jobs->value());
    integers.insert(QStringLiteral("max_network_jobs_per_domain"), max_jobs_domain->value());
    integers.insert(QStringLiteral("api_port"), api_port->value());

    QJsonObject strings;
    strings.insert(QStringLiteral("api_access_key"), api_key->text());

    QJsonObject noneable_strings;
    noneable_strings.insert(QStringLiteral("http_proxy"), http_proxy->text());
    noneable_strings.insert(QStringLiteral("https_proxy"), https_proxy->text());
    noneable_strings.insert(QStringLiteral("no_proxy"), no_proxy->text());

    QJsonObject payload;
    payload.insert(QStringLiteral("booleans"), booleans);
    payload.insert(QStringLiteral("integers"), integers);
    payload.insert(QStringLiteral("strings"), strings);
    payload.insert(QStringLiteral("noneable_strings"), noneable_strings);

    main->notify(QStringLiteral("network_save"), QString(), json_payload(payload));
}
