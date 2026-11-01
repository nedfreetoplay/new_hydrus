// WS-15.1.3c2: export folder review dialog moved verbatim from bridge.cpp (move-only split).

#include <hydrus_client/src/gui/qt/api.cxx.h>
#include "support/support.hpp"

#include <QDialog>
#include <QHBoxLayout>
#include <QHeaderView>
#include <QJsonArray>
#include <QLabel>
#include <QMessageBox>
#include <QPushButton>
#include <QTableWidget>
#include <QVBoxLayout>

void HydrusMainWindow::show_export_review(const ExportReviewDialogState &typed) {
    const QString name = to_qstring(typed.name);
    const int file_count = static_cast<int>(typed.files.size());

    QDialog dialog(this);
    dialog.setWindowTitle(to_qstring(typed.title));
    dialog.resize(860, 520);

    auto *layout = new QVBoxLayout(&dialog);

    auto *summary = new QLabel(to_qstring(typed.summary), &dialog);
    summary->setWordWrap(true);
    layout->addWidget(summary);

    auto *table = new QTableWidget(&dialog);
    table->setColumnCount(5);
    table->setHorizontalHeaderLabels({
        QStringLiteral("export"),
        QStringLiteral("file"),
        QStringLiteral("tags"),
        QStringLiteral("destination"),
        QStringLiteral("state")
    });
    table->horizontalHeader()->setStretchLastSection(false);
    table->horizontalHeader()->setSectionResizeMode(0, QHeaderView::ResizeToContents);
    table->horizontalHeader()->setSectionResizeMode(1, QHeaderView::ResizeToContents);
    table->horizontalHeader()->setSectionResizeMode(2, QHeaderView::Stretch);
    table->horizontalHeader()->setSectionResizeMode(3, QHeaderView::Stretch);
    table->horizontalHeader()->setSectionResizeMode(4, QHeaderView::ResizeToContents);
    table->setSelectionBehavior(QAbstractItemView::SelectRows);
    table->setSelectionMode(QAbstractItemView::SingleSelection);
    table->setEditTriggers(QAbstractItemView::NoEditTriggers);
    table->setTextElideMode(Qt::ElideMiddle);
    table->verticalHeader()->setVisible(false);
    table->setRowCount(file_count);

    for (int index = 0; index < file_count; ++index) {
        const ExportReviewFile &file = typed.files[index];
        const QString hash = to_qstring(file.hash);
        const QString destination = to_qstring(file.destination);
        const QString relative = to_qstring(file.relative);

        for (int column = 0; column < 5; ++column) {
            table->setItem(index, column, new QTableWidgetItem());
        }

        auto *check = table->item(index, 0);
        check->setFlags(Qt::ItemIsEnabled | Qt::ItemIsUserCheckable | Qt::ItemIsSelectable);
        check->setCheckState(file.checked ? Qt::Checked : Qt::Unchecked);
        check->setData(Qt::UserRole, hash);

        table->item(index, 1)->setText(hash);
        table->item(index, 2)->setText(to_qstring(file.tags_text));
        table->item(index, 3)->setText(relative);
        table->item(index, 3)->setToolTip(destination);
        table->item(index, 4)->setText(to_qstring(file.note));
    }

    layout->addWidget(table, 1);

    auto *buttons = new QHBoxLayout();
    auto *check_all = new QPushButton(QStringLiteral("check all"), &dialog);
    auto *check_none = new QPushButton(QStringLiteral("check none"), &dialog);
    auto *export_checked = new QPushButton(QStringLiteral("export checked"), &dialog);
    auto *close = new QPushButton(QStringLiteral("close"), &dialog);
    buttons->addWidget(check_all);
    buttons->addWidget(check_none);
    buttons->addStretch(1);
    buttons->addWidget(export_checked);
    buttons->addWidget(close);
    layout->addLayout(buttons);

    auto set_all = [table](Qt::CheckState value) {
        for (int index = 0; index < table->rowCount(); ++index) {
            QTableWidgetItem *item = table->item(index, 0);

            if (item != nullptr && (item->flags() & Qt::ItemIsUserCheckable)) {
                item->setCheckState(value);
            }
        }
    };

    connect(check_all, &QPushButton::clicked, &dialog, [set_all]() { set_all(Qt::Checked); });
    connect(check_none, &QPushButton::clicked, &dialog, [set_all]() { set_all(Qt::Unchecked); });
    connect(close, &QPushButton::clicked, &dialog, &QDialog::reject);
    connect(export_checked, &QPushButton::clicked, &dialog, [this, &dialog, table, name]() {
        QJsonArray hashes;

        for (int index = 0; index < table->rowCount(); ++index) {
            QTableWidgetItem *item = table->item(index, 0);

            if (item != nullptr && item->checkState() == Qt::Checked) {
                hashes.append(item->data(Qt::UserRole).toString());
            }
        }

        if (hashes.isEmpty()) {
            QMessageBox::information(&dialog, QStringLiteral("export folder"),
                                     QStringLiteral("No files are checked."));
            return;
        }

        QJsonObject payload;
        payload.insert(QStringLiteral("name"), name);
        payload.insert(QStringLiteral("hashes"), hashes);
        notify(UiAction::ExportFolderRun, active_page, json_payload(payload));
        dialog.accept();
    });

    dialog.exec();
}
