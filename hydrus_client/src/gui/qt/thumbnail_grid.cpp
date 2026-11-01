#include <hydrus_client/src/gui/qt/api.cxx.h>
#include "support/support.hpp">

#include <QColor>
#include <QFont>
#include <QIcon>
#include <QJsonDocument>
#include <QJsonObject>
#include <QJsonValue>
#include <QKeyEvent>
#include <QMouseEvent>
#include <QPainter>
#include <QPixmap>
#include <QSignalBlocker>

#include "bridge_api.hpp"

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

// WS-13.13: mouse gestures are configurable; Rust resolves the gesture to an
// action over the active set + `thumbnails -> media -> main_gui` chain.
static QString grid_gesture_for_button(
    Qt::MouseButton button, const QString &press_type, Qt::KeyboardModifiers modifiers) {
    QString name;
    switch (button) {
    case Qt::LeftButton: name = QStringLiteral("left"); break;
    case Qt::RightButton: name = QStringLiteral("right"); break;
    case Qt::MiddleButton: name = QStringLiteral("middle"); break;
    case Qt::BackButton: name = QStringLiteral("back"); break;
    case Qt::ForwardButton: name = QStringLiteral("forward"); break;
    case Qt::TaskButton: name = QStringLiteral("task"); break;
    default: name = QStringLiteral("left"); break;
    }
    QStringList parts;
    if (modifiers & Qt::ControlModifier) {
        parts << QStringLiteral("ctrl");
    }
    if (modifiers & Qt::AltModifier) {
        parts << QStringLiteral("alt");
    }
    if (modifiers & Qt::ShiftModifier) {
        parts << QStringLiteral("shift");
    }
    if (modifiers & Qt::MetaModifier) {
        parts << QStringLiteral("meta");
    }
    parts << name;
    return parts.join(QStringLiteral("+")) + QStringLiteral(" ") + press_type;
}

static QString grid_resolve(const QString &gesture) {
    const QByteArray context_bytes = QByteArrayLiteral("thumbnails");
    const QByteArray gesture_bytes = gesture.toUtf8();
    return to_qstring(hydrus_mouse_gesture_action(
        rust::Str(context_bytes.constData(), context_bytes.size()),
        rust::Str(gesture_bytes.constData(), gesture_bytes.size())));
}

static QString grid_key_resolve(const QString &combo) {
    const QByteArray context_bytes = QByteArrayLiteral("thumbnails");
    const QByteArray combo_bytes = combo.toUtf8();
    return to_qstring(hydrus_keyboard_action(
        rust::Str(context_bytes.constData(), context_bytes.size()),
        rust::Str(combo_bytes.constData(), combo_bytes.size())));
}

void HydrusThumbnailGrid::mousePressEvent(QMouseEvent *event) {
    const QString action =
        grid_resolve(grid_gesture_for_button(event->button(), QStringLiteral("press"), event->modifiers()));
    if (action == QStringLiteral("open_new_page")) {
        if (!page_id_value.isEmpty()) {
            const QByteArray page_bytes = page_id_value.toUtf8();
            app_action(
                *main, UiAction::OpenSelectionInNewPage,
                rust::Str(page_bytes.constData(), page_bytes.size()), rust::Str("", 0));
        }
        event->accept();
        return;
    }
    if (action == QStringLiteral("show_context_menu")) {
        // Let contextMenuEvent open the menu; swallow the press so selection
        // does not change first (matches Hydrus right-click behaviour).
        event->ignore();
        return;
    }
    // select / range_select / toggle_select (and unbound buttons): Qt's
    // ExtendedSelection already implements exactly these modifier semantics.
    QListWidget::mousePressEvent(event);
}

void HydrusThumbnailGrid::mouseDoubleClickEvent(QMouseEvent *event) {
    const QString action = grid_resolve(
        grid_gesture_for_button(event->button(), QStringLiteral("double"), event->modifiers()));
    if (!action.isEmpty() && action != QStringLiteral("open_viewer")) {
        // Bound to something else: do not open the viewer.
        event->ignore();
        return;
    }
    QListWidget::mouseDoubleClickEvent(event);
}

// WS-13.13: Enter opens the viewer for the current item, like Python's
// thumbnails-context keyboard binding (Resolve via Rust so custom sets apply).
void HydrusThumbnailGrid::keyPressEvent(QKeyEvent *event) {
    if (event->key() == Qt::Key_Enter || event->key() == Qt::Key_Return) {
        const QString key = event->key() == Qt::Key_Enter ? QStringLiteral("Enter") : QStringLiteral("Return");
        if (grid_key_resolve(key) == QStringLiteral("open_viewer")) {
            if (QListWidgetItem *item = currentItem()) {
                const QString hash = page_hash_from_item(item);
                if (!hash.isEmpty()) {
                    const QByteArray hash_bytes = hash.toUtf8();
                    const QByteArray page_bytes = page_id_value.toUtf8();
                    app_open_viewer(
                        *main,
                        rust::Str(page_bytes.constData(), page_bytes.size()),
                        rust::Str(hash_bytes.constData(), hash_bytes.size()));
                    event->accept();
                    return;
                }
            }
        }
    }
    QListWidget::keyPressEvent(event);
}
