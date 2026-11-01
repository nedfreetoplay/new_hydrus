
// WS-15.2.4b: typed popup messages dialog (no QJsonObject).

#include <hydrus_client/src/gui/qt/api.cxx.h>
#include "support/support.hpp"

#include <QDialogButtonBox>
#include <QHeaderView>
#include <QLabel>
#include <QPushButton>
#include <QTableWidget>
#include <QVBoxLayout>

HydrusNotificationsDialog::HydrusNotificationsDialog(HydrusMainWindow *main_window, const NotificationsDialogState &state, QWidget *parent)
    : QDialog(parent), main(main_window) {
    setWindowTitle(QStringLiteral("popup messages"));
    resize(720, 460);

    auto *layout = new QVBoxLayout(this);

    summary = new QLabel(to_qstring(state.summary), this);
    layout->addWidget(summary);

    table = new QTableWidget(this);
    table->setColumnCount(3);
    table->setHorizontalHeaderLabels({
        QStringLiteral("time"),
        QStringLiteral("level"),
        QStringLiteral("message"),
    });
    table->verticalHeader()->setVisible(false);
    table->setEditTriggers(QAbstractItemView::NoEditTriggers);
    table->setSelectionBehavior(QAbstractItemView::SelectRows);
    table->horizontalHeader()->setSectionResizeMode(0, QHeaderView::ResizeToContents);
    table->horizontalHeader()->setSectionResizeMode(1, QHeaderView::ResizeToContents);
    table->horizontalHeader()->setSectionResizeMode(2, QHeaderView::Stretch);
    layout->addWidget(table, 1);

    rebuild(state);

    auto *buttons = new QDialogButtonBox(QDialogButtonBox::Close, this);
    auto *clear = new QPushButton(QStringLiteral("clear"), this);
    buttons->addButton(clear, QDialogButtonBox::ActionRole);
    connect(clear, &QPushButton::clicked, this, [this]() {
        table->setRowCount(0);
        summary->setText(QStringLiteral("0 message(s)"));
        main->notify(UiAction::NotificationsClear);
    });
    connect(buttons, &QDialogButtonBox::rejected, this, &QDialog::accept);
    layout->addWidget(buttons);
}

void HydrusNotificationsDialog::rebuild(const NotificationsDialogState &state) {
    const size_t count = state.entries.size();
    table->setRowCount(static_cast<int>(count));

    for (size_t row = 0; row < count; ++row) {
        const NotificationRow &entry = state.entries[row];
        table->setItem(static_cast<int>(row), 0, new QTableWidgetItem(to_qstring(entry.time)));
        table->setItem(static_cast<int>(row), 1, new QTableWidgetItem(to_qstring(entry.level)));
        table->setItem(static_cast<int>(row), 2, new QTableWidgetItem(to_qstring(entry.text)));
    }
}

void HydrusMainWindow::show_notifications(const NotificationsDialogState &state) {
    HydrusNotificationsDialog dialog(this, state, this);
    dialog.exec();
}
