#include "local_import_review.hpp"

// WS-15.1.3c2: local import review dialog moved verbatim from bridge.cpp (move-only split).

#include "bridge.hpp"
#include "support/support.hpp"

#include <QCheckBox>
#include <QFileDialog>
#include <QHBoxLayout>
#include <QHeaderView>
#include <QJsonArray>
#include <QLabel>
#include <QMessageBox>
#include <QPushButton>
#include <QSignalBlocker>
#include <QTableWidget>
#include <QVBoxLayout>

HydrusLocalImportReviewDialog::HydrusLocalImportReviewDialog(
    HydrusMainWindow *main_window, const QJsonObject &state, QWidget *parent)
    : QDialog(parent), main(main_window) {
    setWindowTitle(QStringLiteral("review files to import"));
    setObjectName(QStringLiteral("hydrus_local_import_review"));
    setAttribute(Qt::WA_DeleteOnClose, true);
    resize(960, 560);

    auto *layout = new QVBoxLayout(this);

    summary = new QLabel(this);
    summary->setWordWrap(true);
    layout->addWidget(summary);

    auto *controls = new QHBoxLayout();
    auto *add_files = new QPushButton(QStringLiteral("add files"), this);
    auto *add_folder = new QPushButton(QStringLiteral("add folder"), this);
    auto *remove = new QPushButton(QStringLiteral("remove files"), this);
    search = new QCheckBox(QStringLiteral("search subdirectories"), this);
    controls->addWidget(add_files);
    controls->addWidget(add_folder);
    controls->addWidget(remove);
    controls->addWidget(search);
    controls->addStretch(1);
    layout->addLayout(controls);

    table = new QTableWidget(this);
    table->setObjectName(QStringLiteral("local_import_paths"));
    table->setColumnCount(4);
    table->setHorizontalHeaderLabels({QStringLiteral("#"), QStringLiteral("path"),
                                      QStringLiteral("result / mime"), QStringLiteral("size")});
    table->horizontalHeader()->setSectionResizeMode(0, QHeaderView::ResizeToContents);
    table->horizontalHeader()->setSectionResizeMode(1, QHeaderView::Stretch);
    table->horizontalHeader()->setSectionResizeMode(2, QHeaderView::ResizeToContents);
    table->horizontalHeader()->setSectionResizeMode(3, QHeaderView::ResizeToContents);
    table->verticalHeader()->setVisible(false);
    table->setSelectionBehavior(QAbstractItemView::SelectRows);
    table->setSelectionMode(QAbstractItemView::ExtendedSelection);
    table->setEditTriggers(QAbstractItemView::NoEditTriggers);
    table->setSortingEnabled(true);
    layout->addWidget(table, 1);

    auto *delete_row = new QHBoxLayout();
    warning = new QLabel(this);
    warning->setObjectName(QStringLiteral("HydrusWarning"));
    delete_after_success = new QCheckBox(QStringLiteral("delete original files after successful import"), this);
    delete_row->addWidget(warning, 1);
    delete_row->addWidget(delete_after_success);
    layout->addLayout(delete_row);

    auto *buttons = new QHBoxLayout();
    auto *options = new QPushButton(QStringLiteral("import options..."), this);
    tag_button = new QPushButton(QStringLiteral("add tags/urls with the import >>"), this);
    tag_button->setToolTip(QStringLiteral(
        "Add tags from filenames to local tag services, then start the import."));
    import_button = new QPushButton(QStringLiteral("import now"), this);
    import_button->setObjectName(QStringLiteral("HydrusAccept"));
    auto *close_button = new QPushButton(QStringLiteral("close"), this);
    buttons->addWidget(options);
    buttons->addStretch(1);
    buttons->addWidget(tag_button);
    buttons->addWidget(import_button);
    buttons->addWidget(close_button);
    layout->addLayout(buttons);

    connect(add_files, &QPushButton::clicked, this, [this]() {
        const QStringList paths = QFileDialog::getOpenFileNames(
            this,
            QStringLiteral("import files"),
            QString(),
            QStringLiteral("all files (*.*)"));

        if (paths.isEmpty()) {
            return;
        }

        QJsonObject payload;
        QJsonArray array;
        for (const QString &path : paths) {
            array.append(path);
        }
        payload.insert(QStringLiteral("paths"), array);
        send(QStringLiteral("local_import_add_paths"), payload);
    });
    connect(add_folder, &QPushButton::clicked, this, [this]() {
        const QString path = QFileDialog::getExistingDirectory(
            this,
            QStringLiteral("Select a folder to add."));

        if (path.isEmpty()) {
            return;
        }

        QJsonObject payload;
        payload.insert(QStringLiteral("paths"), QJsonArray{path});
        send(QStringLiteral("local_import_add_paths"), payload);
    });
    connect(remove, &QPushButton::clicked, this, [this]() {
        const QStringList paths = selected_paths();

        if (paths.isEmpty()) {
            QMessageBox::information(this, windowTitle(), QStringLiteral("Select some paths first."));
            return;
        }

        QJsonObject payload;
        QJsonArray array;
        for (const QString &path : paths) {
            array.append(path);
        }
        payload.insert(QStringLiteral("paths"), array);
        send(QStringLiteral("local_import_remove"), payload);
    });
    connect(search, &QCheckBox::toggled, this, [this](bool checked) {
        QJsonObject payload;
        payload.insert(QStringLiteral("value"), checked);
        send(QStringLiteral("local_import_search_subdirectories"), payload);
    });
    connect(delete_after_success, &QCheckBox::toggled, this, [this](bool checked) {
        warning->setText(checked ? QStringLiteral("YOUR ORIGINAL FILES WILL BE DELETED")
                                 : QString());
        QJsonObject payload;
        payload.insert(QStringLiteral("value"), checked);
        send(QStringLiteral("local_import_delete_after_success"), payload);
    });
    connect(options, &QPushButton::clicked, this, [this]() {
        main->notify(QStringLiteral("local_import_options"));
    });
    connect(tag_button, &QPushButton::clicked, this, [this]() {
        main->notify(QStringLiteral("local_import_tagging_open"));
    });
    connect(import_button, &QPushButton::clicked, this, [this]() {
        main->notify(QStringLiteral("local_import_import"));
    });
    connect(close_button, &QPushButton::clicked, this, &QDialog::close);

    set_state(state);
}

