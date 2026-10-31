#include "page_manager.hpp"

// WS-15.1.3c1: page manager moved from bridge.cpp (move-only split; inline
// methods outlined, no logic change).

#include "bridge.hpp"
#include "support/support.hpp"

#include <QDir>
#include <QFileInfo>
#include <QFont>
#include <QHeaderView>
#include <QHBoxLayout>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QJsonValue>
#include <QLineEdit>
#include <QPushButton>
#include <QStyle>
#include <QTreeWidget>
#include <QVBoxLayout>

HydrusPageManager::HydrusPageManager(HydrusMainWindow *main_window, const QJsonObject &state, QWidget *parent)
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

    build(state.value(QStringLiteral("root")).toObject());

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

QIcon HydrusPageManager::icon_for(const QString &kind) const {
    const QString file_name = hydrus_small_icon_file_name(kind);
    if (!file_name.isEmpty()) {
        const rust::String dir_rust = hydrus_static_dir();
        const QString dir = QString::fromUtf8(dir_rust.data(), static_cast<int>(dir_rust.size()));
        if (!dir.isEmpty()) {
            const QString path = QDir(dir).filePath(file_name);
            if (QFileInfo::exists(path)) {
                static QHash<QString, QIcon> cache;
                auto it = cache.find(path);
                if (it != cache.end()) {
                    return it.value();
                }
                QIcon icon(path);
                if (!icon.isNull()) {
                    cache.insert(path, icon);
                    return icon;
                }
            }
        }
    }

    if (kind == QStringLiteral("notebook")) {
        return style()->standardIcon(QStyle::SP_DirIcon);
    }

    if (kind == QStringLiteral("query")) {
        return style()->standardIcon(QStyle::SP_FileDialogContentsView);
    }

    if (kind == QStringLiteral("downloader")) {
        return style()->standardIcon(QStyle::SP_ArrowDown);
    }

    if (kind == QStringLiteral("subscriptions")) {
        return style()->standardIcon(QStyle::SP_BrowserReload);
    }

    if (kind == QStringLiteral("duplicates")) {
        return style()->standardIcon(QStyle::SP_FileDialogDetailedView);
    }

    if (kind == QStringLiteral("import_folders")) {
        return style()->standardIcon(QStyle::SP_DirOpenIcon);
    }

    return style()->standardIcon(QStyle::SP_MessageBoxQuestion);
}

void HydrusPageManager::build(const QJsonObject &root) {
    tree->clear();
    top_levels.clear();

    const QJsonArray children = root.value(QStringLiteral("children")).toArray();

    for (const QJsonValue &value : children) {
        const QJsonObject child = value.toObject();
        top_levels.append(child.value(QStringLiteral("id")).toString());
        add_node(nullptr, child);
    }

    tree->expandAll();
}

QTreeWidgetItem *HydrusPageManager::add_node(QTreeWidgetItem *parent, const QJsonObject &node) {
    const QString id = node.value(QStringLiteral("id")).toString();
    const QString title = node.value(QStringLiteral("title")).toString();
    const QString kind = node.value(QStringLiteral("kind")).toString();
    const int count = node.value(QStringLiteral("count")).toInt();
    const bool active = node.value(QStringLiteral("active")).toBool();

    auto *item = parent == nullptr ? new QTreeWidgetItem(tree) : new QTreeWidgetItem(parent);
    item->setText(0, title.isEmpty() ? QStringLiteral("page") : title);
    item->setText(1, kind);
    item->setText(2, QString::number(count));
    item->setIcon(0, icon_for(kind));
    item->setData(0, Qt::UserRole, id);

    if (active) {
        QFont font = item->font(0);
        font.setBold(true);
        item->setFont(0, font);
    }

    const QJsonArray children = node.value(QStringLiteral("children")).toArray();

    for (const QJsonValue &value : children) {
        add_node(item, value.toObject());
    }

    return item;
}

bool HydrusPageManager::filter_item(QTreeWidgetItem *item, const QString &needle) {
    bool child_visible = false;

    for (int index = 0; index < item->childCount(); ++index) {
        child_visible = filter_item(item->child(index), needle) || child_visible;
    }

    const bool matches = needle.isEmpty()
        || item->text(0).contains(needle, Qt::CaseInsensitive)
        || item->text(1).contains(needle, Qt::CaseInsensitive);
    const bool visible = matches || child_visible;

    item->setHidden(!visible);

    return visible;
}

void HydrusPageManager::apply_filter(const QString &needle) {
    for (int index = 0; index < tree->topLevelItemCount(); ++index) {
        filter_item(tree->topLevelItem(index), needle);
    }
}

QString HydrusPageManager::id_of(QTreeWidgetItem *item) const {
    return item == nullptr ? QString() : item->data(0, Qt::UserRole).toString();
}

void HydrusPageManager::activate_current() {
    const QString id = id_of(tree->currentItem());

    if (id.isEmpty()) {
        return;
    }

    main->notify(QStringLiteral("page_manager_activate"), id);
    accept();
}

void HydrusPageManager::close_selected_pages() {
    QJsonArray ids;

    for (QTreeWidgetItem *item : tree->selectedItems()) {
        bool ancestor_selected = false;

        for (QTreeWidgetItem *parent = item->parent(); parent != nullptr; parent = parent->parent()) {
            if (parent->isSelected()) {
                ancestor_selected = true;
                break;
            }
        }

        if (!ancestor_selected) {
            ids.append(id_of(item));
        }
    }

    if (ids.isEmpty()) {
        return;
    }

    const QByteArray payload = QJsonDocument(ids).toJson(QJsonDocument::Compact);
    main->notify(QStringLiteral("page_manager_close"), QString(), QString::fromUtf8(payload));
    accept();
}

void HydrusPageManager::close_all_pages() {
    QJsonArray ids;

    for (const QString &id : top_levels) {
        ids.append(id);
    }

    if (ids.isEmpty()) {
        return;
    }

    const QByteArray payload = QJsonDocument(ids).toJson(QJsonDocument::Compact);
    main->notify(QStringLiteral("page_manager_close"), QString(), QString::fromUtf8(payload));
    accept();
}
