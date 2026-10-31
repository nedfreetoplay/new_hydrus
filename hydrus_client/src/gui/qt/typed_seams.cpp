// WS-15.2.1: typed seam implementations (Rust structs, no JSON).
//
// This translation unit is the home for HydrusMainWindow methods whose
// parameters are cxx shared structs owned by api.rs. It includes ONLY the
// generated header <hydrus_client/src/gui/qt/api.cxx.h> (plus Qt and the
// declaration-only support/strings.hpp and support/icons.hpp): the generated
// header pulls the exported header set, so including local topic headers
// here would define every widget class twice. As more seams convert in
// WS-15.2.2, their typed method bodies land here (split per screen if it
// grows past 500 lines).
//
// WS-15.2.2a: main-menu bar, notebook contents, page progress and the
// manage-pages dialog (pages/tabs/tree/main menu group).

#include <hydrus_client/src/gui/qt/api.cxx.h>

#include "support/icons.hpp"
#include "support/strings.hpp"

#include <QAction>
#include <QDesktopServices>
#include <QDialog>
#include <QDialogButtonBox>
#include <QDir>
#include <QFileInfo>
#include <QFont>
#include <QFormLayout>
#include <QHeaderView>
#include <QLabel>
#include <QLineEdit>
#include <QMenu>
#include <QMenuBar>
#include <QPlainTextEdit>
#include <QPushButton>
#include <QTabWidget>
#include <QTableWidget>
#include <QTreeWidget>
#include <QUrl>
#include <QVBoxLayout>

#include <vector>

// The class definition (members, other methods) comes from the exported
// bridge.hpp chain included above; the method declarations live in
// main_window.hpp next to forward declarations of the structs.

void HydrusMainWindow::set_status(const StatusBar &status) {
    const QString media = to_qstring(status.media);
    const QString message = to_qstring(status.message);

    status_media->setText(media.isEmpty() ? message : media);
    status_media->setToolTip(message.isEmpty() ? media : message);
    status_bandwidth->setText(to_qstring(status.bandwidth));
    status_idle->setText(to_qstring(status.idle));
    status_hydrus_busy->setText(to_qstring(status.hydrus_busy));
    status_cpu_busy->setText(to_qstring(status.cpu_busy));
    status_db->setText(to_qstring(status.db));
}

void HydrusMainWindow::show_about(const AboutInfo &state) {
    QDialog dialog(this);
    dialog.setWindowTitle(QStringLiteral("about"));
    dialog.resize(680, 520);

    auto *layout = new QVBoxLayout(&dialog);

    auto *title = new QLabel(QStringLiteral("Hydrus Network"), &dialog);
    title->setStyleSheet(QStringLiteral("font-size: 20px; font-weight: bold;"));
    layout->addWidget(title);

    auto *subtitle = new QLabel(
        QStringLiteral("Rust/Qt client · software v%1 · network v%2")
            .arg(state.software_version)
            .arg(state.network_version),
        &dialog);
    layout->addWidget(subtitle);

    auto *tabs = new QTabWidget(&dialog);
    layout->addWidget(tabs, 1);

    auto *overview = new QWidget(tabs);
    auto *form = new QFormLayout(overview);
    const auto add_row = [form](const QString &label, const QString &value) {
        auto *field = new QLabel(value, form->parentWidget());
        field->setTextInteractionFlags(Qt::TextSelectableByMouse);
        field->setWordWrap(true);
        form->addRow(label, field);
    };

    add_row(QStringLiteral("software version"), QString::number(state.software_version));
    add_row(QStringLiteral("network version"), QString::number(state.network_version));
    add_row(QStringLiteral("client api version"), QString::number(state.client_api_version));
    add_row(QStringLiteral("db version"), QString::number(state.db_version));
    add_row(QStringLiteral("client api"), to_qstring(state.api));
    add_row(QStringLiteral("sqlite"), to_qstring(state.sqlite));
    add_row(QStringLiteral("rust crate"), to_qstring(state.rust_crate));
    add_row(QStringLiteral("Qt"), QString::fromLatin1(qVersion()));
    add_row(QStringLiteral("services"), QString::number(state.services));
    add_row(QStringLiteral("file locations"), QString::number(state.locations));
    add_row(QStringLiteral("shortcuts"), QString::number(state.shortcuts));
    add_row(QStringLiteral("db dir"), to_qstring(state.db_dir));
    add_row(QStringLiteral("install dir"), to_qstring(state.install_dir));
    tabs->addTab(overview, QStringLiteral("overview"));

    auto *license = new QPlainTextEdit(tabs);
    license->setReadOnly(true);
    license->setPlainText(
        to_qstring(state.license) + QStringLiteral("\n\n") + to_qstring(state.site));
    tabs->addTab(license, QStringLiteral("license"));

    auto *credits = new QPlainTextEdit(tabs);
    credits->setReadOnly(true);
    credits->setPlainText(to_qstring(state.credits));
    tabs->addTab(credits, QStringLiteral("credits"));

    auto *buttons = new QDialogButtonBox(QDialogButtonBox::Close, &dialog);
    auto *site = new QPushButton(QStringLiteral("open site"), &dialog);
    buttons->addButton(site, QDialogButtonBox::ActionRole);
    const QString site_url = to_qstring(state.site);
    connect(site, &QPushButton::clicked, &dialog, [site_url]() {
        QDesktopServices::openUrl(QUrl(site_url));
    });
    connect(buttons, &QDialogButtonBox::rejected, &dialog, &QDialog::accept);
    layout->addWidget(buttons);

    dialog.exec();
}

