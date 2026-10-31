#include "command_palette.hpp"

// WS-15.1.3c1: command palette moved from bridge.cpp (move-only split; inline
// methods outlined, no logic change).

#include "bridge.hpp"
#include "support/support.hpp"

#include <QLineEdit>
#include <QListWidget>
#include <QVBoxLayout>

HydrusCommandPalette::HydrusCommandPalette(HydrusMainWindow *main_window, QWidget *parent)
    : QDialog(parent), main(main_window) {
    setWindowTitle(QStringLiteral("command palette"));
    resize(520, 420);

    auto *layout = new QVBoxLayout(this);
    filter = new QLineEdit(this);
    filter->setPlaceholderText(QStringLiteral("type a command..."));
    layout->addWidget(filter);

    list = new QListWidget(this);
    list->setSelectionMode(QAbstractItemView::SingleSelection);
    layout->addWidget(list, 1);

    const QList<QPair<QString, QString>> commands = {
        {QStringLiteral("new page..."), QStringLiteral("new_page")},
        {QStringLiteral("manage pages..."), QStringLiteral("page_manager")},
        {QStringLiteral("duplicate current page"), QStringLiteral("duplicate_page")},
        {QStringLiteral("back"), QStringLiteral("history_back")},
        {QStringLiteral("forward"), QStringLiteral("history_forward")},
        {QStringLiteral("refresh current page"), QStringLiteral("refresh_page")},
        {QStringLiteral("options..."), QStringLiteral("options")},
        {QStringLiteral("manage tags..."), QStringLiteral("manage_tags_active")},
        {QStringLiteral("import files..."), QStringLiteral("import_files")},
        {QStringLiteral("import url..."), QStringLiteral("import_url")},
        {QStringLiteral("import folders..."), QStringLiteral("new_import_folders")},
        {QStringLiteral("manage export folders\u2026"), QStringLiteral("manage_export_folders")},
        {QStringLiteral("run all export folders"), QStringLiteral("export_folder_run_all")},
        {QStringLiteral("check all import folders"), QStringLiteral("import_folder_check_all")},
        {QStringLiteral("manage services..."), QStringLiteral("manage_services")},
        {QStringLiteral("client api permissions..."), QStringLiteral("api_permissions")},
        {QStringLiteral("clear orphan files..."), QStringLiteral("orphan_files_scan")},
        {QStringLiteral("review duplicate pending decisions..."), QStringLiteral("duplicates_review")},
        {QStringLiteral("back up database..."), QStringLiteral("db_backup_open")},
        {QStringLiteral("manage network..."), QStringLiteral("options_network")},
        {QStringLiteral("save session as..."), QStringLiteral("session_save_as")},
        {QStringLiteral("backup session"), QStringLiteral("session_backup")},
        {QStringLiteral("check database"), QStringLiteral("db_check")},
        {QStringLiteral("analyze database"), QStringLiteral("db_analyze")},
        {QStringLiteral("vacuum database"), QStringLiteral("db_vacuum")},
        {QStringLiteral("database statistics"), QStringLiteral("db_stats")},
        {QStringLiteral("undo closed page"), QStringLiteral("undo")},
        {QStringLiteral("documentation"), QStringLiteral("docs")},
        {QStringLiteral("about"), QStringLiteral("about")},
        {QStringLiteral("exit"), QStringLiteral("exit")},
    };

    for (const auto &[label, action] : commands) {
        auto *item = new QListWidgetItem(label, list);
        item->setData(Qt::UserRole, action);
    }

    connect(filter, &QLineEdit::textChanged, this, [this](const QString &text) {
        for (int index = 0; index < list->count(); ++index) {
            QListWidgetItem *item = list->item(index);
            item->setHidden(!text.isEmpty()
                && !item->text().contains(text, Qt::CaseInsensitive));
        }

        if (list->count() > 0) {
            for (int index = 0; index < list->count(); ++index) {
                if (!list->item(index)->isHidden()) {
                    list->setCurrentRow(index);
                    break;
                }
            }
        }
    });
    connect(filter, &QLineEdit::returnPressed, this, [this]() { run_current(); });
    connect(list, &QListWidget::itemDoubleClicked, this, [this](QListWidgetItem *) {
        run_current();
    });

    if (list->count() > 0) {
        list->setCurrentRow(0);
    }

    filter->setFocus();
}

void HydrusCommandPalette::run_current() {
    QListWidgetItem *item = list->currentItem();

    if (item == nullptr) {
        return;
    }

    const QString action = item->data(Qt::UserRole).toString();
    accept();

    if (!action.isEmpty()) {
        if (action == QStringLiteral("duplicate_page")
            || action == QStringLiteral("refresh_page")) {
            main->notify(action, main->active_page);
        } else {
            main->notify(action);
        }
    }
}

void show_command_palette(HydrusMainWindow *main) {
    HydrusCommandPalette palette(main);
    palette.exec();
}
