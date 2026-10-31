// WS-15.1.3c2: notes editor dialog moved verbatim from bridge.cpp (move-only split).

#include "bridge.hpp"
#include "support/support.hpp"

#include <QDialog>
#include <QDialogButtonBox>
#include <QHBoxLayout>
#include <QHeaderView>
#include <QJsonArray>
#include <QLabel>
#include <QPushButton>
#include <QTableWidget>
#include <QVBoxLayout>

void HydrusMainWindow::show_notes(const QJsonObject &state) {
    const QString hash = state.value(QStringLiteral("hash")).toString();

    QDialog dialog(this);
    dialog.setWindowTitle(QStringLiteral("notes"));
    dialog.resize(620, 460);

    auto *layout = new QVBoxLayout(&dialog);
    auto *info = new QLabel(hash.left(16) + QStringLiteral("..."), &dialog);
    info->setTextInteractionFlags(Qt::TextSelectableByMouse);
    layout->addWidget(info);

    auto *table = new QTableWidget(&dialog);
    table->setColumnCount(2);
    table->setHorizontalHeaderLabels({QStringLiteral("name"), QStringLiteral("note")});
    table->horizontalHeader()->setStretchLastSection(true);
    table->horizontalHeader()->setSectionResizeMode(0, QHeaderView::ResizeToContents);
    table->verticalHeader()->setVisible(false);

    const QJsonArray notes = state.value(QStringLiteral("notes")).toArray();
    table->setRowCount(notes.size());

    for (int index = 0; index < notes.size(); ++index) {
        const QJsonObject note = notes.at(index).toObject();
        table->setItem(index, 0, new QTableWidgetItem(note.value(QStringLiteral("name")).toString()));
        table->setItem(index, 1, new QTableWidgetItem(note.value(QStringLiteral("text")).toString()));
    }

    table->resizeRowsToContents();
    layout->addWidget(table, 1);

    auto *buttons = new QHBoxLayout();
    auto *add = new QPushButton(QStringLiteral("add note"), &dialog);
    auto *remove = new QPushButton(QStringLiteral("remove"), &dialog);
    buttons->addWidget(add);
    buttons->addWidget(remove);
    buttons->addStretch(1);
    auto *box = new QDialogButtonBox(QDialogButtonBox::Save | QDialogButtonBox::Cancel, &dialog);
    buttons->addWidget(box);
    layout->addLayout(buttons);

    connect(add, &QPushButton::clicked, &dialog, [table]() {
        const int row = table->rowCount();
        table->insertRow(row);
        table->setItem(row, 0, new QTableWidgetItem());
        table->setItem(row, 1, new QTableWidgetItem());
        table->editItem(table->item(row, 0));
    });
    connect(remove, &QPushButton::clicked, &dialog, [table]() {
        const int row = table->currentRow();

        if (row >= 0) {
            table->removeRow(row);
        }
    });
    connect(box, &QDialogButtonBox::accepted, &dialog, &QDialog::accept);
    connect(box, &QDialogButtonBox::rejected, &dialog, &QDialog::reject);

    if (dialog.exec() != QDialog::Accepted) {
        return;
    }

    QJsonArray values;

    for (int row = 0; row < table->rowCount(); ++row) {
        const QString name = table->item(row, 0) == nullptr
            ? QString()
            : table->item(row, 0)->text().trimmed();
        const QString text = table->item(row, 1) == nullptr
            ? QString()
            : table->item(row, 1)->text();

        if (name.isEmpty() && text.trimmed().isEmpty()) {
            continue;
        }

        QJsonObject note;
        note.insert(QStringLiteral("name"), name);
        note.insert(QStringLiteral("text"), text);
        values.append(note);
    }

    QJsonObject payload;
    payload.insert(QStringLiteral("hash"), hash);
    payload.insert(QStringLiteral("notes"), values);

    notify(QStringLiteral("notes_save"), QString(), json_payload(payload));
}
