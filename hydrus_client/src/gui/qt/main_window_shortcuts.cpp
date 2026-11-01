#include <hydrus_client/src/gui/qt/api.cxx.h>

// WS-15.1.3b: HydrusMainWindow shortcuts moved verbatim from bridge.cpp (move-only split).

#include "support/support.hpp"

#include <QKeySequence>
#include <QShortcut>

void show_command_palette(HydrusMainWindow *main);

void HydrusMainWindow::build_shortcuts() {
    auto *new_page = new QShortcut(QKeySequence(QStringLiteral("Ctrl+T")), this);
    connect(new_page, &QShortcut::activated, this, [this]() {
        show_page_chooser(active_notebook.isEmpty() ? QStringLiteral("root") : active_notebook);
    });

    auto *close_page = new QShortcut(QKeySequence(QStringLiteral("Ctrl+W")), this);
    connect(close_page, &QShortcut::activated, this, [this]() {
        if (!active_page.isEmpty()) {
            notify(UiAction::ClosePage, active_page);
        }
    });

    auto *refresh = new QShortcut(QKeySequence(QStringLiteral("Ctrl+R")), this);
    connect(refresh, &QShortcut::activated, this, [this]() {
        if (!active_page.isEmpty()) {
            QByteArray page_bytes = active_page.toUtf8();
            app_refresh_page(*this, rust::Str(page_bytes.constData(), page_bytes.size()));
        }
    });

    auto *undo = new QShortcut(QKeySequence(QStringLiteral("Ctrl+Z")), this);
    connect(undo, &QShortcut::activated, this, [this]() { notify(UiAction::Undo); });

    auto *back = new QShortcut(QKeySequence(QStringLiteral("Alt+Left")), this);
    connect(back, &QShortcut::activated, this, [this]() { app_history_step(*this, -1); });

    auto *forward = new QShortcut(QKeySequence(QStringLiteral("Alt+Right")), this);
    connect(forward, &QShortcut::activated, this, [this]() { app_history_step(*this, 1); });

    auto *page_manager = new QShortcut(QKeySequence(QStringLiteral("Ctrl+Shift+P")), this);
    connect(page_manager, &QShortcut::activated, this, [this]() { notify(UiAction::PageManager); });

    auto *palette = new QShortcut(QKeySequence(QStringLiteral("Ctrl+K")), this);
    connect(palette, &QShortcut::activated, this, [this]() {
        show_command_palette(this);
    });

    auto *duplicate = new QShortcut(QKeySequence(QStringLiteral("Ctrl+Shift+T")), this);
    connect(duplicate, &QShortcut::activated, this, [this]() {
        if (!active_page.isEmpty()) {
            notify(UiAction::DuplicatePage, active_page);
        }
    });

    auto *import = new QShortcut(QKeySequence(QStringLiteral("Ctrl+I")), this);
    connect(import, &QShortcut::activated, this, [this]() { notify(UiAction::ImportFiles); });

    auto *import_url = new QShortcut(QKeySequence(QStringLiteral("Ctrl+U")), this);
    connect(import_url, &QShortcut::activated, this, [this]() { notify(UiAction::ImportUrl); });

    auto *import_folders_shortcut = new QShortcut(QKeySequence(QStringLiteral("Ctrl+Shift+I")), this);
    connect(import_folders_shortcut, &QShortcut::activated, this, [this]() {
        notify(UiAction::NewImportFolders);
    });

    auto *options = new QShortcut(QKeySequence(QStringLiteral("Ctrl+P")), this);
    connect(options, &QShortcut::activated, this, [this]() { notify(UiAction::Options); });

    auto *manage_tags = new QShortcut(QKeySequence(QStringLiteral("Ctrl+M")), this);
    manage_tags->setContext(Qt::ApplicationShortcut);
    connect(manage_tags, &QShortcut::activated, this, [this]() {
        notify(UiAction::ManageTagsActive);
    });

    auto *archive = new QShortcut(QKeySequence(QStringLiteral("F7")), this);
    connect(archive, &QShortcut::activated, this, [this]() { notify(UiAction::ArchiveActive); });

    auto *inbox = new QShortcut(QKeySequence(QStringLiteral("F8")), this);
    connect(inbox, &QShortcut::activated, this, [this]() { notify(UiAction::InboxActive); });

    auto *refresh_f5 = new QShortcut(QKeySequence(QStringLiteral("F5")), this);
    connect(refresh_f5, &QShortcut::activated, this, [this]() {
        if (!active_page.isEmpty()) {
            notify(UiAction::RefreshPage, active_page);
        }
    });

    auto *quit = new QShortcut(QKeySequence(QStringLiteral("Ctrl+Q")), this);
    connect(quit, &QShortcut::activated, this, [this]() { notify(UiAction::Exit); });
}
