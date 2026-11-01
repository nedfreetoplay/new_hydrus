#include <hydrus_client/src/gui/qt/api.cxx.h>

// WS-13.12: manage-shortcuts dialog (read-only editor) + applying the active
// set as QShortcuts. All texts/rows come from Rust; C++ only assigns widgets.

#include "support/support.hpp"

#include <QDialog>
#include <QDialogButtonBox>
#include <QHeaderView>
#include <QKeySequence>
#include <QLabel>
#include <QListWidget>
#include <QObject>
#include <QPushButton>
#include <QShortcut>
#include <QTableWidget>
#include <QVBoxLayout>

static UiAction shortcut_action_for(const QString &action) {
    if (action == QStringLiteral("new_page")) return UiAction::NewPage;
    if (action == QStringLiteral("close_page")) return UiAction::ClosePage;
    if (action == QStringLiteral("refresh_page")) return UiAction::RefreshPage;
    if (action == QStringLiteral("undo")) return UiAction::Undo;
    if (action == QStringLiteral("history_back")) return UiAction::HistoryBack;
    if (action == QStringLiteral("history_forward")) return UiAction::HistoryForward;
    if (action == QStringLiteral("page_manager")) return UiAction::PageManager;
    if (action == QStringLiteral("command_palette")) return UiAction::CommandPalette;
    if (action == QStringLiteral("duplicate_page")) return UiAction::DuplicatePage;
    if (action == QStringLiteral("import_files")) return UiAction::ImportFiles;
    if (action == QStringLiteral("import_url")) return UiAction::ImportUrl;
    if (action == QStringLiteral("new_import_folders")) return UiAction::NewImportFolders;
    if (action == QStringLiteral("options")) return UiAction::Options;
    if (action == QStringLiteral("manage_tags_active")) return UiAction::ManageTagsActive;
    if (action == QStringLiteral("archive_active")) return UiAction::ArchiveActive;
    if (action == QStringLiteral("inbox_active")) return UiAction::InboxActive;
    if (action == QStringLiteral("exit")) return UiAction::Exit;
    return UiAction::None;
}

static void shortcut_fire(HydrusMainWindow *main, const QString &action) {
    const UiAction ui = shortcut_action_for(action);
    if (ui == UiAction::None) {
        return;
    }
    const QString page = main->active_page;
    switch (ui) {
    case UiAction::ClosePage:
    case UiAction::RefreshPage:
    case UiAction::DuplicatePage:
        main->notify(ui, page);
        break;
    case UiAction::HistoryBack:
        app_history_step(*main, -1);
        break;
    case UiAction::HistoryForward:
        app_history_step(*main, 1);
        break;
    default:
        main->notify(ui);
        break;
    }
}

void hydrus_apply_shortcuts(HydrusMainWindow &window, const ShortcutSetsDialogState &state) {
    for (QShortcut *shortcut : window.findChildren<QShortcut *>()) {
        delete shortcut;
    }
    for (const auto &binding : state.bindings) {
        const QString action = to_qstring(binding.action);
        if (shortcut_action_for(action) == UiAction::None) {
            continue;
        }
        for (const auto &combo : binding.combos) {
            const QKeySequence sequence(to_qstring(combo));
            if (sequence.isEmpty()) {
                continue;
            }
            auto *shortcut = new QShortcut(sequence, &window);
            shortcut->setObjectName(QStringLiteral("shortcut-set"));
            QObject::connect(shortcut, &QShortcut::activated, &window, [&window, action]() {
                shortcut_fire(&window, action);
            });
        }
    }
}

void hydrus_show_shortcut_sets(HydrusMainWindow &window, const ShortcutSetsDialogState &state) {
    hydrus_apply_shortcuts(window, state);

    QDialog dialog(&window);
    dialog.setWindowTitle(QStringLiteral("manage shortcuts"));
    dialog.resize(640, 480);

    auto *layout = new QVBoxLayout(&dialog);

    auto *intro = new QLabel(
        QStringLiteral("Sets of keyboard shortcuts. The active set applies immediately."),
        &dialog);
    intro->setWordWrap(true);
    layout->addWidget(intro);

    auto *sets = new QListWidget(&dialog);
    for (const auto &row : state.sets) {
        QString label = to_qstring(row.name);
        if (row.active) {
            label += QStringLiteral(" (active)");
        }
        label += QStringLiteral(" - %1 bindings").arg(row.count);
        sets->addItem(label);
    }
    layout->addWidget(sets);

    auto *bindings = new QTableWidget(&dialog);
    bindings->setColumnCount(2);
    bindings->setHorizontalHeaderLabels({QStringLiteral("action"), QStringLiteral("shortcuts")});
    bindings->horizontalHeader()->setStretchLastSection(true);
    bindings->verticalHeader()->setVisible(false);
    bindings->setEditTriggers(QAbstractItemView::NoEditTriggers);
    bindings->setSelectionBehavior(QAbstractItemView::SelectRows);
    bindings->setRowCount(static_cast<int>(state.bindings.size()));
    int row_index = 0;
    for (const auto &binding : state.bindings) {
        bindings->setItem(
            row_index, 0, new QTableWidgetItem(to_qstring(binding.label)));
        bindings->setItem(
            row_index, 1, new QTableWidgetItem(to_qstring(binding.combo_text)));
        ++row_index;
    }
    layout->addWidget(bindings);

    auto *conflicts = new QLabel(&dialog);
    conflicts->setWordWrap(true);
    if (state.conflicts.empty()) {
        conflicts->setText(QStringLiteral("No conflicting shortcuts in the active set."));
    } else {
        QStringList lines;
        for (const auto &conflict : state.conflicts) {
            lines << to_qstring(conflict);
        }
        conflicts->setText(
            QStringLiteral("Conflicts in the active set:\n") + lines.join(QStringLiteral("\n")));
    }
    layout->addWidget(conflicts);

    auto *note = new QLabel(
        QStringLiteral("Creating, renaming and editing bindings is not implemented yet."),
        &dialog);
    note->setWordWrap(true);
    layout->addWidget(note);

    auto *buttons = new QDialogButtonBox(QDialogButtonBox::Close, &dialog);
    layout->addWidget(buttons);
    QObject::connect(buttons, &QDialogButtonBox::rejected, &dialog, &QDialog::reject);

    dialog.exec();
}
