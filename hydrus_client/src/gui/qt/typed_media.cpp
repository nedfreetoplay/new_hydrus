// WS-15.2.2b: typed thumbnail-grid and file-page seam bodies (Rust structs,
// no JSON). Covers the HydrusMainWindow media glue, HydrusThumbnailGrid and
// HydrusQueryPage; the viewer and metadata panel live in typed_viewer.cpp and
// typed_metadata.cpp (one file per screen, R3).
//
// Same rules as typed_seams.cpp: this translation unit includes ONLY the
// generated header <hydrus_client/src/gui/qt/api.cxx.h> (plus Qt and the
// declaration-only support headers). The generated header pulls the exported
// header set, so including local topic headers here would define every widget
// class twice.

#include <hydrus_client/src/gui/qt/api.cxx.h>

#include "support/pairs.hpp"
#include "support/sort_options.hpp"
#include "support/strings.hpp"
#include "support/widgets.hpp"

#include <QColor>
#include <QFont>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QJsonValue>
#include <QListWidget>
#include <QPainter>
#include <QPixmap>
#include <QSignalBlocker>
#include <QStringList>

// The class definitions (members, other methods) come from the exported
// bridge.hpp chain included above; the typed declarations live in the topic
// headers next to forward declarations of the structs.

// --- HydrusMainWindow glue -------------------------------------------------

void HydrusMainWindow::update_page(const QueryPageState &page) {
    if (HydrusQueryPage *query = find_page(to_qstring(page.page_id))) {
        query->update_state(page);
    }
}

void HydrusMainWindow::set_selection(const SelectionState &selection) {
    if (HydrusQueryPage *page = find_page(to_qstring(selection.page_id))) {
        page->set_selection(selection);
    }
}

void HydrusMainWindow::open_viewer(const ViewerState &state) {
    const QString viewer_id = to_qstring(state.viewer_id);

    if (viewer_id.isEmpty()) {
        return;
    }

    if (HydrusViewerWindow *existing = viewers.value(viewer_id, nullptr)) {
        existing->update_state(state);
        existing->raise();
        existing->activateWindow();
        return;
    }

    auto *viewer = new HydrusViewerWindow(this, state);
    viewer->setAttribute(Qt::WA_DeleteOnClose, true);
    viewers.insert(viewer_id, viewer);
    viewer->show();
}

void HydrusMainWindow::update_viewer(const ViewerState &state) {
    if (HydrusViewerWindow *viewer = viewers.value(to_qstring(state.viewer_id), nullptr)) {
        viewer->update_state(state);
    }
}

// --- HydrusThumbnailGrid ---------------------------------------------------

