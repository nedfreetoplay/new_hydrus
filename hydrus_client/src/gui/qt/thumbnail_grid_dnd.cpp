
// WS-15.1.2b: thumbnail-grid drag&drop, wheel zoom and context menu moved verbatim from bridge.cpp (move-only split).

#include <hydrus_client/src/gui/qt/api.cxx.h>
#include "support/support.hpp"

#include <QAction>
#include <QApplication>
#include <QClipboard>
#include <QDesktopServices>
#include <QDrag>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QMenu>
#include <QMimeData>
#include <QPixmap>
#include <QPoint>
#include <QSize>
#include <QUrl>

void HydrusThumbnailGrid::startDrag(Qt::DropActions supported_actions) {
    Q_UNUSED(supported_actions);

    const QList<QListWidgetItem *> items = selectedItems();

    if (items.isEmpty()) {
        return;
    }

    auto *mime = new QMimeData();
    QList<QUrl> urls;
    QJsonArray hashes;
    QPixmap preview;

    for (QListWidgetItem *item : items) {
        const QString hash = page_hash_from_item(item);

        if (hash.isEmpty()) {
            continue;
        }

        hashes.append(hash);
        const QString file = item->data(Qt::UserRole + 1).toString();

        if (!file.isEmpty()) {
            urls.append(QUrl::fromLocalFile(file));
        }

        if (preview.isNull()) {
            preview = item->icon().pixmap(QSize(96, 80));
        }
    }

    if (hashes.isEmpty()) {
        delete mime;
        return;
    }

    mime->setUrls(urls);
    mime->setData(
        QStringLiteral("application/x-hydrus-files"),
        QJsonDocument(hashes).toJson(QJsonDocument::Compact));

    QDrag drag(this);
    drag.setMimeData(mime);

    if (!preview.isNull()) {
        drag.setPixmap(preview);
        drag.setHotSpot(QPoint(preview.width() / 2, preview.height() / 2));
    }

    drag.exec(Qt::CopyAction);
}

void HydrusThumbnailGrid::dragEnterEvent(QDragEnterEvent *event) {
    if (event->mimeData()->hasFormat(QStringLiteral("application/x-hydrus-files"))
        || event->mimeData()->hasUrls()) {
        event->acceptProposedAction();
        return;
    }

    QListWidget::dragEnterEvent(event);
}

void HydrusThumbnailGrid::dragMoveEvent(QDragMoveEvent *event) {
    if (event->mimeData()->hasFormat(QStringLiteral("application/x-hydrus-files"))
        || event->mimeData()->hasUrls()) {
        event->acceptProposedAction();
        return;
    }

    QListWidget::dragMoveEvent(event);
}

void HydrusThumbnailGrid::dropEvent(QDropEvent *event) {
    if (event->mimeData()->hasFormat(QStringLiteral("application/x-hydrus-files"))) {
        const QByteArray payload = event->mimeData()->data(QStringLiteral("application/x-hydrus-files"));
        rust::Vec<rust::String> hashes;

        for (const QJsonValue &value : QJsonDocument::fromJson(payload).array()) {
            const QByteArray bytes = value.toString().toUtf8();
            hashes.push_back(rust::String(bytes.constData()));
        }

        const QByteArray page_bytes = page_id_value.toUtf8();
        app_add_hashes(
            *main,
            rust::Str(page_bytes.constData(), page_bytes.size()),
            std::move(hashes));
        event->acceptProposedAction();
        return;
    }

    if (event->mimeData()->hasUrls()) {
        QJsonArray paths;

        for (const QUrl &url : event->mimeData()->urls()) {
            if (url.isLocalFile()) {
                paths.append(url.toLocalFile());
            }
        }

        if (!paths.isEmpty()) {
            main->notify(UiAction::LocalImportOpenPaths,
                QString(),
                QString::fromUtf8(QJsonDocument(paths).toJson(QJsonDocument::Compact)));
            event->acceptProposedAction();
            return;
        }
    }

    QListWidget::dropEvent(event);
}

void HydrusThumbnailGrid::wheelEvent(QWheelEvent *event) {
    if (!(event->modifiers() & Qt::ControlModifier)) {
        QListWidget::wheelEvent(event);
        return;
    }

    const double factor = event->angleDelta().y() > 0 ? 1.15 : 1.0 / 1.15;
    const int width = qBound(80, static_cast<int>(iconSize().width() * factor), 300);
    const int height = width * 125 / 150;

    setIconSize(QSize(width, height));
    setGridSize(QSize(width + 12, height + 12));

    for (int index = 0; index < count(); ++index) {
        if (QListWidgetItem *entry = item(index)) {
            entry->setSizeHint(QSize(width + 8, height + 8));
        }
    }

    event->accept();
}

