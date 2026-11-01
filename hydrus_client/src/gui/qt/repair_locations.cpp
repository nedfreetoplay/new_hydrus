
// WS-15.2.4b: typed repair-locations dialog (no QJsonObject in set_state).

#include <hydrus_client/src/gui/qt/api.cxx.h>
#include "support/support.hpp"

#include <QCheckBox>
#include <QFileDialog>
#include <QHBoxLayout>
#include <QHeaderView>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QLabel>
#include <QMessageBox>
#include <QPushButton>
#include <QTableWidget>
#include <QVBoxLayout>

HydrusRepairFileLocationsDialog::HydrusRepairFileLocationsDialog(
    HydrusMainWindow *main_window, const RepairFileLocationsDialogState &state, QWidget *parent)
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
        send(UiAction::RepairLocationsSet, QString::fromUtf8(QJsonDocument(payload).toJson(QJsonDocument::Compact)));
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
        send(UiAction::RepairLocationsProbe, QString::fromUtf8(QJsonDocument(payload).toJson(QJsonDocument::Compact)));
    });
    connect(apply_button, &QPushButton::clicked, this, [this]() {
        QJsonObject payload;
        payload.insert(QStringLiteral("create_thumbnails"), create_thumbnails->isChecked());
        send(UiAction::RepairLocationsApply, QString::fromUtf8(QJsonDocument(payload).toJson(QJsonDocument::Compact)));
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

void HydrusRepairFileLocationsDialog::send(UiAction action, const QString &payload) {
    main->notify(action, QString(), payload);
}

void HydrusRepairFileLocationsDialog::set_state(const RepairFileLocationsDialogState &state) {
    summary->setText(to_qstring(state.summary));
    message->setText(to_qstring(state.message));

    const QString error = to_qstring(state.error);

    if (!error.isEmpty()) {
        QMessageBox::warning(this, windowTitle(), error);
    }

    table->setRowCount(0);

    for (const RepairLocationRow &row : state.rows) {
        const int index = table->rowCount();
        table->insertRow(index);

        table->setItem(index, 0, new QTableWidgetItem(to_qstring(row.old_base)));
        table->setItem(index, 1, new QTableWidgetItem(to_qstring(row.prefix)));
        table->item(index, 1)->setData(Qt::UserRole, to_qstring(row.prefix));
        table->setItem(index, 2, new QTableWidgetItem(to_qstring(row.base)));
        auto *status = new QTableWidgetItem(to_qstring(row.status));

        if (row.exists) {
            status->setForeground(QColor(120, 190, 120));
        } else {
            status->setForeground(QColor(220, 110, 110));
        }

        table->setItem(index, 3, status);
    }
}

void HydrusMainWindow::show_repair_file_locations(const RepairFileLocationsDialogState &state) {
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
