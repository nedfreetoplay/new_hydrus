#include "bridge.hpp"

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
    HydrusMainWindow *main_window, const QJsonObject &state, QWidget *parent)
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
        main->notify(QStringLiteral("export_folder_edit"), QString(), QString());
    });
    connect(edit_button, &QPushButton::clicked, this, [this]() {
        with_selected(QStringLiteral("export_folder_edit"));
    });
    connect(delete_button, &QPushButton::clicked, this, [this]() {
        with_selected(QStringLiteral("export_folder_remove"));
    });
    connect(table, &QTableWidget::cellDoubleClicked, this, [this](int, int) {
        with_selected(QStringLiteral("export_folder_edit"));
    });
    connect(table, &QTableWidget::itemSelectionChanged, this, [this]() {
        refresh_buttons();
    });
    auto *delete_shortcut = new QShortcut(QKeySequence::Delete, table);
    connect(delete_shortcut, &QShortcut::activated, this, [this]() {
        with_selected(QStringLiteral("export_folder_remove"));
    });
    connect(apply, &QPushButton::clicked, this, &QDialog::close);
    connect(cancel, &QPushButton::clicked, this, &QDialog::close);

    set_state(state);
}

void HydrusExportFoldersDialog::set_state(const QJsonObject &state) {
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
            folder.value(QStringLiteral("pretty_type")).toString(),
            folder.value(QStringLiteral("query")).toString(),
            folder.value(QStringLiteral("pretty_period")).toString(),
            folder.value(QStringLiteral("phrase")).toString(),
            folder.value(QStringLiteral("last_error")).toString(),
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

void HydrusExportFoldersDialog::refresh_buttons() {
    const int selected = table->selectionModel()->selectedRows().size();
    edit_button->setEnabled(selected == 1);
    delete_button->setEnabled(selected >= 1);
}

void HydrusExportFoldersDialog::with_selected(const QString &action) {
    const QModelIndexList rows = table->selectionModel()->selectedRows();

    if (action == QStringLiteral("export_folder_edit") && rows.size() != 1) {
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

void HydrusMainWindow::show_export_folders_dialog(const QJsonObject &state) {
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

void HydrusMainWindow::update_export_folders_dialog(const QJsonObject &state) {
    if (export_folders_dialog != nullptr) {
        export_folders_dialog->set_state(state);
    }
}

HydrusExportFoldersPage::HydrusExportFoldersPage(HydrusMainWindow *main_window, const QString &page_id, const QJsonObject &state)
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
        main->notify(QStringLiteral("export_folder_edit"), id);
    });
    connect(edit, &QPushButton::clicked, this, [this]() { with_selected(QStringLiteral("export_folder_edit")); });
    connect(run, &QPushButton::clicked, this, [this]() { with_selected(QStringLiteral("export_folder_run")); });
    connect(review, &QPushButton::clicked, this, [this]() { with_selected(QStringLiteral("export_folder_review")); });
    connect(toggle, &QPushButton::clicked, this, [this]() { with_selected(QStringLiteral("export_folder_toggle")); });
    connect(reveal, &QPushButton::clicked, this, [this]() { with_selected(QStringLiteral("export_folder_reveal")); });
    connect(remove, &QPushButton::clicked, this, [this]() { with_selected(QStringLiteral("export_folder_remove")); });

    update_state(state);
}

const QString &HydrusExportFoldersPage::page_id() const {
    return id;
}

void HydrusExportFoldersPage::with_selected(const QString &action) {
    const int row = table->currentRow();

    if (row < 0 || table->item(row, 0) == nullptr) {
        return;
    }

    main->notify(action, id, table->item(row, 0)->data(Qt::UserRole).toString());
}

void HydrusExportFoldersPage::update_state(const QJsonObject &state) {
    status->setText(state.value(QStringLiteral("status")).toString());

    const QJsonArray folders = state.value(QStringLiteral("folders")).toArray();
    table->setRowCount(folders.size());

    for (int index = 0; index < folders.size(); ++index) {
        const QJsonObject folder = folders.at(index).toObject();

        for (int column = 0; column < 8; ++column) {
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

        const QString query = folder.value(QStringLiteral("query")).toString().trimmed();
        const QString mode = folder.value(QStringLiteral("export_type")).toString();
        const QString sidecar = folder.value(QStringLiteral("sidecar_format")).toString();

        QString mode_text = mode == QStringLiteral("synchronise")
            ? QStringLiteral("synchronise")
            : QStringLiteral("copy");

        if (folder.value(QStringLiteral("delete_from_client")).toBool()) {
            mode_text += QStringLiteral(" + delete from client");
        }

        table->item(index, 1)->setText(folder.value(QStringLiteral("path")).toString());
        table->item(index, 2)->setText(query.isEmpty() ? QStringLiteral("everything") : query);
        table->item(index, 3)->setText(folder.value(QStringLiteral("phrase")).toString());
        table->item(index, 4)->setText(mode_text);
        table->item(index, 5)->setText(sidecar == QStringLiteral("none") ? QStringLiteral("-") : sidecar);
        table->item(index, 6)->setText(folder.value(QStringLiteral("status")).toString());
        table->item(index, 7)->setText(QString::number(folder.value(QStringLiteral("last_exported")).toInt()));
    }

    table->resizeRowsToContents();
}
