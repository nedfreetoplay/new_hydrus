#include <hydrus_client/src/gui/qt/api.cxx.h>

#include "support/support.hpp"

#include <QDialog>
#include <QHBoxLayout>
#include <QHeaderView>
#include <QLabel>
#include <QPushButton>
#include <QTableWidget>
#include <QVBoxLayout>

// WS-15.1.5: import folders manager dialog + page moved verbatim from bridge.cpp (move-only split).

HydrusImportFoldersDialog::HydrusImportFoldersDialog(
    HydrusMainWindow *main_window, const FolderPageState &state, QWidget *parent)
    : QDialog(parent), main(main_window) {
    setWindowTitle(QStringLiteral("edit import folders"));
    setObjectName(QStringLiteral("edit_import_folders_dialog"));
    setAttribute(Qt::WA_DeleteOnClose, true);
    resize(760, 260);

    auto *layout = new QVBoxLayout(this);
    layout->setContentsMargins(8, 8, 8, 8);
    layout->setSpacing(6);

    auto *intro = new QLabel(
        QStringLiteral("Here you can set the client to regularly check certain folders for "
                       "new files to import."),
        this);
    intro->setWordWrap(true);
    layout->addWidget(intro);

    auto *warning = new QLabel(
        QStringLiteral("WARNING: Import folders check (and potentially move/delete!) the "
                       "contents of all subdirectories as well as the base directory!"),
        this);
    warning->setObjectName(QStringLiteral("HydrusWarning"));
    warning->setWordWrap(true);
    layout->addWidget(warning);

    table = new QTableWidget(this);
    table->setColumnCount(4);
    table->setHorizontalHeaderLabels({QStringLiteral("name"),
                                      QStringLiteral("path"),
                                      QStringLiteral("paused"),
                                      QStringLiteral("check period")});
    table->verticalHeader()->setVisible(false);
    table->setSelectionBehavior(QAbstractItemView::SelectRows);
    table->setSelectionMode(QAbstractItemView::SingleSelection);
    table->setEditTriggers(QAbstractItemView::NoEditTriggers);
    table->setWordWrap(false);
    table->setTextElideMode(Qt::ElideRight);
    table->horizontalHeader()->setTextElideMode(Qt::ElideRight);
    table->horizontalHeader()->setDefaultAlignment(Qt::AlignLeft | Qt::AlignVCenter);
    table->horizontalHeader()->setSortIndicator(0, Qt::AscendingOrder);
    table->setColumnWidth(0, static_cast<int>(hydrus_metric_import_width(0)));
    table->setColumnWidth(1, static_cast<int>(hydrus_metric_import_width(1)));
    table->setColumnWidth(2, static_cast<int>(hydrus_metric_import_width(2)));
    table->setColumnWidth(3, static_cast<int>(hydrus_metric_import_width(3)));
    table->horizontalHeader()->setSectionResizeMode(1, QHeaderView::Stretch);
    table->setMinimumHeight(120);
    table->setMaximumHeight(190);
    layout->addWidget(table, 1);

    auto *buttons = new QHBoxLayout();
    buttons->addStretch(1);
    auto *add = new QPushButton(QStringLiteral("add"), this);
    edit_button = new QPushButton(QStringLiteral("edit"), this);
    delete_button = new QPushButton(QStringLiteral("delete"), this);
    add->setMinimumHeight(22);
    edit_button->setMinimumHeight(22);
    delete_button->setMinimumHeight(22);
    buttons->addWidget(add);
    buttons->addWidget(edit_button);
    buttons->addWidget(delete_button);
    layout->addLayout(buttons);

    auto *bottom = new QHBoxLayout();
    bottom->addStretch(1);
    auto *apply = new QPushButton(QStringLiteral("apply"), this);
    apply->setObjectName(QStringLiteral("HydrusAccept"));
    auto *cancel = new QPushButton(QStringLiteral("cancel"), this);
    cancel->setObjectName(QStringLiteral("HydrusCancel"));
    bottom->addWidget(apply);
    bottom->addWidget(cancel);
    layout->addLayout(bottom);

    connect(add, &QPushButton::clicked, this, [this]() {
        main->notify(UiAction::ImportFolderEdit, QString(), QString());
    });
    connect(edit_button, &QPushButton::clicked, this, [this]() {
        with_selected(UiAction::ImportFolderEdit);
    });
    connect(delete_button, &QPushButton::clicked, this, [this]() {
        with_selected(UiAction::ImportFolderRemove);
    });
    connect(table, &QTableWidget::cellDoubleClicked, this, [this](int, int) {
        with_selected(UiAction::ImportFolderEdit);
    });
    connect(table, &QTableWidget::itemSelectionChanged, this, [this]() {
        refresh_buttons();
    });
    connect(apply, &QPushButton::clicked, this, &QDialog::close);
    connect(cancel, &QPushButton::clicked, this, &QDialog::close);

    set_state(state);
}

