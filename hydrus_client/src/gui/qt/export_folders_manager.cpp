#include <hydrus_client/src/gui/qt/api.cxx.h>

#include "support/support.hpp"

#include <QDialog>
#include <QHBoxLayout>
#include <QHeaderView>
#include <QKeySequence>
#include <QLabel>
#include <QPushButton>
#include <QShortcut>
#include <QTableWidget>
#include <QVBoxLayout>

// WS-15.1.5: export folders manager dialog + page moved verbatim from bridge.cpp (move-only split).

HydrusExportFoldersDialog::HydrusExportFoldersDialog(
    HydrusMainWindow *main_window, const FolderPageState &state, QWidget *parent)
    : QDialog(parent), main(main_window) {
    setWindowTitle(QStringLiteral("edit export folders"));
    setObjectName(QStringLiteral("edit_export_folders_dialog"));
    setAttribute(Qt::WA_DeleteOnClose, true);
    resize(723, 170);

    auto *layout = new QVBoxLayout(this);
    layout->setContentsMargins(4, 4, 4, 4);
    layout->setSpacing(4);

    auto *intro = new QLabel(
        QStringLiteral("Here you can set the client to regularly export a certain query to a "
                       "particular location."),
        this);
    layout->addWidget(intro);

    table = new QTableWidget(this);
    table->setColumnCount(7);
    table->setHorizontalHeaderLabels({QStringLiteral("name"),
                                      QStringLiteral("path"),
                                      QStringLiteral("type"),
                                      QStringLiteral("query"),
                                      QStringLiteral("period"),
                                      QStringLiteral("phrase"),
                                      QStringLiteral("recent error?")});
    table->verticalHeader()->setVisible(false);
    table->setSelectionBehavior(QAbstractItemView::SelectRows);
    table->setSelectionMode(QAbstractItemView::ExtendedSelection);
    table->setEditTriggers(QAbstractItemView::NoEditTriggers);
    table->setWordWrap(false);
    table->setTextElideMode(Qt::ElideRight);
    table->setShowGrid(false);
    table->horizontalHeader()->setTextElideMode(Qt::ElideRight);
    table->horizontalHeader()->setDefaultAlignment(Qt::AlignLeft | Qt::AlignVCenter);
    table->horizontalHeader()->setMinimumSectionSize(20);
    table->horizontalHeader()->setStretchLastSection(true);
    table->horizontalHeader()->setSortIndicator(0, Qt::AscendingOrder);

    const int widths[] = {132, 20, 79, 106, 106, 132};

    for (int column = 0; column < 6; ++column) {
        table->setColumnWidth(column, widths[column]);
    }

    table->setMinimumHeight(98);
    layout->addWidget(table, 1);

    auto *buttons = new QHBoxLayout();
    buttons->addStretch(1);
    auto *add = new QPushButton(QStringLiteral("add"), this);
    edit_button = new QPushButton(QStringLiteral("edit"), this);
    delete_button = new QPushButton(QStringLiteral("delete"), this);
    add->setMinimumSize(75, 24);
    edit_button->setMinimumSize(75, 24);
    delete_button->setMinimumSize(75, 24);
    buttons->addWidget(add);
    buttons->addWidget(edit_button);
    buttons->addWidget(delete_button);
    layout->addLayout(buttons);

    auto *bottom = new QHBoxLayout();
    bottom->addStretch(1);
    auto *apply = new QPushButton(QStringLiteral("apply"), this);
    apply->setObjectName(QStringLiteral("HydrusAccept"));
    apply->setMinimumSize(75, 24);
    auto *cancel = new QPushButton(QStringLiteral("cancel"), this);
    cancel->setObjectName(QStringLiteral("HydrusCancel"));
    cancel->setMinimumSize(75, 24);
    bottom->addWidget(apply);
    bottom->addWidget(cancel);
    layout->addLayout(bottom);

    connect(add, &QPushButton::clicked, this, [this]() {
        main->notify(UiAction::ExportFolderEdit, QString(), QString());
    });
    connect(edit_button, &QPushButton::clicked, this, [this]() {
        with_selected(UiAction::ExportFolderEdit);
    });
    connect(delete_button, &QPushButton::clicked, this, [this]() {
        with_selected(UiAction::ExportFolderRemove);
    });
    connect(table, &QTableWidget::cellDoubleClicked, this, [this](int, int) {
        with_selected(UiAction::ExportFolderEdit);
    });
    connect(table, &QTableWidget::itemSelectionChanged, this, [this]() {
        refresh_buttons();
    });
    auto *delete_shortcut = new QShortcut(QKeySequence::Delete, table);
    connect(delete_shortcut, &QShortcut::activated, this, [this]() {
        with_selected(UiAction::ExportFolderRemove);
    });
    connect(apply, &QPushButton::clicked, this, &QDialog::close);
    connect(cancel, &QPushButton::clicked, this, &QDialog::close);

    set_state(state);
}

