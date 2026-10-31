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

HydrusDuplicateMergeOptionsDialog::HydrusDuplicateMergeOptionsDialog(
    HydrusMainWindow *main_window, const QJsonObject &state, QWidget *parent)
    : QDialog(parent), main(main_window) {
    setWindowTitle(QStringLiteral("duplicate merge options"));
    setObjectName(QStringLiteral("hydrus_duplicate_merge_options_dialog"));
    setAttribute(Qt::WA_DeleteOnClose, true);
    resize(760, 620);

    data = state;

    auto *layout = new QVBoxLayout(this);

    auto *help = new QLabel(
        QStringLiteral("Choose what a duplicate merge copies from the worse file. Services that are not "
                       "listed are left untouched. \"This is better\" keeps one file and can move or copy "
                       "metadata; \"same quality\" merges both files in the chosen direction."),
        this);
    help->setWordWrap(true);
    layout->addWidget(help);

    auto *tabs = new QTabWidget(this);
    build_duplicate_merge_page(this, tabs, data, QStringLiteral("this is better"), better);
    build_duplicate_merge_page(this, tabs, data, QStringLiteral("same quality"), same);
    layout->addWidget(tabs, 1);

    auto *buttons = new QHBoxLayout();
    auto *save_button = new QPushButton(QStringLiteral("save"), this);
    auto *close_button = new QPushButton(QStringLiteral("close"), this);
    buttons->addStretch(1);
    buttons->addWidget(save_button);
    buttons->addWidget(close_button);
    layout->addLayout(buttons);

    connect(save_button, &QPushButton::clicked, this, [this]() { save(); });
    connect(close_button, &QPushButton::clicked, this, &QDialog::close);

    set_state(state);

    if (qEnvironmentVariable("HYDRUS_QT_DUPLICATES_MERGE_TAB") == QStringLiteral("same")) {
        tabs->setCurrentIndex(1);
    }
}

void HydrusDuplicateMergeOptionsDialog::set_state(const QJsonObject &state) {
    data = state;

    fill_duplicate_merge_page(better, data, data.value(QStringLiteral("better")).toObject());
    fill_duplicate_merge_page(same, data, data.value(QStringLiteral("same")).toObject());
}

void build_duplicate_merge_page(
    QWidget *parent,
    QTabWidget *tabs,
    const QJsonObject &data,
    const QString &title,
    DuplicateMergePage &page) {
    auto *widget = new QWidget(parent);
    auto *layout = new QVBoxLayout(widget);

    auto *form = new QFormLayout();

    page.archive = new QComboBox(widget);
    page.modified_date = new QComboBox(widget);
    page.urls = new QComboBox(widget);
    page.notes = new QComboBox(widget);

    for (const QJsonValue &action : data.value(QStringLiteral("archive_actions")).toArray()) {
        const QJsonObject entry = action.toObject();
        page.archive->addItem(entry.value(QStringLiteral("label")).toString(), entry.value(QStringLiteral("value")).toString());
    }

    for (QComboBox *combo : {page.modified_date, page.urls, page.notes}) {
        for (const QJsonValue &action : data.value(QStringLiteral("actions")).toArray()) {
            const QJsonObject entry = action.toObject();
            const QString value = entry.value(QStringLiteral("value")).toString();

            if (combo == page.modified_date && (value == QStringLiteral("move") || value == QStringLiteral("add_move"))) {
                continue;
            }

            combo->addItem(entry.value(QStringLiteral("label")).toString(), value);
        }
    }

    form->addRow(QStringLiteral("sync archived status:"), page.archive);
    form->addRow(QStringLiteral("sync file modified time:"), page.modified_date);
    form->addRow(QStringLiteral("sync known urls:"), page.urls);
    form->addRow(QStringLiteral("sync notes:"), page.notes);
    layout->addLayout(form);

    page.tags = new QTableWidget(widget);
    page.tags->setColumnCount(2);
    page.tags->setHorizontalHeaderLabels({QStringLiteral("tag service"), QStringLiteral("action")});
    page.tags->horizontalHeader()->setSectionResizeMode(0, QHeaderView::Stretch);
    page.tags->horizontalHeader()->setSectionResizeMode(1, QHeaderView::ResizeToContents);
    page.tags->verticalHeader()->setVisible(false);
    page.tags->setEditTriggers(QAbstractItemView::NoEditTriggers);
    page.tags->setSelectionMode(QAbstractItemView::NoSelection);

    page.ratings = new QTableWidget(widget);
    page.ratings->setColumnCount(2);
    page.ratings->setHorizontalHeaderLabels({QStringLiteral("rating service"), QStringLiteral("action")});
    page.ratings->horizontalHeader()->setSectionResizeMode(0, QHeaderView::Stretch);
    page.ratings->horizontalHeader()->setSectionResizeMode(1, QHeaderView::ResizeToContents);
    page.ratings->verticalHeader()->setVisible(false);
    page.ratings->setEditTriggers(QAbstractItemView::NoEditTriggers);
    page.ratings->setSelectionMode(QAbstractItemView::NoSelection);

    layout->addWidget(new QLabel(QStringLiteral("tag services"), widget));
    layout->addWidget(page.tags, 1);
    layout->addWidget(new QLabel(QStringLiteral("rating services"), widget));
    layout->addWidget(page.ratings, 1);

    tabs->addTab(widget, title);
}

