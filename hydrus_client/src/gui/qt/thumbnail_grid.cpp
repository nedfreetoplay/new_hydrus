#include "thumbnail_grid.hpp"

// WS-15.1.2b: base thumbnail-grid widget moved verbatim from bridge.cpp (move-only split).

#include "bridge.hpp"
#include "support/support.hpp"

#include <QColor>
#include <QFont>
#include <QIcon>
#include <QJsonDocument>
#include <QJsonObject>
#include <QJsonValue>
#include <QPainter>
#include <QPixmap>
#include <QSignalBlocker>

HydrusThumbnailGrid::HydrusThumbnailGrid(HydrusMainWindow *main_window, HydrusQueryPage *query_page)
    : QListWidget(query_page), main(main_window), page(query_page) {
    page_id_value = query_page->page_id();
    setObjectName(QStringLiteral("HydrusThumbnailGrid"));
    setViewMode(QListWidget::IconMode);
    setResizeMode(QListWidget::Adjust);
    setMovement(QListWidget::Static);
    setUniformItemSizes(true);
    setSpacing(4);
    setWordWrap(false);
    setTextElideMode(Qt::ElideNone);
    setIconSize(QSize(150, 125));
    setGridSize(QSize(162, 137));
    setSelectionMode(QAbstractItemView::ExtendedSelection);
    setDragEnabled(true);
    setAcceptDrops(true);
    setDragDropMode(QAbstractItemView::DragDrop);
    setVerticalScrollMode(QAbstractItemView::ScrollPerPixel);
    setHorizontalScrollBarPolicy(Qt::ScrollBarAlwaysOff);
    setStyleSheet(QStringLiteral(
        "QListWidget#HydrusThumbnailGrid { background-color: #ffffff; border: none; }"
        "QListWidget#HydrusThumbnailGrid::item { border: 1px solid #dfe3e6; background: #ffffff; }"
        "QListWidget#HydrusThumbnailGrid::item:selected { border: 1px solid #01111a; background: #d9f2ff; }"));

    connect(this, &QListWidget::currentItemChanged, this, [this](QListWidgetItem *current, QListWidgetItem *) {
        if (main->building || current == nullptr || page == nullptr) {
            return;
        }

        const QString hash = page_hash_from_item(current);

        if (!hash.isEmpty()) {
            // Keep a deterministic focused hash inside the page.
            if (page->focused_hash() != hash) {
                page->focus_hash(hash);
            }
        }
    });

    connect(this, &QListWidget::itemSelectionChanged, this, [this]() {
        if (main->building) {
            return;
        }

        rust::Vec<rust::String> hashes;

        for (const QString &hash : selected_hashes()) {
            const QByteArray bytes = hash.toUtf8();
            hashes.push_back(rust::String(bytes.constData()));
        }

        const QByteArray page_bytes = page_id_value.toUtf8();
        app_set_selection(
            *main,
            rust::Str(page_bytes.constData(), page_bytes.size()),
            std::move(hashes));
    });

    connect(this, &QListWidget::itemDoubleClicked, this, [this](QListWidgetItem *item) {
        if (main->building || item == nullptr) {
            return;
        }

        const QString hash = page_hash_from_item(item);

        if (!hash.isEmpty()) {
            const QByteArray hash_bytes = hash.toUtf8();
            const QByteArray page_bytes = page_id_value.toUtf8();
            app_open_viewer(*main, rust::Str(page_bytes.constData(), page_bytes.size()), rust::Str(hash_bytes.constData(), hash_bytes.size()));
        }    });
}

