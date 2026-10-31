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

HydrusServicesDialog::HydrusServicesDialog(HydrusMainWindow *main_window, const QJsonObject &state, QWidget *parent)
    : QDialog(parent), main(main_window) {
    setWindowTitle(QStringLiteral("manage services"));
    setObjectName(QStringLiteral("hydrus_services_dialog"));
    setAttribute(Qt::WA_DeleteOnClose, true);
    resize(980, 720);

    auto *layout = new QVBoxLayout(this);
    tabs = new QTabWidget(this);
    layout->addWidget(tabs, 1);

    // Local service manager -------------------------------------------------
    auto *local = new QWidget(tabs);
    auto *local_layout = new QVBoxLayout(local);

    table = new QTableWidget(local);
    table->setColumnCount(5);
    table->setHorizontalHeaderLabels({
        QStringLiteral("name"),
        QStringLiteral("type"),
        QStringLiteral("service key"),
        QStringLiteral("port"),
        QStringLiteral("settings")
    });
    table->horizontalHeader()->setSectionResizeMode(0, QHeaderView::Stretch);
    table->horizontalHeader()->setSectionResizeMode(1, QHeaderView::ResizeToContents);
    table->horizontalHeader()->setSectionResizeMode(2, QHeaderView::Stretch);
    table->horizontalHeader()->setSectionResizeMode(3, QHeaderView::ResizeToContents);
    table->horizontalHeader()->setSectionResizeMode(4, QHeaderView::Stretch);
    table->setSelectionBehavior(QAbstractItemView::SelectRows);
    table->setSelectionMode(QAbstractItemView::SingleSelection);
    table->setEditTriggers(QAbstractItemView::NoEditTriggers);
    table->verticalHeader()->setVisible(false);
    local_layout->addWidget(table, 1);

    auto *buttons = new QHBoxLayout();
    auto *add = new QPushButton(QStringLiteral("add service..."), local);
    auto *edit = new QPushButton(QStringLiteral("edit..."), local);
    auto *remove = new QPushButton(QStringLiteral("delete"), local);
    auto *specifier = new QPushButton(QStringLiteral("service specifier..."), local);
    auto *tag_filter = new QPushButton(QStringLiteral("tag filter / purge..."), local);
    buttons->addWidget(add);
    buttons->addWidget(edit);
    buttons->addWidget(remove);
    buttons->addWidget(specifier);
    buttons->addWidget(tag_filter);
    buttons->addStretch(1);
    local_layout->addLayout(buttons);

    connect(add, &QPushButton::clicked, this, [this]() { add_service(); });
    connect(edit, &QPushButton::clicked, this, [this]() { edit_selected(); });
    connect(remove, &QPushButton::clicked, this, [this]() { delete_selected(); });
    connect(specifier, &QPushButton::clicked, this, [this]() { show_specifier(); });
    connect(tag_filter, &QPushButton::clicked, this, [this]() {
        const QString key = selected_key();

        if (!key.isEmpty()) {
            main->notify(QStringLiteral("tag_filter_open"), QString(), key);
        }
    });
    connect(table, &QTableWidget::cellDoubleClicked, this, [this](int, int) { edit_selected(); });

    tabs->addTab(local, QStringLiteral("services"));

    // Remote server ---------------------------------------------------------
    auto *server = new QWidget(tabs);
    auto *server_layout = new QVBoxLayout(server);

    auto *connection = new QHBoxLayout();
    admin_address = new QLineEdit(server);
    admin_address->setPlaceholderText(QStringLiteral("https://127.0.0.1:45870"));
    admin_key = new QLineEdit(server);
    admin_key->setPlaceholderText(QStringLiteral("administration access key"));
    auto *connect_button = new QPushButton(QStringLiteral("connect"), server);
    connection->addWidget(new QLabel(QStringLiteral("address"), server));
    connection->addWidget(admin_address, 2);
    connection->addWidget(new QLabel(QStringLiteral("key"), server));
    connection->addWidget(admin_key, 2);
    connection->addWidget(connect_button);
    server_layout->addLayout(connection);

    admin_status = new QLabel(server);
    admin_status->setWordWrap(true);
    server_layout->addWidget(admin_status);

    server_layout->addWidget(new QLabel(QStringLiteral("server services"), server));
    remote_services = new QTableWidget(server);
    remote_services->setColumnCount(5);
    remote_services->setHorizontalHeaderLabels({
        QStringLiteral("name"),
        QStringLiteral("type"),
        QStringLiteral("port"),
        QStringLiteral("service key"),
        QStringLiteral("access key")
    });
    remote_services->horizontalHeader()->setSectionResizeMode(0, QHeaderView::Stretch);
    remote_services->horizontalHeader()->setSectionResizeMode(1, QHeaderView::ResizeToContents);
    remote_services->horizontalHeader()->setSectionResizeMode(2, QHeaderView::ResizeToContents);
    remote_services->horizontalHeader()->setSectionResizeMode(3, QHeaderView::Stretch);
    remote_services->horizontalHeader()->setSectionResizeMode(4, QHeaderView::Stretch);
    remote_services->setSelectionBehavior(QAbstractItemView::SelectRows);
    remote_services->setSelectionMode(QAbstractItemView::SingleSelection);
    remote_services->setEditTriggers(QAbstractItemView::NoEditTriggers);
    remote_services->verticalHeader()->setVisible(false);
    server_layout->addWidget(remote_services, 1);

    auto *remote_buttons = new QHBoxLayout();
    remote_key = new QLineEdit(server);
    remote_key->setPlaceholderText(QStringLiteral("selected service access key"));
    auto *use = new QPushButton(QStringLiteral("review accounts"), server);
    auto *add_remote = new QPushButton(QStringLiteral("add service..."), server);
    auto *edit_remote = new QPushButton(QStringLiteral("edit service..."), server);
    auto *remove_remote = new QPushButton(QStringLiteral("delete service"), server);
    auto *lock = new QPushButton(QStringLiteral("lock"), server);
    auto *unlock = new QPushButton(QStringLiteral("unlock"), server);
    auto *regen = new QPushButton(QStringLiteral("regen info"), server);
    auto *info = new QPushButton(QStringLiteral("service info..."), server);
    auto *remote_filter = new QPushButton(QStringLiteral("tag filter..."), server);
    auto *account_types = new QPushButton(QStringLiteral("account types..."), server);
    auto *update_period = new QPushButton(QStringLiteral("update period..."), server);
    auto *nullification_period = new QPushButton(QStringLiteral("nullification..."), server);
    remote_buttons->addWidget(remote_key, 1);
    remote_buttons->addWidget(use);
    remote_buttons->addWidget(add_remote);
    remote_buttons->addWidget(edit_remote);
    remote_buttons->addWidget(remove_remote);
    server_layout->addLayout(remote_buttons);

    auto *period_buttons = new QHBoxLayout();
    period_buttons->addWidget(lock);
    period_buttons->addWidget(unlock);
    period_buttons->addWidget(regen);
    period_buttons->addWidget(info);
    period_buttons->addWidget(remote_filter);
    period_buttons->addWidget(account_types);
    period_buttons->addWidget(update_period);
    period_buttons->addWidget(nullification_period);
    period_buttons->addStretch(1);
    server_layout->addLayout(period_buttons);

    server_layout->addWidget(new QLabel(QStringLiteral("accounts on the selected service"), server));
    accounts = new QTableWidget(server);
    accounts->setColumnCount(6);
    accounts->setHorizontalHeaderLabels({
        QStringLiteral("account key"),
        QStringLiteral("type"),
        QStringLiteral("created"),
        QStringLiteral("expires"),
        QStringLiteral("banned"),
        QStringLiteral("message")
    });
    accounts->horizontalHeader()->setSectionResizeMode(0, QHeaderView::Stretch);
    accounts->horizontalHeader()->setSectionResizeMode(1, QHeaderView::ResizeToContents);
    accounts->horizontalHeader()->setSectionResizeMode(2, QHeaderView::ResizeToContents);
    accounts->horizontalHeader()->setSectionResizeMode(3, QHeaderView::ResizeToContents);
    accounts->horizontalHeader()->setSectionResizeMode(4, QHeaderView::ResizeToContents);
    accounts->horizontalHeader()->setSectionResizeMode(5, QHeaderView::Stretch);
    accounts->setSelectionBehavior(QAbstractItemView::SelectRows);
    accounts->setSelectionMode(QAbstractItemView::SingleSelection);
    accounts->setEditTriggers(QAbstractItemView::NoEditTriggers);
    accounts->verticalHeader()->setVisible(false);
    server_layout->addWidget(accounts, 1);

    auto *account_buttons = new QHBoxLayout();
    auto *ban = new QPushButton(QStringLiteral("ban..."), server);
    auto *unban = new QPushButton(QStringLiteral("unban"), server);
    auto *expires = new QPushButton(QStringLiteral("expires..."), server);
    auto *message = new QPushButton(QStringLiteral("message..."), server);
    auto *delete_content = new QPushButton(QStringLiteral("delete content"), server);
    auto *account_type = new QPushButton(QStringLiteral("account type..."), server);
    account_buttons->addWidget(ban);
    account_buttons->addWidget(unban);
    account_buttons->addWidget(expires);
    account_buttons->addWidget(message);
    account_buttons->addWidget(delete_content);
    account_buttons->addWidget(account_type);
    account_buttons->addStretch(1);
    server_layout->addLayout(account_buttons);

    connect(connect_button, &QPushButton::clicked, this, [this]() { connect_admin(); });
    connect(use, &QPushButton::clicked, this, [this]() { select_remote_service(); });
    connect(add_remote, &QPushButton::clicked, this, [this]() { add_remote_service(); });
    connect(edit_remote, &QPushButton::clicked, this, [this]() { edit_remote_service(); });
    connect(remove_remote, &QPushButton::clicked, this, [this]() { remove_remote_service(); });
    connect(lock, &QPushButton::clicked, this, [this]() { remote_service_action(); });
    connect(unlock, &QPushButton::clicked, this, [this]() { remote_service_action(); });
    connect(regen, &QPushButton::clicked, this, [this]() { remote_service_action(); });
    connect(update_period, &QPushButton::clicked, this, [this]() { remote_service_action(); });
    connect(nullification_period, &QPushButton::clicked, this, [this]() { remote_service_action(); });
    connect(info, &QPushButton::clicked, this, [this]() {
        const QJsonObject admin = data.value(QStringLiteral("admin")).toObject();
        QDialog dialog(this);
        dialog.setWindowTitle(QStringLiteral("service review"));
        dialog.resize(720, 520);
        auto *layout = new QVBoxLayout(&dialog);
        auto *text = new QTextEdit(&dialog);
        text->setReadOnly(true);

        QJsonObject review;
        review.insert(QStringLiteral("options"), admin.value(QStringLiteral("options")));
        review.insert(QStringLiteral("service_info"), admin.value(QStringLiteral("service_info")));
        text->setPlainText(QString::fromUtf8(
            QJsonDocument(review).toJson(QJsonDocument::Indented)));

        layout->addWidget(text, 1);
        auto *buttons = new QDialogButtonBox(QDialogButtonBox::Close, &dialog);
        layout->addWidget(buttons);
        connect(buttons, &QDialogButtonBox::rejected, &dialog, &QDialog::reject);
        dialog.exec();
    });
    connect(remote_filter, &QPushButton::clicked, this, [this]() {
        const QString key = selected_remote_key();

        if (!key.isEmpty()) {
            main->notify(QStringLiteral("tag_filter_open"), QString(), key);
        }
    });
    connect(account_types, &QPushButton::clicked, this, [this]() {
        if (!selected_remote_key().isEmpty()) {
            main->notify(QStringLiteral("account_types_open"), QString(), selected_remote_key());
        }
    });
    connect(ban, &QPushButton::clicked, this, [this]() { account_action(); });
    connect(unban, &QPushButton::clicked, this, [this]() { account_action(); });
    connect(expires, &QPushButton::clicked, this, [this]() { account_action(); });
    connect(message, &QPushButton::clicked, this, [this]() { account_action(); });
    connect(delete_content, &QPushButton::clicked, this, [this]() { account_action(); });
    connect(account_type, &QPushButton::clicked, this, [this]() { account_action(); });
    connect(remote_services, &QTableWidget::itemSelectionChanged, this, [this]() {
        const int row = remote_services->currentRow();

        if (row >= 0) {
            remote_key->setText(cell_value(remote_services, row, 4));
        }
    });

    tabs->addTab(server, QStringLiteral("server"));

    // Credentials -----------------------------------------------------------
    auto *credentials = new QWidget(tabs);
    auto *credentials_layout = new QVBoxLayout(credentials);
    logins = new QTableWidget(credentials);
    logins->setColumnCount(5);
    logins->setHorizontalHeaderLabels({
        QStringLiteral("domain"),
        QStringLiteral("username"),
        QStringLiteral("password"),
        QStringLiteral("access key"),
        QStringLiteral("expires")
    });
    logins->horizontalHeader()->setSectionResizeMode(0, QHeaderView::Stretch);
    logins->horizontalHeader()->setSectionResizeMode(1, QHeaderView::ResizeToContents);
    logins->horizontalHeader()->setSectionResizeMode(2, QHeaderView::ResizeToContents);
    logins->horizontalHeader()->setSectionResizeMode(3, QHeaderView::Stretch);
    logins->horizontalHeader()->setSectionResizeMode(4, QHeaderView::ResizeToContents);
    logins->setSelectionBehavior(QAbstractItemView::SelectRows);
    logins->setSelectionMode(QAbstractItemView::SingleSelection);
    logins->setEditTriggers(QAbstractItemView::NoEditTriggers);
    logins->verticalHeader()->setVisible(false);
    credentials_layout->addWidget(logins, 1);

    auto *login_buttons = new QHBoxLayout();
    auto *add_login = new QPushButton(QStringLiteral("add credentials..."), credentials);
    auto *edit_login_button = new QPushButton(QStringLiteral("edit..."), credentials);
    auto *remove_login_button = new QPushButton(QStringLiteral("remove"), credentials);
    login_buttons->addWidget(add_login);
    login_buttons->addWidget(edit_login_button);
    login_buttons->addWidget(remove_login_button);
    login_buttons->addStretch(1);
    credentials_layout->addLayout(login_buttons);
    connect(add_login, &QPushButton::clicked, this, [this]() { this->edit_login(); });
    connect(edit_login_button, &QPushButton::clicked, this, [this]() { this->edit_login(); });
    connect(remove_login_button, &QPushButton::clicked, this, [this]() { this->remove_login(); });
    connect(logins, &QTableWidget::cellDoubleClicked, this, [this](int, int) { this->edit_login(); });

    tabs->addTab(credentials, QStringLiteral("logins"));

    auto *close = new QPushButton(QStringLiteral("close"), this);
    auto *footer = new QHBoxLayout();
    footer->addStretch(1);
    footer->addWidget(close);
    layout->addLayout(footer);
    connect(close, &QPushButton::clicked, this, &QDialog::close);

    set_state(state);

    const QString requested_tab = qEnvironmentVariable("HYDRUS_QT_SERVICES_TAB");

    if (!requested_tab.isEmpty()) {
        for (int index = 0; index < tabs->count(); ++index) {
            if (tabs->tabText(index) == requested_tab) {
                tabs->setCurrentIndex(index);
                break;
            }
        }
    }

    const QString edit_key = qEnvironmentVariable("HYDRUS_QT_SERVICES_EDIT");

    if (edit_key == QStringLiteral("new")) {
        add_service();
    } else if (!edit_key.isEmpty()) {
        for (int row = 0; row < table->rowCount(); ++row) {
            if (cell_key(table, row).compare(edit_key, Qt::CaseInsensitive) == 0) {
                table->selectRow(row);
                edit_selected();
                break;
            }
        }
    }

    if (qEnvironmentVariableIsSet("HYDRUS_QT_SERVICES_SPECIFIER")) {
        show_specifier();
    }
}