void fill_duplicate_merge_page(
    DuplicateMergePage &page,
    const QJsonObject &data,
    const QJsonObject &options) {
    auto select_value = [](QComboBox *combo, const QString &value) {
        const int index = combo->findData(value);

        if (index >= 0) {
            combo->setCurrentIndex(index);
        }
    };

    select_value(page.archive, options.value(QStringLiteral("sync_archive_action")).toString());
    select_value(page.modified_date, options.value(QStringLiteral("sync_file_modified_date_action")).toString());
    select_value(page.urls, options.value(QStringLiteral("sync_urls_action")).toString());
    select_value(page.notes, options.value(QStringLiteral("sync_notes_action")).toString());

    auto fill_table = [&data](QTableWidget *table, const QJsonArray &services, const QJsonObject &actions) {
        table->setRowCount(0);

        for (const QJsonValue &service_value : services) {
            const QString service = service_value.toString();
            const int row = table->rowCount();
            table->insertRow(row);
            table->setItem(row, 0, new QTableWidgetItem(service));

            auto *combo = new QComboBox(table);

            for (const QJsonValue &action : data.value(QStringLiteral("actions")).toArray()) {
                const QJsonObject entry = action.toObject();
                combo->addItem(entry.value(QStringLiteral("label")).toString(), entry.value(QStringLiteral("value")).toString());
            }

            const int index = combo->findData(actions.value(service).toString());

            if (index >= 0) {
                combo->setCurrentIndex(index);
            } else {
                const int none_index = combo->findData(QStringLiteral("none"));

                if (none_index >= 0) {
                    combo->setCurrentIndex(none_index);
                }
            }

            table->setCellWidget(row, 1, combo);
        }
    };

    fill_table(
        page.tags,
        data.value(QStringLiteral("tag_services")).toArray(),
        options.value(QStringLiteral("tag_service_actions")).toObject());
    fill_table(
        page.ratings,
        data.value(QStringLiteral("rating_services")).toArray(),
        options.value(QStringLiteral("rating_service_actions")).toObject());
}

QJsonObject read_duplicate_merge_page(const DuplicateMergePage &page) {
    auto read_table = [](QTableWidget *table) {
        QJsonObject values;

        for (int row = 0; row < table->rowCount(); ++row) {
            const QTableWidgetItem *service = table->item(row, 0);
            auto *combo = qobject_cast<QComboBox *>(table->cellWidget(row, 1));

            if (service == nullptr || combo == nullptr) {
                continue;
            }

            const QString action = combo->currentData().toString();

            if (action != QStringLiteral("none")) {
                values.insert(service->text(), action);
            }
        }

        return values;
    };

    QJsonObject options;
    options.insert(QStringLiteral("sync_archive_action"), page.archive->currentData().toString());
    options.insert(QStringLiteral("sync_file_modified_date_action"), page.modified_date->currentData().toString());
    options.insert(QStringLiteral("sync_urls_action"), page.urls->currentData().toString());
    options.insert(QStringLiteral("sync_notes_action"), page.notes->currentData().toString());
    options.insert(QStringLiteral("tag_service_actions"), read_table(page.tags));
    options.insert(QStringLiteral("rating_service_actions"), read_table(page.ratings));

    return options;
}

void HydrusDuplicateMergeOptionsDialog::save() {
    for (const auto &[key, page] : std::initializer_list<std::pair<QString, const DuplicateMergePage *>>{
             {QStringLiteral("better"), &better}, {QStringLiteral("same"), &same}}) {
        QJsonObject payload;
        payload.insert(QStringLiteral("action"), key);
        payload.insert(QStringLiteral("options"), read_duplicate_merge_page(*page));
        main->notify(QStringLiteral("duplicates_merge_options"), QString(), json_payload(payload));
    }

    close();
}
