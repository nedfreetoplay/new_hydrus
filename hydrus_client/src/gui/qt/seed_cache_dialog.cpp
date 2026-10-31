#include "seed_cache_dialog.hpp"

// WS-15.1.3c1: file seed cache dialog moved verbatim from bridge.cpp (move-only split).

#include "bridge.hpp"
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
    HydrusMainWindow *main_window, const QJsonObject &state, bool gallery_mode, QWidget *parent)
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
    table->setColumnCount(5);
    table->setHorizontalHeaderLabels({QStringLiteral("status"), QStringLiteral("seed"),
                                      QStringLiteral("created"), QStringLiteral("modified"),
                                      QStringLiteral("note")});
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
            for (const QJsonValue &value : data.value(QStringLiteral("seeds")).toArray()) {
                urls << value.toObject().value(QStringLiteral("url")).toString();
            }
        }

        QApplication::clipboard()->setText(urls.join(QStringLiteral("\n")));
    });
    connect(close_button, &QPushButton::clicked, this, &QDialog::close);

    set_state(state);
}

void HydrusSeedCacheDialog::set_state(const QJsonObject &state) {
    data = state;

    const QString current = filter->currentData().toString();
    const QJsonArray statuses = state.value(QStringLiteral("status_filters")).toArray();

    QSignalBlocker blocker(filter);
    filter->clear();

    for (const QJsonValue &value : statuses) {
        const QJsonObject entry = value.toObject();
        filter->addItem(entry.value(QStringLiteral("label")).toString(),
                        entry.value(QStringLiteral("key")).toString());
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

    const QJsonArray seeds = data.value(QStringLiteral("seeds")).toArray();

    for (const QJsonValue &value : seeds) {
        const QJsonObject seed = value.toObject();
        const QString status = seed.value(QStringLiteral("status")).toString();
        const QString url = seed.value(QStringLiteral("url")).toString();
        const QString note = seed.value(QStringLiteral("note")).toString();

        if (status_filter != QStringLiteral("all") && status != status_filter) {
            continue;
        }

        if (!needle.isEmpty()
            && !url.contains(needle, Qt::CaseInsensitive)
            && !note.contains(needle, Qt::CaseInsensitive)) {
            continue;
        }

        const int row = table->rowCount();
        table->insertRow(row);

        auto *status_item = new QTableWidgetItem(status);

        if (status == QStringLiteral("error") || status == QStringLiteral("failed")) {
            status_item->setForeground(QColor(220, 110, 110));
        } else if (status == QStringLiteral("done") || status == QStringLiteral("success")) {
            status_item->setForeground(QColor(120, 190, 120));
        } else if (status == QStringLiteral("working")) {
            status_item->setForeground(QColor(220, 200, 120));
        }

        table->setItem(row, 0, status_item);

        auto *url_item = new QTableWidgetItem(url);
        url_item->setData(Qt::UserRole, url);
        url_item->setToolTip(url);
        table->setItem(row, 1, url_item);

        auto timestamp = [](const QJsonValue &value) {
            const qint64 seconds = value.toVariant().toLongLong();

            if (seconds <= 0) {
                return QString();
            }

            return QDateTime::fromSecsSinceEpoch(seconds).toString(QStringLiteral("yyyy-MM-dd HH:mm:ss"));
        };

        table->setItem(row, 2, new QTableWidgetItem(timestamp(seed.value(QStringLiteral("created")))));
        table->setItem(row, 3, new QTableWidgetItem(timestamp(seed.value(QStringLiteral("modified")))));
        table->setItem(row, 4, new QTableWidgetItem(note));
    }

    table->setSortingEnabled(true);
    subtitle->setText(data.value(QStringLiteral("subtitle")).toString());

    const QJsonObject counts = data.value(QStringLiteral("counts")).toObject();
    const int failed = counts.value(QStringLiteral("failed")).toInt();
    retry_button->setText(failed > 0 ? QStringLiteral("retry failed (%1)").arg(failed)
                                     : QStringLiteral("retry failed"));
    retry_button->setEnabled(failed > 0 || !selected_urls().isEmpty());
}

void HydrusSeedCacheDialog::send_action(const QString &action) {
    QJsonObject payload;
    payload.insert(QStringLiteral("page"), data.value(QStringLiteral("page")));
    payload.insert(QStringLiteral("action"), action);

    QJsonArray urls;
    for (const QString &url : selected_urls()) {
        urls.append(url);
    }
    payload.insert(QStringLiteral("urls"), urls);

    main->notify(gallery ? QStringLiteral("gallery_seed_log_action")
                         : QStringLiteral("file_seed_cache_action"),
                 QString(),
                 json_payload(payload));
}

void HydrusMainWindow::show_file_seed_cache(const QJsonObject &state) {
    if (file_seed_cache_dialog == nullptr) {
        file_seed_cache_dialog = new HydrusSeedCacheDialog(this, state, false, this);
        connect(file_seed_cache_dialog, &QObject::destroyed, this, [this]() { file_seed_cache_dialog = nullptr; });
    } else {
        file_seed_cache_dialog->set_state(state);
        file_seed_cache_dialog->raise();
        file_seed_cache_dialog->activateWindow();
    }

    file_seed_cache_dialog->show();
}

void HydrusMainWindow::show_gallery_seed_log(const QJsonObject &state) {
    if (gallery_seed_log_dialog == nullptr) {
        gallery_seed_log_dialog = new HydrusSeedCacheDialog(this, state, true, this);
        connect(gallery_seed_log_dialog, &QObject::destroyed, this, [this]() { gallery_seed_log_dialog = nullptr; });
    } else {
        gallery_seed_log_dialog->set_state(state);
        gallery_seed_log_dialog->raise();
        gallery_seed_log_dialog->activateWindow();
    }

    gallery_seed_log_dialog->show();
}