void HydrusServicesDialog::set_state(const QJsonObject &state) {
    data = state;

    const QJsonArray services = state.value(QStringLiteral("services")).toArray();
    table->setRowCount(services.size());

    for (int index = 0; index < services.size(); ++index) {
        const QJsonObject service = services.at(index).toObject();

        for (int column = 0; column < 5; ++column) {
            if (table->item(index, column) == nullptr) {
                table->setItem(index, column, new QTableWidgetItem());
            }
        }

        const QJsonObject dictionary = service.value(QStringLiteral("dictionary")).toObject();

        auto *name_item = table->item(index, 0);
        name_item->setText(service.value(QStringLiteral("name")).toString());
        name_item->setData(Qt::UserRole, service);
        name_item->setData(Qt::UserRole + 1, service.value(QStringLiteral("key")).toString());

        if (service.value(QStringLiteral("builtin")).toBool()) {
            name_item->setForeground(QColor(120, 150, 190));
        } else {
            name_item->setForeground(QBrush());
        }

        table->item(index, 1)->setText(service.value(QStringLiteral("type_short")).toString());
        table->item(index, 2)->setText(service.value(QStringLiteral("key")).toString());
        table->item(index, 3)->setText(dictionary.contains(QStringLiteral("port"))
                                            ? QString::number(dictionary.value(QStringLiteral("port")).toInt())
                                            : QStringLiteral("-"));

        QString summary = QStringLiteral("-");

        if (!dictionary.isEmpty()) {
            summary = QStringLiteral("%1:%2")
                          .arg(dictionary.value(QStringLiteral("host")).toString())
                          .arg(dictionary.value(QStringLiteral("port")).toInt());

            if (dictionary.contains(QStringLiteral("update_period"))) {
                summary += QStringLiteral(" | update every %1 s")
                               .arg(dictionary.value(QStringLiteral("update_period")).toInt());
            }

            if (dictionary.contains(QStringLiteral("nullification_period"))) {
                summary += QStringLiteral(" | nullify every %1 s")
                               .arg(dictionary.value(QStringLiteral("nullification_period")).toInt());
            }
        }

        table->item(index, 4)->setText(summary);
    }

    table->resizeRowsToContents();

    const QJsonObject admin = state.value(QStringLiteral("admin")).toObject();
    admin_address->setText(admin.value(QStringLiteral("address")).toString());
    admin_key->setText(admin.value(QStringLiteral("access_key")).toString());
    admin_status->setText(admin.value(QStringLiteral("status")).toString());

    const QJsonArray remote = admin.value(QStringLiteral("services")).toArray();
    remote_services->setRowCount(remote.size());

    for (int index = 0; index < remote.size(); ++index) {
        const QJsonObject service = remote.at(index).toObject();

        for (int column = 0; column < 5; ++column) {
            if (remote_services->item(index, column) == nullptr) {
                remote_services->setItem(index, column, new QTableWidgetItem());
            }
        }

        auto *name_item = remote_services->item(index, 0);
        name_item->setText(service.value(QStringLiteral("name")).toString());
        name_item->setData(Qt::UserRole, service);
        remote_services->item(index, 1)->setText(service.value(QStringLiteral("type")).toString());
        remote_services->item(index, 2)->setText(QString::number(service.value(QStringLiteral("port")).toInt()));
        remote_services->item(index, 3)->setText(service.value(QStringLiteral("key")).toString());
        remote_services->item(index, 4)->setText(service.value(QStringLiteral("access_key")).toString());

        if (service.value(QStringLiteral("selected")).toBool()) {
            remote_services->selectRow(index);
        }
    }

    remote_services->resizeRowsToContents();

    if (!opened_account_types
        && qEnvironmentVariableIsSet("HYDRUS_QT_SERVICES_ACCOUNT_TYPES")
        && remote_services->rowCount() > 0) {
        opened_account_types = true;
        const QJsonObject first = remote_services->item(0, 0)->data(Qt::UserRole).toJsonObject();
        const QString key = first.value(QStringLiteral("key")).toString();
        remote_services->selectRow(0);

        QJsonObject select;
        select.insert(QStringLiteral("action"), QStringLiteral("select"));
        select.insert(QStringLiteral("service_key"), key);
        select.insert(QStringLiteral("access_key"),
                      first.value(QStringLiteral("access_key")).toString().isEmpty()
                          ? admin_key->text().trimmed()
                          : first.value(QStringLiteral("access_key")).toString());
        main->notify(QStringLiteral("admin_service_action"), QString(), json_payload(select));
        main->notify(QStringLiteral("account_types_open"), QString(), key);
    }

    const QJsonArray account_list = admin.value(QStringLiteral("accounts")).toArray();
    accounts->setRowCount(account_list.size());

    for (int index = 0; index < account_list.size(); ++index) {
        const QJsonObject account = account_list.at(index).toObject();

        for (int column = 0; column < 6; ++column) {
            if (accounts->item(index, column) == nullptr) {
                accounts->setItem(index, column, new QTableWidgetItem());
            }
        }

        auto *key_item = accounts->item(index, 0);
        key_item->setText(account.value(QStringLiteral("key")).toString());
        key_item->setData(Qt::UserRole, account);

        const QJsonValue expires = account.value(QStringLiteral("expires"));
        accounts->item(index, 1)->setText(account.value(QStringLiteral("type")).toString());
        accounts->item(index, 2)->setText(QString::number(account.value(QStringLiteral("created")).toInt()));
        accounts->item(index, 3)->setText(expires.isNull() ? QStringLiteral("never")
                                                           : QString::number(expires.toInt()));
        accounts->item(index, 4)->setText(account.value(QStringLiteral("banned")).toBool()
                                              ? account.value(QStringLiteral("banned_reason")).toString()
                                              : QStringLiteral("-"));
        accounts->item(index, 5)->setText(account.value(QStringLiteral("message")).toString());
    }

    accounts->resizeRowsToContents();

    const QJsonArray login_list = state.value(QStringLiteral("logins")).toArray();
    logins->setRowCount(login_list.size());

    for (int index = 0; index < login_list.size(); ++index) {
        const QJsonObject login = login_list.at(index).toObject();

        for (int column = 0; column < 5; ++column) {
            if (logins->item(index, column) == nullptr) {
                logins->setItem(index, column, new QTableWidgetItem());
            }
        }

        auto *domain_item = logins->item(index, 0);
        domain_item->setText(login.value(QStringLiteral("domain")).toString());
        domain_item->setData(Qt::UserRole, login);
        logins->item(index, 1)->setText(login.value(QStringLiteral("username")).toString());
        logins->item(index, 2)->setText(login.value(QStringLiteral("password")).toString().isEmpty()
                                            ? QStringLiteral("-")
                                            : QStringLiteral("********"));
        logins->item(index, 3)->setText(login.value(QStringLiteral("access_key")).toString());
        const QJsonValue expires = login.value(QStringLiteral("expires"));
        logins->item(index, 4)->setText(expires.isNull() ? QStringLiteral("never")
                                                         : QString::number(expires.toInt()));
    }

    logins->resizeRowsToContents();
}