void HydrusThumbnailGrid::contextMenuEvent(QContextMenuEvent *event) {
    QListWidgetItem *item = itemAt(event->pos());

    if (item == nullptr || page_hash_from_item(item).isEmpty()) {
        return;
    }

    const QString hash = page_hash_from_item(item);

    if (!item->isSelected()) {
        clearSelection();
        item->setSelected(true);
    }

    setCurrentItem(item);

    QMenu menu(this);

    const QString file = item->data(Qt::UserRole + 1).toString();
    const QString url = item->data(Qt::UserRole + 2).toString();

    QAction *open = menu.addAction(QStringLiteral("open in media viewer"));
    QAction *external = menu.addAction(QStringLiteral("open externally"));
    external->setEnabled(!file.isEmpty());
    QAction *copy_hash = menu.addAction(QStringLiteral("copy hash"));
    QAction *copy_url = menu.addAction(QStringLiteral("copy url"));
    copy_url->setEnabled(!url.isEmpty());
    menu.addSeparator();
    QAction *tags = menu.addAction(QStringLiteral("manage tags..."));
    QAction *copy_tags = menu.addAction(QStringLiteral("copy tags"));
    QAction *paste_tags = menu.addAction(QStringLiteral("paste tags"));
    QAction *apply_tags = menu.addAction(QStringLiteral("apply tags to selection..."));
    QAction *notes = menu.addAction(QStringLiteral("manage notes..."));
    QAction *timestamps = menu.addAction(QStringLiteral("edit timestamps..."));
    QAction *viewing_stats = menu.addAction(QStringLiteral("file viewing statistics..."));
    auto *ratings = menu.addMenu(QStringLiteral("rating"));
    QAction *like = ratings->addAction(QStringLiteral("like"));
    QAction *dislike = ratings->addAction(QStringLiteral("dislike"));
    QAction *clear_rating = ratings->addAction(QStringLiteral("clear"));
    menu.addSeparator();
    QAction *archive = menu.addAction(QStringLiteral("archive"));
    QAction *inbox = menu.addAction(QStringLiteral("inbox"));
    QAction *remove = menu.addAction(QStringLiteral("delete..."));

    QAction *chosen = menu.exec(event->globalPos());

    if (chosen == nullptr) {
        return;
    }

    if (chosen == external) {
        if (!file.isEmpty()) {
            QDesktopServices::openUrl(QUrl::fromLocalFile(file));
        }

        return;
    }

    if (chosen == copy_hash) {
        QApplication::clipboard()->setText(hash);
        return;
    }

    if (chosen == copy_url) {
        if (!url.isEmpty()) {
            QApplication::clipboard()->setText(url);
        }

        return;
    }

    if (chosen == notes) {
        const QByteArray hash_bytes = hash.toUtf8();
        app_manage_notes(*main, rust::Str(hash_bytes.constData(), hash_bytes.size()));
        return;
    }

    if (chosen == timestamps) {
        main->notify(UiAction::EditTimestamps);
        return;
    }

    if (chosen == copy_tags) {
        main->notify(UiAction::CopyTags);
        return;
    }

    if (chosen == paste_tags) {
        main->notify(UiAction::PasteTags,
            QString(),
            QApplication::clipboard()->text());
        return;
    }

    if (chosen == apply_tags) {
        main->notify(UiAction::ApplyTagsSelection);
        return;
    }

    if (chosen == viewing_stats) {
        main->notify(UiAction::ViewingStats, page->page_id(), hash);
        return;
    }

    if (chosen == like || chosen == dislike || chosen == clear_rating) {
        // Parity: the old JSON seam sent no service key here, so the service
        // stays empty and Rust reports "rating service is missing".
        const QByteArray hash_bytes = hash.toUtf8();
        const QString action = chosen == like ? QStringLiteral("like")
            : chosen == dislike ? QStringLiteral("dislike")
                                : QStringLiteral("clear");
        const QByteArray action_bytes = action.toUtf8();
        app_set_rating(
            *main,
            rust::Str(hash_bytes.constData(), hash_bytes.size()),
            rust::Str(),
            rust::Str(action_bytes.constData(), action_bytes.size()),
            chosen == like ? 1.0 : 0.0);
        return;
    }

    UiAction action = UiAction::None;

    if (chosen == open) {
        action = UiAction::OpenViewer;
    } else if (chosen == tags) {
        action = UiAction::ManageTags;
    } else if (chosen == archive) {
        action = UiAction::ArchiveHashes;
    } else if (chosen == inbox) {
        action = UiAction::InboxHashes;
    } else if (chosen == remove) {
        action = UiAction::DeleteHashes;
    }

    if (action == UiAction::None) {
        return;
    }

    QString payload = hash;

    const bool hashes_action = action == UiAction::ArchiveHashes
        || action == UiAction::InboxHashes
        || action == UiAction::DeleteHashes;
    if (hashes_action) {
        QJsonArray hashes;
        const QStringList selected = selected_hashes();

        if (selected.isEmpty()) {
            hashes.append(hash);
        } else {
            for (const QString &selected_hash : selected) {
                hashes.append(selected_hash);
            }
        }

        payload = QString::fromUtf8(QJsonDocument(hashes).toJson(QJsonDocument::Compact));
    }

    const QByteArray page_bytes = page->page_id().toUtf8();
    const QByteArray payload_bytes = payload.toUtf8();
    app_action(*main, action, rust::Str(page_bytes.constData(), page_bytes.size()), rust::Str(payload_bytes.constData(), payload_bytes.size()));
}