void HydrusExportFoldersDialog::set_state(const FolderPageState &state) {
    QString selected_name;

    if (table->currentRow() >= 0 && table->item(table->currentRow(), 0) != nullptr) {
        selected_name =
            table->item(table->currentRow(), 0)->data(Qt::UserRole).toString();
    }

    const rust::Vec<FolderRow> &folders = state.folders;

    table->setSortingEnabled(false);
    table->setRowCount(int(folders.size()));

    for (size_t index = 0; index < folders.size(); ++index) {
        const FolderRow &folder = folders[index];
        const int row = int(index);
        const QString name = to_qstring(folder.name);
        const QStringList cells = {
            name,
            to_qstring(folder.path),
            to_qstring(folder.mode_text),
            to_qstring(folder.query_text),
            to_qstring(folder.period_text),
            to_qstring(folder.phrase_text),
            to_qstring(folder.error_text),
        };

        for (int column = 0; column < cells.size(); ++column) {
            auto *item = table->item(row, column);

            if (item == nullptr) {
                item = new QTableWidgetItem();
                table->setItem(row, column, item);
            }

            item->setText(cells[column]);
            item->setToolTip(cells[column]);
        }

        table->item(row, 0)->setData(Qt::UserRole, name);
    }

    table->setSortingEnabled(true);
    table->sortItems(0, Qt::AscendingOrder);

    if (!selected_name.isEmpty()) {
        for (int row = 0; row < table->rowCount(); ++row) {
            if (table->item(row, 0)->data(Qt::UserRole).toString() == selected_name) {
                table->selectRow(row);
                break;
            }
        }
    }

    refresh_buttons();
}

void HydrusExportFoldersDialog::refresh_buttons() {
    const int selected = table->selectionModel()->selectedRows().size();
    edit_button->setEnabled(selected == 1);
    delete_button->setEnabled(selected >= 1);
}

void HydrusExportFoldersDialog::with_selected(UiAction action) {
    const QModelIndexList rows = table->selectionModel()->selectedRows();

    if (action == UiAction::ExportFolderEdit && rows.size() != 1) {
        return;
    }

    QStringList names;

    for (const QModelIndex &index : rows) {
        if (table->item(index.row(), 0) != nullptr) {
            names.append(table->item(index.row(), 0)->data(Qt::UserRole).toString());
        }
    }

    for (const QString &name : names) {
        main->notify(action, QString(), name);
    }
}

void HydrusMainWindow::show_export_folders_dialog(const FolderPageState &state) {
    if (export_folders_dialog == nullptr) {
        export_folders_dialog = new HydrusExportFoldersDialog(this, state, this);
        connect(export_folders_dialog, &QObject::destroyed, this, [this]() {
            export_folders_dialog = nullptr;
        });
    } else {
        export_folders_dialog->set_state(state);
    }

    export_folders_dialog->show();
    export_folders_dialog->raise();
    export_folders_dialog->activateWindow();
}

void HydrusMainWindow::update_export_folders_dialog(const FolderPageState &state) {
    if (export_folders_dialog != nullptr) {
        export_folders_dialog->set_state(state);
    }
}

