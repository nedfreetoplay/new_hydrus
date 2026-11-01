
// WS-15.2.4b: typed DuplicateCustomActionDialogState (no JSON parsing).

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

MergeOptionsInfo to_merge_info(const MergeOptionsData &options);

MergeServiceActionInfo to_service_action(const MergeServiceActionRow &entry) {
    return {to_qstring(entry.service), to_qstring(entry.action)};
}

MergeOptionsPageInfo to_page_info(const MergeOptionsPage &page) {
    MergeOptionsPageInfo out;
    out.archive_action = to_qstring(page.archive_action);
    out.modified_date_action = to_qstring(page.modified_date_action);
    out.urls_action = to_qstring(page.urls_action);
    out.notes_action = to_qstring(page.notes_action);
    for (const MergeServiceActionRow &entry : page.tag_actions) {
        out.tag_actions.push_back(to_service_action(entry));
    }
    for (const MergeServiceActionRow &entry : page.rating_actions) {
        out.rating_actions.push_back(to_service_action(entry));
    }
    return out;
}

MergeOptionsInfo to_merge_info(const MergeOptionsData &options) {
    MergeOptionsInfo info;
    info.better = to_page_info(options.better);
    info.same = to_page_info(options.same);
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
    return info;
}

}  // namespace

HydrusDuplicateCustomActionDialog::HydrusDuplicateCustomActionDialog(
    HydrusMainWindow *main_window, const DuplicateCustomActionDialogState &state, QWidget *parent)
    : QDialog(parent), main(main_window) {
    setWindowTitle(QStringLiteral("custom duplicate action"));
    setObjectName(QStringLiteral("hydrus_duplicate_custom_action_dialog"));
    setAttribute(Qt::WA_DeleteOnClose, true);
    resize(800, 700);

    page_id = to_qstring(state.page_id);
    const MergeOptionsInfo data = to_merge_info(state.options);

    auto *layout = new QVBoxLayout(this);

    auto *help = new QLabel(
        QStringLiteral("Choose one action and customise the merge and delete options for this pair only. "
                       "Saved merge options are not changed."),
        this);
    help->setWordWrap(true);
    layout->addWidget(help);

    if (state.has_pair) {
        auto *summary = new QLabel(to_qstring(state.pair_summary), this);
        summary->setWordWrap(true);
        summary->setStyleSheet(QStringLiteral("font-family: monospace;"));
        layout->addWidget(summary);
    }

    auto *form = new QFormLayout();

    action = new QComboBox(this);
    for (const DuplicateChoiceRow &entry : state.duplicate_types) {
        action->addItem(to_qstring(entry.label), to_qstring(entry.value));
    }

    better_side = new QComboBox(this);
    for (const DuplicateChoiceRow &entry : state.better_sides) {
        better_side->addItem(to_qstring(entry.label), to_qstring(entry.value));
    }

    delete_mode = new QComboBox(this);
    for (const DuplicateChoiceRow &entry : state.delete_modes) {
        delete_mode->addItem(to_qstring(entry.label), to_qstring(entry.value));
    }

    form->addRow(QStringLiteral("action:"), action);
    form->addRow(QStringLiteral("better file:"), better_side);
    form->addRow(QStringLiteral("delete:"), delete_mode);
    layout->addLayout(form);

    tabs = new QTabWidget(this);
    build_duplicate_merge_page(this, tabs, data, QStringLiteral("this is better"), better);
    build_duplicate_merge_page(this, tabs, data, QStringLiteral("same quality"), same);
    fill_duplicate_merge_page(better, data, data.better);
    fill_duplicate_merge_page(same, data, data.same);
    layout->addWidget(tabs, 1);

    auto *buttons = new QHBoxLayout();
    auto *cancel = new QPushButton(QStringLiteral("cancel"), this);
    auto *apply_button = new QPushButton(QStringLiteral("apply"), this);
    apply_button->setDefault(true);
    buttons->addStretch(1);
    buttons->addWidget(cancel);
    buttons->addWidget(apply_button);
    layout->addLayout(buttons);

    auto update_controls = [this]() {
        const QString kind = action->currentData().toString();
        const QByteArray kind_bytes = kind.toUtf8();
        const rust::Str kind_str(kind_bytes.constData(), kind_bytes.size());
        const bool better_action = hydrus_avail_better_side(kind_str);

        better_side->setEnabled(better_action);
        tabs->setTabEnabled(0, better_action);
        tabs->setTabEnabled(1, hydrus_avail_same_tab(kind_str));

        if (better_action) {
            tabs->setCurrentIndex(0);
        } else if (hydrus_avail_same_tab(kind_str)) {
            tabs->setCurrentIndex(1);
        }
    };

    connect(action, &QComboBox::currentIndexChanged, this, [update_controls](int) { update_controls(); });
    connect(cancel, &QPushButton::clicked, this, &QDialog::close);
    connect(apply_button, &QPushButton::clicked, this, [this]() { apply(); });

    update_controls();
}

void HydrusDuplicateCustomActionDialog::apply() {
    const DuplicateMergePageValues better_values = read_duplicate_merge_page(better);
    const DuplicateMergePageValues same_values = read_duplicate_merge_page(same);

    const QByteArray page_bytes = page_id.toUtf8();
    const QByteArray action_bytes = action->currentData().toString().toUtf8();
    const QByteArray better_side_bytes = better_side->currentData().toString().toUtf8();
    const QByteArray delete_bytes = delete_mode->currentData().toString().toUtf8();
    const QByteArray better_archive_bytes = better_values.archive.toUtf8();
    const QByteArray better_modified_bytes = better_values.modified_date.toUtf8();
    const QByteArray better_urls_bytes = better_values.urls.toUtf8();
    const QByteArray better_notes_bytes = better_values.notes.toUtf8();
    const QByteArray same_archive_bytes = same_values.archive.toUtf8();
    const QByteArray same_modified_bytes = same_values.modified_date.toUtf8();
    const QByteArray same_urls_bytes = same_values.urls.toUtf8();
    const QByteArray same_notes_bytes = same_values.notes.toUtf8();

    app_duplicates_custom_apply(
        *main,
        rust::Str(page_bytes.constData(), page_bytes.size()),
        rust::Str(action_bytes.constData(), action_bytes.size()),
        rust::Str(better_side_bytes.constData(), better_side_bytes.size()),
        rust::Str(delete_bytes.constData(), delete_bytes.size()),
        rust::Str(better_archive_bytes.constData(), better_archive_bytes.size()),
        rust::Str(better_modified_bytes.constData(), better_modified_bytes.size()),
        rust::Str(better_urls_bytes.constData(), better_urls_bytes.size()),
        rust::Str(better_notes_bytes.constData(), better_notes_bytes.size()),
        to_rust_vec(better_values.tag_services),
        to_rust_vec(better_values.tag_actions),
        to_rust_vec(better_values.rating_services),
        to_rust_vec(better_values.rating_actions),
        rust::Str(same_archive_bytes.constData(), same_archive_bytes.size()),
        rust::Str(same_modified_bytes.constData(), same_modified_bytes.size()),
        rust::Str(same_urls_bytes.constData(), same_urls_bytes.size()),
        rust::Str(same_notes_bytes.constData(), same_notes_bytes.size()),
        to_rust_vec(same_values.tag_services),
        to_rust_vec(same_values.tag_actions),
        to_rust_vec(same_values.rating_services),
        to_rust_vec(same_values.rating_actions));

    close();
}
