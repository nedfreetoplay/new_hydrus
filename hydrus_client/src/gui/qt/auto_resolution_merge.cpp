
// WS-15.2.4b: typed DuplicatesMergeOptionsDialogState (no JSON parsing).

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

MergeOptionsInfo to_merge_info(const MergeOptionsData &options) {
    MergeOptionsInfo info;
    auto to_page = [](const MergeOptionsPage &page) {
        MergeOptionsPageInfo out;
        out.archive_action = to_qstring(page.archive_action);
        out.modified_date_action = to_qstring(page.modified_date_action);
        out.urls_action = to_qstring(page.urls_action);
        out.notes_action = to_qstring(page.notes_action);
        for (const MergeServiceActionRow &entry : page.tag_actions) {
            out.tag_actions.push_back({to_qstring(entry.service), to_qstring(entry.action)});
        }
        for (const MergeServiceActionRow &entry : page.rating_actions) {
            out.rating_actions.push_back({to_qstring(entry.service), to_qstring(entry.action)});
        }
        return out;
    };
    info.better = to_page(options.better);
    info.same = to_page(options.same);
    for (const rust::String &service : options.tag_services) {
        info.tag_services.append(to_qstring(service));
    }
    for (const rust::String &service : options.rating_services) {
        info.rating_services.append(to_qstring(service));
    }
    for (const DuplicateChoiceRow &entry : options.actions) {
        info.actions.push_back({to_qstring(entry.value), to_qstring(entry.label)});
    }
    for (const DuplicateChoiceRow &entry : options.archive_actions) {
        info.archive_actions.push_back({to_qstring(entry.value), to_qstring(entry.label)});
    }
    Q_UNUSED(options.better_summary);
    Q_UNUSED(options.same_summary);
    return info;
}

}  // namespace

HydrusDuplicateMergeOptionsDialog::HydrusDuplicateMergeOptionsDialog(
    HydrusMainWindow *main_window, const DuplicatesMergeOptionsDialogState &state, QWidget *parent)
    : QDialog(parent), main(main_window) {
    setWindowTitle(QStringLiteral("duplicate merge options"));
    setObjectName(QStringLiteral("hydrus_duplicate_merge_options_dialog"));
    setAttribute(Qt::WA_DeleteOnClose, true);
    resize(760, 620);

    data = to_merge_info(state.options);

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

void HydrusDuplicateMergeOptionsDialog::set_state(const DuplicatesMergeOptionsDialogState &state) {
    data = to_merge_info(state.options);

    fill_duplicate_merge_page(better, data, data.better);
    fill_duplicate_merge_page(same, data, data.same);
}

void build_duplicate_merge_page(
    QWidget *parent,
    QTabWidget *tabs,
    const MergeOptionsInfo &data,
    const QString &title,
    DuplicateMergePage &page) {
    auto *widget = new QWidget(parent);
    auto *layout = new QVBoxLayout(widget);

    auto *form = new QFormLayout();

    page.archive = new QComboBox(widget);
    page.modified_date = new QComboBox(widget);
    page.urls = new QComboBox(widget);
    page.notes = new QComboBox(widget);

    for (const ChoiceInfo &action : data.archive_actions) {
        page.archive->addItem(action.label, action.value);
    }

    for (QComboBox *combo : {page.modified_date, page.urls, page.notes}) {
        for (const ChoiceInfo &action : data.actions) {
            if (combo == page.modified_date && (action.value == QStringLiteral("move") || action.value == QStringLiteral("add_move"))) {
                continue;
            }

            combo->addItem(action.label, action.value);
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
    const MergeOptionsInfo &data,
    const MergeOptionsPageInfo &options) {
    auto select_value = [](QComboBox *combo, const QString &value) {
        const int index = combo->findData(value);

        if (index >= 0) {
            combo->setCurrentIndex(index);
        }
    };

    select_value(page.archive, options.archive_action);
    select_value(page.modified_date, options.modified_date_action);
    select_value(page.urls, options.urls_action);
    select_value(page.notes, options.notes_action);

    auto fill_table = [&data](QTableWidget *table, const QStringList &services, const std::vector<MergeServiceActionInfo> &actions) {
        table->setRowCount(0);

        for (const QString &service : services) {
            const int row = table->rowCount();
            table->insertRow(row);
            table->setItem(row, 0, new QTableWidgetItem(service));

            auto *combo = new QComboBox(table);

            for (const ChoiceInfo &action : data.actions) {
                combo->addItem(action.label, action.value);
            }

            QString current;
            for (const MergeServiceActionInfo &entry : actions) {
                if (entry.service == service) {
                    current = entry.action;
                    break;
                }
            }

            const int index = combo->findData(current);

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

    fill_table(page.tags, data.tag_services, options.tag_actions);
    fill_table(page.ratings, data.rating_services, options.rating_actions);
}

DuplicateMergePageValues read_duplicate_merge_page(const DuplicateMergePage &page) {
    auto read_table = [](QTableWidget *table, QStringList *services, QStringList *actions) {
        for (int row = 0; row < table->rowCount(); ++row) {
            const QTableWidgetItem *service = table->item(row, 0);
            auto *combo = qobject_cast<QComboBox *>(table->cellWidget(row, 1));

            if (service == nullptr || combo == nullptr) {
                continue;
            }

            services->append(service->text());
            actions->append(combo->currentData().toString());
        }
    };

    DuplicateMergePageValues values;
    values.archive = page.archive->currentData().toString();
    values.modified_date = page.modified_date->currentData().toString();
    values.urls = page.urls->currentData().toString();
    values.notes = page.notes->currentData().toString();
    read_table(page.tags, &values.tag_services, &values.tag_actions);
    read_table(page.ratings, &values.rating_services, &values.rating_actions);

    return values;
}

void send_merge_options(
    HydrusMainWindow *main,
    const QString &kind,
    const DuplicateMergePageValues &values) {
    const QByteArray kind_bytes = kind.toUtf8();
    const QByteArray archive_bytes = values.archive.toUtf8();
    const QByteArray modified_bytes = values.modified_date.toUtf8();
    const QByteArray urls_bytes = values.urls.toUtf8();
    const QByteArray notes_bytes = values.notes.toUtf8();
    app_duplicates_merge_options_save(
        *main,
        rust::Str(kind_bytes.constData(), kind_bytes.size()),
        rust::Str(archive_bytes.constData(), archive_bytes.size()),
        rust::Str(modified_bytes.constData(), modified_bytes.size()),
        rust::Str(urls_bytes.constData(), urls_bytes.size()),
        rust::Str(notes_bytes.constData(), notes_bytes.size()),
        to_rust_vec(values.tag_services),
        to_rust_vec(values.tag_actions),
        to_rust_vec(values.rating_services),
        to_rust_vec(values.rating_actions));
}

void HydrusDuplicateMergeOptionsDialog::save() {
    for (const auto &[key, page] : std::initializer_list<std::pair<QString, const DuplicateMergePage *>>{
             {QStringLiteral("better"), &better}, {QStringLiteral("same"), &same}}) {
        send_merge_options(main, key, read_duplicate_merge_page(*page));
    }

    close();
}