void HydrusMainWindow::show_viewing_stats(const ViewingStats &state) {
    QDialog dialog(this);
    dialog.setWindowTitle(QStringLiteral("file viewing statistics"));
    dialog.resize(560, 260);

    auto *layout = new QVBoxLayout(&dialog);
    auto *hash = new QLabel(
        QStringLiteral("file: %1").arg(to_qstring(state.hash)),
        &dialog);
    hash->setTextInteractionFlags(Qt::TextSelectableByMouse);
    layout->addWidget(hash);

    auto *table = new QTableWidget(&dialog);
    table->setColumnCount(4);
    table->setHorizontalHeaderLabels({
        QStringLiteral("canvas"),
        QStringLiteral("views"),
        QStringLiteral("viewtime (s)"),
        QStringLiteral("last viewed"),
    });
    table->verticalHeader()->setVisible(false);
    table->setEditTriggers(QAbstractItemView::NoEditTriggers);
    table->horizontalHeader()->setSectionResizeMode(QHeaderView::Stretch);

    table->setRowCount(static_cast<int>(state.rows.size()));

    for (size_t row = 0; row < state.rows.size(); ++row) {
        const ViewingStatRow &entry = state.rows[row];
        const int table_row = static_cast<int>(row);
        table->setItem(table_row, 0, new QTableWidgetItem(to_qstring(entry.canvas)));
        table->setItem(table_row, 1, new QTableWidgetItem(QString::number(entry.views)));
        // Parity: the JSON seam rendered this integer through toDouble, so keep
        // the double formatting for large values instead of switching to int.
        table->setItem(
            table_row, 2,
            new QTableWidgetItem(QString::number(static_cast<double>(entry.viewtime_secs))));
        table->setItem(table_row, 3, new QTableWidgetItem(to_qstring(entry.last_viewed)));
    }

    layout->addWidget(table, 1);

    auto *buttons = new QDialogButtonBox(QDialogButtonBox::Close, &dialog);
    connect(buttons, &QDialogButtonBox::rejected, &dialog, &QDialog::accept);
    layout->addWidget(buttons);

    dialog.exec();
}

namespace {
// WS-15.2.2a: parses the per-screen JSON blobs kept inside PageRow.state
// (R1 exception: arbitrary per-screen shape until later subtasks).
QJsonObject parse_string(const rust::String &text);
} // namespace

// WS-15.2.2a: main-menu bar as flat rows. Depth 0 rows are top-level menus;
// deeper rows are their items; a submenu row owns the rows that follow it
// at depth + 1. Mirrors the old recursive QJsonArray reader row for row.
void HydrusMainWindow::set_menus(const MenuBar &state) {
    menuBar()->clear();

    std::vector<QMenu *> stack;

    for (const MenuEntry &entry : state.entries) {
        if (entry.depth <= 0) {
            QMenu *menu = menuBar()->addMenu(to_qstring(entry.title));
            menu->setToolTipsVisible(true);
            stack.clear();
            stack.push_back(menu);
            continue;
        }

        while (static_cast<int>(stack.size()) > entry.depth) {
            stack.pop_back();
        }

        if (stack.empty()) {
            continue;
        }

        QMenu *parent = stack.back();

        if (entry.is_separator) {
            parent->addSeparator();
            continue;
        }

        if (entry.is_submenu) {
            QMenu *submenu = parent->addMenu(to_qstring(entry.label));
            submenu->setToolTipsVisible(true);
            stack.push_back(submenu);
            continue;
        }

        const QString label = to_qstring(entry.label);

        if (label.isEmpty()) {
            continue;
        }

        QAction *action = parent->addAction(label);
        action->setEnabled(entry.enabled);

        if (entry.checkable) {
            action->setCheckable(true);
            action->setChecked(entry.checked);
        }

        const QString tooltip = to_qstring(entry.tooltip);

        if (!tooltip.isEmpty()) {
            action->setToolTip(tooltip);
        }

        const QString action_id = to_qstring(entry.action);
        const QString item_target = to_qstring(entry.target);
        const QString payload = to_qstring(entry.payload);

        connect(action, &QAction::triggered, this, [this, action_id, item_target, payload]() {
            if (!action_id.isEmpty()) {
                notify(action_id, item_target, payload);
            }
        });
    }
}

