#include "repair_locations.hpp"

// WS-15.1.3c2: repair file locations dialog moved verbatim from bridge.cpp (move-only split).

#include "bridge.hpp"
#include "support/support.hpp"

#include <QCheckBox>
#include <QFileDialog>
#include <QHBoxLayout>
#include <QHeaderView>
#include <QJsonArray>
#include <QLabel>
#include <QMessageBox>
#include <QPushButton>
#include <QTableWidget>
#include <QVBoxLayout>

HydrusRepairFileLocationsDialog::HydrusRepairFileLocationsDialog(
    HydrusMainWindow *main_window, const QJsonObject &state, QWidget *parent)
    : QDialog(parent), main(main_window) {
    setWindowTitle(QStringLiteral("repair file locations"));
    setObjectName(QStringLiteral("hydrus_repair_file_locations"));
    setAttribute(Qt::WA_DeleteOnClose, true);
    resize(980, 540);

    auto *layout = new QVBoxLayout(this);

    auto *help = new QLabel(
        QStringLiteral("Some expected file storage directories were not found. Set the correct "
                       "base folder for each prefix, or probe a folder that contains the old "
                       "client_files/thumbnails layout. File prefixes must be corrected; "
                       "thumbnail prefixes can be recreated empty."),
        this);
    help->setWordWrap(true);
    layout->addWidget(help);

    summary = new QLabel(this);
    summary->setWordWrap(true);
    layout->addWidget(summary);

    table = new QTableWidget(this);
    table->setObjectName(QStringLiteral("repair_file_locations"));
    table->setColumnCount(4);
    table->setHorizontalHeaderLabels({QStringLiteral("current base"), QStringLiteral("prefix"),
                                      QStringLiteral("corrected base"), QStringLiteral("status")});
    table->horizontalHeader()->setSectionResizeMode(0, QHeaderView::Stretch);
    table->horizontalHeader()->setSectionResizeMode(1, QHeaderView::ResizeToContents);
    table->horizontalHeader()->setSectionResizeMode(2, QHeaderView::Stretch);
    table->horizontalHeader()->setSectionResizeMode(3, QHeaderView::ResizeToContents);
    table->verticalHeader()->setVisible(false);
    table->setSelectionBehavior(QAbstractItemView::SelectRows);
    table->setSelectionMode(QAbstractItemView::ExtendedSelection);
    table->setEditTriggers(QAbstractItemView::NoEditTriggers);
    layout->addWidget(table, 1);

    message = new QLabel(this);
    message->setWordWrap(true);
    layout->addWidget(message);

    auto *controls = new QHBoxLayout();
    create_thumbnails = new QCheckBox(
        QStringLiteral("create empty folders for missing thumbnail paths"), this);
    create_thumbnails->setChecked(true);
    auto *set_button = new QPushButton(QStringLiteral("set correct location"), this);
    auto *probe_button = new QPushButton(
        QStringLiteral("add a possibly correct location (let the client figure out what it contains)"),
        this);
    auto *apply_button = new QPushButton(QStringLiteral("apply"), this);
    apply_button->setObjectName(QStringLiteral("HydrusAccept"));
    auto *close_button = new QPushButton(QStringLiteral("close"), this);
    controls->addWidget(create_thumbnails);
    controls->addStretch(1);
    controls->addWidget(set_button);
    controls->addWidget(probe_button);
    controls->addWidget(apply_button);
    controls->addWidget(close_button);
    layout->addLayout(controls);

    connect(set_button, &QPushButton::clicked, this, [this]() {
        const QStringList prefixes = selected_prefixes();

        if (prefixes.isEmpty()) {
            QMessageBox::information(this, windowTitle(), QStringLiteral("Select one or more rows first."));
            return;
        }

        const QString path = QFileDialog::getExistingDirectory(
            this,
            QStringLiteral("Select correct location."));

        if (path.isEmpty()) {
            return;
        }

        QJsonArray updates;
        for (const QString &prefix : prefixes) {
            QJsonObject update;
            update.insert(QStringLiteral("prefix"), prefix);
            update.insert(QStringLiteral("base"), path);
            updates.append(update);
        }

        QJsonObject payload;
        payload.insert(QStringLiteral("updates"), updates);
        send(QStringLiteral("repair_locations_set"), payload);
    });
    connect(probe_button, &QPushButton::clicked, this, [this]() {
        const QString path = QFileDialog::getExistingDirectory(
            this,
            QStringLiteral("Select the potential correct location."));

        if (path.isEmpty()) {
            return;
        }

        QJsonObject payload;
        payload.insert(QStringLiteral("path"), path);
        send(QStringLiteral("repair_locations_probe"), payload);
    });
    connect(apply_button, &QPushButton::clicked, this, [this]() {
        QJsonObject payload;
        payload.insert(QStringLiteral("create_thumbnails"), create_thumbnails->isChecked());
        send(QStringLiteral("repair_locations_apply"), payload);
    });
    connect(close_button, &QPushButton::clicked, this, &QDialog::close);

    set_state(state);
}