HydrusExportFoldersPage::HydrusExportFoldersPage(HydrusMainWindow *main_window, const QString &page_id, const FolderPageState &state)
    : QWidget(), main(main_window), id(page_id) {
    auto *layout = new QVBoxLayout(this);
    layout->setContentsMargins(6, 6, 6, 6);

    status = new QLabel(this);
    status->setWordWrap(true);
    layout->addWidget(status);

    table = new QTableWidget(this);
    table->setColumnCount(8);
    table->setHorizontalHeaderLabels({
        QStringLiteral("name"),
        QStringLiteral("destination"),
        QStringLiteral("search"),
        QStringLiteral("filename"),
        QStringLiteral("mode"),
        QStringLiteral("sidecar"),
        QStringLiteral("status"),
        QStringLiteral("last exported")
    });
    table->horizontalHeader()->setStretchLastSection(true);
    table->horizontalHeader()->setSectionResizeMode(1, QHeaderView::Stretch);
    table->horizontalHeader()->setSectionResizeMode(2, QHeaderView::Stretch);
    table->horizontalHeader()->setSectionResizeMode(3, QHeaderView::Stretch);
    table->setSelectionBehavior(QAbstractItemView::SelectRows);
    table->setSelectionMode(QAbstractItemView::SingleSelection);
    table->setEditTriggers(QAbstractItemView::NoEditTriggers);
    table->verticalHeader()->setVisible(false);
    layout->addWidget(table, 1);

    auto *buttons = new QHBoxLayout();
    auto *add = new QPushButton(QStringLiteral("add folder..."), this);
    auto *edit = new QPushButton(QStringLiteral("edit..."), this);
    auto *run = new QPushButton(QStringLiteral("run now"), this);
    auto *review = new QPushButton(QStringLiteral("review files..."), this);
    auto *toggle = new QPushButton(QStringLiteral("pause/resume"), this);
    auto *reveal = new QPushButton(QStringLiteral("open folder"), this);
    auto *remove = new QPushButton(QStringLiteral("remove"), this);
    buttons->addWidget(add);
    buttons->addWidget(edit);
    buttons->addWidget(run);
    buttons->addWidget(review);
    buttons->addWidget(toggle);
    buttons->addWidget(reveal);
    buttons->addWidget(remove);
    buttons->addStretch(1);
    layout->addLayout(buttons);

    connect(add, &QPushButton::clicked, this, [this]() {
        main->notify(UiAction::ExportFolderEdit, id);
    });
    connect(edit, &QPushButton::clicked, this, [this]() { with_selected(UiAction::ExportFolderEdit); });
    connect(run, &QPushButton::clicked, this, [this]() { with_selected(UiAction::ExportFolderRun); });
    connect(review, &QPushButton::clicked, this, [this]() { with_selected(UiAction::ExportFolderReview); });
    connect(toggle, &QPushButton::clicked, this, [this]() { with_selected(UiAction::ExportFolderToggle); });
    connect(reveal, &QPushButton::clicked, this, [this]() { with_selected(UiAction::ExportFolderReveal); });
    connect(remove, &QPushButton::clicked, this, [this]() { with_selected(UiAction::ExportFolderRemove); });

    update_state(state);
}

const QString &HydrusExportFoldersPage::page_id() const {
    return id;
}

void HydrusExportFoldersPage::with_selected(UiAction action) {
    const int row = table->currentRow();

    if (row < 0 || table->item(row, 0) == nullptr) {
        return;
    }

    main->notify(action, id, table->item(row, 0)->data(Qt::UserRole).toString());
}

void HydrusExportFoldersPage::update_state(const FolderPageState &state) {
    status->setText(to_qstring(state.summary));

    const rust::Vec<FolderRow> &folders = state.folders;
    table->setRowCount(int(folders.size()));

    for (size_t index = 0; index < folders.size(); ++index) {
        const FolderRow &folder = folders[index];
        const int row = int(index);

        for (int column = 0; column < 8; ++column) {
            if (table->item(row, column) == nullptr) {
                table->setItem(row, column, new QTableWidgetItem());
            }
        }

        auto *name = table->item(row, 0);
        name->setText(to_qstring(folder.name));
        name->setData(Qt::UserRole, to_qstring(folder.name));

        if (folder.paused) {
            name->setForeground(QColor(160, 160, 160));
        } else {
            name->setForeground(QBrush());
        }

        table->item(row, 1)->setText(to_qstring(folder.path));
        table->item(row, 2)->setText(to_qstring(folder.query_text));
        table->item(row, 3)->setText(to_qstring(folder.phrase_text));
        table->item(row, 4)->setText(to_qstring(folder.mode_text));
        table->item(row, 5)->setText(to_qstring(folder.filters_text));
        table->item(row, 6)->setText(to_qstring(folder.status));
        table->item(row, 7)->setText(to_qstring(folder.last_queued));
    }

    table->resizeRowsToContents();
}