void HydrusImportFoldersDialog::set_state(const FolderPageState &state) {
    QString selected_name;

    if (table->currentRow() >= 0 && table->item(table->currentRow(), 0) != nullptr) {
        selected_name =
            table->item(table->currentRow(), 0)->data(Qt::UserRole).toString();
    }

    const rust::Vec<FolderRow> &folders = state.folders;

    // WS-15.3.2d: row order is the Rust single source of truth.
    rust::Vec<rust::String> name_keys;
    name_keys.reserve(folders.size());
    for (const FolderRow &folder : folders) {
        name_keys.push_back(folder.name);
    }
    const rust::Vec<int64_t> order = hydrus_sort_order_strings(std::move(name_keys), true);

    table->setSortingEnabled(false);
    table->setRowCount(int(folders.size()));

    for (size_t pos = 0; pos < order.size(); ++pos) {
        const size_t index = size_t(order[pos]);
        const FolderRow &folder = folders[index];
        const QString name = to_qstring(folder.name);
        const QStringList cells = {
            name,
            to_qstring(folder.path),
            folder.paused ? QStringLiteral("yes") : QString(),
            to_qstring(folder.period_text),
        };

        for (int column = 0; column < cells.size(); ++column) {
            const int row = int(pos);
            auto *item = table->item(row, column);

            if (item == nullptr) {
                item = new QTableWidgetItem();
                table->setItem(row, column, item);
            }

            item->setText(cells[column]);
            item->setToolTip(cells[column]);
        }

        table->item(int(pos), 0)->setData(Qt::UserRole, name);
    }

    table->setSortingEnabled(true);

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

void HydrusImportFoldersDialog::refresh_buttons() {
    const int64_t selected = table->currentRow() >= 0 ? 1 : 0;
    edit_button->setEnabled(hydrus_avail_manager_edit(selected));
    delete_button->setEnabled(hydrus_avail_manager_delete(selected));
}

void HydrusImportFoldersDialog::with_selected(UiAction action) {
    const int row = table->currentRow();

    if (row < 0 || table->item(row, 0) == nullptr) {
        return;
    }

    main->notify(action, QString(), table->item(row, 0)->data(Qt::UserRole).toString());
}

void HydrusMainWindow::show_import_folders_dialog(const FolderPageState &state) {
    if (import_folders_dialog == nullptr) {
        import_folders_dialog = new HydrusImportFoldersDialog(this, state, this);
        connect(import_folders_dialog, &QObject::destroyed, this, [this]() {
            import_folders_dialog = nullptr;
        });
    } else {
        import_folders_dialog->set_state(state);
    }

    import_folders_dialog->show();
    import_folders_dialog->raise();
    import_folders_dialog->activateWindow();
}

void HydrusMainWindow::update_import_folders_dialog(const FolderPageState &state) {
    if (import_folders_dialog != nullptr) {
        import_folders_dialog->set_state(state);
    }
}

HydrusImportFoldersPage::HydrusImportFoldersPage(HydrusMainWindow *main_window, const QString &page_id, const FolderPageState &state)
    : QWidget(), main(main_window), id(page_id) {
    auto *layout = new QVBoxLayout(this);
    layout->setContentsMargins(6, 6, 6, 6);

    status = new QLabel(this);
    status->setWordWrap(true);
    layout->addWidget(status);

    table = new QTableWidget(this);
    table->setColumnCount(7);
    table->setHorizontalHeaderLabels({
        QStringLiteral("name"),
        QStringLiteral("path"),
        QStringLiteral("check every"),
        QStringLiteral("after import"),
        QStringLiteral("filters"),
        QStringLiteral("status"),
        QStringLiteral("last queued")
    });
    table->horizontalHeader()->setStretchLastSection(true);
    table->horizontalHeader()->setSectionResizeMode(1, QHeaderView::Stretch);
    table->horizontalHeader()->setSectionResizeMode(4, QHeaderView::Stretch);
    table->setSelectionBehavior(QAbstractItemView::SelectRows);
    table->setSelectionMode(QAbstractItemView::SingleSelection);
    table->setEditTriggers(QAbstractItemView::NoEditTriggers);
    table->verticalHeader()->setVisible(false);
    layout->addWidget(table, 1);

    auto *buttons = new QHBoxLayout();
    auto *add = new QPushButton(QStringLiteral("add folder..."), this);
    auto *edit = new QPushButton(QStringLiteral("edit..."), this);
    auto *check = new QPushButton(QStringLiteral("check now"), this);
    auto *toggle = new QPushButton(QStringLiteral("pause/resume"), this);
    auto *remove = new QPushButton(QStringLiteral("remove"), this);
    buttons->addWidget(add);
    buttons->addWidget(edit);
    buttons->addWidget(check);
    buttons->addWidget(toggle);
    buttons->addWidget(remove);
    buttons->addStretch(1);
    layout->addLayout(buttons);

    connect(add, &QPushButton::clicked, this, [this]() {
        main->notify(UiAction::ImportFolderEdit, id);
    });
    connect(edit, &QPushButton::clicked, this, [this]() { with_selected(UiAction::ImportFolderEdit); });
    connect(check, &QPushButton::clicked, this, [this]() { with_selected(UiAction::ImportFolderCheck); });
    connect(toggle, &QPushButton::clicked, this, [this]() { with_selected(UiAction::ImportFolderToggle); });
    connect(remove, &QPushButton::clicked, this, [this]() { with_selected(UiAction::ImportFolderRemove); });

    update_state(state);
}

const QString &HydrusImportFoldersPage::page_id() const {
    return id;
}

void HydrusImportFoldersPage::with_selected(UiAction action) {
    const int row = table->currentRow();

    if (row < 0 || table->item(row, 0) == nullptr) {
        return;
    }

    main->notify(action, id, table->item(row, 0)->data(Qt::UserRole).toString());
}

void HydrusImportFoldersPage::update_state(const FolderPageState &state) {
    status->setText(to_qstring(state.summary));

    const rust::Vec<FolderRow> &folders = state.folders;
    table->setRowCount(int(folders.size()));

    for (size_t index = 0; index < folders.size(); ++index) {
        const FolderRow &folder = folders[index];
        const int row = int(index);

        for (int column = 0; column < 7; ++column) {
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
        table->item(row, 2)->setText(to_qstring(folder.period_text));
        table->item(row, 3)->setText(to_qstring(folder.extra));
        table->item(row, 4)->setText(to_qstring(folder.filters_text));
        table->item(row, 5)->setText(to_qstring(folder.status));
        table->item(row, 6)->setText(to_qstring(folder.last_queued));
    }

    table->resizeRowsToContents();
}
