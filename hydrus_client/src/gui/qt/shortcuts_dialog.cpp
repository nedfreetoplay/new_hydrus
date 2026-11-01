#include <hydrus_client/src/gui/qt/api.cxx.h>

// WS-13.12: manage-shortcuts dialog (full editor) + applying the active
// set as QShortcuts. All texts/rows come from Rust; C++ only assigns widgets.

#include "support/support.hpp"

#include <QComboBox>
#include <QDialog>
#include <QDialogButtonBox>
#include <QFile>
#include <QFileDialog>

static rust::String rs(const QString &s) {
    const QByteArray bytes = s.toUtf8();
    return rust::String(bytes.constData());
}
#include <QHeaderView>
#include <QInputDialog>
#include <QKeySequence>
#include <QLabel>
#include <QListWidget>
#include <QMessageBox>
#include <QObject>
#include <QPushButton>
#include <QShortcut>
#include <QTableWidget>
#include <QTextStream>
#include <QVBoxLayout>
#include <QHBoxLayout>

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

static QString selected_set_name(QListWidget *sets, const ShortcutSetsDialogState &state) {
    const int row = sets->currentRow();
    if (row < 0 || row >= static_cast<int>(state.sets.size())) {
        return to_qstring(state.active_name);
    }
    return to_qstring(state.sets[row].name);
}

