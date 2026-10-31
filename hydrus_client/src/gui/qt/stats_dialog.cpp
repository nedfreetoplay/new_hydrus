// WS-15.1.3c2: file viewing statistics dialog moved verbatim from bridge.cpp (move-only split).

#include "bridge.hpp"
#include "support/support.hpp"

#include <QDialog>
#include <QDialogButtonBox>
#include <QHeaderView>
#include <QJsonArray>
#include <QLabel>
#include <QTableWidget>
#include <QVBoxLayout>

void HydrusMainWindow::show_viewing_stats(const QJsonObject &state) {
    QDialog dialog(this);
    dialog.setWindowTitle(QStringLiteral("file viewing statistics"));
    dialog.resize(560, 260);

    auto *layout = new QVBoxLayout(&dialog);
    auto *hash = new QLabel(
        QStringLiteral("file: %1").arg(state.value(QStringLiteral("hash")).toString()),
        &dialog);
    hash->setTextInteractionFlags(Qt::TextSelectableByMouse);
    layout->addWidget(hash);

    auto *table = new QTableWidget(&dialog);
    table->setColumnCount(4);
    table->setHorizontalHeaderLabels({
        QStringLiteral("canvas"),
        QStringLiteral("views"),
        QStringLiteral("viewtime (s)"),
        QStringLiteral("last viewed"),
    });
    table->verticalHeader()->setVisible(false);
    table->setEditTriggers(QAbstractItemView::NoEditTriggers);
    table->horizontalHeader()->setSectionResizeMode(QHeaderView::Stretch);

    const QJsonArray rows = state.value(QStringLiteral("rows")).toArray();
    table->setRowCount(rows.size());

    for (int row = 0; row < rows.size(); ++row) {
        const QJsonObject entry = rows.at(row).toObject();
        table->setItem(row, 0, new QTableWidgetItem(entry.value(QStringLiteral("canvas")).toString()));
        table->setItem(row, 1, new QTableWidgetItem(QString::number(entry.value(QStringLiteral("views")).toInt())));
        table->setItem(row, 2, new QTableWidgetItem(QString::number(entry.value(QStringLiteral("viewtime")).toDouble())));
        table->setItem(row, 3, new QTableWidgetItem(entry.value(QStringLiteral("last_viewed")).toString()));
    }

    layout->addWidget(table, 1);

    auto *buttons = new QDialogButtonBox(QDialogButtonBox::Close, &dialog);
    connect(buttons, &QDialogButtonBox::rejected, &dialog, &QDialog::accept);
    layout->addWidget(buttons);

    dialog.exec();
}
