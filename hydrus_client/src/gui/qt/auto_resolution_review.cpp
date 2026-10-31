#include "auto_resolution.hpp"

// WS-15.1.3a: moved verbatim from bridge.cpp (move-only split).

#include "bridge.hpp"
#include "support/support.hpp"

#include <QApplication>
#include <QCheckBox>
#include <QClipboard>
#include <QComboBox>
#include <QDateTime>
#include <QDialog>
#include <QDialogButtonBox>
#include <QDoubleSpinBox>
#include <QFormLayout>
#include <QHBoxLayout>
#include <QHeaderView>
#include <QInputDialog>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QJsonValue>
#include <QLabel>
#include <QLineEdit>
#include <QListWidget>
#include <QMessageBox>
#include <QPlainTextEdit>
#include <QPushButton>
#include <QSpinBox>
#include <QTableWidget>
#include <QTabWidget>
#include <QTextEdit>
#include <QTimer>
#include <QVBoxLayout>
#include <algorithm>

HydrusDuplicateReviewDialog::HydrusDuplicateReviewDialog(HydrusMainWindow *main_window, const QJsonObject &state, QWidget *parent)
    : QDialog(parent), main(main_window) {
    setWindowTitle(QStringLiteral("duplicate auto-resolution review"));
    setObjectName(QStringLiteral("hydrus_duplicate_review_dialog"));
    setAttribute(Qt::WA_DeleteOnClose, true);
    resize(900, 520);

    auto *layout = new QVBoxLayout(this);

    status = new QLabel(this);
    status->setWordWrap(true);
    layout->addWidget(status);

    tabs = new QTabWidget(this);

    auto make_table = [this](const QStringList &headers) {
        auto *table = new QTableWidget(this);
        table->setColumnCount(headers.size());
        table->setHorizontalHeaderLabels(headers);
        table->setSelectionBehavior(QAbstractItemView::SelectRows);
        table->setSelectionMode(QAbstractItemView::ExtendedSelection);
        table->setEditTriggers(QAbstractItemView::NoEditTriggers);
        table->verticalHeader()->setVisible(false);
        table->horizontalHeader()->setStretchLastSection(true);
        table->setIconSize(QSize(110, 90));

        for (int column = 0; column < 2 && column < table->columnCount(); ++column) {
            table->horizontalHeader()->setSectionResizeMode(column, QHeaderView::Fixed);
            table->setColumnWidth(column, 116);
        }

        return table;
    };

    pending = make_table({
        QStringLiteral("A"),
        QStringLiteral("B"),
        QStringLiteral("distance"),
        QStringLiteral("rule"),
        QStringLiteral("decision")
    });
    denied = make_table({
        QStringLiteral("A"),
        QStringLiteral("B"),
        QStringLiteral("rule")
    });
    actioned = make_table({
        QStringLiteral("A"),
        QStringLiteral("B"),
        QStringLiteral("decision"),
        QStringLiteral("rule"),
        QStringLiteral("denied")
    });

    install_pair_list_actions(pending, main);
    install_pair_list_actions(denied, main);
    install_pair_list_actions(actioned, main);

    tabs->addTab(pending, QStringLiteral("pending"));
    tabs->addTab(denied, QStringLiteral("denied"));
    tabs->addTab(actioned, QStringLiteral("taken"));
    layout->addWidget(tabs, 1);

    auto *buttons = new QHBoxLayout();
    auto *approve_selected = new QPushButton(QStringLiteral("approve selected"), this);
    auto *deny_selected = new QPushButton(QStringLiteral("deny selected"), this);
    auto *approve_all = new QPushButton(QStringLiteral("approve all"), this);
    auto *deny_all = new QPushButton(QStringLiteral("deny all"), this);
    auto *close = new QPushButton(QStringLiteral("close"), this);
    buttons->addWidget(approve_selected);
    buttons->addWidget(deny_selected);
    buttons->addWidget(approve_all);
    buttons->addWidget(deny_all);
    buttons->addStretch(1);
    buttons->addWidget(close);
    layout->addLayout(buttons);

    QObject::connect(approve_selected, &QPushButton::clicked, this, [this]() { apply_selected(true); });
    QObject::connect(deny_selected, &QPushButton::clicked, this, [this]() { apply_selected(false); });
    QObject::connect(approve_all, &QPushButton::clicked, this, [this]() { apply_all(true); });
    QObject::connect(deny_all, &QPushButton::clicked, this, [this]() { apply_all(false); });
    QObject::connect(close, &QPushButton::clicked, this, &QDialog::close);

    set_state(state);
}

void HydrusDuplicateReviewDialog::set_state(const QJsonObject &state) {
    status->setText(state.value(QStringLiteral("status")).toString());

    fill_pair_table(
        pending,
        state.value(QStringLiteral("pending")).toArray(),
        2,
        {
            QStringLiteral("distance"),
            QStringLiteral("rule"),
            QStringLiteral("decision"),
        });
    fill_pair_table(
        denied,
        state.value(QStringLiteral("denied")).toArray(),
        2,
        {QStringLiteral("rule")});
    fill_pair_table(
        actioned,
        state.value(QStringLiteral("actioned")).toArray(),
        2,
        {
            QStringLiteral("decision"),
            QStringLiteral("rule"),
            QStringLiteral("banned"),
        });
}

QList<int> HydrusDuplicateReviewDialog::selected_pending_rows() const {
    QList<int> rows;

    for (const QModelIndex &index : pending->selectionModel()->selectedRows()) {
        if (!rows.contains(index.row())) {
            rows.append(index.row());
        }
    }

    std::sort(rows.begin(), rows.end());

    return rows;
}

void HydrusDuplicateReviewDialog::apply_selected(bool approve) {
    const QList<int> rows = selected_pending_rows();

    if (rows.isEmpty()) {
        QMessageBox::information(this, QStringLiteral("duplicate review"),
                                 QStringLiteral("Select at least one pending pair first."));
        return;
    }

    QJsonArray indices;
    for (int row : rows) {
        indices.append(row);
    }

    QJsonObject payload;
    payload.insert(QStringLiteral("indices"), indices);
    payload.insert(QStringLiteral("approve"), approve);
    main->notify(QStringLiteral("duplicates_review_apply"), QString(), json_payload(payload));
}

void HydrusDuplicateReviewDialog::apply_all(bool approve) {
    QJsonObject payload;
    payload.insert(QStringLiteral("all"), true);
    payload.insert(QStringLiteral("approve"), approve);
    main->notify(QStringLiteral("duplicates_review_apply"), QString(), json_payload(payload));
}
