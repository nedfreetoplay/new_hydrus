#include "notifications_dialog.hpp"

// WS-15.1.3c2: popup messages dialog moved verbatim from bridge.cpp (move-only split).

#include "bridge.hpp"
#include "support/support.hpp"

#include <QDialogButtonBox>
#include <QHeaderView>
#include <QLabel>
#include <QPushButton>
#include <QTableWidget>
#include <QVBoxLayout>

HydrusNotificationsDialog::HydrusNotificationsDialog(HydrusMainWindow *main_window, const QJsonObject &state, QWidget *parent)
    : QDialog(parent), main(main_window) {
    setWindowTitle(QStringLiteral("popup messages"));
    resize(720, 460);

    auto *layout = new QVBoxLayout(this);

    summary = new QLabel(
        QStringLiteral("%1 message(s)").arg(state.value(QStringLiteral("count")).toInt()),
        this);
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

    rebuild(state.value(QStringLiteral("entries")).toArray());

    auto *buttons = new QDialogButtonBox(QDialogButtonBox::Close, this);
    auto *clear = new QPushButton(QStringLiteral("clear"), this);
    buttons->addButton(clear, QDialogButtonBox::ActionRole);
    connect(clear, &QPushButton::clicked, this, [this]() {
        table->setRowCount(0);
        summary->setText(QStringLiteral("0 message(s)"));
        main->notify(QStringLiteral("notifications_clear"));
    });
    connect(buttons, &QDialogButtonBox::rejected, this, &QDialog::accept);
    layout->addWidget(buttons);
}

void HydrusNotificationsDialog::rebuild(const QJsonArray &entries) {
    table->setRowCount(entries.size());

    for (int row = 0; row < entries.size(); ++row) {
        const QJsonObject entry = entries.at(row).toObject();

        table->setItem(row, 0, new QTableWidgetItem(entry.value(QStringLiteral("time")).toString()));
        table->setItem(row, 1, new QTableWidgetItem(entry.value(QStringLiteral("level")).toString()));
        table->setItem(row, 2, new QTableWidgetItem(entry.value(QStringLiteral("text")).toString()));
    }
}

void HydrusMainWindow::show_notifications(const QJsonObject &state) {
    HydrusNotificationsDialog dialog(this, state, this);
    dialog.exec();
}
