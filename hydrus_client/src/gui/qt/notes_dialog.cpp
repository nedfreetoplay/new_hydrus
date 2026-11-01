// WS-15.1.3c2: notes editor dialog moved verbatim from bridge.cpp (move-only split).

#include <hydrus_client/src/gui/qt/api.cxx.h>
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

void HydrusMainWindow::show_notes(const NotesDialogState &state) {
    const QString hash = to_qstring(state.hash);

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

    const rust::Vec<NoteState> &notes = state.notes;
    table->setRowCount(int(notes.size()));

    for (size_t index = 0; index < notes.size(); ++index) {
        const NoteState &note = notes[index];
        table->setItem(int(index), 0, new QTableWidgetItem(to_qstring(note.name)));
        table->setItem(int(index), 1, new QTableWidgetItem(to_qstring(note.text)));
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

    rust::Vec<rust::String> names;
    rust::Vec<rust::String> texts;

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

        const QByteArray name_bytes = name.toUtf8();
        const QByteArray text_bytes = text.toUtf8();
        names.push_back(rust::String(name_bytes.constData(), static_cast<size_t>(name_bytes.size())));
        texts.push_back(rust::String(text_bytes.constData(), static_cast<size_t>(text_bytes.size())));
    }

    const QByteArray hash_bytes = hash.toUtf8();
    app_notes_save(
        *this,
        rust::Str(hash_bytes.constData(), static_cast<size_t>(hash_bytes.size())),
        names,
        texts);
}