void HydrusThumbnailGrid::set_media(const rust::Vec<MediaItem> &media) {
    const QString focused = focused_hash();

    QSignalBlocker blocker(this);

    clear();

    const int base_width = qBound(48, 150 * scale_percent / 100, 900);
    const int base_height = qBound(40, base_width * 5 / 6, 900);
    setIconSize(QSize(base_width, base_height));

    if (!masonry) {
        setGridSize(QSize(base_width + 12, base_height + 12));
    }

    if (media.empty()) {
        return;
    }

    for (const MediaItem &entry : media) {
        auto *item = new QListWidgetItem(this);
        item->setData(Qt::UserRole, to_qstring(entry.hash));
        item->setData(Qt::UserRole + 1, to_qstring(entry.file));
        item->setData(Qt::UserRole + 2, to_qstring(entry.url));

        QStringList badges;

        if (entry.inbox) {
            badges.append(QStringLiteral("inbox"));
        }

        if (entry.duplicate) {
            badges.append(QStringLiteral("dup"));
        }

        if (entry.has_rating) {
            badges.append(QString::number(entry.rating, 'g', 3));
        }

        QPixmap preview = thumbnail_pixmap(to_qstring(entry.thumb));

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
        item->setToolTip(to_qstring(entry.hash));

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

// Compatibility overload for the duplicates media seam: the duplicates page
// still arrives as JSON until its own subtask and converts to the same rows.
void HydrusThumbnailGrid::set_media(const QJsonArray &media) {
    rust::Vec<MediaItem> items;

    for (const QJsonValue &value : media) {
        const QJsonObject object = value.toObject();
        MediaItem item;
        const auto assign = [](rust::String &field, const QJsonValue &text) {
            const QByteArray bytes = text.toString().toUtf8();
            field = rust::String(bytes.constData());
        };
        assign(item.hash, object.value(QStringLiteral("hash")));
        assign(item.thumb, object.value(QStringLiteral("thumb")));
        assign(item.file, object.value(QStringLiteral("file")));
        assign(item.url, object.value(QStringLiteral("url")));
        item.inbox = object.value(QStringLiteral("inbox")).toBool(false);
        item.duplicate = object.value(QStringLiteral("duplicate")).toBool(false);
        const QJsonValue rating = object.value(QStringLiteral("rating"));
        item.has_rating = object.contains(QStringLiteral("rating")) && !rating.isNull();
        item.rating = rating.toDouble(0.0);
        items.push_back(item);
    }

    set_media(items);
}

// --- HydrusQueryPage -------------------------------------------------------

void HydrusQueryPage::update_state(const QueryPageState &state) {
    const QString query = to_qstring(state.query);

    if (search->text() != query) {
        search->setText(query);
    }

    const QString location_text = to_qstring(state.location);
    location_value = location_text.isEmpty() ? QStringLiteral("my files") : location_text;
    location->setText(location_value);

    tag_services.clear();

    for (const rust::String &service : state.tag_services) {
        tag_services.append(to_qstring(service));
    }

    tag_service_value = to_qstring(state.tag_service);
    tag_service->setText(
        tag_service_value.isEmpty() ? QStringLiteral("all known tags") : tag_service_value);

    const QString sort_key = to_qstring(state.sort);

    if (!sort_key.isEmpty()) {
        const bool ascending = state.sort_ascending;
        const HydrusSortOption *option = find_sort_option(sort_key);
        sort_value = sort_key;
        sort_ascending_value = ascending;
        sort_type->setText(QString::fromUtf8(option->label));
        sort_order->setText(
            QString::fromUtf8(ascending ? option->ascending : option->descending));
    }

    predicates->clear();

    for (const rust::String &predicate : state.predicates) {
        const QString text = to_qstring(predicate);
        auto *item = new QListWidgetItem(text, predicates);
        item->setForeground(
            text.startsWith(QStringLiteral("system:")) ? QColor(150, 90, 10) : QColor(32, 32, 32));
    }

    selection_tags->clear();

    for (const rust::String &tag : state.tags) {
        selection_tags->addItem(to_qstring(tag));
    }

    selection_tags->sortItems(
        tag_sort_order->text() == QStringLiteral("a-z") ? Qt::AscendingOrder
                                                        : Qt::DescendingOrder);

    const QString focused = to_qstring(state.focused);

    grid->set_layout(state.thumb_masonry, state.thumb_horizontal, state.thumb_scale_percent);
    grid->set_media(state.media);
    update_progress(state.loading);

    if (!focused.isEmpty()) {
        grid->set_focused(focused);
    }
}

void HydrusQueryPage::set_selection(const SelectionState &selection) {
    QStringList tags;

    for (const rust::String &tag : selection.tags) {
        tags.append(to_qstring(tag));
    }

    selection_tags->clear();
    selection_tags->addItems(tags);
    selection_tags->sortItems(
        tag_sort_order->text() == QStringLiteral("a-z") ? Qt::AscendingOrder
                                                        : Qt::DescendingOrder);

    const QString focused = to_qstring(selection.hash);
    grid->set_focused(focused);

    const QString preview_path = to_qstring(selection.preview_path);

    if (!preview_path.isEmpty() && preview->load_image(preview_path)) {
        return;
    }

    preview->clear_image();
}

