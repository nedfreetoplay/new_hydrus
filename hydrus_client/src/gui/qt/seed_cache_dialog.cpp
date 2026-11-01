
// WS-15.1.3c1: file seed cache dialog moved verbatim from bridge.cpp (move-only split).

#include <hydrus_client/src/gui/qt/api.cxx.h>
#include "support/support.hpp"

#include <QApplication>
#include <QClipboard>
#include <QComboBox>
#include <QDateTime>
#include <QHBoxLayout>
#include <QHeaderView>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QJsonValue>
#include <QLabel>
#include <QLineEdit>
#include <QMessageBox>
#include <QPushButton>
#include <QTableWidget>
#include <QVBoxLayout>

HydrusSeedCacheDialog::HydrusSeedCacheDialog(
    HydrusMainWindow *main_window, const SeedCacheState &state, bool gallery_mode, QWidget *parent)
    : QDialog(parent), main(main_window), gallery(gallery_mode) {
    setWindowTitle(gallery ? QStringLiteral("gallery seed log") : QStringLiteral("file seed cache"));
    setObjectName(gallery ? QStringLiteral("hydrus_gallery_seed_log")
                          : QStringLiteral("hydrus_file_seed_cache"));
    setAttribute(Qt::WA_DeleteOnClose, true);
    resize(940, 560);

    auto *layout = new QVBoxLayout(this);

    subtitle = new QLabel(this);
    subtitle->setWordWrap(true);
    layout->addWidget(subtitle);

    auto *controls = new QHBoxLayout();
    filter = new QComboBox(this);
    filter->setObjectName(QStringLiteral("seed_cache_filter"));
    search = new QLineEdit(this);
    search->setPlaceholderText(QStringLiteral("filter by url or note"));
    search->setClearButtonEnabled(true);
    controls->addWidget(new QLabel(QStringLiteral("show:"), this));
    controls->addWidget(filter);
    controls->addWidget(search, 1);
    layout->addLayout(controls);

    table = new QTableWidget(this);
    table->setObjectName(QStringLiteral("seed_cache_table"));
    table->setColumnCount(static_cast<int>(state.headers.size()));
    {
        QStringList header_labels;
        for (const rust::String &header : state.headers) {
            header_labels << to_qstring(header);
        }
        table->setHorizontalHeaderLabels(header_labels);
    }
    table->horizontalHeader()->setSectionResizeMode(0, QHeaderView::ResizeToContents);
    table->horizontalHeader()->setSectionResizeMode(1, QHeaderView::Stretch);
    table->horizontalHeader()->setSectionResizeMode(2, QHeaderView::ResizeToContents);
    table->horizontalHeader()->setSectionResizeMode(3, QHeaderView::ResizeToContents);
    table->horizontalHeader()->setSectionResizeMode(4, QHeaderView::Stretch);
    table->verticalHeader()->setVisible(false);
    table->setSelectionBehavior(QAbstractItemView::SelectRows);
    table->setSelectionMode(QAbstractItemView::ExtendedSelection);
    table->setEditTriggers(QAbstractItemView::NoEditTriggers);
    table->setSortingEnabled(true);
    layout->addWidget(table, 1);

    auto *buttons = new QHBoxLayout();
    retry_button = new QPushButton(QStringLiteral("retry failed"), this);
    auto *remove_button = new QPushButton(QStringLiteral("remove"), this);
    auto *clear_finished = new QPushButton(QStringLiteral("clear finished"), this);
    auto *clear_all = new QPushButton(QStringLiteral("clear all"), this);
    auto *copy = new QPushButton(QStringLiteral("copy urls"), this);
    auto *close_button = new QPushButton(QStringLiteral("close"), this);

    buttons->addWidget(retry_button);
    buttons->addWidget(remove_button);
    buttons->addWidget(clear_finished);
    buttons->addWidget(clear_all);
    buttons->addWidget(copy);
    buttons->addStretch(1);
    buttons->addWidget(close_button);
    layout->addLayout(buttons);

    connect(filter, QOverload<int>::of(&QComboBox::currentIndexChanged), this, [this](int) { rebuild(); });
    connect(search, &QLineEdit::textChanged, this, [this](const QString &) { rebuild(); });
    connect(retry_button, &QPushButton::clicked, this, [this]() { send_action(QStringLiteral("retry")); });
    connect(remove_button, &QPushButton::clicked, this, [this]() {
        if (selected_urls().isEmpty()) {
            QMessageBox::information(this, windowTitle(), QStringLiteral("Select one or more seeds first."));
            return;
        }

        send_action(QStringLiteral("remove"));
    });
    connect(clear_finished, &QPushButton::clicked, this, [this]() {
        send_action(QStringLiteral("clear_finished"));
    });
    connect(clear_all, &QPushButton::clicked, this, [this]() {
        if (QMessageBox::question(this, windowTitle(), QStringLiteral("Remove every seed in this log?"))
            == QMessageBox::Yes) {
            send_action(QStringLiteral("clear"));
        }
    });
    connect(copy, &QPushButton::clicked, this, [this]() {
        QStringList urls = selected_urls();

        if (urls.isEmpty()) {
            for (const SeedRowData &row : rows) {
                urls << row.url;
            }
        }

        QApplication::clipboard()->setText(urls.join(QStringLiteral("\n")));
    });
    connect(close_button, &QPushButton::clicked, this, &QDialog::close);

    set_state(state);
}

