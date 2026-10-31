#include "page_manager.hpp"

// WS-15.1.3c1: page manager moved from bridge.cpp (move-only split; inline
// methods outlined, no logic change).
// WS-15.2.2a: dialog contents arrive as a typed PageTree (ctor and tree
// building live in typed_seams.cpp); close actions call app_close_pages.

#include "bridge.hpp"
#include "support/support.hpp"

#include <QDir>
#include <QFileInfo>
#include <QFont>
#include <QHeaderView>
#include <QHBoxLayout>
#include <QLineEdit>
#include <QPushButton>
#include <QStyle>
#include <QTreeWidget>
#include <QVBoxLayout>

#include <utility>

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
    rust::Vec<rust::String> ids;

    for (QTreeWidgetItem *item : tree->selectedItems()) {
        bool ancestor_selected = false;

        for (QTreeWidgetItem *parent = item->parent(); parent != nullptr; parent = parent->parent()) {
            if (parent->isSelected()) {
                ancestor_selected = true;
                break;
            }
        }

        if (!ancestor_selected) {
            const QByteArray bytes = id_of(item).toUtf8();
            ids.push_back(rust::String(bytes.constData()));
        }
    }

    if (ids.size() == 0) {
        return;
    }

    app_close_pages(*main, std::move(ids));
    accept();
}

void HydrusPageManager::close_all_pages() {
    rust::Vec<rust::String> ids;

    for (const QString &id : top_levels) {
        const QByteArray bytes = id.toUtf8();
        ids.push_back(rust::String(bytes.constData()));
    }

    if (ids.size() == 0) {
        return;
    }

    app_close_pages(*main, std::move(ids));
    accept();
}
