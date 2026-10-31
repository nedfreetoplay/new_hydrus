// WS-15.1.3c2: export folder review dialog moved verbatim from bridge.cpp (move-only split).

#include "bridge.hpp"
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

void HydrusMainWindow::show_export_review(const QJsonObject &state) {
    const QString name = state.value(QStringLiteral("name")).toString();
    const QJsonArray files = state.value(QStringLiteral("files")).toArray();

    QDialog dialog(this);
    dialog.setWindowTitle(QStringLiteral("review export folder '%1'").arg(name));
    dialog.resize(860, 520);

    auto *layout = new QVBoxLayout(&dialog);

    auto *summary = new QLabel(
        QStringLiteral("%1 -> %2\n%3 files match the search. Uncheck anything you do not want "
                       "to export this run.")
            .arg(name, state.value(QStringLiteral("destination")).toString())
            .arg(files.size()),
        &dialog);
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
    table->setRowCount(files.size());

    for (int index = 0; index < files.size(); ++index) {
        const QJsonObject file = files.at(index).toObject();
        const QString hash = file.value(QStringLiteral("hash")).toString();
        const bool safe = file.value(QStringLiteral("safe")).toBool();
        const bool source_exists = file.value(QStringLiteral("source_exists")).toBool(true);
        const bool exists = file.value(QStringLiteral("exists")).toBool();
        const QString destination = file.value(QStringLiteral("destination")).toString();
        const QString root_dir = state.value(QStringLiteral("destination")).toString();
        const QString relative = destination.startsWith(root_dir)
            ? destination.mid(root_dir.size()).replace(QLatin1Char('\\'), QLatin1Char('/'))
            : destination;

        for (int column = 0; column < 5; ++column) {
            table->setItem(index, column, new QTableWidgetItem());
        }

        auto *check = table->item(index, 0);
        check->setFlags(Qt::ItemIsEnabled | Qt::ItemIsUserCheckable | Qt::ItemIsSelectable);
        check->setCheckState(safe && source_exists ? Qt::Checked : Qt::Unchecked);
        check->setData(Qt::UserRole, hash);

        QStringList tags;

        for (const QJsonValue &tag : file.value(QStringLiteral("tags")).toArray()) {
            tags.append(tag.toString());
        }

        QString note = QStringLiteral("ready");

        if (!source_exists) {
            note = QStringLiteral("source missing");
        } else if (!safe) {
            note = QStringLiteral("unsafe filename");
        } else if (exists) {
            note = QStringLiteral("will be updated");
        }

        table->item(index, 1)->setText(hash);
        table->item(index, 2)->setText(tags.join(QStringLiteral(", ")));
        table->item(index, 3)->setText(relative);
        table->item(index, 3)->setToolTip(destination);
        table->item(index, 4)->setText(note);
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
        notify(QStringLiteral("export_folder_run"), active_page, json_payload(payload));
        dialog.accept();
    });

    dialog.exec();
}
