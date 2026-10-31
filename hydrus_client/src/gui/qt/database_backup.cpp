#include "database_backup.hpp"

// WS-15.1.3c2: database backup dialog moved verbatim from bridge.cpp (move-only split).

#include "bridge.hpp"
#include "support/support.hpp"

#include <QDialog>
#include <QFileDialog>
#include <QFormLayout>
#include <QHBoxLayout>
#include <QLabel>
#include <QLineEdit>
#include <QMessageBox>
#include <QPushButton>
#include <QStandardPaths>
#include <QVBoxLayout>

HydrusDatabaseBackupDialog::HydrusDatabaseBackupDialog(HydrusMainWindow *main_window, const QJsonObject &state, QWidget *parent)
    : QDialog(parent), main(main_window) {
    setWindowTitle(QStringLiteral("database backup"));
    setObjectName(QStringLiteral("hydrus_database_backup_dialog"));
    setAttribute(Qt::WA_DeleteOnClose, true);
    resize(640, 260);

    auto *layout = new QVBoxLayout(this);

    auto *help = new QLabel(
        QStringLiteral("A consistent copy of client.db is written as client.db in the backup directory. "
                       "The existing copy is replaced."),
        this);
    help->setWordWrap(true);
    layout->addWidget(help);

    auto *form = new QFormLayout();
    auto *path_row = new QHBoxLayout();
    path = new QLineEdit(this);
    path->setReadOnly(true);
    auto *change = new QPushButton(QStringLiteral("change path..."), this);
    path_row->addWidget(path, 1);
    path_row->addWidget(change);
    form->addRow(QStringLiteral("backup directory"), path_row);
    last_backup = new QLabel(this);
    form->addRow(QStringLiteral("last backup"), last_backup);
    layout->addLayout(form);

    status = new QLabel(this);
    status->setWordWrap(true);
    layout->addWidget(status);
    layout->addStretch(1);

    auto *buttons = new QHBoxLayout();
    auto *run = new QPushButton(QStringLiteral("back up now"), this);
    auto *close = new QPushButton(QStringLiteral("close"), this);
    buttons->addWidget(run);
    buttons->addStretch(1);
    buttons->addWidget(close);
    layout->addLayout(buttons);

    connect(change, &QPushButton::clicked, this, [this]() { choose_path(); });
    connect(run, &QPushButton::clicked, this, [this]() {
        if (QMessageBox::question(this, QStringLiteral("database backup"),
                                  QStringLiteral("Back up the database now?"))
            == QMessageBox::Yes) {
            main->notify(QStringLiteral("db_backup_run"));
        }
    });
    connect(close, &QPushButton::clicked, this, &QDialog::close);

    set_state(state);
}

void HydrusDatabaseBackupDialog::set_state(const QJsonObject &state) {
    const QString directory = state.value(QStringLiteral("path")).toString();
    path->setText(directory.isEmpty() ? QStringLiteral("(not set)") : directory);
    path->setCursorPosition(0);
    last_backup->setText(state.value(QStringLiteral("last_backup")).toString());

    const QString target = state.value(QStringLiteral("target")).toString();
    status->setText(target.isEmpty()
                        ? QStringLiteral("Set a backup directory to enable backups.")
                        : QStringLiteral("target: %1").arg(target));
}

void HydrusDatabaseBackupDialog::choose_path() {
    const QString directory = QFileDialog::getExistingDirectory(
        this, QStringLiteral("backup directory"),
        QStandardPaths::writableLocation(QStandardPaths::DocumentsLocation));

    if (!directory.isEmpty()) {
        main->notify(QStringLiteral("db_backup_set_path"), QString(), directory);
    }
}

void HydrusMainWindow::show_database_backup(const QJsonObject &state) {
    if (database_backup_dialog == nullptr) {
        database_backup_dialog = new HydrusDatabaseBackupDialog(this, state, this);
        connect(database_backup_dialog, &QObject::destroyed, this, [this]() { database_backup_dialog = nullptr; });
        database_backup_dialog->show();
    } else {
        database_backup_dialog->set_state(state);
        database_backup_dialog->raise();
        database_backup_dialog->activateWindow();
    }
}

void HydrusMainWindow::update_database_backup(const QJsonObject &state) {
    if (database_backup_dialog != nullptr) {
        database_backup_dialog->set_state(state);
    }
}