// WS-15.2.2a: notebook contents as flat preorder rows. The envelope
// (id/title/kind/current/depth) is typed; `state` keeps the per-screen JSON
// blob (R1 exception) that each screen converts in later subtasks.
void HydrusMainWindow::build_pages(const Pages &state) {
    if (root == nullptr) {
        return;
    }

    building = true;

    this->pages.clear();
    downloaders.clear();
    subscriptions.clear();
    duplicates.clear();
    import_folders.clear();
    export_folders.clear();
    notebooks.clear();
    notebooks.insert(QStringLiteral("root"), root);

    clear_notebook(root);

    size_t cursor = 0;
    populate_notebook(root, state, cursor, 0, to_qstring(state.current));

    QString current = root->current_page_id();
    QString parent = QStringLiteral("root");

    while (HydrusNotebook *nested = notebooks.value(current, nullptr)) {
        parent = current;
        current = nested->current_page_id();
    }

    active_page = current;
    active_notebook = parent;

    building = false;
}

void HydrusMainWindow::populate_notebook(
    HydrusNotebook *notebook,
    const Pages &state,
    size_t &cursor,
    int depth,
    const QString &current) {
    const rust::Vec<PageRow> &rows = state.rows;

    while (cursor < rows.size()) {
        const PageRow &row = rows[cursor];

        if (row.depth < depth) {
            break;
        }

        if (row.depth > depth) {
            ++cursor;
            continue;
        }

        ++cursor;

        const QString page_id = to_qstring(row.id);
        const QString title = to_qstring(row.title);

        if (row.kind == PageKind::Notebook) {
            auto *nested = new HydrusNotebook(this, page_id, notebook);
            notebooks.insert(page_id, nested);
            notebook->add_page_widget(
                page_id,
                title.isEmpty() ? QStringLiteral("page of pages") : title,
                nested);
            populate_notebook(nested, state, cursor, depth + 1, to_qstring(row.current));
        } else if (row.kind == PageKind::Query) {
            auto *page = new HydrusQueryPage(this, page_id, parse_string(row.state));
            this->pages.insert(page_id, page);
            notebook->add_page_widget(
                page_id, title.isEmpty() ? QStringLiteral("files") : title, page);
        } else if (row.kind == PageKind::Downloader) {
            auto *page = new HydrusDownloaderPage(this, page_id, parse_string(row.state));
            downloaders.insert(page_id, page);
            notebook->add_page_widget(
                page_id, title.isEmpty() ? QStringLiteral("download") : title, page);
        } else if (row.kind == PageKind::Subscriptions) {
            auto *page = new HydrusSubscriptionsPage(this, page_id, parse_string(row.state));
            subscriptions.insert(page_id, page);
            notebook->add_page_widget(
                page_id, title.isEmpty() ? QStringLiteral("subscriptions") : title, page);
        } else if (row.kind == PageKind::Duplicates) {
            auto *page = new HydrusDuplicatesPage(this, page_id, parse_string(row.state));
            duplicates.insert(page_id, page);
            notebook->add_page_widget(
                page_id, title.isEmpty() ? QStringLiteral("duplicates") : title, page);
        } else if (row.kind == PageKind::ImportFolders) {
            auto *page = new HydrusImportFoldersPage(this, page_id, parse_string(row.state));
            import_folders.insert(page_id, page);
            notebook->add_page_widget(
                page_id, title.isEmpty() ? QStringLiteral("import folders") : title, page);
        } else if (row.kind == PageKind::ExportFolders) {
            auto *page = new HydrusExportFoldersPage(this, page_id, parse_string(row.state));
            export_folders.insert(page_id, page);
            notebook->add_page_widget(
                page_id, title.isEmpty() ? QStringLiteral("export folders") : title, page);
        } else {
            const QString message =
                parse_string(row.state).value(QStringLiteral("message")).toString();
            auto *placeholder = new QWidget(notebook);
            auto *layout = new QVBoxLayout(placeholder);
            auto *label = new QLabel(
                message.isEmpty() ? QStringLiteral("Unknown page type.") : message,
                placeholder);
            label->setAlignment(Qt::AlignCenter);
            label->setWordWrap(true);
            layout->addWidget(label);
            notebook->add_page_widget(
                page_id, title.isEmpty() ? QStringLiteral("page") : title, placeholder);
        }
    }

    notebook->finalize_notebook(current);
}

