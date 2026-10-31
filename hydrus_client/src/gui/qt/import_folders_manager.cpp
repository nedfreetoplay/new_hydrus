#include "bridge.hpp"

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
    HydrusMainWindow *main_window, const QJsonObject &state, QWidget *parent)
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
    table->setColumnWidth(0, 200);
    table->setColumnWidth(1, 430);
    table->setColumnWidth(2, 70);
    table->setColumnWidth(3, 140);
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
        main->notify(QStringLiteral("import_folder_edit"), QString(), QString());
    });
    connect(edit_button, &QPushButton::clicked, this, [this]() {
        with_selected(QStringLiteral("import_folder_edit"));
    });
    connect(delete_button, &QPushButton::clicked, this, [this]() {
        with_selected(QStringLiteral("import_folder_remove"));
    });
    connect(table, &QTableWidget::cellDoubleClicked, this, [this](int, int) {
        with_selected(QStringLiteral("import_folder_edit"));
    });
    connect(table, &QTableWidget::itemSelectionChanged, this, [this]() {
        refresh_buttons();
    });
    connect(apply, &QPushButton::clicked, this, &QDialog::close);
    connect(cancel, &QPushButton::clicked, this, &QDialog::close);

    set_state(state);
}

void HydrusImportFoldersDialog::set_state(const QJsonObject &state) {
    data = state;

    QString selected_name;

    if (table->currentRow() >= 0 && table->item(table->currentRow(), 0) != nullptr) {
        selected_name =
            table->item(table->currentRow(), 0)->data(Qt::UserRole).toString();
    }

    const QJsonArray folders = state.value(QStringLiteral("folders")).toArray();

    table->setSortingEnabled(false);
    table->setRowCount(folders.size());

    for (int index = 0; index < folders.size(); ++index) {
        const QJsonObject folder = folders.at(index).toObject();
        const QString name = folder.value(QStringLiteral("name")).toString();
        const QStringList cells = {
            name,
            folder.value(QStringLiteral("path")).toString(),
            folder.value(QStringLiteral("paused")).toBool() ? QStringLiteral("yes") : QString(),
            folder.value(QStringLiteral("pretty_period")).toString(),
        };

        for (int column = 0; column < cells.size(); ++column) {
            auto *item = table->item(index, column);

            if (item == nullptr) {
                item = new QTableWidgetItem();
                table->setItem(index, column, item);
            }

            item->setText(cells[column]);
            item->setToolTip(cells[column]);
        }

        table->item(index, 0)->setData(Qt::UserRole, name);
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

void HydrusImportFoldersDialog::refresh_buttons() {
    const bool selected = table->currentRow() >= 0;
    edit_button->setEnabled(selected);
    delete_button->setEnabled(selected);
}

void HydrusImportFoldersDialog::with_selected(const QString &action) {
    const int row = table->currentRow();

    if (row < 0 || table->item(row, 0) == nullptr) {
        return;
    }

    main->notify(action, QString(), table->item(row, 0)->data(Qt::UserRole).toString());
}

void HydrusMainWindow::show_import_folders_dialog(const QJsonObject &state) {
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

void HydrusMainWindow::update_import_folders_dialog(const QJsonObject &state) {
    if (import_folders_dialog != nullptr) {
        import_folders_dialog->set_state(state);
    }
}

HydrusImportFoldersPage::HydrusImportFoldersPage(HydrusMainWindow *main_window, const QString &page_id, const QJsonObject &state)
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
        main->notify(QStringLiteral("import_folder_edit"), id);
    });
    connect(edit, &QPushButton::clicked, this, [this]() { with_selected(QStringLiteral("import_folder_edit")); });
    connect(check, &QPushButton::clicked, this, [this]() { with_selected(QStringLiteral("import_folder_check")); });
    connect(toggle, &QPushButton::clicked, this, [this]() { with_selected(QStringLiteral("import_folder_toggle")); });
    connect(remove, &QPushButton::clicked, this, [this]() { with_selected(QStringLiteral("import_folder_remove")); });

    update_state(state);
}

const QString &HydrusImportFoldersPage::page_id() const {
    return id;
}

void HydrusImportFoldersPage::with_selected(const QString &action) {
    const int row = table->currentRow();

    if (row < 0 || table->item(row, 0) == nullptr) {
        return;
    }

    main->notify(action, id, table->item(row, 0)->data(Qt::UserRole).toString());
}

void HydrusImportFoldersPage::update_state(const QJsonObject &state) {
    status->setText(state.value(QStringLiteral("status")).toString());

    const QJsonArray folders = state.value(QStringLiteral("folders")).toArray();
    table->setRowCount(folders.size());

    for (int index = 0; index < folders.size(); ++index) {
        const QJsonObject folder = folders.at(index).toObject();

        for (int column = 0; column < 7; ++column) {
            if (table->item(index, column) == nullptr) {
                table->setItem(index, column, new QTableWidgetItem());
            }
        }

        auto *name = table->item(index, 0);
        name->setText(folder.value(QStringLiteral("name")).toString());
        name->setData(Qt::UserRole, folder.value(QStringLiteral("name")).toString());

        if (folder.value(QStringLiteral("paused")).toBool()) {
            name->setForeground(QColor(160, 160, 160));
        } else {
            name->setForeground(QBrush());
        }

        const QString action = folder.value(QStringLiteral("action")).toString();
        QString action_text = action;

        if (action == QStringLiteral("move")) {
            action_text = QStringLiteral("move -> %1").arg(
                folder.value(QStringLiteral("action_path")).toString());
        }

        QStringList filters;
        const QString extensions = folder.value(QStringLiteral("extensions")).toString().trimmed();
        const qint64 min_size = folder.value(QStringLiteral("min_size")).toVariant().toLongLong();
        const qint64 max_size = folder.value(QStringLiteral("max_size")).toVariant().toLongLong();
        const qint64 skip = folder.value(QStringLiteral("last_modified_skip")).toVariant().toLongLong();

        if (!extensions.isEmpty()) {
            filters.append(QStringLiteral("ext: %1").arg(extensions));
        }

        if (min_size > 0 || max_size > 0) {
            filters.append(QStringLiteral("%1 - %2 bytes").arg(min_size).arg(
                max_size > 0 ? QString::number(max_size) : QStringLiteral("any")));
        }

        if (skip > 0) {
            filters.append(QStringLiteral("skip files newer than %1 s").arg(skip));
        }

        table->item(index, 1)->setText(folder.value(QStringLiteral("path")).toString());
        table->item(index, 2)->setText(
            QStringLiteral("%1 s").arg(folder.value(QStringLiteral("period")).toInt()));
        table->item(index, 3)->setText(action_text);
        table->item(index, 4)->setText(filters.isEmpty() ? QStringLiteral("any") : filters.join(QStringLiteral("; ")));
        table->item(index, 5)->setText(folder.value(QStringLiteral("status")).toString());
        table->item(index, 6)->setText(QString::number(folder.value(QStringLiteral("last_imported")).toInt()));
    }

    table->resizeRowsToContents();
}