QStringList HydrusRepairFileLocationsDialog::selected_prefixes() const {
    QStringList prefixes;

    for (const QTableWidgetItem *item : table->selectedItems()) {
        if (item->column() != 1) {
            continue;
        }

        const QString prefix = item->data(Qt::UserRole).toString();

        if (!prefix.isEmpty() && !prefixes.contains(prefix)) {
            prefixes << prefix;
        }
    }

    return prefixes;
}

void HydrusRepairFileLocationsDialog::send(const QString &action, const QJsonObject &payload) {
    main->notify(action, QString(), payload.isEmpty() ? QString() : json_payload(payload));
}

void HydrusRepairFileLocationsDialog::set_state(const QJsonObject &state) {
    data = state;
    summary->setText(state.value(QStringLiteral("summary")).toString());
    message->setText(state.value(QStringLiteral("message")).toString());

    const QString error = state.value(QStringLiteral("error")).toString();

    if (!error.isEmpty()) {
        QMessageBox::warning(this, windowTitle(), error);
    }

    table->setRowCount(0);

    for (const QJsonValue &value : state.value(QStringLiteral("rows")).toArray()) {
        const QJsonObject row = value.toObject();
        const int index = table->rowCount();
        table->insertRow(index);

        table->setItem(index, 0, new QTableWidgetItem(row.value(QStringLiteral("old_base")).toString()));
        table->setItem(index, 1, new QTableWidgetItem(row.value(QStringLiteral("prefix")).toString()));
        table->item(index, 1)->setData(Qt::UserRole, row.value(QStringLiteral("prefix")).toString());
        table->setItem(index, 2, new QTableWidgetItem(row.value(QStringLiteral("base")).toString()));
        auto *status = new QTableWidgetItem(row.value(QStringLiteral("status")).toString());

        if (row.value(QStringLiteral("exists")).toBool()) {
            status->setForeground(QColor(120, 190, 120));
        } else {
            status->setForeground(QColor(220, 110, 110));
        }

        table->setItem(index, 3, status);
    }
}

void HydrusMainWindow::show_repair_file_locations(const QJsonObject &state) {
    if (repair_file_locations_dialog == nullptr) {
        repair_file_locations_dialog = new HydrusRepairFileLocationsDialog(this, state, this);
        connect(repair_file_locations_dialog, &QObject::destroyed, this, [this]() {
            repair_file_locations_dialog = nullptr;
        });
        repair_file_locations_dialog->show();
    } else {
        repair_file_locations_dialog->set_state(state);
        repair_file_locations_dialog->raise();
        repair_file_locations_dialog->activateWindow();
    }
}

void HydrusMainWindow::close_repair_file_locations() {
    if (repair_file_locations_dialog != nullptr) {
        repair_file_locations_dialog->close();
    }
}