void HydrusMainWindow::update_page_progress(const PageProgress &progress) {
    if (HydrusQueryPage *page = find_page(to_qstring(progress.page_id))) {
        page->update_progress(progress.loading);
    }
}

namespace {

QJsonObject parse_string(const rust::String &text) {
    return parse_object(rust::Str(text.data(), text.size()));
}

QString page_kind_name(PageKind kind) {
    switch (kind) {
    case PageKind::Notebook:
        return QStringLiteral("notebook");
    case PageKind::Query:
        return QStringLiteral("query");
    case PageKind::Downloader:
        return QStringLiteral("downloader");
    case PageKind::Subscriptions:
        return QStringLiteral("subscriptions");
    case PageKind::Duplicates:
        return QStringLiteral("duplicates");
    case PageKind::ImportFolders:
        return QStringLiteral("import_folders");
    case PageKind::ExportFolders:
        return QStringLiteral("export_folders");
    case PageKind::Placeholder:
        return QStringLiteral("placeholder");
    }

    return QStringLiteral("placeholder");
}

} // namespace

HydrusPageManager::HydrusPageManager(
    HydrusMainWindow *main_window,
    const PageTree &page_tree,
    QWidget *parent)
    : QDialog(parent), main(main_window) {
    setWindowTitle(QStringLiteral("manage pages"));
    resize(560, 560);

    auto *layout = new QVBoxLayout(this);

    filter = new QLineEdit(this);
    filter->setPlaceholderText(QStringLiteral("filter pages..."));
    layout->addWidget(filter);

    tree = new QTreeWidget(this);
    tree->setColumnCount(3);
    tree->setHeaderLabels({QStringLiteral("page"), QStringLiteral("type"), QStringLiteral("files")});
    tree->setSelectionMode(QAbstractItemView::ExtendedSelection);
    tree->header()->setSectionResizeMode(0, QHeaderView::Stretch);
    tree->header()->setSectionResizeMode(1, QHeaderView::ResizeToContents);
    tree->header()->setSectionResizeMode(2, QHeaderView::ResizeToContents);
    layout->addWidget(tree, 1);

    auto *buttons = new QHBoxLayout();
    auto *activate = new QPushButton(QStringLiteral("activate"), this);
    auto *close_selected = new QPushButton(QStringLiteral("close selected"), this);
    auto *close_all = new QPushButton(QStringLiteral("close all"), this);
    auto *close_dialog = new QPushButton(QStringLiteral("close"), this);

    buttons->addWidget(activate);
    buttons->addWidget(close_selected);
    buttons->addWidget(close_all);
    buttons->addStretch(1);
    buttons->addWidget(close_dialog);
    layout->addLayout(buttons);

    build(page_tree);

    connect(filter, &QLineEdit::textChanged, this, [this](const QString &text) {
        apply_filter(text.trimmed());
    });
    connect(activate, &QPushButton::clicked, this, [this]() { activate_current(); });
    connect(close_selected, &QPushButton::clicked, this, [this]() { close_selected_pages(); });
    connect(close_all, &QPushButton::clicked, this, [this]() { close_all_pages(); });
    connect(close_dialog, &QPushButton::clicked, this, &QDialog::reject);
    connect(tree, &QTreeWidget::itemDoubleClicked, this, [this](QTreeWidgetItem *, int) {
        activate_current();
    });
}

void HydrusPageManager::build(const PageTree &page_tree) {
    tree->clear();
    top_levels.clear();

    std::vector<QTreeWidgetItem *> stack;

    for (const PageTreeRow &row : page_tree.rows) {
        while (static_cast<int>(stack.size()) > row.depth) {
            stack.pop_back();
        }

        QTreeWidgetItem *parent = stack.empty() ? nullptr : stack.back();
        auto *item =
            parent == nullptr ? new QTreeWidgetItem(tree) : new QTreeWidgetItem(parent);

        const QString id = to_qstring(row.id);
        const QString title = to_qstring(row.title);
        const QString kind = page_kind_name(row.kind);

        item->setText(0, title.isEmpty() ? QStringLiteral("page") : title);
        item->setText(1, kind);
        item->setText(2, QString::number(static_cast<qlonglong>(row.count)));
        item->setIcon(0, icon_for(kind));
        item->setData(0, Qt::UserRole, id);

        if (row.active) {
            QFont font = item->font(0);
            font.setBold(true);
            item->setFont(0, font);
        }

        if (row.depth == 0) {
            top_levels.append(id);
        }

        stack.push_back(item);
    }

    tree->expandAll();
}
