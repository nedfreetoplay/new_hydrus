#include "orphan_files.hpp"

// WS-15.1.3c2: clear orphan files dialog moved verbatim from bridge.cpp (move-only split).

#include "bridge.hpp"
#include "support/support.hpp"

#include <QDialog>
#include <QFileDialog>
#include <QHBoxLayout>
#include <QHeaderView>
#include <QJsonArray>
#include <QLabel>
#include <QMessageBox>
#include <QPushButton>
#include <QStandardPaths>
#include <QStringList>
#include <QTableWidget>
#include <QVBoxLayout>

HydrusOrphanFilesDialog::HydrusOrphanFilesDialog(HydrusMainWindow *main_window, const QJsonObject &state, QWidget *parent)
    : QDialog(parent), main(main_window) {
    setWindowTitle(QStringLiteral("clear orphan files"));
    setObjectName(QStringLiteral("hydrus_orphan_files_dialog"));
    setAttribute(Qt::WA_DeleteOnClose, true);
    resize(880, 520);

    auto *layout = new QVBoxLayout(this);

    auto *help = new QLabel(
        QStringLiteral("Files in the media storage whose hash is not in the database. "
                       "Thumbnails are included. Anything that is not named after a hash is left alone."),
        this);
    help->setWordWrap(true);
    layout->addWidget(help);

    status = new QLabel(this);
    status->setObjectName(QStringLiteral("orphan_files_status"));
    layout->addWidget(status);

    table = new QTableWidget(this);
    table->setColumnCount(4);
    table->setHorizontalHeaderLabels({
        QStringLiteral("kind"),
        QStringLiteral("size"),
        QStringLiteral("hash"),
        QStringLiteral("path")
    });
    table->horizontalHeader()->setSectionResizeMode(0, QHeaderView::ResizeToContents);
    table->horizontalHeader()->setSectionResizeMode(1, QHeaderView::ResizeToContents);
    table->horizontalHeader()->setSectionResizeMode(2, QHeaderView::Interactive);
    table->setColumnWidth(2, 260);
    table->horizontalHeader()->setSectionResizeMode(3, QHeaderView::Stretch);
    table->setSelectionBehavior(QAbstractItemView::SelectRows);
    table->setSelectionMode(QAbstractItemView::ExtendedSelection);
    table->setEditTriggers(QAbstractItemView::NoEditTriggers);
    table->setWordWrap(false);
    table->verticalHeader()->setVisible(false);
    layout->addWidget(table, 1);

    auto *buttons = new QHBoxLayout();
    auto *rescan = new QPushButton(QStringLiteral("scan again"), this);
    auto *delete_selected_button = new QPushButton(QStringLiteral("delete selected"), this);
    auto *delete_all_button = new QPushButton(QStringLiteral("delete all"), this);
    auto *move_selected_button = new QPushButton(QStringLiteral("move selected to..."), this);
    auto *move_all_button = new QPushButton(QStringLiteral("move all to..."), this);
    buttons->addWidget(rescan);
    buttons->addWidget(delete_selected_button);
    buttons->addWidget(delete_all_button);
    buttons->addWidget(move_selected_button);
    buttons->addWidget(move_all_button);
    buttons->addStretch(1);
    layout->addLayout(buttons);

    connect(rescan, &QPushButton::clicked, this, [this]() {
        main->notify(QStringLiteral("orphan_files_scan"));
    });
    connect(delete_selected_button, &QPushButton::clicked, this, [this]() { delete_selected(false); });
    connect(delete_all_button, &QPushButton::clicked, this, [this]() { delete_selected(true); });
    connect(move_selected_button, &QPushButton::clicked, this, [this]() { move_selected(false); });
    connect(move_all_button, &QPushButton::clicked, this, [this]() { move_selected(true); });

    auto *close = new QPushButton(QStringLiteral("close"), this);
    auto *footer = new QHBoxLayout();
    footer->addStretch(1);
    footer->addWidget(close);
    layout->addLayout(footer);
    connect(close, &QPushButton::clicked, this, &QDialog::close);

    set_state(state);
}

