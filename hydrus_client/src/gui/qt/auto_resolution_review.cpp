
// WS-15.2.4b: typed DuplicateReviewDialogState (no JSON parsing).

#include <hydrus_client/src/gui/qt/api.cxx.h>
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

namespace {

QJsonArray review_rows(const rust::Vec<ReviewPairRow> &rows, bool with_distance, bool with_rule, bool with_decision, bool with_banned) {
    QJsonArray array;
    for (const ReviewPairRow &row : rows) {
        QJsonObject entry;
        entry.insert(QStringLiteral("a"), to_qstring(row.hash_a));
        entry.insert(QStringLiteral("b"), to_qstring(row.hash_b));
        entry.insert(QStringLiteral("a_thumb"), to_qstring(row.thumb_a));
        entry.insert(QStringLiteral("b_thumb"), to_qstring(row.thumb_b));
        if (with_distance) {
            entry.insert(QStringLiteral("distance"), static_cast<qint64>(row.distance));
        }
        if (with_rule) {
            entry.insert(QStringLiteral("rule"), to_qstring(row.rule));
        }
        if (with_decision) {
            entry.insert(QStringLiteral("decision"), to_qstring(row.decision));
            entry.insert(QStringLiteral("decision_label"), to_qstring(row.decision_label));
        }
        if (with_banned) {
            entry.insert(QStringLiteral("banned"), row.banned);
        }
        array.append(entry);
    }
    return array;
}

}  // namespace

HydrusDuplicateReviewDialog::HydrusDuplicateReviewDialog(HydrusMainWindow *main_window, const DuplicateReviewDialogState &state, QWidget *parent)
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
            table->setColumnWidth(column, static_cast<int>(hydrus_metric_review_width()));
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

void HydrusDuplicateReviewDialog::set_state(const DuplicateReviewDialogState &state) {
    status->setText(to_qstring(state.status));

    fill_pair_table(
        pending,
        review_rows(state.pending, true, true, true, false),
        2,
        {
            QStringLiteral("distance"),
            QStringLiteral("rule"),
            QStringLiteral("decision"),
        });
    fill_pair_table(
        denied,
        review_rows(state.denied, false, true, false, false),
        2,
        {QStringLiteral("rule")});
    fill_pair_table(
        actioned,
        review_rows(state.actioned, false, true, true, true),
        2,
        {
            QStringLiteral("decision"),
            QStringLiteral("rule"),
            QStringLiteral("banned"),
        });
}

QList<int> HydrusDuplicateReviewDialog::selected_pending_rows() const {
    rust::Vec<int64_t> input;
    input.reserve(pending->selectionModel()->selectedRows().size());

    for (const QModelIndex &index : pending->selectionModel()->selectedRows()) {
        input.push_back(index.row());
    }

    // WS-15.3.2d: order is the Rust single source of truth (sort + dedup).
    const rust::Vec<int64_t> ordered = hydrus_sort_selected_rows(std::move(input));
    QList<int> rows;
    rows.reserve(int(ordered.size()));

    for (int64_t row : ordered) {
        rows.append(int(row));
    }

    return rows;
}

void HydrusDuplicateReviewDialog::apply_selected(bool approve) {
    const QList<int> rows = selected_pending_rows();

    if (rows.isEmpty()) {
        QMessageBox::information(this, QStringLiteral("duplicate review"),
                                 QStringLiteral("Select at least one pending pair first."));
        return;
    }

    rust::Vec<int64_t> indices;
    indices.reserve(rows.size());
    for (int row : rows) {
        indices.push_back(row);
    }

    app_duplicates_review_apply(*main, std::move(indices), approve, false);
}

void HydrusDuplicateReviewDialog::apply_all(bool approve) {
    app_duplicates_review_apply(*main, rust::Vec<int64_t>(), approve, true);
}