HydrusThumbnailGrid::HydrusThumbnailGrid(HydrusMainWindow *main_window, HydrusDuplicatesPage *dup_page_ptr)
    : QListWidget(dup_page_ptr), main(main_window), page(nullptr), dup_page(dup_page_ptr) {
    page_id_value = dup_page_ptr->page_id();
    setObjectName(QStringLiteral("HydrusThumbnailGrid"));
    setViewMode(QListWidget::IconMode);
    setResizeMode(QListWidget::Adjust);
    setMovement(QListWidget::Static);
    setUniformItemSizes(true);
    setSpacing(4);
    setWordWrap(false);
    setTextElideMode(Qt::ElideNone);
    setIconSize(QSize(150, 125));
    setGridSize(QSize(162, 137));
    setSelectionMode(QAbstractItemView::ExtendedSelection);
    setDragEnabled(true);
    setAcceptDrops(true);
    setDragDropMode(QAbstractItemView::DragDrop);
    setVerticalScrollMode(QAbstractItemView::ScrollPerPixel);
    setHorizontalScrollBarPolicy(Qt::ScrollBarAlwaysOff);
    setStyleSheet(QStringLiteral(
        "QListWidget#HydrusThumbnailGrid { background-color: #ffffff; border: none; }"
        "QListWidget#HydrusThumbnailGrid::item { border: 1px solid #dfe3e6; background: #ffffff; }"
        "QListWidget#HydrusThumbnailGrid::item:selected { border: 1px solid #01111a; background: #d9f2ff; }"));

    connect(this, &QListWidget::currentItemChanged, this, [this](QListWidgetItem *current, QListWidgetItem *) {
        if (main->building || current == nullptr) {
            return;
        }
        // no focused hash tracking for the duplicates page
    });

    connect(this, &QListWidget::itemSelectionChanged, this, [this]() {
        if (main->building) {
            return;
        }

        rust::Vec<rust::String> hashes;

        for (const QString &hash : selected_hashes()) {
            const QByteArray bytes = hash.toUtf8();
            hashes.push_back(rust::String(bytes.constData()));
        }

        const QByteArray page_bytes = page_id_value.toUtf8();
        app_set_selection(
            *main,
            rust::Str(page_bytes.constData(), page_bytes.size()),
            std::move(hashes));
    });

    connect(this, &QListWidget::itemDoubleClicked, this, [this](QListWidgetItem *item) {
        if (main->building || item == nullptr) {
            return;
        }

        const QString hash = page_hash_from_item(item);

        if (!hash.isEmpty()) {
            const QByteArray hash_bytes = hash.toUtf8();
            const QByteArray page_bytes = page_id_value.toUtf8();
            app_open_viewer(*main, rust::Str(page_bytes.constData(), page_bytes.size()), rust::Str(hash_bytes.constData(), hash_bytes.size()));
        }    });
}

void HydrusThumbnailGrid::set_layout(bool masonry_layout, bool horizontal_scroll, int scale) {
    masonry = masonry_layout;
    horizontal = horizontal_scroll;
    scale_percent = static_cast<int>(hydrus_metric_scale_clamp(scale));

    setFlow(horizontal ? QListView::TopToBottom : QListView::LeftToRight);
    setWrapping(true);
    setUniformItemSizes(!masonry);
    setHorizontalScrollBarPolicy(
        horizontal ? Qt::ScrollBarAsNeeded : Qt::ScrollBarAlwaysOff);
    setVerticalScrollBarPolicy(
        horizontal ? Qt::ScrollBarAlwaysOff : Qt::ScrollBarAsNeeded);

    if (masonry) {
        setGridSize(QSize());
    }
}

void HydrusThumbnailGrid::set_focused(const QString &hash) {
    if (hash.isEmpty()) {
        return;
    }

    for (int index = 0; index < count(); ++index) {
        QListWidgetItem *item = this->item(index);

        if (item != nullptr && page_hash_from_item(item) == hash) {
            QSignalBlocker blocker(this);
            setCurrentItem(item);
            scrollToItem(item, QAbstractItemView::EnsureVisible);
            return;
        }
    }
}

QString HydrusThumbnailGrid::focused_hash() const {
    return page_hash_from_item(currentItem());
}

QStringList HydrusThumbnailGrid::selected_hashes() const {
    QStringList hashes;

    for (QListWidgetItem *item : selectedItems()) {
        const QString hash = page_hash_from_item(item);

        if (!hash.isEmpty()) {
            hashes.append(hash);
        }
    }

    return hashes;
}