QStringList HydrusLocalImportReviewDialog::selected_paths() const {
    QStringList paths;

    for (const QTableWidgetItem *item : table->selectedItems()) {
        if (item->column() != 1) {
            continue;
        }

        const QString path = item->data(Qt::UserRole).toString();

        if (!path.isEmpty() && !paths.contains(path)) {
            paths << path;
        }
    }

    return paths;
}

void HydrusLocalImportReviewDialog::send(const QString &action, const QJsonObject &payload) {
    main->notify(action, QString(), payload.isEmpty() ? QString() : json_payload(payload));
}

void HydrusLocalImportReviewDialog::set_state(const QJsonObject &state) {
    data = state;

    summary->setText(state.value(QStringLiteral("summary")).toString()
                     + (state.value(QStringLiteral("note")).toString().isEmpty()
                            ? QString()
                            : QStringLiteral("\n%1").arg(state.value(QStringLiteral("note")).toString())));

    {
        QSignalBlocker blocker(search);
        search->setChecked(state.value(QStringLiteral("search_subdirectories")).toBool(true));
    }
    {
        QSignalBlocker blocker(delete_after_success);
        const bool checked = state.value(QStringLiteral("delete_after_success")).toBool(false);
        delete_after_success->setChecked(checked);
        warning->setText(checked ? QStringLiteral("YOUR ORIGINAL FILES WILL BE DELETED")
                                 : QString());
    }

    const QStringList selected = selected_paths();

    table->setSortingEnabled(false);
    table->setRowCount(0);

    for (const QJsonValue &value : state.value(QStringLiteral("paths")).toArray()) {
        const QJsonObject parse = value.toObject();
        const QString path = parse.value(QStringLiteral("path")).toString();
        const int row = table->rowCount();
        table->insertRow(row);

        table->setItem(row, 0, new QTableWidgetItem(QString::number(parse.value(QStringLiteral("index")).toInt())));

        auto *path_item = new QTableWidgetItem(path);
        path_item->setData(Qt::UserRole, path);
        path_item->setToolTip(path);
        table->setItem(row, 1, path_item);

        auto *result_item = new QTableWidgetItem(parse.value(QStringLiteral("result_text")).toString());

        if (parse.value(QStringLiteral("bad")).toBool()) {
            result_item->setForeground(QColor(220, 110, 110));
        } else if (parse.value(QStringLiteral("sidecar")).toBool()) {
            result_item->setForeground(QColor(170, 170, 170));
        } else {
            result_item->setForeground(QColor(120, 190, 120));
        }

        table->setItem(row, 2, result_item);
        table->setItem(row, 3, new QTableWidgetItem(parse.value(QStringLiteral("size_display")).toString()));
    }

    table->setSortingEnabled(true);

    for (int row = 0; row < table->rowCount(); ++row) {
        if (selected.contains(table->item(row, 1)->data(Qt::UserRole).toString())) {
            table->selectRow(row);
        }
    }

    const bool can_import = state.value(QStringLiteral("can_import")).toBool(false);
    import_button->setEnabled(can_import);
    tag_button->setEnabled(can_import);
    import_button->setText(can_import
        ? QStringLiteral("import now (%1)").arg(state.value(QStringLiteral("good_count")).toInt())
        : QStringLiteral("import now"));
}

void HydrusMainWindow::show_local_import_review(const QJsonObject &state) {
    if (local_import_review_dialog == nullptr) {
        local_import_review_dialog = new HydrusLocalImportReviewDialog(this, state, this);
        connect(local_import_review_dialog, &QObject::destroyed, this, [this]() {
            local_import_review_dialog = nullptr;
        });
        local_import_review_dialog->show();
    } else {
        local_import_review_dialog->set_state(state);
        local_import_review_dialog->raise();
        local_import_review_dialog->activateWindow();
    }
}

void HydrusMainWindow::close_local_import_review() {
    if (local_import_tagging_dialog != nullptr) {
        local_import_tagging_dialog->close();
    }

    if (local_import_review_dialog != nullptr) {
        local_import_review_dialog->close();
    }
}