void hydrus_show_shortcut_sets(HydrusMainWindow &window, const ShortcutSetsDialogState &state) {
    hydrus_apply_shortcuts(window, state);

    QDialog dialog(&window);
    dialog.setWindowTitle(QStringLiteral("manage shortcuts"));
    dialog.resize(680, 520);

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
    for (size_t i = 0; i < state.sets.size(); ++i) {
        if (to_qstring(state.sets[i].name) == to_qstring(state.active_name)) {
            sets->setCurrentRow(static_cast<int>(i));
            break;
        }
    }
    layout->addWidget(sets);

    auto *set_buttons = new QHBoxLayout();
    auto *add_btn = new QPushButton(QStringLiteral("add"), &dialog);
    auto *dup_btn = new QPushButton(QStringLiteral("duplicate"), &dialog);
    auto *rename_btn = new QPushButton(QStringLiteral("rename"), &dialog);
    auto *delete_btn = new QPushButton(QStringLiteral("delete"), &dialog);
    auto *use_btn = new QPushButton(QStringLiteral("use this set"), &dialog);
    set_buttons->addWidget(add_btn);
    set_buttons->addWidget(dup_btn);
    set_buttons->addWidget(rename_btn);
    set_buttons->addWidget(delete_btn);
    set_buttons->addWidget(use_btn);
    layout->addLayout(set_buttons);

    auto *bindings = new QTableWidget(&dialog);
    bindings->setColumnCount(2);
    bindings->setHorizontalHeaderLabels({QStringLiteral("action"), QStringLiteral("shortcuts")});
    bindings->horizontalHeader()->setStretchLastSection(true);
    bindings->verticalHeader()->setVisible(false);
    bindings->setEditTriggers(QAbstractItemView::NoEditTriggers);
    bindings->setSelectionBehavior(QAbstractItemView::SelectRows);
    // WS-13.13: per-context keyboard rows with inheritance labels; the main
    // context keeps editing the flat set bindings.
    const bool is_main_context = to_qstring(state.active_context) == QStringLiteral("main_gui");
    const size_t key_count = is_main_context ? state.bindings.size() : state.context_bindings.size();
    bindings->setRowCount(static_cast<int>(key_count));
    for (size_t row_index = 0; row_index < key_count; ++row_index) {
        const ShortcutBindingRow &binding = is_main_context
            ? state.bindings[row_index]
            : state.context_bindings[row_index];
        bindings->setItem(static_cast<int>(row_index), 0, new QTableWidgetItem(to_qstring(binding.label)));
        bindings->setItem(static_cast<int>(row_index), 1, new QTableWidgetItem(to_qstring(binding.combo_text)));
    }
    layout->addWidget(bindings);

    auto *context_row = new QHBoxLayout();
    auto *context_label = new QLabel(QStringLiteral("context:"), &dialog);
    auto *context_combo = new QComboBox(&dialog);
    for (const auto &context : state.contexts) {
        context_combo->addItem(to_qstring(context));
    }
    context_combo->setCurrentText(to_qstring(state.active_context));
    context_row->addWidget(context_label);
    context_row->addWidget(context_combo, 1);
    layout->addLayout(context_row);

    auto *mouse_table = new QTableWidget(&dialog);
    mouse_table->setColumnCount(2);
    mouse_table->setHorizontalHeaderLabels({QStringLiteral("mouse action"), QStringLiteral("gestures")});
    mouse_table->horizontalHeader()->setStretchLastSection(true);
    mouse_table->verticalHeader()->setVisible(false);
    mouse_table->setEditTriggers(QAbstractItemView::NoEditTriggers);
    mouse_table->setSelectionBehavior(QAbstractItemView::SelectRows);
    mouse_table->setRowCount(static_cast<int>(state.mouse_bindings.size()));
    int mouse_row = 0;
    for (const auto &binding : state.mouse_bindings) {
        mouse_table->setItem(mouse_row, 0, new QTableWidgetItem(to_qstring(binding.label)));
        mouse_table->setItem(mouse_row, 1, new QTableWidgetItem(to_qstring(binding.gesture_text)));
        ++mouse_row;
    }
    layout->addWidget(mouse_table);

    auto *mouse_buttons = new QHBoxLayout();
    auto *mouse_edit_btn = new QPushButton(QStringLiteral("edit mouse"), &dialog);
    auto *mouse_clear_btn = new QPushButton(QStringLiteral("clear mouse"), &dialog);
    mouse_buttons->addWidget(mouse_edit_btn);
    mouse_buttons->addWidget(mouse_clear_btn);
    layout->addLayout(mouse_buttons);
    QObject::connect(
        context_combo, &QComboBox::currentTextChanged, &dialog, [&](const QString &text) {
            app_shortcut_context_open(window, rs(text));
            dialog.accept();
        });
    QObject::connect(mouse_edit_btn, &QPushButton::clicked, &dialog, [&]() {
        const int row = mouse_table->currentRow();
        if (row < 0 || row >= static_cast<int>(state.mouse_bindings.size())) {
            return;
        }
        const QString action = to_qstring(state.mouse_bindings[row].action);
        const QString current = to_qstring(state.mouse_bindings[row].gesture_text);
        bool ok = false;
        const QString text = QInputDialog::getText(
            &dialog, QStringLiteral("edit mouse shortcut"),
            QStringLiteral("Enter comma-separated gestures for '%1' (<button> <press/release/double/drag>):").arg(action),
            QLineEdit::Normal, current, &ok);
        if (!ok) {
            return;
        }
        rust::Vec<rust::String> gestures;
        for (const QString &part : text.split(QStringLiteral(","))) {
            const QString trimmed = part.trimmed();
            if (!trimmed.isEmpty()) {
                gestures.push_back(rs(trimmed));
            }
        }
        app_shortcut_mouse_save(
            window, rs(selected_set_name(sets, state)),
            rs(to_qstring(state.active_context)), rs(action), gestures);
        dialog.accept();
    });
    QObject::connect(mouse_clear_btn, &QPushButton::clicked, &dialog, [&]() {
        const int row = mouse_table->currentRow();
        if (row < 0 || row >= static_cast<int>(state.mouse_bindings.size())) {
            return;
        }
        app_shortcut_mouse_save(
            window, rs(selected_set_name(sets, state)),
            rs(to_qstring(state.active_context)),
            rs(to_qstring(state.mouse_bindings[row].action)), rust::Vec<rust::String>());
        dialog.accept();
    });

    auto *bind_buttons = new QHBoxLayout();
    auto *edit_btn = new QPushButton(QStringLiteral("edit"), &dialog);
    auto *clear_btn = new QPushButton(QStringLiteral("clear"), &dialog);
    auto *import_btn = new QPushButton(QStringLiteral("import"), &dialog);
    auto *export_btn = new QPushButton(QStringLiteral("export"), &dialog);
    bind_buttons->addWidget(edit_btn);
    bind_buttons->addWidget(clear_btn);
    bind_buttons->addWidget(import_btn);
    bind_buttons->addWidget(export_btn);
    layout->addLayout(bind_buttons);

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

    QObject::connect(add_btn, &QPushButton::clicked, &dialog, [&]() {
        bool ok = false;
        const QString name = QInputDialog::getText(
            &dialog, QStringLiteral("add shortcut set"),
            QStringLiteral("Enter a name for the new set:"), QLineEdit::Normal,
            QStringLiteral("new shortcuts"), &ok);
        if (ok && !name.trimmed().isEmpty()) {
            app_shortcut_set_create(window, rs(name.trimmed()));
            dialog.accept();
        }
    });
    QObject::connect(dup_btn, &QPushButton::clicked, &dialog, [&]() {
        app_shortcut_set_duplicate(window, rs(selected_set_name(sets, state)));
        dialog.accept();
    });
    QObject::connect(rename_btn, &QPushButton::clicked, &dialog, [&]() {
        const QString current = selected_set_name(sets, state);
        bool ok = false;
        const QString name = QInputDialog::getText(
            &dialog, QStringLiteral("rename shortcut set"),
            QStringLiteral("Enter a new name:"), QLineEdit::Normal, current, &ok);
        if (ok && !name.trimmed().isEmpty()) {
            app_shortcut_set_rename(window, rs(current), rs(name.trimmed()));
            dialog.accept();
        }
    });
    QObject::connect(delete_btn, &QPushButton::clicked, &dialog, [&]() {
        const QString current = selected_set_name(sets, state);
        if (QMessageBox::question(
                &dialog, QStringLiteral("delete shortcut set"),
                QStringLiteral("Remove '%1'?").arg(current))
            == QMessageBox::Yes) {
            app_shortcut_set_delete(window, rs(current));
            dialog.accept();
        }
    });
    QObject::connect(use_btn, &QPushButton::clicked, &dialog, [&]() {
        app_shortcut_set_activate(window, rs(selected_set_name(sets, state)));
        dialog.accept();
    });
    QObject::connect(edit_btn, &QPushButton::clicked, &dialog, [&]() {
        const int row = bindings->currentRow();
        if (row < 0 || row >= static_cast<int>(key_count)) {
            return;
        }
        const QString action = is_main_context
            ? to_qstring(state.bindings[row].action)
            : to_qstring(state.context_bindings[row].action);
        const QString current = is_main_context
            ? to_qstring(state.bindings[row].combo_text)
            : to_qstring(state.context_bindings[row].combo_text);
        bool ok = false;
        const QString text = QInputDialog::getText(
            &dialog, QStringLiteral("edit shortcut"),
            QStringLiteral("Enter comma-separated shortcuts for '%1':").arg(action),
            QLineEdit::Normal, current, &ok);
        if (!ok) {
            return;
        }
        rust::Vec<rust::String> combos;
        for (const QString &part : text.split(QStringLiteral(","))) {
            const QString trimmed = part.trimmed();
            if (!trimmed.isEmpty()) {
                combos.push_back(rs(trimmed));
            }
        }
        if (is_main_context) {
            app_shortcut_binding_save(
                window, rs(selected_set_name(sets, state)), rs(action), combos);
        } else {
            app_shortcut_key_save(
                window, rs(selected_set_name(sets, state)),
                rs(to_qstring(state.active_context)), rs(action), combos);
        }
        dialog.accept();
    });
    QObject::connect(clear_btn, &QPushButton::clicked, &dialog, [&]() {
        const int row = bindings->currentRow();
        if (row < 0 || row >= static_cast<int>(key_count)) {
            return;
        }
        const QString action = is_main_context
            ? to_qstring(state.bindings[row].action)
            : to_qstring(state.context_bindings[row].action);
        if (is_main_context) {
            app_shortcut_binding_save(
                window, rs(selected_set_name(sets, state)),
                rs(action), rust::Vec<rust::String>());
        } else {
            app_shortcut_key_save(
                window, rs(selected_set_name(sets, state)),
                rs(to_qstring(state.active_context)),
                rs(action), rust::Vec<rust::String>());
        }
        dialog.accept();
    });
    QObject::connect(import_btn, &QPushButton::clicked, &dialog, [&]() {
        const QString path = QFileDialog::getOpenFileName(
            &dialog, QStringLiteral("import shortcut set"), QString(),
            QStringLiteral("JSON files (*.json);;All files (*)"));
        if (path.isEmpty()) {
            return;
        }
        QFile file(path);
        if (!file.open(QIODevice::ReadOnly | QIODevice::Text)) {
            QMessageBox::warning(
                &dialog, QStringLiteral("import shortcut set"),
                QStringLiteral("Could not read that file!"));
            return;
        }
        QTextStream stream(&file);
        app_shortcut_set_import(window, rs(stream.readAll()));
        dialog.accept();
    });
    QObject::connect(export_btn, &QPushButton::clicked, &dialog, [&]() {
        const QString current = selected_set_name(sets, state);
        const QString path = QFileDialog::getSaveFileName(
            &dialog, QStringLiteral("export shortcut set"),
            current + QStringLiteral(".json"),
            QStringLiteral("JSON files (*.json);;All files (*)"));
        if (path.isEmpty()) {
            return;
        }
        const rust::String text = hydrus_shortcut_set_export(rs(current));
        QFile file(path);
        if (text.empty() || !file.open(QIODevice::WriteOnly | QIODevice::Text)) {
            QMessageBox::warning(
                &dialog, QStringLiteral("export shortcut set"),
                QStringLiteral("Could not write that file!"));
            return;
        }
        QTextStream stream(&file);
        stream << to_qstring(text);
    });

    auto *buttons = new QDialogButtonBox(QDialogButtonBox::Close, &dialog);
    layout->addWidget(buttons);
    QObject::connect(buttons, &QDialogButtonBox::rejected, &dialog, &QDialog::reject);

    dialog.exec();
}