void HydrusSeedCacheDialog::set_state(const SeedCacheState &state) {
    page_id = to_qstring(state.page);
    subtitle_text = to_qstring(state.subtitle);
    retry_label_text = to_qstring(state.retry_label);
    failed_count = static_cast<int>(state.failed);
    statuses.clear();
    rows.clear();

    for (const SeedFilter &entry : state.status_filters) {
        statuses.append({to_qstring(entry.key), to_qstring(entry.label)});
    }

    for (const SeedRow &seed : state.seeds) {
        rows.append({to_qstring(seed.status),
                     to_qstring(seed.color),
                     to_qstring(seed.url),
                     to_qstring(seed.note),
                     seed.created,
                     seed.modified});
    }

    const QString current = filter->currentData().toString();

    QSignalBlocker blocker(filter);
    filter->clear();

    for (const auto &entry : statuses) {
        filter->addItem(entry.second, entry.first);
    }

    const int index = filter->findData(current);
    filter->setCurrentIndex(index >= 0 ? index : 0);
    blocker.unblock();

    rebuild();
}

QStringList HydrusSeedCacheDialog::selected_urls() const {
    QStringList urls;

    for (const QTableWidgetItem *item : table->selectedItems()) {
        if (item->column() != 1) {
            continue;
        }

        const QString url = item->data(Qt::UserRole).toString();

        if (!url.isEmpty() && !urls.contains(url)) {
            urls << url;
        }
    }

    return urls;
}

void HydrusSeedCacheDialog::rebuild() {
    const QString status_filter = filter->currentData().toString();
    const QString needle = search->text().trimmed();

    table->setSortingEnabled(false);
    table->setRowCount(0);

    for (const SeedRowData &seed : rows) {
        if (status_filter != QStringLiteral("all") && seed.status != status_filter) {
            continue;
        }

        if (!needle.isEmpty()
            && !seed.url.contains(needle, Qt::CaseInsensitive)
            && !seed.note.contains(needle, Qt::CaseInsensitive)) {
            continue;
        }

        const int row = table->rowCount();
        table->insertRow(row);

        auto *status_item = new QTableWidgetItem(seed.status);

        if (!seed.color.isEmpty()) {
            status_item->setForeground(QColor(seed.color));
        }

        table->setItem(row, 0, status_item);

        auto *url_item = new QTableWidgetItem(seed.url);
        url_item->setData(Qt::UserRole, seed.url);
        url_item->setToolTip(seed.url);
        table->setItem(row, 1, url_item);

        auto timestamp = [](qint64 seconds) {
            if (seconds <= 0) {
                return QString();
            }

            return QDateTime::fromSecsSinceEpoch(seconds).toString(QStringLiteral("yyyy-MM-dd HH:mm:ss"));
        };

        table->setItem(row, 2, new QTableWidgetItem(timestamp(seed.created)));
        table->setItem(row, 3, new QTableWidgetItem(timestamp(seed.modified)));
        table->setItem(row, 4, new QTableWidgetItem(seed.note));
    }

    table->setSortingEnabled(true);
    subtitle->setText(subtitle_text);

    const int failed = failed_count;
    retry_button->setText(retry_label_text);
    retry_button->setEnabled(failed > 0 || !selected_urls().isEmpty());
}

void HydrusSeedCacheDialog::send_action(const QString &action) {
    const QByteArray page_bytes = page_id.toUtf8();
    const QByteArray action_bytes = action.toUtf8();

    if (gallery) {
        app_gallery_seed_action(
            *main,
            rust::Str(page_bytes.constData(), page_bytes.size()),
            rust::Str(action_bytes.constData(), action_bytes.size()),
            to_rust_vec(selected_urls()));
    } else {
        app_file_seed_action(
            *main,
            rust::Str(page_bytes.constData(), page_bytes.size()),
            rust::Str(action_bytes.constData(), action_bytes.size()),
            to_rust_vec(selected_urls()));
    }
}

