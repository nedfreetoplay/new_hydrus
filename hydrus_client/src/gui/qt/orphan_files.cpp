
// WS-15.2.4b: typed orphan-files dialog (no QJsonObject).

#include <hydrus_client/src/gui/qt/api.cxx.h>
#include "support/support.hpp"

#include <QDialog>
#include <QFileDialog>
#include <QHBoxLayout>
#include <QHeaderView>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QLabel>
#include <QMessageBox>
#include <QPushButton>
#include <QStandardPaths>
#include <QStringList>
#include <QTableWidget>
#include <QVBoxLayout>

HydrusOrphanFilesDialog::HydrusOrphanFilesDialog(HydrusMainWindow *main_window, const OrphanFilesDialogState &state, QWidget *parent)
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
        main->notify(UiAction::OrphanFilesScan);
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

void HydrusOrphanFilesDialog::set_state(const OrphanFilesDialogState &state) {
    const size_t count = state.orphans.size();
    const int previous_row = table->currentRow();
    table->setRowCount(static_cast<int>(count));

    for (size_t index = 0; index < count; ++index) {
        const OrphanFileRow &entry = state.orphans[index];
        const int row = static_cast<int>(index);

        for (int column = 0; column < 4; ++column) {
            if (table->item(row, column) == nullptr) {
                table->setItem(row, column, new QTableWidgetItem());
            }
        }

        auto *kind = table->item(row, 0);
        kind->setText(to_qstring(entry.kind));
        kind->setData(Qt::UserRole, to_qstring(entry.path));
        table->item(row, 1)->setText(to_qstring(entry.size_display));
        table->item(row, 2)->setText(to_qstring(entry.hash));
        table->item(row, 3)->setText(to_qstring(entry.path));
    }

    if (previous_row >= 0 && previous_row < table->rowCount()) {
        table->selectRow(previous_row);
    }

    table->resizeRowsToContents();
    status->setText(QStringLiteral("%1 — %2 file(s), %3 thumbnail(s), %4")
                        .arg(to_qstring(state.status))
                        .arg(static_cast<qlonglong>(state.files))
                        .arg(static_cast<qlonglong>(state.thumbnails))
                        .arg(to_qstring(state.bytes_display)));
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
    main->notify(UiAction::OrphanFilesDelete, QString(), json_payload(payload));
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
    main->notify(UiAction::OrphanFilesMove, QString(), json_payload(payload));
}

void HydrusMainWindow::show_orphan_files(const OrphanFilesDialogState &state) {
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

void HydrusMainWindow::update_orphan_files(const OrphanFilesDialogState &state) {
    if (orphan_files_dialog != nullptr) {
        orphan_files_dialog->set_state(state);
    }
}