void HydrusOrphanFilesDialog::set_state(const QJsonObject &state) {
    data = state;

    const QJsonArray orphans = state.value(QStringLiteral("orphans")).toArray();
    const int previous_row = table->currentRow();
    table->setRowCount(orphans.size());

    for (int index = 0; index < orphans.size(); ++index) {
        const QJsonObject entry = orphans.at(index).toObject();

        for (int column = 0; column < 4; ++column) {
            if (table->item(index, column) == nullptr) {
                table->setItem(index, column, new QTableWidgetItem());
            }
        }

        auto *kind = table->item(index, 0);
        kind->setText(entry.value(QStringLiteral("kind")).toString());
        kind->setData(Qt::UserRole, entry.value(QStringLiteral("path")).toString());
        table->item(index, 1)->setText(
            entry.value(QStringLiteral("size_display")).toString());
        table->item(index, 2)->setText(entry.value(QStringLiteral("hash")).toString());
        table->item(index, 3)->setText(entry.value(QStringLiteral("path")).toString());
    }

    if (previous_row >= 0 && previous_row < table->rowCount()) {
        table->selectRow(previous_row);
    }

    table->resizeRowsToContents();
    status->setText(QStringLiteral("%1 — %2 file(s), %3 thumbnail(s), %4")
                        .arg(state.value(QStringLiteral("status")).toString())
                        .arg(state.value(QStringLiteral("files")).toInt())
                        .arg(state.value(QStringLiteral("thumbnails")).toInt())
                        .arg(state.value(QStringLiteral("bytes_display")).toString()));
}

QStringList HydrusOrphanFilesDialog::selected_paths(bool all) const {
    QStringList paths;

    if (all) {
        for (int row = 0; row < table->rowCount(); ++row) {
            paths.append(table->item(row, 0)->data(Qt::UserRole).toString());
        }

        return paths;
    }

    for (const QModelIndex &index : table->selectionModel()->selectedRows()) {
        paths.append(table->item(index.row(), 0)->data(Qt::UserRole).toString());
    }

    return paths;
}

void HydrusOrphanFilesDialog::delete_selected(bool all) {
    const QStringList paths = selected_paths(all);

    if (paths.isEmpty()) {
        QMessageBox::information(this, QStringLiteral("clear orphan files"),
                                 QStringLiteral("Select the orphan files to delete first."));
        return;
    }

    if (QMessageBox::question(this, QStringLiteral("delete orphan files"),
                              QStringLiteral("Permanently delete %1 orphan file(s)? This cannot be undone.")
                                  .arg(paths.size()))
        != QMessageBox::Yes) {
        return;
    }

    QJsonObject payload;
    payload.insert(QStringLiteral("paths"), QJsonArray::fromStringList(paths));
    main->notify(QStringLiteral("orphan_files_delete"), QString(), json_payload(payload));
}

void HydrusOrphanFilesDialog::move_selected(bool all) {
    const QStringList paths = selected_paths(all);

    if (paths.isEmpty()) {
        QMessageBox::information(this, QStringLiteral("clear orphan files"),
                                 QStringLiteral("Select the orphan files to move first."));
        return;
    }

    const QString directory = QFileDialog::getExistingDirectory(
        this, QStringLiteral("move orphan files to"),
        QStandardPaths::writableLocation(QStandardPaths::DownloadLocation));

    if (directory.isEmpty()) {
        return;
    }

    QJsonObject payload;
    payload.insert(QStringLiteral("paths"), QJsonArray::fromStringList(paths));
    payload.insert(QStringLiteral("target"), directory);
    main->notify(QStringLiteral("orphan_files_move"), QString(), json_payload(payload));
}

void HydrusMainWindow::show_orphan_files(const QJsonObject &state) {
    if (orphan_files_dialog == nullptr) {
        orphan_files_dialog = new HydrusOrphanFilesDialog(this, state, this);
        connect(orphan_files_dialog, &QObject::destroyed, this, [this]() { orphan_files_dialog = nullptr; });
        orphan_files_dialog->show();
    } else {
        orphan_files_dialog->set_state(state);
        orphan_files_dialog->raise();
        orphan_files_dialog->activateWindow();
    }
}

void HydrusMainWindow::update_orphan_files(const QJsonObject &state) {
    if (orphan_files_dialog != nullptr) {
        orphan_files_dialog->set_state(state);
    }
}
