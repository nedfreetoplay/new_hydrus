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

        QJsonArray hashes;

        for (const QString &hash : selected_hashes()) {
            hashes.append(hash);
        }

        const QByteArray payload = QJsonDocument(hashes).toJson(QJsonDocument::Compact);
        main->notify(
            QStringLiteral("selection_changed"),
            page_id_value,
            QString::fromUtf8(payload));
    });

    connect(this, &QListWidget::itemDoubleClicked, this, [this](QListWidgetItem *item) {
        if (main->building || item == nullptr) {
            return;
        }

        const QString hash = page_hash_from_item(item);

        if (!hash.isEmpty()) {
            QByteArray hash_bytes = hash.toUtf8();
            QByteArray page_bytes = page_id_value.toUtf8();
            app_action(*main, rust::Str("open_viewer", 11), rust::Str(page_bytes.constData(), page_bytes.size()), rust::Str(hash_bytes.constData(), hash_bytes.size()));
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

        QJsonArray hashes;

        for (const QString &hash : selected_hashes()) {
            hashes.append(hash);
        }

        const QByteArray payload = QJsonDocument(hashes).toJson(QJsonDocument::Compact);
        main->notify(
            QStringLiteral("selection_changed"),
            page_id_value,
            QString::fromUtf8(payload));
    });

    connect(this, &QListWidget::itemDoubleClicked, this, [this](QListWidgetItem *item) {
        if (main->building || item == nullptr) {
            return;
        }

        const QString hash = page_hash_from_item(item);

        if (!hash.isEmpty()) {
            QByteArray hash_bytes = hash.toUtf8();
            QByteArray page_bytes = page_id_value.toUtf8();
            app_action(*main, rust::Str("open_viewer", 11), rust::Str(page_bytes.constData(), page_bytes.size()), rust::Str(hash_bytes.constData(), hash_bytes.size()));
        }    });
}

void HydrusThumbnailGrid::set_layout(bool masonry_layout, bool horizontal_scroll, int scale) {
    masonry = masonry_layout;
    horizontal = horizontal_scroll;
    scale_percent = qBound(25, scale, 400);

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

void HydrusThumbnailGrid::set_media(const QJsonArray &media) {
    const QString focused = focused_hash();

    QSignalBlocker blocker(this);

    clear();

    const int base_width = qBound(48, 150 * scale_percent / 100, 900);
    const int base_height = qBound(40, base_width * 5 / 6, 900);
    setIconSize(QSize(base_width, base_height));

    if (!masonry) {
        setGridSize(QSize(base_width + 12, base_height + 12));
    }

    if (media.isEmpty()) {
        return;
    }

    for (const QJsonValue &value : media) {
        const QJsonObject object = value.toObject();
        const QString hash = object.value(QStringLiteral("hash")).toString();
        const QString thumb = object.value(QStringLiteral("thumb")).toString();

        auto *item = new QListWidgetItem(this);
        item->setData(Qt::UserRole, hash);
        item->setData(Qt::UserRole + 1, object.value(QStringLiteral("file")).toString());
        item->setData(Qt::UserRole + 2, object.value(QStringLiteral("url")).toString());

        QStringList badges;

        if (object.value(QStringLiteral("inbox")).toBool(false)) {
            badges.append(QStringLiteral("inbox"));
        }

        if (object.value(QStringLiteral("duplicate")).toBool(false)) {
            badges.append(QStringLiteral("dup"));
        }

        if (object.contains(QStringLiteral("rating"))
            && !object.value(QStringLiteral("rating")).isNull()) {
            badges.append(QString::number(object.value(QStringLiteral("rating")).toDouble(), 'g', 3));
        }

        QPixmap preview = thumbnail_pixmap(thumb);

        if (!preview.isNull()) {
            if (masonry) {
                const int height = qBound(40, preview.height() * base_width / qMax(1, preview.width()), base_width * 3);
                preview = preview.scaled(
                    base_width,
                    height,
                    Qt::KeepAspectRatio,
                    Qt::SmoothTransformation);
            } else {
                preview = preview.scaled(
                    base_width,
                    base_height,
                    Qt::KeepAspectRatio,
                    Qt::SmoothTransformation);
            }
        }

        if (!badges.isEmpty() && !preview.isNull()) {
            QPainter painter(&preview);
            painter.setRenderHint(QPainter::Antialiasing);
            QFont badge_font = painter.font();
            badge_font.setBold(true);
            badge_font.setPointSize(8);
            painter.setFont(badge_font);

            const QRect bar(0, preview.height() - 18, preview.width(), 18);
            painter.fillRect(bar, QColor(0, 0, 0, 150));
            painter.setPen(QColor(240, 240, 240));
            painter.drawText(
                bar.adjusted(4, 0, -4, 0),
                Qt::AlignLeft | Qt::AlignVCenter,
                badges.join(QLatin1Char(' ')));
            painter.end();
        }

        item->setIcon(QIcon(preview));
        item->setToolTip(hash);

        if (masonry) {
            const int height = preview.isNull() ? base_height : preview.height();
            item->setSizeHint(QSize(base_width + 8, height + 8));
        } else {
            item->setSizeHint(QSize(base_width + 8, base_height + 8));
        }

        item->setTextAlignment(Qt::AlignCenter);
    }

    set_focused(focused);
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
