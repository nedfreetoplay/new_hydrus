
// WS-15.1.3c1: command palette moved from bridge.cpp (move-only split; inline
// methods outlined, no logic change).

#include <hydrus_client/src/gui/qt/api.cxx.h>
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

    const QList<QPair<QString, UiAction>> commands = {
        {QStringLiteral("new page..."), UiAction::NewPage},
        {QStringLiteral("manage pages..."), UiAction::PageManager},
        {QStringLiteral("duplicate current page"), UiAction::DuplicatePage},
        {QStringLiteral("back"), UiAction::HistoryBack},
        {QStringLiteral("forward"), UiAction::HistoryForward},
        {QStringLiteral("refresh current page"), UiAction::RefreshPage},
        {QStringLiteral("options..."), UiAction::Options},
        {QStringLiteral("manage tags..."), UiAction::ManageTagsActive},
        {QStringLiteral("import files..."), UiAction::ImportFiles},
        {QStringLiteral("import url..."), UiAction::ImportUrl},
        {QStringLiteral("import folders..."), UiAction::NewImportFolders},
        {QStringLiteral("manage export folders\u2026"), UiAction::ManageExportFolders},
        {QStringLiteral("run all export folders"), UiAction::ExportFolderRunAll},
        {QStringLiteral("check all import folders"), UiAction::ImportFolderCheckAll},
        {QStringLiteral("manage services..."), UiAction::ManageServices},
        {QStringLiteral("client api permissions..."), UiAction::ApiPermissions},
        {QStringLiteral("clear orphan files..."), UiAction::OrphanFilesScan},
        {QStringLiteral("review duplicate pending decisions..."), UiAction::DuplicatesReview},
        {QStringLiteral("back up database..."), UiAction::DbBackupOpen},
        {QStringLiteral("manage network..."), UiAction::OptionsNetwork},
        {QStringLiteral("save session as..."), UiAction::SessionSaveAs},
        {QStringLiteral("backup session"), UiAction::SessionBackup},
        {QStringLiteral("check database"), UiAction::DbCheck},
        {QStringLiteral("analyze database"), UiAction::DbAnalyze},
        {QStringLiteral("vacuum database"), UiAction::DbVacuum},
        {QStringLiteral("database statistics"), UiAction::DbStats},
        {QStringLiteral("undo closed page"), UiAction::Undo},
        {QStringLiteral("documentation"), UiAction::Docs},
        {QStringLiteral("about"), UiAction::About},
        {QStringLiteral("exit"), UiAction::Exit},
    };

    for (const auto &[label, action] : commands) {
        auto *item = new QListWidgetItem(label, list);
        item->setData(Qt::UserRole, static_cast<int>(action));
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

    const UiAction ua = static_cast<UiAction>(item->data(Qt::UserRole).toInt());
    accept();

    if (ua != UiAction::None) {
        if (ua == UiAction::DuplicatePage || ua == UiAction::RefreshPage) {
            main->notify(ua, main->active_page);
        } else {
            main->notify(ua);
        }
    }
}

void show_command_palette(HydrusMainWindow *main) {
    HydrusCommandPalette palette(main);
    palette.exec();
}
