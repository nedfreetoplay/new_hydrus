#include "bridge.hpp"

#include <QAction>
#include <QApplication>
#include <QCheckBox>
#include <QClipboard>
#include <QComboBox>
#include <QCompleter>
#include <QCoreApplication>
#include <QDebug>
#include <QDateTime>
#include <QDateTimeEdit>
#include <QDesktopServices>
#include <QDialog>
#include <QDialogButtonBox>
#include <QDir>
#include <QDoubleSpinBox>
#include <QDrag>
#include <QDragEnterEvent>
#include <QDropEvent>
#include <QFileDialog>
#include <QFileInfo>
#include <QFormLayout>
#include <QFrame>
#include <QGridLayout>
#include <QGroupBox>
#include <QHBoxLayout>
#include <QHeaderView>
#include <QInputDialog>
#include <QJsonDocument>
#include <QJsonParseError>
#include <QKeyEvent>
#include <QKeySequence>
#include <QLabel>
#include <QLineEdit>
#include <QListWidget>
#include <QMenu>
#include <QMenuBar>
#include <QMessageBox>
#include <QMimeData>
#include <QMovie>
#include <QMouseEvent>
#include <QPainter>
#include <QPlainTextEdit>
#include <QPixmap>
#include <QPixmapCache>
#include <QPushButton>
#include <QRegularExpression>
#include <QScreen>
#include <QScrollArea>
#include <QScrollBar>
#include <QSettings>
#include <QShortcut>
#include <QSignalBlocker>
#include <QSlider>
#include <QSpinBox>
#include <QSplitter>
#include <QStringListModel>
#include <QStackedWidget>
#include <QStatusBar>
#include <QStyle>
#include <QStyleFactory>
#include <QSystemTrayIcon>
#include <QTabBar>
#include <QTableWidget>
#include <QTextEdit>
#include <QTimer>
#include <QRandomGenerator>
#include <QSet>
#include <QStandardPaths>
#include <QToolButton>
#include <QTreeWidget>
#include <QUrl>
#include <QVBoxLayout>

#include <algorithm>
#include <functional>

void show_or_predicate_dialog(HydrusMainWindow &main, const QString &page_id, const QString &initial);

namespace {

QString to_qstring(rust::Str text) {
    return QString::fromUtf8(text.data(), static_cast<int>(text.size()));
}

QString from_rust(rust::Str text) {
    return QString::fromUtf8(text.data(), static_cast<int>(text.size()));
}

QJsonObject parse_object(rust::Str text) {
    QJsonParseError error{};
    const QJsonDocument document = QJsonDocument::fromJson(QByteArray(text.data(), static_cast<int>(text.size())), &error);

    if (error.error != QJsonParseError::NoError || !document.isObject()) {
        return {};
    }

    return document.object();
}

QJsonArray parse_array(rust::Str text) {
    QJsonParseError error{};
    const QJsonDocument document = QJsonDocument::fromJson(QByteArray(text.data(), static_cast<int>(text.size())), &error);

    if (error.error != QJsonParseError::NoError || !document.isArray()) {
        return {};
    }

    return document.array();
}

QStringList json_to_strings(const QJsonArray &array) {
    QStringList result;

    for (const QJsonValue &value : array) {
        result.append(value.toString());
    }

    return result;
}

QByteArray json_bytes(const QJsonObject &object) {
    return QJsonDocument(object).toJson(QJsonDocument::Compact);
}

QPixmap thumbnail_pixmap(const QString &path) {
    const int width = 150;
    const int height = 125;

    QPixmap box(width, height);
    box.fill(QColor(52, 52, 52));

    QPixmap source;

    if (!path.isEmpty()) {
        source.load(path);
    }

    QPainter painter(&box);

    if (source.isNull()) {
        painter.setPen(QColor(120, 128, 144));
        painter.drawRect(0, 0, width - 1, height - 1);
        painter.drawText(box.rect(), Qt::AlignCenter, QStringLiteral("no thumb"));
    } else {
        const QPixmap scaled = source.scaled(width, height, Qt::KeepAspectRatio, Qt::SmoothTransformation);
        painter.drawPixmap((width - scaled.width()) / 2, (height - scaled.height()) / 2, scaled);
    }

    painter.end();

    return box;
}

QString page_hash_from_item(const QListWidgetItem *item) {
    if (item == nullptr) {
        return {};
    }

    return item->data(Qt::UserRole).toString();
}

void update_line_edit(QLineEdit *edit, const QString &value) {
    if (edit != nullptr && !edit->hasFocus() && edit->text() != value) {
        edit->setText(value);
    }
}

QWidget *field_row(const QString &label, QWidget *field, QWidget *parent) {
    auto *row = new QWidget(parent);
    auto *layout = new QHBoxLayout(row);
    layout->setContentsMargins(0, 0, 0, 0);
    layout->setSpacing(6);

    auto *caption = new QLabel(label, row);
    caption->setMinimumWidth(90);
    layout->addWidget(caption);
    layout->addWidget(field, 1);

    return row;
}

QString json_payload(const QJsonObject &object) {
    return QString::fromUtf8(QJsonDocument(object).toJson(QJsonDocument::Compact));
}

} // namespace

// ---------------------------------------------------------------------------
// HydrusCanvas
// ---------------------------------------------------------------------------

HydrusCanvas::HydrusCanvas(QWidget *parent) : QGraphicsView(parent) {
    scene = new QGraphicsScene(this);
    setScene(scene);

    setRenderHints(QPainter::Antialiasing | QPainter::SmoothPixmapTransform);
    setDragMode(QGraphicsView::ScrollHandDrag);
    setTransformationAnchor(QGraphicsView::AnchorUnderMouse);
    setResizeAnchor(QGraphicsView::AnchorViewCenter);
    setFrameShape(QFrame::NoFrame);
    setBackgroundBrush(QColor(52, 52, 52));
    setMinimumSize(120, 120);
}

bool HydrusCanvas::load_image(const QString &path) {
    clear_image();

    auto *loaded_movie = new QMovie(path, QByteArray(), this);

    if (loaded_movie->isValid()) {
        movie = loaded_movie;
        pixmap_item = scene->addPixmap(movie->currentPixmap());
        scene->setSceneRect(movie->currentPixmap().rect());

        connect(movie, &QMovie::frameChanged, this, [this](int) {
            if (movie == nullptr || pixmap_item == nullptr) {
                return;
            }

            const QPixmap frame = movie->currentPixmap();
            pixmap_item->setPixmap(frame);
            scene->setSceneRect(frame.rect());

            if (fit_mode) {
                fitInView(scene->sceneRect(), Qt::KeepAspectRatio);
            }
        });

        movie->start();
        reset_zoom();

        return true;
    }

    delete loaded_movie;

    QPixmap pixmap;

    if (!QPixmapCache::find(path, &pixmap)) {
        pixmap.load(path);

        if (!pixmap.isNull()) {
            QPixmapCache::insert(path, pixmap);
        }
    }

    if (pixmap.isNull()) {
        clear_image();
        return false;
    }

    scene->clear();
    pixmap_item = scene->addPixmap(pixmap);
    scene->setSceneRect(pixmap.rect());
    reset_zoom();

    return true;
}

void HydrusCanvas::clear_image() {
    if (movie != nullptr) {
        movie->stop();
        movie->deleteLater();
        movie = nullptr;
    }

    scene->clear();
    pixmap_item = nullptr;
    resetTransform();
    fit_mode = true;
    rotation = 0;
}

void HydrusCanvas::toggle_animation() {
    if (movie == nullptr) {
        return;
    }

    if (movie->state() == QMovie::Running) {
        movie->setPaused(true);
    } else {
        movie->setPaused(false);
    }
}

bool HydrusCanvas::is_animated() const {
    return movie != nullptr;
}

void HydrusCanvas::fit_image() {
    if (pixmap_item != nullptr) {
        fitInView(scene->sceneRect(), Qt::KeepAspectRatio);
        fit_mode = true;
    }
}

void HydrusCanvas::reset_zoom() {
    resetTransform();
    rotation = 0;
    fit_image();
}

void HydrusCanvas::zoom_by(double factor) {
    if (pixmap_item == nullptr) {
        return;
    }

    fit_mode = false;
    scale(factor, factor);
}

void HydrusCanvas::zoom_to(double factor) {
    if (pixmap_item == nullptr) {
        return;
    }

    fit_mode = false;
    resetTransform();
    rotate(rotation);
    scale(factor, factor);
    centerOn(scene->sceneRect().center());
}

void HydrusCanvas::set_background_mode(const QString &mode) {
    if (mode == QStringLiteral("black")) {
        setBackgroundBrush(QColor(0, 0, 0));
        return;
    }

    if (mode == QStringLiteral("light")) {
        setBackgroundBrush(QColor(230, 230, 230));
        return;
    }

    if (mode == QStringLiteral("mid")) {
        setBackgroundBrush(QColor(96, 96, 96));
        return;
    }

    if (mode == QStringLiteral("checkerboard")) {
        QPixmap tile(16, 16);
        tile.fill(QColor(58, 58, 58));
        QPainter painter(&tile);
        painter.fillRect(QRect(0, 0, 8, 8), QColor(72, 72, 72));
        painter.fillRect(QRect(8, 8, 8, 8), QColor(72, 72, 72));
        painter.end();
        setBackgroundBrush(QBrush(tile));
        return;
    }

    setBackgroundBrush(QColor(52, 52, 52));
}

void HydrusCanvas::set_pan_mode(bool enabled) {
    setDragMode(enabled ? QGraphicsView::ScrollHandDrag : QGraphicsView::NoDrag);
    setCursor(enabled ? Qt::OpenHandCursor : Qt::ArrowCursor);
}

void HydrusCanvas::rotate_by(int degrees) {
    if (pixmap_item == nullptr) {
        return;
    }

    fit_mode = false;
    rotation = (rotation + degrees) % 360;
    resetTransform();
    rotate(rotation);
    fit_image();
    resetTransform();
    rotate(rotation);
    centerOn(scene->sceneRect().center());
}

void HydrusCanvas::set_recenter_on_resize(bool value) {
    recenter_on_resize = value;
}

void HydrusCanvas::resizeEvent(QResizeEvent *event) {
    QGraphicsView::resizeEvent(event);

    if (fit_mode) {
        fit_image();
    } else if (recenter_on_resize && pixmap_item != nullptr) {
        centerOn(scene->sceneRect().center());
    }
}

void HydrusCanvas::wheelEvent(QWheelEvent *event) {
    if (event->modifiers() & Qt::ControlModifier) {
        zoom_by(event->angleDelta().y() > 0 ? 1.15 : 1.0 / 1.15);
        event->accept();
        return;
    }

    QGraphicsView::wheelEvent(event);
}

// ---------------------------------------------------------------------------
// HydrusThumbnailGrid
// ---------------------------------------------------------------------------

HydrusThumbnailGrid::HydrusThumbnailGrid(HydrusMainWindow *main_window, HydrusQueryPage *query_page)
    : QListWidget(query_page), main(main_window), page(query_page) {
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
        "QListWidget#HydrusThumbnailGrid { background-color: #343434; border: none; }"
        "QListWidget#HydrusThumbnailGrid::item { border: 1px solid transparent; }"
        "QListWidget#HydrusThumbnailGrid::item:selected { border: 1px solid #dfe3e6; background: #707f8f; }"));

    connect(this, &QListWidget::currentItemChanged, this, [this](QListWidgetItem *current, QListWidgetItem *) {
        if (main->building || current == nullptr) {
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
            page->page_id(),
            QString::fromUtf8(payload));
    });

    connect(this, &QListWidget::itemDoubleClicked, this, [this](QListWidgetItem *item) {
        if (main->building || item == nullptr) {
            return;
        }

        const QString hash = page_hash_from_item(item);

        if (!hash.isEmpty()) {
            QByteArray hash_bytes = hash.toUtf8();
            QByteArray page_bytes = page->page_id().toUtf8();
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
        auto *empty = new QListWidgetItem(QStringLiteral("no files"), this);
        empty->setFlags(Qt::NoItemFlags);
        empty->setForeground(QColor(170, 170, 170));
        empty->setSizeHint(QSize(120, 32));
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
        main->notify(
            QStringLiteral("add_hashes_to_page"),
            page->page_id(),
            QString::fromUtf8(payload));
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
            main->notify(
                QStringLiteral("import_paths"),
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
        QJsonObject payload;
        payload.insert(QStringLiteral("hash"), hash);
        main->notify(
            QStringLiteral("notes_manage"),
            QString(),
            QString::fromUtf8(QJsonDocument(payload).toJson(QJsonDocument::Compact)));
        return;
    }

    if (chosen == timestamps) {
        main->notify(QStringLiteral("edit_timestamps"));
        return;
    }

    if (chosen == copy_tags) {
        main->notify(QStringLiteral("copy_tags"));
        return;
    }

    if (chosen == paste_tags) {
        main->notify(
            QStringLiteral("paste_tags"),
            QString(),
            QApplication::clipboard()->text());
        return;
    }

    if (chosen == apply_tags) {
        main->notify(QStringLiteral("apply_tags_selection"));
        return;
    }

    if (chosen == viewing_stats) {
        main->notify(QStringLiteral("viewing_stats"), page->page_id(), hash);
        return;
    }

    if (chosen == like || chosen == dislike || chosen == clear_rating) {
        QJsonObject payload;
        payload.insert(QStringLiteral("hash"), hash);
        payload.insert(
            QStringLiteral("action"),
            chosen == like ? QStringLiteral("like")
                           : chosen == dislike ? QStringLiteral("dislike")
                                               : QStringLiteral("clear"));
        payload.insert(QStringLiteral("value"), chosen == like ? 1.0 : 0.0);
        main->notify(
            QStringLiteral("rating_set"),
            QString(),
            QString::fromUtf8(QJsonDocument(payload).toJson(QJsonDocument::Compact)));
        return;
    }

    QString action;

    if (chosen == open) {
        action = QStringLiteral("open_viewer");
    } else if (chosen == tags) {
        action = QStringLiteral("manage_tags");
    } else if (chosen == archive) {
        action = QStringLiteral("archive_hashes");
    } else if (chosen == inbox) {
        action = QStringLiteral("inbox_hashes");
    } else if (chosen == remove) {
        action = QStringLiteral("delete_hashes");
    }

    if (action.isEmpty()) {
        return;
    }

    QString payload = hash;

    if (action.endsWith(QStringLiteral("_hashes"))) {
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

    const QByteArray action_bytes = action.toUtf8();
    const QByteArray page_bytes = page->page_id().toUtf8();
    const QByteArray payload_bytes = payload.toUtf8();
    app_action(*main, rust::Str(action_bytes.constData(), action_bytes.size()), rust::Str(page_bytes.constData(), page_bytes.size()), rust::Str(payload_bytes.constData(), payload_bytes.size()));
}

// ---------------------------------------------------------------------------
// HydrusMetadataPanel
// ---------------------------------------------------------------------------

HydrusMetadataPanel::HydrusMetadataPanel(HydrusMainWindow *main_window, QWidget *parent)
    : QWidget(parent), main(main_window) {
    auto *layout = new QVBoxLayout(this);
    layout->setContentsMargins(0, 0, 0, 0);

    auto *tabs = new QTabWidget(this);
    layout->addWidget(tabs);

    auto *tag_page = new QWidget(tabs);
    auto *tag_layout = new QVBoxLayout(tag_page);
    tag_layout->setContentsMargins(4, 4, 4, 4);
    tag_layout->setSpacing(4);

    services = new QComboBox(tag_page);
    tag_layout->addWidget(services);

    tag_filter = new QLineEdit(tag_page);
    tag_filter->setPlaceholderText(QStringLiteral("filter tags"));
    tag_layout->addWidget(tag_filter);

    display_tags = new QCheckBox(QStringLiteral("show display tags (siblings applied)"), tag_page);
    display_tags->setChecked(true);
    tag_layout->addWidget(display_tags);

    tags = new QListWidget(tag_page);
    tags->setSelectionMode(QAbstractItemView::ExtendedSelection);
    tag_layout->addWidget(tags, 1);

    tag_input = new QLineEdit(tag_page);
    tag_input->setPlaceholderText(QStringLiteral("add a tag"));
    completer = new QCompleter(main->suggestions(), this);
    completer->setCaseSensitivity(Qt::CaseInsensitive);
    completer->setFilterMode(Qt::MatchContains);
    tag_input->setCompleter(completer);
    tag_layout->addWidget(tag_input);

    auto *tag_buttons = new QHBoxLayout();
    auto *add_button = new QPushButton(QStringLiteral("add"), tag_page);
    auto *remove_button = new QPushButton(QStringLiteral("remove"), tag_page);
    tag_buttons->addWidget(add_button);
    tag_buttons->addWidget(remove_button);
    tag_buttons->addStretch(1);
    tag_layout->addLayout(tag_buttons);

    tabs->addTab(tag_page, QStringLiteral("tags"));

    auto *ratings_page = new QWidget(tabs);
    ratings_layout = new QVBoxLayout(ratings_page);
    ratings_layout->setContentsMargins(6, 6, 6, 6);
    ratings_layout->setSpacing(6);
    ratings = ratings_page;
    tabs->addTab(ratings_page, QStringLiteral("ratings"));

    auto *notes_page = new QWidget(tabs);
    auto *notes_layout = new QVBoxLayout(notes_page);
    notes_layout->setContentsMargins(4, 4, 4, 4);
    notes = new QListWidget(notes_page);
    notes_layout->addWidget(notes, 1);
    auto *edit_notes_button = new QPushButton(QStringLiteral("edit notes..."), notes_page);
    notes_layout->addWidget(edit_notes_button);
    tabs->addTab(notes_page, QStringLiteral("notes"));

    connect(services, QOverload<int>::of(&QComboBox::currentIndexChanged), this, [this](int) {
        rebuild_tags();
    });
    connect(tag_filter, &QLineEdit::textChanged, this, [this](const QString &) { rebuild_tags(); });
    connect(display_tags, &QCheckBox::clicked, this, [this](bool) { rebuild_tags(); });
    connect(add_button, &QPushButton::clicked, this, [this]() { add_tag(); });
    connect(remove_button, &QPushButton::clicked, this, [this]() { remove_selected_tags(); });
    connect(tag_input, &QLineEdit::returnPressed, this, [this]() { add_tag(); });
    connect(edit_notes_button, &QPushButton::clicked, this, [this]() { edit_notes(); });
    connect(tags, &QListWidget::itemDoubleClicked, this, [this](QListWidgetItem *) { remove_selected_tags(); });
}

void HydrusMetadataPanel::set_state(const QJsonObject &metadata) {
    data = metadata;
    hash = metadata.value(QStringLiteral("hash")).toString();

    const QString previous = services->currentData().toString();
    QSignalBlocker blocker(services);
    services->clear();

    const QJsonArray tag_services = metadata.value(QStringLiteral("tag_services")).toArray();

    for (const QJsonValue &value : tag_services) {
        const QJsonObject service = value.toObject();
        services->addItem(
            service.value(QStringLiteral("name")).toString(),
            service.value(QStringLiteral("key")).toString());
    }

    if (services->count() == 0) {
        services->addItem(QStringLiteral("no local tag service"), QString());
    }

    if (!previous.isEmpty()) {
        const int index = services->findData(previous);

        if (index >= 0) {
            services->setCurrentIndex(index);
        }
    }

    blocker.unblock();
    rebuild_tags();
    rebuild_ratings(metadata.value(QStringLiteral("ratings")).toArray());

    notes->clear();

    for (const QJsonValue &value : metadata.value(QStringLiteral("notes")).toArray()) {
        const QJsonObject note = value.toObject();
        QString text = note.value(QStringLiteral("text")).toString().simplified();

        if (text.size() > 100) {
            text = text.left(100) + QStringLiteral("...");
        }

        notes->addItem(QStringLiteral("%1: %2").arg(note.value(QStringLiteral("name")).toString(), text));
    }

    const QJsonArray note_array = metadata.value(QStringLiteral("notes")).toArray();

    if (note_array.isEmpty()) {
        notes->addItem(QStringLiteral("no notes"));
    }
}

void HydrusMetadataPanel::set_suggestions(const QStringList &suggestions) {
    completer->setModel(new QStringListModel(suggestions, completer));
}

void HydrusMetadataPanel::rebuild_tags() {
    tags->clear();

    const QString key = services->currentData().toString();
    const QJsonArray tag_services = data.value(QStringLiteral("tag_services")).toArray();
    const QString filter = tag_filter->text().trimmed();

    for (const QJsonValue &value : tag_services) {
        const QJsonObject service = value.toObject();

        if (service.value(QStringLiteral("key")).toString() != key) {
            continue;
        }

        const bool show_display = display_tags->isChecked();
        const QJsonArray names = service.value(
            show_display ? QStringLiteral("display_tags") : QStringLiteral("tags")).toArray();
        const QJsonObject sources = service.value(QStringLiteral("display_tag_sources")).toObject();

        for (const QJsonValue &tag : names) {
            const QString text = tag.toString();

            if (filter.isEmpty() || text.contains(filter, Qt::CaseInsensitive)) {
                auto *item = new QListWidgetItem(text, tags);

                if (show_display) {
                    const QStringList storage = json_to_strings(
                        sources.value(text).toArray());

                    if (!storage.isEmpty()) {
                        item->setToolTip(QStringLiteral("storage tags: %1").arg(
                            storage.join(QStringLiteral(", "))));
                    }
                }
            }
        }

        break;
    }
}

void HydrusMetadataPanel::rebuild_ratings(const QJsonArray &ratings_value) {
    while (QLayoutItem *item = ratings_layout->takeAt(0)) {
        if (QWidget *widget = item->widget()) {
            widget->deleteLater();
        }

        delete item;
    }

    if (ratings_value.isEmpty()) {
        auto *label = new QLabel(QStringLiteral("No rating services. Create one in services."), ratings);
        label->setWordWrap(true);
        ratings_layout->addWidget(label);
    }

    for (const QJsonValue &value : ratings_value) {
        const QJsonObject rating = value.toObject();
        const QString service = rating.value(QStringLiteral("key")).toString();
        const QString kind = rating.value(QStringLiteral("kind")).toString();
        const QString name = rating.value(QStringLiteral("name")).toString();

        auto *row = new QWidget(ratings);
        auto *row_layout = new QVBoxLayout(row);
        row_layout->setContentsMargins(0, 0, 0, 0);
        row_layout->setSpacing(2);

        auto *caption = new QLabel(name, row);
        caption->setStyleSheet(QStringLiteral("font-weight: bold;"));
        row_layout->addWidget(caption);

        if (kind == QStringLiteral("numerical")) {
            const double current = rating.value(QStringLiteral("value")).toDouble();
            auto *slider_row = new QWidget(row);
            auto *slider_layout = new QHBoxLayout(slider_row);
            slider_layout->setContentsMargins(0, 0, 0, 0);
            auto *slider = new QSlider(Qt::Horizontal, slider_row);
            slider->setRange(0, 100);
            slider->setValue(static_cast<int>(current * 100.0 + 0.5));
            auto *label = new QLabel(QStringLiteral("%1%").arg(slider->value()), slider_row);
            label->setMinimumWidth(42);
            slider_layout->addWidget(slider, 1);
            slider_layout->addWidget(label);
            connect(slider, &QSlider::valueChanged, label, [label](int value) {
                label->setText(QStringLiteral("%1%").arg(value));
            });
            connect(slider, &QSlider::sliderReleased, this, [this, service, slider]() {
                send_rating(service, QStringLiteral("set"), slider->value() / 100.0);
            });
            row_layout->addWidget(slider_row);
        } else if (kind == QStringLiteral("like")) {
            auto *buttons = new QWidget(row);
            auto *buttons_layout = new QHBoxLayout(buttons);
            buttons_layout->setContentsMargins(0, 0, 0, 0);
            auto *like = new QPushButton(QStringLiteral("like"), buttons);
            auto *dislike = new QPushButton(QStringLiteral("dislike"), buttons);
            auto *clear = new QPushButton(QStringLiteral("clear"), buttons);
            const QString current = rating.value(QStringLiteral("value")).toString();
            like->setCheckable(true);
            dislike->setCheckable(true);
            like->setChecked(current == QStringLiteral("like"));
            dislike->setChecked(current == QStringLiteral("dislike"));
            connect(like, &QPushButton::clicked, this, [this, service]() {
                send_rating(service, QStringLiteral("like"));
            });
            connect(dislike, &QPushButton::clicked, this, [this, service]() {
                send_rating(service, QStringLiteral("dislike"));
            });
            connect(clear, &QPushButton::clicked, this, [this, service]() {
                send_rating(service, QStringLiteral("clear"));
            });
            buttons_layout->addWidget(like);
            buttons_layout->addWidget(dislike);
            buttons_layout->addWidget(clear);
            buttons_layout->addStretch(1);
            row_layout->addWidget(buttons);
        } else {
            auto *buttons = new QWidget(row);
            auto *buttons_layout = new QHBoxLayout(buttons);
            buttons_layout->setContentsMargins(0, 0, 0, 0);
            auto *minus = new QPushButton(QStringLiteral("-1"), buttons);
            auto *plus = new QPushButton(QStringLiteral("+1"), buttons);
            auto *clear = new QPushButton(QStringLiteral("clear"), buttons);
            auto *label = new QLabel(QStringLiteral("value: %1").arg(rating.value(QStringLiteral("value")).toInt()), buttons);
            connect(minus, &QPushButton::clicked, this, [this, service]() {
                send_rating(service, QStringLiteral("set"), -1.0);
            });
            connect(plus, &QPushButton::clicked, this, [this, service]() {
                send_rating(service, QStringLiteral("set"), 1.0);
            });
            connect(clear, &QPushButton::clicked, this, [this, service]() {
                send_rating(service, QStringLiteral("clear"));
            });
            buttons_layout->addWidget(minus);
            buttons_layout->addWidget(plus);
            buttons_layout->addWidget(clear);
            buttons_layout->addWidget(label);
            buttons_layout->addStretch(1);
            row_layout->addWidget(buttons);
        }

        ratings_layout->addWidget(row);
    }

    ratings_layout->addStretch(1);
}

void HydrusMetadataPanel::send_tag(const QString &tag, bool add) {
    if (hash.isEmpty() || tag.trimmed().isEmpty()) {
        return;
    }

    QJsonObject payload;
    payload.insert(QStringLiteral("hash"), hash);
    payload.insert(QStringLiteral("service"), services->currentData().toString());
    payload.insert(QStringLiteral("tag"), tag.trimmed());

    main->notify(
        add ? QStringLiteral("tag_add") : QStringLiteral("tag_remove"),
        QString(),
        json_payload(payload));

    if (add) {
        if (tags->findItems(tag.trimmed(), Qt::MatchExactly).isEmpty()) {
            tags->addItem(tag.trimmed());
        }
    } else {
        for (QListWidgetItem *item : tags->findItems(tag.trimmed(), Qt::MatchExactly)) {
            delete tags->takeItem(tags->row(item));
        }
    }
}

void HydrusMetadataPanel::send_rating(const QString &service, const QString &action, double value) {
    if (hash.isEmpty()) {
        return;
    }

    QJsonObject payload;
    payload.insert(QStringLiteral("hash"), hash);
    payload.insert(QStringLiteral("service"), service);
    payload.insert(QStringLiteral("action"), action);
    payload.insert(QStringLiteral("value"), value);

    main->notify(QStringLiteral("rating_set"), QString(), json_payload(payload));
}

void HydrusMetadataPanel::add_tag() {
    const QString tag = tag_input->text().trimmed();

    if (tag.isEmpty()) {
        return;
    }

    send_tag(tag, true);
    tag_input->clear();
}

void HydrusMetadataPanel::remove_selected_tags() {
    QStringList selected;

    for (QListWidgetItem *item : tags->selectedItems()) {
        selected.append(item->text());
    }

    for (const QString &tag : selected) {
        send_tag(tag, false);
    }
}

void HydrusMetadataPanel::edit_notes() {
    if (hash.isEmpty()) {
        return;
    }

    QJsonObject payload;
    payload.insert(QStringLiteral("hash"), hash);
    main->notify(QStringLiteral("notes_manage"), QString(), json_payload(payload));
}

// ---------------------------------------------------------------------------
// Search autocomplete
// ---------------------------------------------------------------------------

// Hydrus' AC dropdown groups matches into tabs and shows how many files each
// predicate selects. The popup keeps focus in the line edit so typing can
// continue while Arrow keys move through the list.
class HydrusAutocompletePopup : public QFrame {
public:
    HydrusAutocompletePopup(QWidget *anchor, std::function<void(const QString &)> insert)
        : QFrame(anchor->window()), anchor(anchor), insert(std::move(insert)) {
        setFrameShape(QFrame::StyledPanel);
        setFrameShadow(QFrame::Raised);

        auto *layout = new QVBoxLayout(this);
        layout->setContentsMargins(0, 0, 0, 0);
        layout->setSpacing(0);

        tabs = new QTabBar(this);
        tabs->setExpanding(false);
        tabs->setDrawBase(false);
        layout->addWidget(tabs);

        list = new QListWidget(this);
        list->setFocusPolicy(Qt::NoFocus);
        list->setSelectionMode(QAbstractItemView::SingleSelection);
        list->setUniformItemSizes(true);
        layout->addWidget(list, 1);

        connect(tabs, &QTabBar::currentChanged, this, [this](int) { rebuild_list(); });
        connect(list, &QListWidget::itemClicked, this, [this](QListWidgetItem *item) {
            if (item != nullptr) {
                activate(item->data(Qt::UserRole).toString());
            }
        });
    }

    void set_state(const QJsonObject &state) {
        entries = state.value(QStringLiteral("entries")).toArray();
        raw = state.value(QStringLiteral("raw")).toString();
        rebuild_tabs();

        if (entries.isEmpty()) {
            hide();
            return;
        }

        const int width = qMax(320, anchor->width());
        const int rows = qBound(1, list->count(), 12);
        resize(width, tabs->sizeHint().height() + rows * 24 + 12);

        const QPoint position = anchor->mapTo(parentWidget(), QPoint(0, anchor->height()));
        move(position);
        raise();
        show();
        list->setCurrentRow(0);
    }

    void move_selection(int delta) {
        if (!isVisible() || list->count() == 0) {
            return;
        }

        int row = list->currentRow() + delta;

        if (row < 0) {
            row = list->count() - 1;
        } else if (row >= list->count()) {
            row = 0;
        }

        list->setCurrentRow(row);
    }

    QString current_text() const {
        QListWidgetItem *item = list->currentItem();

        return item == nullptr ? QString() : item->data(Qt::UserRole).toString();
    }

    QString raw_text() const {
        return raw;
    }

    bool is_open() const {
        return isVisible() && list->count() > 0;
    }

private:
    QString category_at(int index) const {
        return index < 0 ? QString() : tabs->tabData(index).toString();
    }

    void rebuild_tabs() {
        const QString previous = category_at(tabs->currentIndex());
        QSignalBlocker blocker(tabs);

        while (tabs->count() > 0) {
            tabs->removeTab(0);
        }

        const QStringList order{
            QStringLiteral("tags"),
            QStringLiteral("namespaces"),
            QStringLiteral("wildcards"),
            QStringLiteral("system"),
        };

        for (const QString &category : order) {
            int count = 0;
            QString label = category;

            for (const QJsonValue &value : entries) {
                if (value.toObject().value(QStringLiteral("category")).toString() == category) {
                    count += 1;
                }
            }

            if (count == 0) {
                continue;
            }

            if (category == QStringLiteral("tags")) {
                label = QStringLiteral("tags");
            } else if (category == QStringLiteral("namespaces")) {
                label = QStringLiteral("namespaces");
            } else if (category == QStringLiteral("wildcards")) {
                label = QStringLiteral("wildcards");
            } else {
                label = QStringLiteral("system");
            }

            const int index = tabs->addTab(QStringLiteral("%1 (%2)").arg(label).arg(count));
            tabs->setTabData(index, category);
        }

        int restore = 0;

        for (int index = 0; index < tabs->count(); ++index) {
            if (category_at(index) == previous) {
                restore = index;
                break;
            }
        }

        if (tabs->count() > 0) {
            tabs->setCurrentIndex(restore);
        }

        rebuild_list();
    }

    void rebuild_list() {
        list->clear();
        const QString category = category_at(tabs->currentIndex());

        for (const QJsonValue &value : entries) {
            const QJsonObject entry = value.toObject();

            if (entry.value(QStringLiteral("category")).toString() != category) {
                continue;
            }

            const QString text = entry.value(QStringLiteral("text")).toString();
            QString label = text;
            const QJsonValue count = entry.value(QStringLiteral("count"));

            if (!count.isNull() && count.isDouble()) {
                label = QStringLiteral("%1  (%2)").arg(text).arg(static_cast<qint64>(count.toDouble()));
            }

            auto *item = new QListWidgetItem(label, list);
            item->setData(Qt::UserRole, text);
            item->setToolTip(text);
        }

        if (list->count() > 0) {
            list->setCurrentRow(0);
        }
    }

    void activate(const QString &text) {
        hide();

        if (!text.isEmpty()) {
            insert(text);
        }
    }

    QWidget *anchor = nullptr;
    QTabBar *tabs = nullptr;
    QListWidget *list = nullptr;
    QJsonArray entries;
    QString raw;
    std::function<void(const QString &)> insert;
};

// ---------------------------------------------------------------------------
// HydrusQueryPage
// ---------------------------------------------------------------------------

HydrusQueryPage::HydrusQueryPage(HydrusMainWindow *main_window, const QString &page_id, const QJsonObject &state)
    : QWidget(), main(main_window), id(page_id) {
    auto *layout = new QHBoxLayout(this);
    layout->setContentsMargins(0, 0, 0, 0);
    layout->setSpacing(0);

    auto *splitter = new QSplitter(Qt::Horizontal, this);

    auto *sidebar = new QWidget(splitter);
    auto *sidebar_layout = new QVBoxLayout(sidebar);
    sidebar_layout->setContentsMargins(6, 6, 6, 6);
    sidebar_layout->setSpacing(4);

    auto *search_title = new QLabel(QStringLiteral("search"), sidebar);
    search_title->setStyleSheet(QStringLiteral("font-weight: bold;"));
    sidebar_layout->addWidget(search_title);

    location = new QComboBox(sidebar);
    location->addItem(QStringLiteral("my files"));
    location->addItem(QStringLiteral("all local files"));
    location->addItem(QStringLiteral("trash"));
    sidebar_layout->addWidget(location);

    connect(location, &QComboBox::currentTextChanged, this, [this](const QString &value) {
        if (main->building || value.isEmpty()) {
            return;
        }

        main->notify(QStringLiteral("set_location"), id, value);
    });

    tag_service = new QComboBox(sidebar);
    sidebar_layout->addWidget(tag_service);

    connect(tag_service, &QComboBox::currentTextChanged, this, [this](const QString &value) {
        if (main->building || value.isEmpty()) {
            return;
        }

        main->notify(QStringLiteral("set_tag_service"), id, value);
    });

    search = new QLineEdit(sidebar);
    search->setPlaceholderText(QStringLiteral("tags: blue eyes, -sky, system:inbox"));
    sidebar_layout->addWidget(search);

    autocomplete = new HydrusAutocompletePopup(search, [this](const QString &text) {
        insert_autocomplete(text);
    });
    search->installEventFilter(this);

    connect(search, &QLineEdit::textEdited, this, [this](const QString &text) {
        text_before_completion = text;
        request_autocomplete();
    });

    search_button = new QPushButton(QStringLiteral("search"), sidebar);
    sidebar_layout->addWidget(search_button);

    auto *or_button = new QPushButton(QStringLiteral("OR..."), sidebar);
    or_button->setToolTip(QStringLiteral("Combine tags or system predicates with OR."));
    sidebar_layout->addWidget(or_button);

    connect(or_button, &QPushButton::clicked, this, [this]() {
        QStringList seed;

        for (const QString &term : search->text().split(' ', Qt::SkipEmptyParts)) {
            seed.append(term);
        }

        show_or_predicate_dialog(*main, id, seed.join(QLatin1Char('\n')));
    });

    sort = new QComboBox(sidebar);
    sort->addItems({
        QStringLiteral("hash id"),
        QStringLiteral("import time"),
        QStringLiteral("size"),
        QStringLiteral("mime"),
        QStringLiteral("width"),
        QStringLiteral("height"),
        QStringLiteral("ratio"),
        QStringLiteral("pixels"),
        QStringLiteral("duration"),
        QStringLiteral("frames"),
        QStringLiteral("words"),
    });
    sidebar_layout->addWidget(sort);

    sort_ascending = new QCheckBox(QStringLiteral("ascending"), sidebar);
    sort_ascending->setChecked(true);
    sidebar_layout->addWidget(sort_ascending);

    const auto send_sort = [this]() {
        if (main->building) {
            return;
        }

        QJsonObject request;
        request.insert(QStringLiteral("sort"), sort->currentText());
        request.insert(QStringLiteral("ascending"), sort_ascending->isChecked());
        main->notify(
            QStringLiteral("set_sort"),
            id,
            QString::fromUtf8(QJsonDocument(request).toJson(QJsonDocument::Compact)));
    };

    connect(sort, &QComboBox::currentTextChanged, this, [send_sort](const QString &) { send_sort(); });
    connect(sort_ascending, &QCheckBox::toggled, this, [send_sort](bool) { send_sort(); });

    status_label = new QLabel(sidebar);
    status_label->setWordWrap(true);
    sidebar_layout->addWidget(status_label);

    auto *tags_title = new QLabel(QStringLiteral("metadata"), sidebar);
    tags_title->setStyleSheet(QStringLiteral("font-weight: bold;"));
    sidebar_layout->addWidget(tags_title);

    metadata = new HydrusMetadataPanel(main, sidebar);
    sidebar_layout->addWidget(metadata, 1);

    auto *vertical = new QSplitter(Qt::Vertical, splitter);
    vertical->addWidget(sidebar);

    preview = new HydrusCanvas(vertical);
    preview->setMinimumHeight(140);
    vertical->addWidget(preview);
    vertical->setStretchFactor(0, 1);
    vertical->setStretchFactor(1, 0);
    vertical->setSizes({400, 260});

    splitter->addWidget(vertical);

    results = new QWidget(splitter);
    auto *results_layout = new QVBoxLayout(results);
    results_layout->setContentsMargins(0, 0, 0, 0);
    results_layout->setSpacing(0);

    auto *header = new QWidget(results);
    header->setObjectName(QStringLiteral("HydrusResultsHeader"));
    auto *header_layout = new QHBoxLayout(header);
    header_layout->setContentsMargins(8, 4, 8, 4);
    header_layout->setSpacing(8);

    count_label = new QLabel(QStringLiteral("0 files"), header);
    count_label->setStyleSheet(QStringLiteral("font-weight: bold;"));
    header_layout->addWidget(count_label);
    header_layout->addStretch(1);

    loading_label = new QLabel(header);
    loading_label->setVisible(false);
    header_layout->addWidget(loading_label);

    import_progress = new QProgressBar(header);
    import_progress->setFixedWidth(190);
    import_progress->setTextVisible(true);
    import_progress->setVisible(false);
    header_layout->addWidget(import_progress);

    results_layout->addWidget(header);

    grid = new HydrusThumbnailGrid(main, this);
    results_layout->addWidget(grid, 1);

    loading_panel = new QWidget(results);
    loading_panel->setObjectName(QStringLiteral("HydrusLoadingPanel"));
    loading_panel->setStyleSheet(
        QStringLiteral("QWidget#HydrusLoadingPanel { background: rgba(30, 30, 30, 210); }"));
    auto *loading_layout = new QVBoxLayout(loading_panel);
    loading_layout->addStretch(1);
    loading_text = new QLabel(QStringLiteral("loading files..."), loading_panel);
    loading_text->setAlignment(Qt::AlignHCenter);
    loading_text->setStyleSheet(
        QStringLiteral("font-size: 15px; color: #e6e6e6; background: transparent;"));
    loading_layout->addWidget(loading_text);
    auto *loading_bar = new QProgressBar(loading_panel);
    loading_bar->setRange(0, 0);
    loading_bar->setTextVisible(false);
    loading_bar->setFixedWidth(240);
    loading_bar->setStyleSheet(
        QStringLiteral("QProgressBar { background: #2a2a2a; border: 1px solid #555; }"
                       "QProgressBar::chunk { background: #4a90d9; }"));
    loading_layout->addWidget(loading_bar, 0, Qt::AlignHCenter);
    loading_layout->addStretch(2);
    loading_panel->hide();

    splitter->addWidget(results);
    splitter->setStretchFactor(0, 0);
    splitter->setStretchFactor(1, 1);
    splitter->setSizes({330, 900});

    layout->addWidget(splitter);

    connect(search, &QLineEdit::returnPressed, this, [this]() {
        const QByteArray page_bytes = id.toUtf8();
        const QByteArray query_bytes = search->text().toUtf8();
        app_action(*main, rust::Str("search", 6), rust::Str(page_bytes.constData(), page_bytes.size()), rust::Str(query_bytes.constData(), query_bytes.size()));
    });

    connect(search_button, &QPushButton::clicked, this, [this]() {
        const QByteArray page_bytes = id.toUtf8();
        const QByteArray query_bytes = search->text().toUtf8();
        app_action(*main, rust::Str("search", 6), rust::Str(page_bytes.constData(), page_bytes.size()), rust::Str(query_bytes.constData(), query_bytes.size()));
    });

    update_state(state);
}

const QString &HydrusQueryPage::page_id() const {
    return id;
}

void HydrusQueryPage::show_autocomplete(const QJsonObject &state) {
    if (main->building || state.value(QStringLiteral("page_id")).toString() != id) {
        return;
    }

    autocomplete->set_state(state);
}

void HydrusQueryPage::insert_autocomplete(const QString &text) {
    QString current = search->text();
    const QString raw = autocomplete == nullptr ? QString() : autocomplete->raw_text();

    if (!raw.isEmpty() && current.endsWith(raw)) {
        current.chop(raw.size());
    } else {
        current.clear();
    }

    if (!current.isEmpty() && !current.endsWith(QLatin1Char(' '))) {
        current += QLatin1Char(' ');
    }

    QString inserted = text;

    if (!inserted.endsWith(QLatin1Char(' ')) && !inserted.endsWith(QLatin1Char(':'))) {
        inserted += QLatin1Char(' ');
    }

    search->setText(current + inserted);
    text_before_completion = search->text();
    search->setFocus();
}

void HydrusQueryPage::request_autocomplete() {
    if (main->building) {
        return;
    }

    main->notify(QStringLiteral("autocomplete"), id, search->text());
}

bool HydrusQueryPage::eventFilter(QObject *watched, QEvent *event) {
    if (watched == search && event->type() == QEvent::FocusOut && autocomplete != nullptr) {
        QTimer::singleShot(150, this, [this]() {
            if (!search->hasFocus()) {
                autocomplete->hide();
            }
        });
    }

    if (watched == search && autocomplete != nullptr && autocomplete->is_open()
        && event->type() == QEvent::KeyPress) {
        auto *key = static_cast<QKeyEvent *>(event);

        if (key->key() == Qt::Key_Down) {
            autocomplete->move_selection(1);
            return true;
        }

        if (key->key() == Qt::Key_Up) {
            autocomplete->move_selection(-1);
            return true;
        }

        if (key->key() == Qt::Key_Escape) {
            autocomplete->hide();
            return true;
        }

        if (key->key() == Qt::Key_Return || key->key() == Qt::Key_Enter) {
            const QString text = autocomplete->current_text();

            if (!text.isEmpty()) {
                insert_autocomplete(text);
                autocomplete->hide();
                return true;
            }
        }
    }

    return QWidget::eventFilter(watched, event);
}

void HydrusQueryPage::update_progress(const QJsonObject &state) {
    const int count = state.value(QStringLiteral("count")).toInt();

    count_label->setText(
        count == 1 ? QStringLiteral("1 file") : QStringLiteral("%1 files").arg(count));

    const QJsonValue importing = state.value(QStringLiteral("importing"));

    if (importing.isObject()) {
        const QJsonObject progress = importing.toObject();
        const int done = progress.value(QStringLiteral("done")).toInt();
        const int total = progress.value(QStringLiteral("total")).toInt();

        loading_label->setText(progress.value(QStringLiteral("message")).toString());
        import_progress->setRange(0, qMax(1, total));
        import_progress->setValue(qBound(0, done, qMax(1, total)));
        import_progress->setFormat(QStringLiteral("%1 / %2").arg(done).arg(total));
        loading_label->setVisible(true);
        import_progress->setVisible(true);
    } else {
        loading_label->setVisible(false);
        import_progress->setVisible(false);
    }

    const bool loading = state.value(QStringLiteral("loading")).toBool();

    if (loading) {
        loading_panel->setGeometry(results->rect());
        loading_panel->raise();
        loading_panel->show();
    } else {
        loading_panel->hide();
    }
}

void HydrusQueryPage::resizeEvent(QResizeEvent *event) {
    QWidget::resizeEvent(event);

    if (loading_panel != nullptr && loading_panel->isVisible() && results != nullptr) {
        loading_panel->setGeometry(results->rect());
    }
}

void HydrusQueryPage::update_state(const QJsonObject &state) {
    const QString query = state.value(QStringLiteral("query")).toString();

    if (search->text() != query) {
        search->setText(query);
    }

    const QString location_value = state.value(QStringLiteral("location")).toString();

    if (!location_value.isEmpty() && location->currentText() != location_value) {
        const QSignalBlocker blocker(location);
        const int index = location->findText(location_value);

        if (index >= 0) {
            location->setCurrentIndex(index);
        }
    }

    QStringList tag_service_names;

    for (const QJsonValue &value : state.value(QStringLiteral("tag_services")).toArray()) {
        tag_service_names.append(value.toString());
    }

    bool rebuild_services = tag_service_names.size() != tag_service->count();

    for (int index = 0; !rebuild_services && index < tag_service_names.size(); ++index) {
        rebuild_services = tag_service->itemText(index) != tag_service_names[index];
    }

    if (rebuild_services) {
        const QSignalBlocker blocker(tag_service);
        tag_service->clear();
        tag_service->addItems(tag_service_names);
    }

    QString tag_service_value = state.value(QStringLiteral("tag_service")).toString();

    if (tag_service_value.isEmpty() && !tag_service_names.isEmpty()) {
        tag_service_value = tag_service_names.first();
    }

    if (!tag_service_value.isEmpty() && tag_service->currentText() != tag_service_value) {
        const QSignalBlocker blocker(tag_service);
        const int index = tag_service->findText(tag_service_value);

        if (index >= 0) {
            tag_service->setCurrentIndex(index);
        }
    }

    const QString sort_value = state.value(QStringLiteral("sort")).toString();

    if (!sort_value.isEmpty() && sort->currentText() != sort_value) {
        const QSignalBlocker blocker(sort);
        const int index = sort->findText(sort_value);

        if (index >= 0) {
            sort->setCurrentIndex(index);
        }
    }

    const bool ascending = state.value(QStringLiteral("sort_ascending")).toBool(true);

    if (sort_ascending->isChecked() != ascending) {
        const QSignalBlocker blocker(sort_ascending);
        sort_ascending->setChecked(ascending);
    }

    QString status = state.value(QStringLiteral("status")).toString();
    const QString empty_text = state.value(QStringLiteral("empty_text")).toString();

    if (!empty_text.isEmpty()) {
        status += QStringLiteral("\n") + empty_text;
    }

    status_label->setText(status);

    const QJsonObject metadata_state = state.value(QStringLiteral("metadata")).toObject();
    metadata->set_state(metadata_state);

    const QString focused = state.value(QStringLiteral("focused")).toString();
    const QJsonObject thumbnails = state.value(QStringLiteral("thumbnails")).toObject();

    grid->set_layout(
        thumbnails.value(QStringLiteral("masonry")).toBool(false),
        thumbnails.value(QStringLiteral("horizontal")).toBool(false),
        thumbnails.value(QStringLiteral("scale_percent")).toInt(100));
    grid->set_media(state.value(QStringLiteral("media")).toArray());
    update_progress(state);

    if (!focused.isEmpty()) {
        grid->set_focused(focused);
    }
}

void HydrusQueryPage::set_selection(
    const QString &hash,
    const QStringList &tags,
    const QString &preview_path,
    const QJsonObject &metadata_state) {
    Q_UNUSED(tags);
    metadata->set_state(metadata_state);

    grid->set_focused(hash);

    if (!preview_path.isEmpty() && preview->load_image(preview_path)) {
        return;
    }

    preview->clear_image();
}

void HydrusQueryPage::set_suggestions(const QStringList &suggestions) {
    metadata->set_suggestions(suggestions);
}

void HydrusQueryPage::focus_hash(const QString &hash) {
    const QByteArray page_bytes = id.toUtf8();
    const QByteArray hash_bytes = hash.toUtf8();
    app_action(*main, rust::Str("focus_hash", 10), rust::Str(page_bytes.constData(), page_bytes.size()), rust::Str(hash_bytes.constData(), hash_bytes.size()));
}

void HydrusQueryPage::open_focused() {
    const QString hash = focused_hash();

    if (hash.isEmpty()) {
        return;
    }

    const QByteArray page_bytes = id.toUtf8();
    const QByteArray hash_bytes = hash.toUtf8();
    app_action(*main, rust::Str("open_viewer", 11), rust::Str(page_bytes.constData(), page_bytes.size()), rust::Str(hash_bytes.constData(), hash_bytes.size()));
}

void HydrusQueryPage::archive_focused() {
    const QString hash = focused_hash();

    if (hash.isEmpty()) {
        return;
    }

    const QByteArray page_bytes = id.toUtf8();
    const QByteArray hash_bytes = hash.toUtf8();
    app_action(*main, rust::Str("archive_files", 13), rust::Str(page_bytes.constData(), page_bytes.size()), rust::Str(hash_bytes.constData(), hash_bytes.size()));
}

void HydrusQueryPage::inbox_focused() {
    const QString hash = focused_hash();

    if (hash.isEmpty()) {
        return;
    }

    const QByteArray page_bytes = id.toUtf8();
    const QByteArray hash_bytes = hash.toUtf8();
    app_action(*main, rust::Str("inbox_files", 11), rust::Str(page_bytes.constData(), page_bytes.size()), rust::Str(hash_bytes.constData(), hash_bytes.size()));
}

void HydrusQueryPage::delete_focused() {
    const QString hash = focused_hash();

    if (hash.isEmpty()) {
        return;
    }

    const QByteArray page_bytes = id.toUtf8();
    const QByteArray hash_bytes = hash.toUtf8();
    app_action(*main, rust::Str("delete_files", 12), rust::Str(page_bytes.constData(), page_bytes.size()), rust::Str(hash_bytes.constData(), hash_bytes.size()));
}

QString HydrusQueryPage::focused_hash() const {
    return grid->focused_hash();
}

// ---------------------------------------------------------------------------
// HydrusDownloaderPage
// ---------------------------------------------------------------------------

HydrusDownloaderPage::HydrusDownloaderPage(HydrusMainWindow *main_window, const QString &page_id, const QJsonObject &state)
    : QWidget(), main(main_window), id(page_id) {
    kind = state.value(QStringLiteral("kind")).toString();

    auto *layout = new QVBoxLayout(this);
    layout->setContentsMargins(6, 6, 6, 6);
    layout->setSpacing(6);

    auto *form = new QFormLayout();
    form->setContentsMargins(0, 0, 0, 0);

    input = new QPlainTextEdit(this);
    input->setPlaceholderText(QStringLiteral("https://example.com/file.jpg\none url per line"));
    input->setMinimumHeight(64);
    input->setMaximumHeight(110);
    input_row = field_row(QStringLiteral("urls"), input, this);
    form->addRow(input_row);

    url = new QLineEdit(this);
    url->setPlaceholderText(QStringLiteral("https://example.com/gallery?q={query}"));
    url_row = field_row(QStringLiteral("page url"), url, this);
    form->addRow(url_row);

    query = new QLineEdit(this);
    query->setPlaceholderText(QStringLiteral("blue eyes"));
    filter = new QLineEdit(this);
    filter->setPlaceholderText(QStringLiteral("optional link filter"));
    auto *query_widget = new QWidget(this);
    auto *query_layout = new QHBoxLayout(query_widget);
    query_layout->setContentsMargins(0, 0, 0, 0);
    query_layout->addWidget(query, 1);
    query_layout->addWidget(filter, 1);
    query_row = field_row(QStringLiteral("search"), query_widget, this);
    form->addRow(query_row);

    site = new QComboBox(this);
    site_row = field_row(QStringLiteral("site parser"), site, this);
    form->addRow(site_row);

    subject = new QLineEdit(this);
    subject->setPlaceholderText(QStringLiteral("watcher subject"));
    subject_row = field_row(QStringLiteral("subject"), subject, this);
    form->addRow(subject_row);

    period = new QSpinBox(this);
    period->setRange(60, 2592000);
    period->setSingleStep(300);
    period->setValue(3600);
    period_row = field_row(QStringLiteral("check every"), period, this);
    form->addRow(period_row);

    layout->addLayout(form);

    auto *controls = new QHBoxLayout();
    start_button = new QPushButton(QStringLiteral("start"), this);
    pause_button = new QPushButton(QStringLiteral("pause"), this);
    clear_button = new QPushButton(QStringLiteral("clear finished"), this);
    results_button = new QPushButton(QStringLiteral("files"), this);
    auto_button = new QPushButton(QStringLiteral("automatic checks: off"), this);

    controls->addWidget(start_button);
    controls->addWidget(pause_button);
    controls->addWidget(clear_button);
    controls->addWidget(results_button);
    controls->addWidget(auto_button);
    controls->addStretch(1);
    layout->addLayout(controls);

    status_label = new QLabel(this);
    status_label->setWordWrap(true);
    layout->addWidget(status_label);

    auto *splitter = new QSplitter(Qt::Vertical, this);

    queue = new QTableWidget(splitter);
    queue->setColumnCount(4);
    queue->setHorizontalHeaderLabels({QStringLiteral("url"), QStringLiteral("domain"), QStringLiteral("status"), QStringLiteral("note")});
    queue->horizontalHeader()->setStretchLastSection(true);
    queue->horizontalHeader()->setSectionResizeMode(0, QHeaderView::Stretch);
    queue->horizontalHeader()->setSectionResizeMode(1, QHeaderView::ResizeToContents);
    queue->horizontalHeader()->setSectionResizeMode(2, QHeaderView::ResizeToContents);
    queue->setSelectionBehavior(QAbstractItemView::SelectRows);
    queue->setSelectionMode(QAbstractItemView::SingleSelection);
    queue->setEditTriggers(QAbstractItemView::NoEditTriggers);
    queue->verticalHeader()->setVisible(false);
    splitter->addWidget(queue);

    log = new QPlainTextEdit(splitter);
    log->setReadOnly(true);
    log->setMaximumBlockCount(400);
    splitter->addWidget(log);
    splitter->setStretchFactor(0, 3);
    splitter->setStretchFactor(1, 1);

    layout->addWidget(splitter, 1);

    connect(start_button, &QPushButton::clicked, this, [this]() { start(); });
    connect(pause_button, &QPushButton::clicked, this, [this]() {
        main->notify(QStringLiteral("downloader_pause"), id);
    });
    connect(clear_button, &QPushButton::clicked, this, [this]() {
        main->notify(QStringLiteral("downloader_clear"), id);
    });
    connect(results_button, &QPushButton::clicked, this, [this]() {
        main->notify(QStringLiteral("downloader_open_results"), id);
    });
    connect(auto_button, &QPushButton::clicked, this, [this]() {
        main->notify(QStringLiteral("watcher_auto"), id);
    });
    connect(queue, &QTableWidget::cellDoubleClicked, this, [this](int, int) { retry_selected(); });

    update_state(state);
}

const QString &HydrusDownloaderPage::page_id() const {
    return id;
}

void HydrusDownloaderPage::start() {
    QJsonObject payload;
    payload.insert(QStringLiteral("input"), input->toPlainText());
    payload.insert(QStringLiteral("url"), url->text());
    payload.insert(QStringLiteral("query"), query->text());
    payload.insert(QStringLiteral("site"), site->currentData().toString());
    payload.insert(QStringLiteral("filter"), filter->text());
    payload.insert(QStringLiteral("subject"), subject->text());
    payload.insert(QStringLiteral("period"), period->value());
    payload.insert(QStringLiteral("auto"), auto_button->text().endsWith(QStringLiteral("on")));

    const QString action = kind == QStringLiteral("watcher") ? QStringLiteral("watcher_check") : QStringLiteral("downloader_start");
    main->notify(action, id, json_payload(payload));
}

void HydrusDownloaderPage::retry_selected() {
    const int row = queue->currentRow();

    if (row < 0) {
        return;
    }

    QTableWidgetItem *item = queue->item(row, 0);

    if (item == nullptr) {
        return;
    }

    main->notify(QStringLiteral("downloader_retry"), id, item->data(Qt::UserRole).toString());
}

void HydrusDownloaderPage::update_state(const QJsonObject &state) {
    kind = state.value(QStringLiteral("kind")).toString();

    const bool urls = kind == QStringLiteral("urls") || kind == QStringLiteral("simple");
    const bool gallery = kind == QStringLiteral("gallery");
    const bool watcher = kind == QStringLiteral("watcher");

    input_row->setVisible(urls);
    url_row->setVisible(gallery || watcher);
    query_row->setVisible(gallery);
    site_row->setVisible(gallery || watcher);
    subject_row->setVisible(watcher);
    period_row->setVisible(watcher);
    pause_button->setVisible(!watcher);
    auto_button->setVisible(watcher);

    if (urls) {
        input->setPlaceholderText(kind == QStringLiteral("simple")
            ? QStringLiteral("https://example.com/post/123\none page url per line")
            : QStringLiteral("https://example.com/file.jpg\none file url per line"));
    }

    if (!input->hasFocus()) {
        const QString value = state.value(QStringLiteral("input")).toString();

        if (input->toPlainText() != value) {
            input->setPlainText(value);
        }
    }

    update_line_edit(url, state.value(QStringLiteral("url")).toString());
    update_line_edit(query, state.value(QStringLiteral("query")).toString());
    update_line_edit(filter, state.value(QStringLiteral("filter")).toString());

    if (site->count() == 0) {
        site->addItem(QStringLiteral("auto detect"), QString());

        for (const QJsonValue &value : state.value(QStringLiteral("sites")).toArray()) {
            const QJsonObject parser = value.toObject();
            site->addItem(parser.value(QStringLiteral("title")).toString(), parser.value(QStringLiteral("id")).toString());
        }
    }

    const int site_index = site->findData(state.value(QStringLiteral("site")).toString());

    if (site_index >= 0 && !site->hasFocus()) {
        QSignalBlocker blocker(site);
        site->setCurrentIndex(site_index);
    }
    update_line_edit(subject, state.value(QStringLiteral("subject")).toString());

    if (!period->hasFocus()) {
        period->setValue(state.value(QStringLiteral("check_period")).toInt(3600));
    }

    const bool running = state.value(QStringLiteral("running")).toBool();
    const bool paused = state.value(QStringLiteral("paused")).toBool();
    const bool auto_watch = state.value(QStringLiteral("auto_watch")).toBool();
    const int imported = state.value(QStringLiteral("imported")).toInt();
    const int failed = state.value(QStringLiteral("failed")).toInt();
    const int result_count = state.value(QStringLiteral("result_count")).toInt();

    start_button->setText(watcher ? QStringLiteral("check now") : QStringLiteral("start"));
    start_button->setEnabled(!running);
    pause_button->setText(paused ? QStringLiteral("resume") : QStringLiteral("pause"));
    results_button->setText(QStringLiteral("files (%1)").arg(result_count));
    results_button->setEnabled(result_count > 0);
    auto_button->setText(auto_watch
        ? QStringLiteral("automatic checks: on")
        : QStringLiteral("automatic checks: off"));

    QString status = state.value(QStringLiteral("status")).toString();

    if ((imported > 0 || failed > 0) && !status.contains(QStringLiteral("imported"))) {
        status += QStringLiteral("  -  imported %1, failed %2").arg(imported).arg(failed);
    }

    if (running) {
        status += paused ? QStringLiteral("  (paused)") : QStringLiteral("  (working)");
    }

    status_label->setText(status);
    log->setPlainText(state.value(QStringLiteral("log")).toString());

    const QJsonArray items = state.value(QStringLiteral("queue")).toArray();

    if (queue->rowCount() != items.size() || queue->columnCount() != 4) {
        queue->setRowCount(items.size());
    }

    for (int index = 0; index < items.size(); ++index) {
        const QJsonObject item = items.at(index).toObject();
        const QString item_url = item.value(QStringLiteral("url")).toString();
        const QString item_status = item.value(QStringLiteral("status")).toString();

        auto *url_item = queue->item(index, 0);

        if (url_item == nullptr) {
            url_item = new QTableWidgetItem();
            queue->setItem(index, 0, url_item);
            queue->setItem(index, 1, new QTableWidgetItem());
            queue->setItem(index, 2, new QTableWidgetItem());
            queue->setItem(index, 3, new QTableWidgetItem());
        }

        url_item->setText(item_url);
        url_item->setData(Qt::UserRole, item_url);
        url_item->setToolTip(item_url);

        queue->item(index, 1)->setText(item.value(QStringLiteral("domain")).toString());
        queue->item(index, 1)->setToolTip(item.value(QStringLiteral("domain")).toString());

        QTableWidgetItem *status_item = queue->item(index, 2);
        status_item->setText(item_status);

        if (item_status == QStringLiteral("error")) {
            status_item->setForeground(QColor(220, 110, 110));
        } else if (item_status == QStringLiteral("done")) {
            status_item->setForeground(QColor(120, 190, 120));
        } else if (item_status == QStringLiteral("downloading")) {
            status_item->setForeground(QColor(220, 200, 120));
        }

        queue->item(index, 3)->setText(item.value(QStringLiteral("note")).toString());
        queue->item(index, 3)->setToolTip(item.value(QStringLiteral("note")).toString());
    }

    queue->resizeRowsToContents();
}

// ---------------------------------------------------------------------------
// HydrusSubscriptionsPage
// ---------------------------------------------------------------------------

HydrusSubscriptionsPage::HydrusSubscriptionsPage(HydrusMainWindow *main_window, const QString &page_id, const QJsonObject &state)
    : QWidget(), main(main_window), id(page_id) {
    auto *layout = new QVBoxLayout(this);
    layout->setContentsMargins(6, 6, 6, 6);

    status = new QLabel(this);
    layout->addWidget(status);

    table = new QTableWidget(this);
    table->setColumnCount(7);
    table->setHorizontalHeaderLabels({
        QStringLiteral("name"),
        QStringLiteral("period"),
        QStringLiteral("last check"),
        QStringLiteral("next check"),
        QStringLiteral("imported"),
        QStringLiteral("failed"),
        QStringLiteral("status")
    });
    table->horizontalHeader()->setStretchLastSection(true);
    table->horizontalHeader()->setSectionResizeMode(0, QHeaderView::ResizeToContents);
    table->setSelectionBehavior(QAbstractItemView::SelectRows);
    table->setSelectionMode(QAbstractItemView::SingleSelection);
    table->setEditTriggers(QAbstractItemView::NoEditTriggers);
    table->verticalHeader()->setVisible(false);
    layout->addWidget(table, 1);

    auto *buttons = new QHBoxLayout();
    auto *add = new QPushButton(QStringLiteral("add subscription..."), this);
    auto *check = new QPushButton(QStringLiteral("check now"), this);
    auto *toggle = new QPushButton(QStringLiteral("pause / resume"), this);
    auto *remove = new QPushButton(QStringLiteral("delete"), this);
    auto *results = new QPushButton(QStringLiteral("open files"), this);
    buttons->addWidget(add);
    buttons->addWidget(check);
    buttons->addWidget(toggle);
    buttons->addWidget(remove);
    buttons->addWidget(results);
    buttons->addStretch(1);
    layout->addLayout(buttons);

    connect(add, &QPushButton::clicked, this, [this]() { add_subscription(); });
    connect(check, &QPushButton::clicked, this, [this]() { with_selected(QStringLiteral("subscription_check")); });
    connect(toggle, &QPushButton::clicked, this, [this]() { with_selected(QStringLiteral("subscription_toggle")); });
    connect(remove, &QPushButton::clicked, this, [this]() { with_selected(QStringLiteral("subscription_delete")); });
    connect(results, &QPushButton::clicked, this, [this]() { with_selected(QStringLiteral("subscription_open_results")); });

    update_state(state);
}

const QString &HydrusSubscriptionsPage::page_id() const {
    return id;
}

void HydrusSubscriptionsPage::with_selected(const QString &action) {
    const int row = table->currentRow();

    if (row < 0) {
        return;
    }

    QTableWidgetItem *item = table->item(row, 0);

    if (item != nullptr) {
        main->notify(action, id, item->data(Qt::UserRole).toString());
    }
}

void HydrusSubscriptionsPage::add_subscription() {
    QDialog dialog(this);
    dialog.setWindowTitle(QStringLiteral("add subscription"));
    auto *layout = new QVBoxLayout(&dialog);
    auto *form = new QFormLayout();

    auto *name = new QLineEdit(&dialog);
    auto *url = new QLineEdit(&dialog);
    auto *query = new QLineEdit(&dialog);
    auto *site = new QComboBox(&dialog);
    auto *filter = new QLineEdit(&dialog);
    auto *period = new QSpinBox(&dialog);
    period->setRange(60, 31536000);
    period->setValue(86400);

    site->addItem(QStringLiteral("auto detect"), QString());

    for (const QJsonValue &value : sites) {
        const QJsonObject parser = value.toObject();
        site->addItem(parser.value(QStringLiteral("title")).toString(), parser.value(QStringLiteral("id")).toString());
    }

    url->setPlaceholderText(QStringLiteral("https://example.com/search?q={query}"));
    query->setPlaceholderText(QStringLiteral("search terms for {query}"));
    form->addRow(QStringLiteral("name"), name);
    form->addRow(QStringLiteral("gallery url"), url);
    form->addRow(QStringLiteral("search terms"), query);
    form->addRow(QStringLiteral("site parser"), site);
    form->addRow(QStringLiteral("link filter"), filter);
    form->addRow(QStringLiteral("period (seconds)"), period);

    layout->addLayout(form);
    auto *buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, &dialog);
    connect(buttons, &QDialogButtonBox::accepted, &dialog, &QDialog::accept);
    QObject::connect(buttons, &QDialogButtonBox::rejected, &dialog, &QDialog::reject);
    layout->addWidget(buttons);

    if (dialog.exec() != QDialog::Accepted) {
        return;
    }

    QJsonObject payload;
    payload.insert(QStringLiteral("name"), name->text());
    payload.insert(QStringLiteral("url"), url->text());
    payload.insert(QStringLiteral("query"), query->text());
    payload.insert(QStringLiteral("site"), site->currentData().toString());
    payload.insert(QStringLiteral("filter"), filter->text());
    payload.insert(QStringLiteral("period"), period->value());

    main->notify(QStringLiteral("subscription_add"), id, json_payload(payload));
}

void HydrusSubscriptionsPage::update_state(const QJsonObject &state) {
    sites = state.value(QStringLiteral("sites")).toArray();
    const QJsonArray subscriptions = state.value(QStringLiteral("subscriptions")).toArray();

    status->setText(QStringLiteral("%1 subscriptions").arg(subscriptions.size()));
    table->setRowCount(subscriptions.size());

    for (int index = 0; index < subscriptions.size(); ++index) {
        const QJsonObject subscription = subscriptions.at(index).toObject();
        const QString name = subscription.value(QStringLiteral("name")).toString();

        for (int column = 0; column < table->columnCount(); ++column) {
            if (table->item(index, column) == nullptr) {
                table->setItem(index, column, new QTableWidgetItem());
            }
        }

        auto *name_item = table->item(index, 0);
        name_item->setText(name);
        name_item->setData(Qt::UserRole, name);
        name_item->setToolTip(subscription.value(QStringLiteral("url")).toString());

        table->item(index, 1)->setText(QStringLiteral("%1 s").arg(subscription.value(QStringLiteral("period")).toInt()));
        table->item(index, 2)->setText(subscription.value(QStringLiteral("last_check")).toInt() > 0
            ? QDateTime::fromSecsSinceEpoch(subscription.value(QStringLiteral("last_check")).toInt()).toString(QStringLiteral("yyyy-MM-dd hh:mm"))
            : QStringLiteral("never"));
        table->item(index, 3)->setText(subscription.value(QStringLiteral("running")).toBool()
            ? QStringLiteral("now")
            : QStringLiteral("in %1 s").arg(subscription.value(QStringLiteral("due_in")).toInt()));
        table->item(index, 4)->setText(QString::number(subscription.value(QStringLiteral("imported")).toInt()));
        table->item(index, 5)->setText(QString::number(subscription.value(QStringLiteral("failed")).toInt()));
        table->item(index, 6)->setText(subscription.value(QStringLiteral("status")).toString());

        if (subscription.value(QStringLiteral("paused")).toBool()) {
            table->item(index, 6)->setText(QStringLiteral("paused - ") + table->item(index, 6)->text());
        }
    }

    table->resizeRowsToContents();
}

// ---------------------------------------------------------------------------
// HydrusDuplicatesPage
// ---------------------------------------------------------------------------

HydrusDuplicatesPage::HydrusDuplicatesPage(HydrusMainWindow *main_window, const QString &page_id, const QJsonObject &state)
    : QWidget(), main(main_window), id(page_id) {
    auto *layout = new QVBoxLayout(this);
    layout->setContentsMargins(6, 6, 6, 6);
    layout->setSpacing(6);

    auto *search_panel = new QGroupBox(QStringLiteral("potential duplicate pairs search"), this);
    auto *search_layout = new QGridLayout(search_panel);

    dupe_search_type = new QComboBox(search_panel);
    dupe_search_type->addItem(QStringLiteral("at least one file matches the search"), QStringLiteral("one_file"));
    dupe_search_type->addItem(QStringLiteral("both files match the search"), QStringLiteral("both_files"));
    dupe_search_type->addItem(QStringLiteral("the two files match different searches"), QStringLiteral("different_searches"));

    search_query_1 = new QLineEdit(search_panel);
    search_query_1->setPlaceholderText(QStringLiteral("tags and system predicates for the first file"));
    search_location_1 = new QComboBox(search_panel);
    search_location_1->addItem(QStringLiteral("my files"));
    search_location_1->addItem(QStringLiteral("all local files"));
    search_location_1->addItem(QStringLiteral("trash"));

    search_query_2 = new QLineEdit(search_panel);
    search_query_2->setPlaceholderText(QStringLiteral("tags and system predicates for the second file"));
    search_location_2 = new QComboBox(search_panel);
    search_location_2->addItem(QStringLiteral("my files"));
    search_location_2->addItem(QStringLiteral("all local files"));
    search_location_2->addItem(QStringLiteral("trash"));
    search_row_2_label = new QLabel(QStringLiteral("search 2:"), search_panel);

    pixel_dupes = new QComboBox(search_panel);
    pixel_dupes->addItem(QStringLiteral("can be pixel dupes"), QStringLiteral("allowed"));
    pixel_dupes->addItem(QStringLiteral("must be pixel dupes"), QStringLiteral("required"));
    pixel_dupes->addItem(QStringLiteral("must not be pixel dupes"), QStringLiteral("excluded"));

    search_distance = new QSpinBox(search_panel);
    search_distance->setRange(0, 64);
    search_distance->setSingleStep(2);
    search_distance->setToolTip(QStringLiteral(
        "The max hamming distance allowed in the search. Higher distances mean more pairs and more false positives."));

    search_summary = new QLabel(search_panel);
    search_summary->setWordWrap(true);

    search_layout->addWidget(dupe_search_type, 0, 0, 1, 4);
    search_layout->addWidget(new QLabel(QStringLiteral("search 1:"), search_panel), 1, 0);
    search_layout->addWidget(search_query_1, 1, 1);
    search_layout->addWidget(search_location_1, 1, 2, 1, 2);
    search_layout->addWidget(search_row_2_label, 2, 0);
    search_layout->addWidget(search_query_2, 2, 1);
    search_layout->addWidget(search_location_2, 2, 2, 1, 2);
    search_layout->addWidget(new QLabel(QStringLiteral("pixel dupes:"), search_panel), 3, 0);
    search_layout->addWidget(pixel_dupes, 3, 1);
    search_layout->addWidget(new QLabel(QStringLiteral("max search distance:"), search_panel), 3, 2);
    search_layout->addWidget(search_distance, 3, 3);
    search_layout->addWidget(search_summary, 4, 0, 1, 4);
    search_layout->setColumnStretch(1, 1);

    layout->addWidget(search_panel);

    auto *header = new QHBoxLayout();
    status = new QLabel(this);
    distance = new QLabel(this);
    auto *refresh_button = new QPushButton(QStringLiteral("refresh"), this);
    refresh_button->setObjectName(QStringLiteral("refresh_duplicates"));
    refresh = refresh_button;
    delete_both = new QPushButton(QStringLiteral("delete both"), this);
    rules = new QPushButton(QStringLiteral("auto-resolution..."), this);
    merge_options = new QPushButton(QStringLiteral("merge options..."), this);
    auto *review = new QPushButton(QStringLiteral("review pending..."), this);
    batch = new QSpinBox(this);
    batch->setRange(5, 1024);
    batch->setToolTip(QStringLiteral("maximum pairs prepared in one batch"));
    next_batch = new QPushButton(QStringLiteral("next batch"), this);
    header->addWidget(status, 1);
    header->addWidget(distance);
    header->addWidget(new QLabel(QStringLiteral("batch:"), this));
    header->addWidget(batch);
    header->addWidget(next_batch);
    header->addWidget(rules);
    header->addWidget(merge_options);
    header->addWidget(review);
    header->addWidget(refresh_button);
    header->addWidget(delete_both);
    layout->addLayout(header);

    auto *splitter = new QSplitter(Qt::Horizontal, this);

    auto make_side = [&](bool left) {
        auto *side = new QWidget(splitter);
        auto *side_layout = new QVBoxLayout(side);
        side_layout->setContentsMargins(4, 4, 4, 4);

        auto *hash_label = new QLabel(side);
        hash_label->setWordWrap(true);
        hash_label->setStyleSheet(QStringLiteral("font-family: monospace;"));
        side_layout->addWidget(hash_label);

        auto *canvas = new HydrusCanvas(side);
        canvas->setMinimumSize(280, 280);
        side_layout->addWidget(canvas, 1);

        auto *info_label = new QLabel(side);
        info_label->setWordWrap(true);
        side_layout->addWidget(info_label);

        auto *tags_label = new QLabel(QStringLiteral("tags"), side);
        tags_label->setStyleSheet(QStringLiteral("font-weight: bold;"));
        side_layout->addWidget(tags_label);
        auto *tags = new QListWidget(side);
        tags->setSelectionMode(QAbstractItemView::NoSelection);
        side_layout->addWidget(tags, 1);

        auto *urls_label = new QLabel(QStringLiteral("urls"), side);
        urls_label->setStyleSheet(QStringLiteral("font-weight: bold;"));
        side_layout->addWidget(urls_label);
        auto *urls = new QListWidget(side);
        urls->setSelectionMode(QAbstractItemView::NoSelection);
        urls->setMaximumHeight(80);
        side_layout->addWidget(urls);

        auto *keep = new QPushButton(QStringLiteral("this is better"), side);
        keep->setStyleSheet(QStringLiteral("font-weight: bold;"));
        side_layout->addWidget(keep);

        if (left) {
            hash_a = hash_label;
            canvas_a = canvas;
            info_a = info_label;
            tags_a = tags;
            urls_a = urls;
            keep_a = keep;
        } else {
            hash_b = hash_label;
            canvas_b = canvas;
            info_b = info_label;
            tags_b = tags;
            urls_b = urls;
            keep_b = keep;
        }
    };

    make_side(true);
    make_side(false);
    splitter->setStretchFactor(0, 1);
    splitter->setStretchFactor(1, 1);
    layout->addWidget(splitter, 1);

    auto *bottom = new QHBoxLayout();
    same = new QPushButton(QStringLiteral("they are the same"), this);
    not_duplicates = new QPushButton(QStringLiteral("not duplicates"), this);
    skip = new QPushButton(QStringLiteral("skip"), this);
    bottom->addStretch(1);
    bottom->addWidget(same);
    bottom->addWidget(not_duplicates);
    bottom->addWidget(skip);
    bottom->addStretch(1);
    layout->addLayout(bottom);

    connect(keep_a, &QPushButton::clicked, this, [this]() { act(QStringLiteral("duplicates_keep_a")); });
    connect(keep_b, &QPushButton::clicked, this, [this]() { act(QStringLiteral("duplicates_keep_b")); });
    connect(same, &QPushButton::clicked, this, [this]() { act(QStringLiteral("duplicates_same")); });
    connect(not_duplicates, &QPushButton::clicked, this, [this]() { act(QStringLiteral("duplicates_false")); });
    connect(skip, &QPushButton::clicked, this, [this]() { act(QStringLiteral("duplicates_skip")); });
    connect(refresh_button, &QPushButton::clicked, this, [this]() { act(QStringLiteral("duplicates_refresh")); });
    connect(next_batch, &QPushButton::clicked, this, [this]() { act(QStringLiteral("duplicates_next_batch")); });
    connect(batch, &QSpinBox::valueChanged, this, [this](int value) {
        main->notify(QStringLiteral("duplicates_batch_size"), id, QString::number(value));
    });
    connect(rules, &QPushButton::clicked, this, [this]() {
        main->notify(QStringLiteral("duplicates_rules"));
    });
    connect(merge_options, &QPushButton::clicked, this, [this]() {
        main->notify(QStringLiteral("duplicates_merge_options_dialog"));
    });
    connect(review, &QPushButton::clicked, this, [this]() {
        main->notify(QStringLiteral("duplicates_review"));
    });
    connect(delete_both, &QPushButton::clicked, this, [this]() {
        if (QMessageBox::question(this, QStringLiteral("delete both"), QStringLiteral("Move both files to the trash?")) == QMessageBox::Yes) {
            act(QStringLiteral("duplicates_delete_both"));
        }
    });

    connect(dupe_search_type, &QComboBox::currentIndexChanged, this, [this]() {
        update_search_controls();
        notify_search_context();
    });
    connect(search_location_1, &QComboBox::currentTextChanged, this, [this]() { notify_search_context(); });
    connect(search_location_2, &QComboBox::currentTextChanged, this, [this]() { notify_search_context(); });
    connect(search_query_1, &QLineEdit::editingFinished, this, [this]() { notify_search_context(); });
    connect(search_query_2, &QLineEdit::editingFinished, this, [this]() { notify_search_context(); });
    connect(pixel_dupes, &QComboBox::currentIndexChanged, this, [this]() {
        update_search_controls();
        notify_search_context();
    });
    connect(search_distance, &QSpinBox::valueChanged, this, [this]() { notify_search_context(); });

    update_search_controls();

    update_state(state);
}

void HydrusDuplicatesPage::notify_search_context() {
    const QString search_type = dupe_search_type->currentData().toString();

    if (search_type != QStringLiteral("different_searches")) {
        const QSignalBlocker blocker(search_location_2);
        search_location_2->setCurrentText(search_location_1->currentText());
    }

    QJsonObject payload;
    payload.insert(QStringLiteral("query_1"), search_query_1->text());
    payload.insert(QStringLiteral("location_1"), search_location_1->currentText());
    payload.insert(QStringLiteral("query_2"), search_query_2->text());
    payload.insert(QStringLiteral("location_2"), search_location_2->currentText());
    payload.insert(QStringLiteral("dupe_search_type"), search_type);
    payload.insert(QStringLiteral("pixel_dupes"), pixel_dupes->currentData().toString());
    payload.insert(QStringLiteral("max_distance"), search_distance->value());

    main->notify(QStringLiteral("duplicates_search_context"), id, json_payload(payload));
}

void HydrusDuplicatesPage::update_search_controls() {
    const bool different_searches = dupe_search_type->currentData().toString() == QStringLiteral("different_searches");

    search_row_2_label->setVisible(different_searches);
    search_query_2->setVisible(different_searches);
    search_location_2->setVisible(different_searches);

    search_distance->setEnabled(pixel_dupes->currentData().toString() != QStringLiteral("required"));
}

const QString &HydrusDuplicatesPage::page_id() const {
    return id;
}

void HydrusDuplicatesPage::act(const QString &action) {
    main->notify(action, id);
}

void HydrusDuplicatesPage::update_state(const QJsonObject &state) {
    const int index = state.value(QStringLiteral("index")).toInt();
    const int total = state.value(QStringLiteral("total")).toInt();
    const QString page_status = state.value(QStringLiteral("status")).toString();

    status->setText(total == 0
        ? page_status
        : QStringLiteral("%1  -  %2/%3").arg(page_status).arg(index).arg(total));

    {
        const QSignalBlocker blocker(batch);
        batch->setValue(state.value(QStringLiteral("batch_size")).toInt(100));
    }

    const QJsonObject search = state.value(QStringLiteral("search")).toObject();

    if (!search.isEmpty()) {
        auto set_combo_data = [](QComboBox *combo, const QString &value) {
            const int index = combo->findData(value);

            if (index >= 0) {
                const QSignalBlocker blocker(combo);
                combo->setCurrentIndex(index);
            }
        };

        const QSignalBlocker query_1_blocker(search_query_1);
        const QSignalBlocker query_2_blocker(search_query_2);
        const QSignalBlocker distance_blocker(search_distance);

        search_query_1->setText(search.value(QStringLiteral("query_1")).toString());
        search_query_2->setText(search.value(QStringLiteral("query_2")).toString());
        search_distance->setValue(search.value(QStringLiteral("max_distance")).toInt(4));
        set_combo_data(dupe_search_type, search.value(QStringLiteral("dupe_search_type")).toString());
        set_combo_data(pixel_dupes, search.value(QStringLiteral("pixel_dupes")).toString());

        {
            const QSignalBlocker location_1_blocker(search_location_1);
            const QSignalBlocker location_2_blocker(search_location_2);
            const QString location_1 = search.value(QStringLiteral("location_1")).toString();
            const QString location_2 = search.value(QStringLiteral("location_2")).toString();

            if (!location_1.isEmpty()) {
                search_location_1->setCurrentText(location_1);
            }

            if (!location_2.isEmpty()) {
                search_location_2->setCurrentText(location_2);
            }
        }

        const int matched = search.value(QStringLiteral("matched")).toInt();
        const int searched = state.value(QStringLiteral("searched")).toInt();
        search_summary->setText(QStringLiteral("%1 of %2 pairs match  -  %3")
            .arg(matched)
            .arg(searched)
            .arg(search.value(QStringLiteral("summary")).toString()));
    }

    update_search_controls();

    const QJsonValue pair_value = state.value(QStringLiteral("pair"));

    if (!pair_value.isObject()) {
        distance->setText(QString());
        hash_a->setText(QStringLiteral("no pair"));
        hash_b->setText(QStringLiteral("no pair"));
        info_a->setText(QStringLiteral("the potential duplicate queue is empty"));
        info_b->clear();
        tags_a->clear();
        tags_b->clear();
        urls_a->clear();
        urls_b->clear();
        canvas_a->clear_image();
        canvas_b->clear_image();

        for (QPushButton *button : {keep_a, keep_b, same, not_duplicates, skip, delete_both}) {
            button->setEnabled(false);
        }

        return;
    }

    for (QPushButton *button : {keep_a, keep_b, same, not_duplicates, skip, delete_both}) {
        button->setEnabled(true);
    }

    const QJsonObject pair = pair_value.toObject();
    const QJsonObject a = pair.value(QStringLiteral("a")).toObject();
    const QJsonObject b = pair.value(QStringLiteral("b")).toObject();

    distance->setText(QStringLiteral("distance %1").arg(pair.value(QStringLiteral("distance")).toInt()));

    auto fill_side = [](const QJsonObject &object, QLabel *hash, QLabel *info, HydrusCanvas *canvas, QListWidget *tags, QListWidget *urls) {
        hash->setText(object.value(QStringLiteral("hash")).toString());
        info->setText(object.value(QStringLiteral("info")).toString());

        tags->clear();
        tags->addItems(json_to_strings(object.value(QStringLiteral("tags")).toArray()));

        urls->clear();
        urls->addItems(json_to_strings(object.value(QStringLiteral("urls")).toArray()));

        const QString preview = object.value(QStringLiteral("preview")).toString();

        if (!preview.isEmpty() && canvas->load_image(preview)) {
            return;
        }

        canvas->clear_image();
    };

    fill_side(a, hash_a, info_a, canvas_a, tags_a, urls_a);
    fill_side(b, hash_b, info_b, canvas_b, tags_b, urls_b);
}


HydrusViewerWindow::HydrusViewerWindow(HydrusMainWindow *main_window, const QJsonObject &state)
    : QWidget(nullptr), main(main_window) {
    id = state.value(QStringLiteral("viewer_id")).toString();
    page_id = state.value(QStringLiteral("page_id")).toString();
    hash = state.value(QStringLiteral("hash")).toString();

    setWindowFlag(Qt::Window, true);
    setAttribute(Qt::WA_DeleteOnClose, true);
    setWindowTitle(QStringLiteral("media viewer"));

    auto *layout = new QVBoxLayout(this);
    layout->setContentsMargins(0, 0, 0, 0);
    layout->setSpacing(0);

    auto *toolbar = new QWidget(this);
    auto *toolbar_layout = new QHBoxLayout(toolbar);
    toolbar_layout->setContentsMargins(4, 4, 4, 4);

    auto *previous = new QPushButton(QStringLiteral("<"), toolbar);
    auto *next = new QPushButton(QStringLiteral(">"), toolbar);
    auto *fit = new QPushButton(QStringLiteral("fit"), toolbar);
    auto *actual_size = new QPushButton(QStringLiteral("100%"), toolbar);
    auto *zoom_in = new QPushButton(QStringLiteral("+"), toolbar);
    auto *zoom_out = new QPushButton(QStringLiteral("-"), toolbar);
    pan_button = new QPushButton(QStringLiteral("pan"), toolbar);
    pan_button->setCheckable(true);
    background = new QComboBox(toolbar);
    background->addItems({
        QStringLiteral("dark"),
        QStringLiteral("black"),
        QStringLiteral("mid"),
        QStringLiteral("light"),
        QStringLiteral("checkerboard"),
    });
    auto *manage = new QPushButton(QStringLiteral("manage tags..."), toolbar);
    animation_button = new QPushButton(QStringLiteral("pause"), toolbar);
    animation_button->setEnabled(false);
    auto *external = new QPushButton(QStringLiteral("open externally"), toolbar);
    auto *copy_hash = new QPushButton(QStringLiteral("copy hash"), toolbar);
    auto *copy_url = new QPushButton(QStringLiteral("copy url"), toolbar);
    auto *close_button = new QPushButton(QStringLiteral("close"), toolbar);

    toolbar_layout->addWidget(previous);
    toolbar_layout->addWidget(next);
    toolbar_layout->addSpacing(12);
    toolbar_layout->addWidget(fit);
    toolbar_layout->addWidget(actual_size);
    toolbar_layout->addWidget(zoom_out);
    toolbar_layout->addWidget(zoom_in);
    toolbar_layout->addWidget(pan_button);
    toolbar_layout->addWidget(background);
    toolbar_layout->addWidget(animation_button);
    toolbar_layout->addStretch(1);
    toolbar_layout->addWidget(external);
    toolbar_layout->addWidget(copy_hash);
    toolbar_layout->addWidget(copy_url);
    toolbar_layout->addWidget(manage);
    toolbar_layout->addWidget(close_button);
    layout->addWidget(toolbar);

    auto *body = new QSplitter(Qt::Horizontal, this);

    metadata = new HydrusMetadataPanel(main, body);
    metadata->setMinimumWidth(240);
    metadata->setMaximumWidth(380);
    body->addWidget(metadata);

    canvas = new HydrusCanvas(body);
    body->addWidget(canvas);
    body->setStretchFactor(0, 0);
    body->setStretchFactor(1, 1);
    body->setSizes({240, 900});

    hover_frame = new QWidget(canvas);
    hover_frame->setStyleSheet(QStringLiteral(
        "QWidget { background: rgba(20, 20, 20, 170); border-radius: 4px; }"
        "QPushButton { background: #4a4a4a; color: #f0f0f0; padding: 3px 8px; }"));
    auto *hover_layout = new QHBoxLayout(hover_frame);
    hover_layout->setContentsMargins(6, 4, 6, 4);
    hover_layout->setSpacing(4);

    const auto add_hover = [this, hover_layout](const QString &label, const QString &action) {
        auto *button = new QPushButton(label, hover_frame);
        button->setAutoDefault(false);
        connect(button, &QPushButton::clicked, this, [this, action]() {
            if (!hash.isEmpty()) {
                main->notify(action, page_id, hash);
            }
        });
        hover_layout->addWidget(button);
    };

    add_hover(QStringLiteral("archive"), QStringLiteral("archive_files"));
    add_hover(QStringLiteral("inbox"), QStringLiteral("inbox_files"));
    add_hover(QStringLiteral("delete"), QStringLiteral("delete_files"));
    add_hover(QStringLiteral("tags"), QStringLiteral("manage_tags"));

    auto *notes_button = new QPushButton(QStringLiteral("notes"), hover_frame);
    notes_button->setAutoDefault(false);
    connect(notes_button, &QPushButton::clicked, this, [this]() {
        if (hash.isEmpty()) {
            return;
        }

        QJsonObject payload;
        payload.insert(QStringLiteral("hash"), hash);
        main->notify(
            QStringLiteral("notes_manage"),
            QString(),
            QString::fromUtf8(QJsonDocument(payload).toJson(QJsonDocument::Compact)));
    });
    hover_layout->addWidget(notes_button);

    hover_frame->hide();
    canvas->installEventFilter(this);

    layout->addWidget(body, 1);

    status = new QLabel(this);
    status->setContentsMargins(6, 2, 6, 4);
    layout->addWidget(status);

    connect(previous, &QPushButton::clicked, this, [this]() { navigate(QStringLiteral("prev")); });
    connect(next, &QPushButton::clicked, this, [this]() { navigate(QStringLiteral("next")); });
    connect(fit, &QPushButton::clicked, this, [this]() { canvas->fit_image(); });
    connect(actual_size, &QPushButton::clicked, this, [this]() { canvas->zoom_to(1.0); });
    connect(zoom_in, &QPushButton::clicked, this, [this]() { canvas->zoom_by(zoom_step); });
    connect(zoom_out, &QPushButton::clicked, this, [this]() { canvas->zoom_by(1.0 / zoom_step); });
    connect(pan_button, &QPushButton::toggled, this, [this](bool enabled) {
        canvas->set_pan_mode(enabled);
        QSettings settings;
        settings.setValue(QStringLiteral("viewer/pan"), enabled);
    });
    connect(background, &QComboBox::currentTextChanged, this, [this](const QString &mode) {
        canvas->set_background_mode(mode);
        QSettings settings;
        settings.setValue(QStringLiteral("viewer/background"), mode);
    });

    {
        QSettings settings;
        const QString mode = settings.value(QStringLiteral("viewer/background"), QStringLiteral("dark")).toString();
        const bool pan = settings.value(QStringLiteral("viewer/pan"), true).toBool();
        background->setCurrentText(mode);
        pan_button->setChecked(pan);
        canvas->set_background_mode(mode);
        canvas->set_pan_mode(pan);
    }
    connect(animation_button, &QPushButton::clicked, this, [this]() {
        canvas->toggle_animation();

        if (animation_button->text() == QStringLiteral("pause")) {
            animation_button->setText(QStringLiteral("play"));
        } else {
            animation_button->setText(QStringLiteral("pause"));
        }
    });
    connect(external, &QPushButton::clicked, this, [this]() {
        if (!file_path.isEmpty()) {
            QDesktopServices::openUrl(QUrl::fromLocalFile(file_path));
        }
    });
    connect(copy_hash, &QPushButton::clicked, this, [this]() {
        QApplication::clipboard()->setText(hash);
    });
    connect(copy_url, &QPushButton::clicked, this, [this]() {
        if (!url.isEmpty()) {
            QApplication::clipboard()->setText(url);
        }
    });
    connect(manage, &QPushButton::clicked, this, [this]() {
        const QByteArray page_bytes = page_id.toUtf8();
        const QByteArray hash_bytes = hash.toUtf8();
        app_action(*main, rust::Str("manage_tags", 11), rust::Str(page_bytes.constData(), page_bytes.size()), rust::Str(hash_bytes.constData(), hash_bytes.size()));
    });
    connect(close_button, &QPushButton::clicked, this, [this]() { QWidget::close(); });

    auto bind_shortcut = [this](const QString &key, auto callable) {
        auto *shortcut = new QShortcut(QKeySequence(key), this);
        shortcut->setContext(Qt::WindowShortcut);
        connect(shortcut, &QShortcut::activated, this, callable);
    };

    bind_shortcut(QStringLiteral("Right"), [this]() { navigate(QStringLiteral("next")); });
    bind_shortcut(QStringLiteral("Space"), [this]() { navigate(QStringLiteral("next")); });
    bind_shortcut(QStringLiteral("Left"), [this]() { navigate(QStringLiteral("prev")); });
    bind_shortcut(QStringLiteral("Backspace"), [this]() { navigate(QStringLiteral("prev")); });
    bind_shortcut(QStringLiteral("Escape"), [this]() { QWidget::close(); });
    bind_shortcut(QStringLiteral("F"), [this]() { canvas->fit_image(); });
    bind_shortcut(QStringLiteral("+"), [this]() { canvas->zoom_by(zoom_step); });
    bind_shortcut(QStringLiteral("="), [this]() { canvas->zoom_by(zoom_step); });
    bind_shortcut(QStringLiteral("-"), [this]() { canvas->zoom_by(1.0 / zoom_step); });
    bind_shortcut(QStringLiteral("0"), [this]() { canvas->reset_zoom(); });
    bind_shortcut(QStringLiteral("R"), [this]() { canvas->rotate_by(90); });
    bind_shortcut(QStringLiteral("Shift+R"), [this]() { canvas->rotate_by(-90); });
    bind_shortcut(QStringLiteral("P"), [this]() { animation_button->click(); });
    bind_shortcut(QStringLiteral("F11"), [this]() {
        if (isFullScreen()) {
            showNormal();
        } else {
            showFullScreen();
        }
    });
    bind_shortcut(QStringLiteral("M"), [this]() {
        const QByteArray page_bytes = page_id.toUtf8();
        const QByteArray hash_bytes = hash.toUtf8();
        app_action(*main, rust::Str("manage_tags", 11), rust::Str(page_bytes.constData(), page_bytes.size()), rust::Str(hash_bytes.constData(), hash_bytes.size()));
    });

    bind_shortcut(QStringLiteral("A"), [this]() {
        const QByteArray page_bytes = page_id.toUtf8();
        const QByteArray hash_bytes = hash.toUtf8();
        app_action(*main, rust::Str("archive_files", 13), rust::Str(page_bytes.constData(), page_bytes.size()), rust::Str(hash_bytes.constData(), hash_bytes.size()));
    });

    bind_shortcut(QStringLiteral("I"), [this]() {
        const QByteArray page_bytes = page_id.toUtf8();
        const QByteArray hash_bytes = hash.toUtf8();
        app_action(*main, rust::Str("inbox_files", 11), rust::Str(page_bytes.constData(), page_bytes.size()), rust::Str(hash_bytes.constData(), hash_bytes.size()));
    });

    bind_shortcut(QStringLiteral("Delete"), [this]() {
        const QByteArray page_bytes = page_id.toUtf8();
        const QByteArray hash_bytes = hash.toUtf8();
        app_action(*main, rust::Str("delete_files", 12), rust::Str(page_bytes.constData(), page_bytes.size()), rust::Str(hash_bytes.constData(), hash_bytes.size()));
    });

    update_state(state);

    QSettings settings;
    const QByteArray geometry = settings.value(QStringLiteral("viewer/geometry")).toByteArray();

    if (!geometry.isEmpty()) {
        restoreGeometry(geometry);
    } else {
        resize(1100, 820);
    }

    if (qEnvironmentVariableIsSet("HYDRUS_QT_HOVER")) {
        QTimer::singleShot(300, this, [this]() { show_hover_frame(); });
    }
}

const QString &HydrusViewerWindow::viewer_id() const {
    return id;
}

void HydrusViewerWindow::update_state(const QJsonObject &state) {
    page_id = state.value(QStringLiteral("page_id")).toString();
    hash = state.value(QStringLiteral("hash")).toString();
    file_path = state.value(QStringLiteral("file_path")).toString();
    url = state.value(QStringLiteral("url")).toString();

    const QString path = state.value(QStringLiteral("path")).toString();
    const QString title = state.value(QStringLiteral("title")).toString();
    const bool on_top = state.value(QStringLiteral("always_on_top")).toBool(false);
    const bool borderless = state.value(QStringLiteral("frameless")).toBool(false);

    if (on_top != always_on_top || borderless != frameless) {
        always_on_top = on_top;
        frameless = borderless;

        Qt::WindowFlags flags = Qt::Window;
        const Qt::WindowFlags old_flags = windowFlags();

        if (always_on_top) {
            flags |= Qt::WindowStaysOnTopHint;
        }

        if (frameless) {
            flags |= Qt::FramelessWindowHint;
        }

        if (flags != old_flags) {
            const bool was_visible = isVisible();
            setWindowFlags(flags);

            if (was_visible) {
                show();
            }
        }
    }

    const double step = state.value(QStringLiteral("zoom_step_percent")).toDouble(25.0);

    if (step >= 5.0) {
        zoom_step = 1.0 + step / 100.0;
    }

    canvas->set_recenter_on_resize(state.value(QStringLiteral("recenter")).toBool(true));

    setWindowTitle(title.isEmpty() ? QStringLiteral("media viewer") : title);

    if (!path.isEmpty()) {
        canvas->load_image(path);
    } else {
        canvas->clear_image();
    }

    animation_button->setEnabled(canvas->is_animated());
    animation_button->setText(QStringLiteral("pause"));

    for (const QString &neighbour : {
             state.value(QStringLiteral("previous_path")).toString(),
             state.value(QStringLiteral("next_path")).toString(),
         }) {
        if (neighbour.isEmpty()) {
            continue;
        }

        QPixmap cached;

        if (!QPixmapCache::find(neighbour, &cached)) {
            cached.load(neighbour);

            if (!cached.isNull()) {
                QPixmapCache::insert(neighbour, cached);
            }
        }
    }

    metadata->set_state(state.value(QStringLiteral("metadata")).toObject());
    metadata->set_suggestions(main->suggestions());

    status->setText(state.value(QStringLiteral("info")).toString());
}

void HydrusViewerWindow::show_hover_frame() {
    if (hover_frame == nullptr || canvas == nullptr) {
        return;
    }

    hover_frame->adjustSize();
    hover_frame->move(8, 8);
    hover_frame->raise();
    hover_frame->show();
}

bool HydrusViewerWindow::eventFilter(QObject *watched, QEvent *event) {
    if (watched == canvas && event->type() == QEvent::MouseMove && hover_frame != nullptr) {
        auto *mouse = static_cast<QMouseEvent *>(event);

        if (mouse->pos().y() < 90) {
            show_hover_frame();
        } else {
            hover_frame->hide();
        }
    }

    return QWidget::eventFilter(watched, event);
}

void HydrusViewerWindow::navigate(const QString &direction) {
    const QByteArray viewer_bytes = id.toUtf8();
    const QByteArray direction_bytes = direction.toUtf8();
    app_action(*main, rust::Str("viewer_nav", 10), rust::Str(viewer_bytes.constData(), viewer_bytes.size()), rust::Str(direction_bytes.constData(), direction_bytes.size()));
}

void HydrusViewerWindow::closeEvent(QCloseEvent *event) {
    QSettings settings;
    settings.setValue(QStringLiteral("viewer/geometry"), saveGeometry());
    settings.sync();

    const QByteArray viewer_bytes = id.toUtf8();
    app_action(*main, rust::Str("viewer_close", 12), rust::Str(viewer_bytes.constData(), viewer_bytes.size()), rust::Str());

    QWidget::closeEvent(event);
}

// ---------------------------------------------------------------------------
// HydrusTabBar
// ---------------------------------------------------------------------------

namespace {
const QString kPageMimeType = QStringLiteral("application/x-hydrus-page");
}

// A tab bar that starts a drag for pages and accepts drops from any notebook.
// Ordering and cross-notebook moves are applied by the Rust page model.
class HydrusTabBar : public QTabBar {
public:
    HydrusTabBar(HydrusMainWindow *main_window, HydrusNotebook *notebook_widget, QWidget *parent = nullptr)
        : QTabBar(parent), main(main_window), notebook(notebook_widget) {
        setAcceptDrops(true);
        setMovable(false);
        setDrawBase(false);
    }

protected:
    void mousePressEvent(QMouseEvent *event) override {
        if (event->button() == Qt::LeftButton) {
            press_position = event->pos();
        }

        QTabBar::mousePressEvent(event);
    }

    void mouseMoveEvent(QMouseEvent *event) override {
        if (!(event->buttons() & Qt::LeftButton)) {
            QTabBar::mouseMoveEvent(event);
            return;
        }

        if ((event->pos() - press_position).manhattanLength() < QApplication::startDragDistance()) {
            QTabBar::mouseMoveEvent(event);
            return;
        }

        const int index = tabAt(press_position);
        const QPoint pressed = press_position;
        press_position = QPoint(-1000, -1000);

        if (index < 0) {
            return;
        }

        const QString page_id = tabData(index).toString();

        if (page_id.isEmpty() || page_id == QStringLiteral("__plus__")) {
            return;
        }

        auto *mime = new QMimeData();
        mime->setData(kPageMimeType, page_id.toUtf8());

        auto *drag = new QDrag(this);
        drag->setMimeData(mime);
        drag->exec(Qt::MoveAction);

        Q_UNUSED(pressed);
    }

    void dragEnterEvent(QDragEnterEvent *event) override {
        if (event->mimeData()->hasFormat(kPageMimeType)) {
            event->acceptProposedAction();
            return;
        }

        QTabBar::dragEnterEvent(event);
    }

    void dragMoveEvent(QDragMoveEvent *event) override {
        if (event->mimeData()->hasFormat(kPageMimeType)) {
            event->acceptProposedAction();
            return;
        }

        QTabBar::dragMoveEvent(event);
    }

    void dropEvent(QDropEvent *event) override {
        if (!event->mimeData()->hasFormat(kPageMimeType)) {
            QTabBar::dropEvent(event);
            return;
        }

        const QString source = QString::fromUtf8(event->mimeData()->data(kPageMimeType));
        const int index = tabAt(event->position().toPoint());

        QJsonObject request;
        request.insert(QStringLiteral("target"), notebook->notebook_id());
        request.insert(QStringLiteral("position"), QStringLiteral("inside"));

        if (index >= 0 && index < count() - 1) {
            const QString target = tabData(index).toString();

            if (!target.isEmpty() && target != QStringLiteral("__plus__") && target != source) {
                request.insert(QStringLiteral("target"), target);
                request.insert(QStringLiteral("position"), QStringLiteral("before"));
            }
        }

        const QByteArray payload = QJsonDocument(request).toJson(QJsonDocument::Compact);
        main->notify(QStringLiteral("move_page"), source, QString::fromUtf8(payload));
        event->acceptProposedAction();
    }

private:
    HydrusMainWindow *main = nullptr;
    HydrusNotebook *notebook = nullptr;
    QPoint press_position{-1000, -1000};
};

// ---------------------------------------------------------------------------
// HydrusNotebook
// ---------------------------------------------------------------------------

HydrusNotebook::HydrusNotebook(HydrusMainWindow *main_window, const QString &notebook_id, QWidget *parent)
    : QTabWidget(parent), main(main_window), id(notebook_id) {
    setTabBar(new HydrusTabBar(main_window, this, this));
    setTabsClosable(true);
    setMovable(false);
    setDocumentMode(true);
    setAcceptDrops(true);

    connect(this, &QTabWidget::tabCloseRequested, this, [this](int index) { request_close(index); });

    connect(tabBar(), &QTabBar::tabBarDoubleClicked, this, [this](int index) {
        if (index < 0 || index == plus_index) {
            return;
        }

        const QString page_id = tabBar()->tabData(index).toString();

        if (page_id.isEmpty()) {
            return;
        }

        bool ok = false;
        const QString name = QInputDialog::getText(
            this,
            QStringLiteral("rename page"),
            QStringLiteral("page name"),
            QLineEdit::Normal,
            tabText(index),
            &ok);

        if (ok && !name.isEmpty()) {
            QByteArray page_bytes = page_id.toUtf8();
            QByteArray name_bytes = name.toUtf8();
            app_action(*main, rust::Str("rename_page", 11), rust::Str(page_bytes.constData(), page_bytes.size()), rust::Str(name_bytes.constData(), name_bytes.size()));
        }
    });

    connect(this, &QTabWidget::currentChanged, this, [this](int index) {
        if (main->building || index < 0) {
            return;
        }

        if (index == plus_index) {
            if (last_normal_index >= 0 && last_normal_index < count()) {
                setCurrentIndex(last_normal_index);
            }

            main->show_page_chooser(id);
            return;
        }

        last_normal_index = index;

        const QString page_id = tabBar()->tabData(index).toString();

        if (page_id.isEmpty()) {
            return;
        }

        main->active_page = page_id;
        main->active_notebook = id;

        QByteArray page_bytes = page_id.toUtf8();
        QByteArray notebook_bytes = id.toUtf8();
        app_action(*main, rust::Str("page_selected", 13), rust::Str(page_bytes.constData(), page_bytes.size()), rust::Str(notebook_bytes.constData(), notebook_bytes.size()));
    });

    tabBar()->setContextMenuPolicy(Qt::CustomContextMenu);
    connect(tabBar(), &QTabBar::customContextMenuRequested, this, [this](const QPoint &position) {
        show_tab_menu(position);
    });

    tabBar()->installEventFilter(this);
}

bool HydrusNotebook::eventFilter(QObject *watched, QEvent *event) {
    if (watched == tabBar() && event->type() == QEvent::MouseButtonRelease) {
        auto *mouse = static_cast<QMouseEvent *>(event);

        if (mouse->button() == Qt::MiddleButton) {
            request_close(tabBar()->tabAt(mouse->pos()));
            return true;
        }
    }

    return QTabWidget::eventFilter(watched, event);
}

const QString &HydrusNotebook::notebook_id() const {
    return id;
}

void HydrusNotebook::add_page_widget(const QString &page_id, const QString &title, QWidget *page) {
    const int index = plus_index >= 0 ? plus_index : count();
    const int inserted = insertTab(index, page, title);

    tabBar()->setTabData(inserted, page_id);
    tabBar()->setTabToolTip(inserted, title);

    if (plus_index >= 0 && inserted <= plus_index) {
        plus_index += 1;
    }
}

void HydrusNotebook::install_plus_tab() {
    if (plus_index >= 0) {
        return;
    }

    auto *blank = new QWidget(this);
    plus_index = addTab(blank, QStringLiteral("+"));
    tabBar()->setTabData(plus_index, QStringLiteral("__plus__"));
    tabBar()->setTabToolTip(plus_index, QStringLiteral("open a new page"));
    tabBar()->setTabButton(plus_index, QTabBar::RightSide, nullptr);
    tabBar()->setTabButton(plus_index, QTabBar::LeftSide, nullptr);
}

void HydrusNotebook::finalize_notebook(const QString &current_page_id) {
    install_plus_tab();

    int target = -1;

    if (!current_page_id.isEmpty()) {
        for (int index = 0; index < count(); ++index) {
            if (tabBar()->tabData(index).toString() == current_page_id) {
                target = index;
                break;
            }
        }
    }

    if (target < 0 && count() > 1) {
        target = 0;
    }

    if (target >= 0) {
        last_normal_index = target;
        setCurrentIndex(target);
    } else {
        last_normal_index = 0;
    }
}

void HydrusNotebook::clear_pages() {
    plus_index = -1;

    while (count() > 0) {
        QWidget *page = widget(0);
        removeTab(0);

        if (page != nullptr) {
            page->deleteLater();
        }
    }
}

QString HydrusNotebook::current_page_id() const {
    const int index = currentIndex();

    if (index < 0) {
        return {};
    }

    const QString tab_data = tabBar()->tabData(index).toString();

    if (tab_data == QStringLiteral("__plus__")) {
        return {};
    }

    return tab_data;
}

void HydrusNotebook::request_close(int index) {
    if (index < 0 || index == plus_index) {
        return;
    }

    const QString page_id = tabBar()->tabData(index).toString();

    if (page_id.isEmpty()) {
        return;
    }

    QByteArray page_bytes = page_id.toUtf8();
    app_action(*main, rust::Str("close_page", 10), rust::Str(page_bytes.constData(), page_bytes.size()), rust::Str());
}

void HydrusNotebook::show_tab_menu(const QPoint &position) {
    const int index = tabBar()->tabAt(position);

    if (index < 0 || index == plus_index) {
        QMenu menu(this);
        QAction *new_page = menu.addAction(QStringLiteral("new page..."));
        QAction *chosen = menu.exec(tabBar()->mapToGlobal(position));

        if (chosen == new_page) {
            main->show_page_chooser(id);
        }

        return;
    }

    const QString page_id = tabBar()->tabData(index).toString();

    QMenu menu(this);
    QAction *close = menu.addAction(QStringLiteral("close page"));
    QAction *rename = menu.addAction(QStringLiteral("rename page..."));
    QAction *new_page = menu.addAction(QStringLiteral("new page..."));
    menu.addSeparator();
    QAction *close_others = menu.addAction(QStringLiteral("close other pages"));
    menu.addSeparator();
    QAction *refresh = menu.addAction(QStringLiteral("refresh"));

    QAction *chosen = menu.exec(tabBar()->mapToGlobal(position));

    if (chosen == nullptr) {
        return;
    }

    if (chosen == close) {
        request_close(index);
        return;
    }

    if (chosen == new_page) {
        main->show_page_chooser(id);
        return;
    }

    if (chosen == refresh) {
        QByteArray page_bytes = page_id.toUtf8();
        app_action(*main, rust::Str("refresh_page", 12), rust::Str(page_bytes.constData(), page_bytes.size()), rust::Str());
        return;
    }

    if (chosen == close_others) {
        QByteArray page_bytes = page_id.toUtf8();
        app_action(*main, rust::Str("close_other_pages", 17), rust::Str(page_bytes.constData(), page_bytes.size()), rust::Str());
        return;
    }

    if (chosen == rename) {
        bool ok = false;
        const QString name = QInputDialog::getText(
            this,
            QStringLiteral("rename page"),
            QStringLiteral("page name"),
            QLineEdit::Normal,
            tabText(index),
            &ok);

        if (ok && !name.isEmpty()) {
            QByteArray page_bytes = page_id.toUtf8();
            QByteArray name_bytes = name.toUtf8();
            app_action(*main, rust::Str("rename_page", 11), rust::Str(page_bytes.constData(), page_bytes.size()), rust::Str(name_bytes.constData(), name_bytes.size()));
        }
    }
}

// ---------------------------------------------------------------------------
// Page chooser
// ---------------------------------------------------------------------------

class HydrusPageChooser : public QDialog {
public:
    explicit HydrusPageChooser(const QString &notebook_id, QWidget *parent = nullptr)
        : QDialog(parent), notebook(notebook_id) {
        setWindowTitle(QStringLiteral("new page"));
        resize(460, 320);

        auto *layout = new QVBoxLayout(this);
        stack = new QStackedWidget(this);
        layout->addWidget(stack, 1);

        build_home();
        build_search();
        build_download();
        build_special();

        auto *buttons = new QDialogButtonBox(QDialogButtonBox::Cancel, this);
        connect(buttons, &QDialogButtonBox::rejected, this, &QDialog::reject);
        layout->addWidget(buttons);
    }

    QString result;

private:
    QWidget *make_page() {
        auto *page = new QWidget(stack);
        auto *layout = new QVBoxLayout(page);
        layout->setSpacing(8);
        stack->addWidget(page);
        page_layout = layout;
        return page;
    }

    QPushButton *add_button(const QString &text, const QString &type) {
        auto *button = new QPushButton(text, stack->currentWidget());
        button->setMinimumHeight(34);
        connect(button, &QPushButton::clicked, this, [this, type]() {
            result = type;
            accept();
        });
        page_layout->addWidget(button);
        return button;
    }

    QPushButton *add_nav(const QString &text, int target) {
        auto *button = new QPushButton(text, stack->currentWidget());
        connect(button, &QPushButton::clicked, this, [this, target]() { stack->setCurrentIndex(target); });
        page_layout->addWidget(button);
        return button;
    }

    void build_home() {
        make_page();
        page_layout->addWidget(new QLabel(QStringLiteral("choose a page type"), stack->currentWidget()));
        add_nav(QStringLiteral("file search"), 1);
        add_nav(QStringLiteral("download"), 2);
        add_nav(QStringLiteral("special"), 3);
        page_layout->addStretch(1);
    }

    void build_search() {
        make_page();
        page_layout->addWidget(new QLabel(QStringLiteral("file search"), stack->currentWidget()));
        add_button(QStringLiteral("my files"), QStringLiteral("query:my files"));
        add_button(QStringLiteral("all local files"), QStringLiteral("query:all local files"));
        add_button(QStringLiteral("trash"), QStringLiteral("query:trash"));
        add_nav(QStringLiteral("back"), 0);
        page_layout->addStretch(1);
    }

    void build_download() {
        make_page();
        page_layout->addWidget(new QLabel(QStringLiteral("download"), stack->currentWidget()));
        add_button(QStringLiteral("urls"), QStringLiteral("urls"));
        add_button(QStringLiteral("watcher"), QStringLiteral("watcher"));
        add_button(QStringLiteral("gallery"), QStringLiteral("gallery"));
        add_button(QStringLiteral("simple downloader"), QStringLiteral("simple"));
        add_button(QStringLiteral("subscriptions"), QStringLiteral("subscriptions"));
        add_nav(QStringLiteral("back"), 0);
        page_layout->addStretch(1);
    }

    void build_special() {
        make_page();
        page_layout->addWidget(new QLabel(QStringLiteral("special"), stack->currentWidget()));
        add_button(QStringLiteral("page of pages"), QStringLiteral("notebook"));
        add_button(QStringLiteral("duplicates processing"), QStringLiteral("duplicates"));
        add_button(QStringLiteral("import folders"), QStringLiteral("import_folders"));
        add_button(QStringLiteral("export folders"), QStringLiteral("export_folders"));
        add_nav(QStringLiteral("back"), 0);
        page_layout->addStretch(1);
    }

    QString notebook;
    QStackedWidget *stack = nullptr;
    QVBoxLayout *page_layout = nullptr;
};

// ---------------------------------------------------------------------------
// Options and manager dialogs
// ---------------------------------------------------------------------------

namespace {

struct OptionField {
    QString key;
    QString label;
    QString type;
    bool noneable = false;
    QCheckBox *boolean = nullptr;
    QSpinBox *integer = nullptr;
    QLineEdit *text = nullptr;
    QWidget *page = nullptr;
    QWidget *label_widget = nullptr;
    QWidget *input_widget = nullptr;
};

class HydrusOptionsDialog : public QDialog {
public:
    HydrusOptionsDialog(HydrusMainWindow *main_window, const QJsonObject &state, QWidget *parent)
        : QDialog(parent), main(main_window) {
        setWindowTitle(QStringLiteral("options"));
        resize(720, 600);

        auto *layout = new QVBoxLayout(this);
        search = new QLineEdit(this);
        search->setPlaceholderText(QStringLiteral("search options..."));
        layout->addWidget(search);

        auto *tabs = new QTabWidget(this);
        layout->addWidget(tabs, 1);

        const QJsonArray tab_array = state.value(QStringLiteral("tabs")).toArray();

        for (const QJsonValue &tab_value : tab_array) {
            const QJsonObject tab = tab_value.toObject();
            auto *page = new QWidget(tabs);
            auto *form = new QFormLayout(page);
            form->setContentsMargins(8, 8, 8, 8);

            for (const QJsonValue &field_value : tab.value(QStringLiteral("fields")).toArray()) {
                const QJsonObject field = field_value.toObject();
                OptionField entry;
                entry.key = field.value(QStringLiteral("key")).toString();
                entry.type = field.value(QStringLiteral("type")).toString();
                entry.noneable = field.value(QStringLiteral("noneable")).toBool(false);
                entry.page = page;

                const QString label = field.value(QStringLiteral("label")).toString();
                entry.label = label;
                auto *label_widget = new QLabel(label, page);
                entry.label_widget = label_widget;
                const QJsonValue value = field.value(QStringLiteral("value"));

                if (entry.type == QStringLiteral("bool")) {
                    entry.boolean = new QCheckBox(page);
                    entry.boolean->setChecked(value.toBool());
                    form->addRow(label_widget, entry.boolean);
                    entry.input_widget = entry.boolean;
                } else if (entry.type == QStringLiteral("int")) {
                    entry.integer = new QSpinBox(page);
                    entry.integer->setRange(-2'000'000'000, 2'000'000'000);
                    entry.integer->setValue(value.toInt());
                    form->addRow(label_widget, entry.integer);
                    entry.input_widget = entry.integer;
                } else if (entry.type == QStringLiteral("string")) {
                    entry.text = new QLineEdit(page);
                    entry.text->setText(value.toString());
                    form->addRow(label_widget, entry.text);
                    entry.input_widget = entry.text;
                } else {
                    auto *readonly = new QLabel(value.toString(), page);
                    readonly->setTextInteractionFlags(Qt::TextSelectableByMouse);
                    form->addRow(label_widget, readonly);
                    entry.input_widget = readonly;
                }

                fields.append(entry);
            }

            tabs->addTab(page, tab.value(QStringLiteral("title")).toString());
        }

        connect(search, &QLineEdit::textChanged, this, [this, tabs](const QString &text) {
            const QString needle = text.trimmed();

            for (const OptionField &field : fields) {
                const bool visible = needle.isEmpty()
                    || field.label.contains(needle, Qt::CaseInsensitive)
                    || field.key.contains(needle, Qt::CaseInsensitive);

                if (field.label_widget != nullptr) {
                    field.label_widget->setVisible(visible);
                }
                if (field.input_widget != nullptr) {
                    field.input_widget->setVisible(visible);
                }
            }

            for (int index = 0; index < tabs->count(); ++index) {
                QWidget *page = tabs->widget(index);
                bool any_visible = false;

                for (const OptionField &field : fields) {
                    if (field.page == page
                        && field.input_widget != nullptr
                        && !field.input_widget->isHidden()) {
                        any_visible = true;
                        break;
                    }
                }

                tabs->setTabVisible(index, any_visible || needle.isEmpty());
            }
        });

        auto *buttons = new QDialogButtonBox(QDialogButtonBox::Save | QDialogButtonBox::Cancel, this);
        connect(buttons, &QDialogButtonBox::accepted, this, [this]() { apply(); });
        connect(buttons, &QDialogButtonBox::rejected, this, &QDialog::reject);
        layout->addWidget(buttons);
    }

private:
    void apply() {
        QJsonObject booleans;
        QJsonObject integers;
        QJsonObject strings;
        QJsonObject noneable_strings;

        for (const OptionField &field : fields) {
            if (field.boolean != nullptr) {
                booleans.insert(field.key, field.boolean->isChecked());
            } else if (field.integer != nullptr) {
                integers.insert(field.key, field.integer->value());
            } else if (field.text != nullptr) {
                if (field.noneable) {
                    noneable_strings.insert(field.key, field.text->text());
                } else {
                    strings.insert(field.key, field.text->text());
                }
            }
        }

        QJsonObject payload;
        payload.insert(QStringLiteral("booleans"), booleans);
        payload.insert(QStringLiteral("integers"), integers);
        payload.insert(QStringLiteral("strings"), strings);
        payload.insert(QStringLiteral("noneable_strings"), noneable_strings);

        main->notify(QStringLiteral("options_apply"), QString(), json_payload(payload));
        accept();
    }

    HydrusMainWindow *main = nullptr;
    QLineEdit *search = nullptr;
    QVector<OptionField> fields;
};

} // namespace

namespace {

class HydrusUrlClassDialog : public QDialog {
public:
    HydrusUrlClassDialog(const QJsonObject &entry, const QJsonArray &types, QWidget *parent = nullptr)
        : QDialog(parent), data(entry) {
        setWindowTitle(QStringLiteral("url class"));
        resize(620, 560);

        auto *layout = new QVBoxLayout(this);
        auto *form = new QFormLayout();

        name = new QLineEdit(this);
        type = new QComboBox(this);
        scheme = new QLineEdit(this);
        netloc = new QLineEdit(this);
        path = new QLineEdit(this);
        parameters = new QPlainTextEdit(this);
        parameters->setMaximumHeight(80);
        parameters->setPlaceholderText(QStringLiteral("one name=value per line"));
        example = new QLineEdit(this);
        match_subdomains = new QCheckBox(QStringLiteral("match subdomains"), this);
        keep_subdomains = new QCheckBox(QStringLiteral("keep the matched subdomain"), this);
        multiple = new QCheckBox(QStringLiteral("can produce multiple files"), this);
        associate = new QCheckBox(QStringLiteral("associate with files"), this);
        associate->setChecked(true);

        for (const QJsonValue &value : types) {
            const QJsonObject item = value.toObject();
            type->addItem(item.value(QStringLiteral("label")).toString(), item.value(QStringLiteral("value")).toInt());
        }

        form->addRow(QStringLiteral("name"), name);
        form->addRow(QStringLiteral("url type"), type);
        form->addRow(QStringLiteral("preferred scheme"), scheme);
        form->addRow(QStringLiteral("domain"), netloc);
        form->addRow(QStringLiteral("path"), path);
        form->addRow(QStringLiteral("parameters"), parameters);
        form->addRow(QStringLiteral("example url"), example);
        form->addRow(match_subdomains);
        form->addRow(keep_subdomains);
        form->addRow(multiple);
        form->addRow(associate);
        layout->addLayout(form);

        auto *test_row = new QWidget(this);
        auto *test_layout = new QHBoxLayout(test_row);
        test_layout->setContentsMargins(0, 0, 0, 0);
        test = new QLineEdit(test_row);
        test->setPlaceholderText(QStringLiteral("paste a url to test matching"));
        auto *test_button = new QPushButton(QStringLiteral("test"), test_row);
        test_layout->addWidget(test, 1);
        test_layout->addWidget(test_button);
        layout->addWidget(test_row);

        result = new QLabel(this);
        result->setWordWrap(true);
        layout->addWidget(result);

        auto *buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, this);
        connect(buttons, &QDialogButtonBox::accepted, this, &QDialog::accept);
        connect(buttons, &QDialogButtonBox::rejected, this, &QDialog::reject);
        connect(test_button, &QPushButton::clicked, this, [this]() { run_test(); });
        layout->addWidget(buttons);

        name->setText(data.value(QStringLiteral("name")).toString());
        scheme->setText(data.value(QStringLiteral("preferred_scheme")).toString());
        if (scheme->text().isEmpty()) {
            scheme->setText(QStringLiteral("https"));
        }
        netloc->setText(data.value(QStringLiteral("netloc")).toString());
        path->setText(json_to_strings(data.value(QStringLiteral("components")).toArray()).join(QStringLiteral("/")));
        parameters->setPlainText(data.value(QStringLiteral("parameters")).toString());
        example->setText(data.value(QStringLiteral("example_url")).toString());
        match_subdomains->setChecked(data.value(QStringLiteral("match_subdomains")).toBool());
        keep_subdomains->setChecked(data.value(QStringLiteral("keep_matched_subdomains")).toBool());
        multiple->setChecked(data.value(QStringLiteral("can_produce_multiple_files")).toBool());
        associate->setChecked(data.value(QStringLiteral("should_be_associated_with_files")).toBool(true));

        const int index = type->findData(data.value(QStringLiteral("url_type")).toInt(0));
        if (index >= 0) {
            type->setCurrentIndex(index);
        }
    }

    QJsonObject payload() const {
        QJsonObject value;
        value.insert(QStringLiteral("original"), data.value(QStringLiteral("name")).toString());
        value.insert(QStringLiteral("name"), name->text().trimmed());
        value.insert(QStringLiteral("url_type"), type->currentData().toInt());
        value.insert(QStringLiteral("preferred_scheme"), scheme->text().trimmed());
        value.insert(QStringLiteral("netloc"), netloc->text().trimmed());
        value.insert(QStringLiteral("path"), path->text().trimmed());
        value.insert(QStringLiteral("parameters"), parameters->toPlainText());
        value.insert(QStringLiteral("example_url"), example->text().trimmed());
        value.insert(QStringLiteral("match_subdomains"), match_subdomains->isChecked());
        value.insert(QStringLiteral("keep_matched_subdomains"), keep_subdomains->isChecked());
        value.insert(QStringLiteral("can_produce_multiple_files"), multiple->isChecked());
        value.insert(QStringLiteral("should_be_associated_with_files"), associate->isChecked());

        return value;
    }

private:
    void run_test() {
        const QUrl url(test->text().trimmed());

        if (!url.isValid() || url.host().isEmpty()) {
            result->setText(QStringLiteral("not a valid http(s) url"));
            return;
        }

        QString domain = netloc->text().trimmed().toLower();
        domain.remove(QStringLiteral("www."));
        QString host = url.host().toLower();
        host.remove(QStringLiteral("www."));

        if (host != domain
            && !(match_subdomains->isChecked() && host.endsWith(QStringLiteral(".") + domain))) {
            result->setText(QStringLiteral("domain does not match %1").arg(domain));
            return;
        }

        const QStringList expected = path->text().trimmed().split(QLatin1Char('/'), Qt::SkipEmptyParts);
        const QStringList actual = url.path().trimmed().split(QLatin1Char('/'), Qt::SkipEmptyParts);

        if (expected.size() != actual.size()) {
            result->setText(QStringLiteral("path has %1 segments; expected %2").arg(actual.size()).arg(expected.size()));
            return;
        }

        for (int index = 0; index < expected.size(); ++index) {
            const QString component = expected.at(index);

            if (component.startsWith(QLatin1Char('{')) && component.endsWith(QLatin1Char('}'))) {
                continue;
            }

            if (component != actual.at(index)) {
                result->setText(QStringLiteral("segment %1 does not match").arg(index + 1));
                return;
            }
        }

        result->setText(QStringLiteral("matches this url class"));
    }

    QJsonObject data;
    QLineEdit *name = nullptr;
    QComboBox *type = nullptr;
    QLineEdit *scheme = nullptr;
    QLineEdit *netloc = nullptr;
    QLineEdit *path = nullptr;
    QPlainTextEdit *parameters = nullptr;
    QLineEdit *example = nullptr;
    QCheckBox *match_subdomains = nullptr;
    QCheckBox *keep_subdomains = nullptr;
    QCheckBox *multiple = nullptr;
    QCheckBox *associate = nullptr;
    QLineEdit *test = nullptr;
    QLabel *result = nullptr;
};

} // namespace

namespace {

QJsonObject service_type_entry(const QJsonObject &data, int value) {
    for (const QJsonValue &entry : data.value(QStringLiteral("types")).toArray()) {
        const QJsonObject object = entry.toObject();

        if (object.value(QStringLiteral("value")).toInt() == value) {
            return object;
        }
    }

    return {};
}

QString service_family(const QJsonObject &data, int value) {
    return service_type_entry(data, value).value(QStringLiteral("family")).toString();
}

void build_service_editor(HydrusMainWindow *main, QWidget *parent, const QJsonObject &data, const QJsonObject &service) {
    const QString original = service.value(QStringLiteral("original")).toString();
    const QJsonObject dictionary = service.value(QStringLiteral("dictionary")).toObject();

    QDialog dialog(parent);
    dialog.setWindowTitle(original.isEmpty() ? QStringLiteral("add service")
                                             : QStringLiteral("edit service"));
    dialog.resize(620, 520);

    auto *layout = new QVBoxLayout(&dialog);
    auto *form = new QFormLayout();

    auto *name = new QLineEdit(service.value(QStringLiteral("name")).toString(), &dialog);
    auto *type = new QComboBox(&dialog);

    for (const QJsonValue &entry : data.value(QStringLiteral("types")).toArray()) {
        const QJsonObject object = entry.toObject();
        type->addItem(object.value(QStringLiteral("label")).toString(),
                      object.value(QStringLiteral("value")).toInt());
    }

    const int type_index = type->findData(service.value(QStringLiteral("type_int")).toInt());
    type->setCurrentIndex(type_index >= 0 ? type_index : 0);

    auto *host = new QLineEdit(dictionary.value(QStringLiteral("host")).toString(), &dialog);
    host->setPlaceholderText(QStringLiteral("127.0.0.1"));
    auto *port = new QSpinBox(&dialog);
    port->setRange(1, 65535);
    port->setValue(dictionary.value(QStringLiteral("port")).toInt(45871));
    auto *access_key = new QLineEdit(dictionary.value(QStringLiteral("access_key")).toString(), &dialog);
    access_key->setPlaceholderText(QStringLiteral("hex access key from the server administrator"));

    auto *update_period = new QSpinBox(&dialog);
    update_period->setRange(0, 365 * 86400);
    update_period->setValue(dictionary.value(QStringLiteral("update_period")).toInt(86400));
    update_period->setSuffix(QStringLiteral(" s"));
    auto *nullification_period = new QSpinBox(&dialog);
    nullification_period->setRange(0, 365 * 86400);
    nullification_period->setValue(dictionary.value(QStringLiteral("nullification_period")).toInt(90 * 86400));
    nullification_period->setSuffix(QStringLiteral(" s"));

    auto *max_storage = new QSpinBox(&dialog);
    max_storage->setRange(0, 1000000000);
    max_storage->setValue(dictionary.value(QStringLiteral("max_storage")).toInt());
    max_storage->setSuffix(QStringLiteral(" MB"));
    auto *log_uploader_ips = new QCheckBox(QStringLiteral("log uploader IPs"), &dialog);
    log_uploader_ips->setChecked(dictionary.value(QStringLiteral("log_uploader_ips")).toBool(false));

    form->addRow(QStringLiteral("name"), name);
    form->addRow(QStringLiteral("type"), type);
    form->addRow(QStringLiteral("host"), host);
    form->addRow(QStringLiteral("port"), port);
    form->addRow(QStringLiteral("access key"), access_key);
    form->addRow(QStringLiteral("update period"), update_period);
    form->addRow(QStringLiteral("nullification period"), nullification_period);
    form->addRow(QStringLiteral("max storage"), max_storage);
    form->addRow(log_uploader_ips);
    layout->addLayout(form);

    auto *note = new QLabel(
        QStringLiteral("Local services need only a name. Repository and administration "
                       "services also store the host, port and access key the client uses to "
                       "contact them."),
        &dialog);
    note->setWordWrap(true);
    layout->addWidget(note);

    auto sync_fields = [&]() {
        const int value = type->currentData().toInt();
        const QString family = service_family(data, value);
        const bool remote = family == QStringLiteral("remote");
        const bool repository = service_type_entry(data, value)
                                    .value(QStringLiteral("label"))
                                    .toString()
                                    .contains(QStringLiteral("repository"));

        host->setEnabled(remote);
        port->setEnabled(remote);
        access_key->setEnabled(remote);
        update_period->setEnabled(repository);
        nullification_period->setEnabled(repository);
        max_storage->setEnabled(value == 1);
        log_uploader_ips->setEnabled(value == 1);
    };

    QObject::connect(type, &QComboBox::currentIndexChanged, &dialog, [&](int) { sync_fields(); });
    sync_fields();

    auto *buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, &dialog);
    layout->addWidget(buttons);
    QObject::connect(buttons, &QDialogButtonBox::rejected, &dialog, &QDialog::reject);
    QObject::connect(buttons, &QDialogButtonBox::accepted, &dialog, [&]() {
        if (name->text().trimmed().isEmpty()) {
            QMessageBox::warning(&dialog, QStringLiteral("service"), QStringLiteral("Enter a name."));
            return;
        }

        const QString family = service_family(data, type->currentData().toInt());

        if (family == QStringLiteral("remote") && access_key->text().trimmed().isEmpty()) {
            QMessageBox::warning(&dialog, QStringLiteral("service"),
                                 QStringLiteral("Remote services need an access key."));
            return;
        }

        if (family == QStringLiteral("remote") && host->text().trimmed().isEmpty()) {
            QMessageBox::warning(&dialog, QStringLiteral("service"),
                                 QStringLiteral("Remote services need a host."));
            return;
        }

        dialog.accept();
    });

    if (dialog.exec() != QDialog::Accepted) {
        return;
    }

    QJsonObject payload;
    payload.insert(QStringLiteral("original"), original);
    payload.insert(QStringLiteral("name"), name->text().trimmed());
    payload.insert(QStringLiteral("type_int"), type->currentData().toInt());

    QJsonObject next_dictionary;
    const QString family = service_family(data, type->currentData().toInt());
    const bool repository = type->currentText().contains(QStringLiteral("repository"));

    if (family == QStringLiteral("remote")) {
        next_dictionary.insert(QStringLiteral("host"), host->text().trimmed());
        next_dictionary.insert(QStringLiteral("port"), port->value());
        next_dictionary.insert(QStringLiteral("access_key"),
                               access_key->text().trimmed().toLower());
    }

    if (repository) {
        next_dictionary.insert(QStringLiteral("update_period"), update_period->value());
        next_dictionary.insert(QStringLiteral("nullification_period"), nullification_period->value());
    }

    if (type->currentData().toInt() == 1) {
        next_dictionary.insert(QStringLiteral("max_storage"), max_storage->value());
        next_dictionary.insert(QStringLiteral("log_uploader_ips"), log_uploader_ips->isChecked());
    }

    payload.insert(QStringLiteral("dictionary"), next_dictionary);
    main->notify(QStringLiteral("service_save"), QString(), json_payload(payload));
}

QString cell_value(QTableWidget *table, int row, int column) {
    QTableWidgetItem *item = table->item(row, column);

    return item == nullptr ? QString() : item->text();
}

QString cell_key(QTableWidget *table, int row) {
    QTableWidgetItem *item = table->item(row, 0);

    return item == nullptr ? QString() : item->data(Qt::UserRole + 1).toString();
}

} // namespace

HydrusServicesDialog::HydrusServicesDialog(HydrusMainWindow *main_window, const QJsonObject &state, QWidget *parent)
    : QDialog(parent), main(main_window) {
    setWindowTitle(QStringLiteral("manage services"));
    setObjectName(QStringLiteral("hydrus_services_dialog"));
    setAttribute(Qt::WA_DeleteOnClose, true);
    resize(980, 720);

    auto *layout = new QVBoxLayout(this);
    tabs = new QTabWidget(this);
    layout->addWidget(tabs, 1);

    // Local service manager -------------------------------------------------
    auto *local = new QWidget(tabs);
    auto *local_layout = new QVBoxLayout(local);

    table = new QTableWidget(local);
    table->setColumnCount(5);
    table->setHorizontalHeaderLabels({
        QStringLiteral("name"),
        QStringLiteral("type"),
        QStringLiteral("service key"),
        QStringLiteral("port"),
        QStringLiteral("settings")
    });
    table->horizontalHeader()->setSectionResizeMode(0, QHeaderView::Stretch);
    table->horizontalHeader()->setSectionResizeMode(1, QHeaderView::ResizeToContents);
    table->horizontalHeader()->setSectionResizeMode(2, QHeaderView::Stretch);
    table->horizontalHeader()->setSectionResizeMode(3, QHeaderView::ResizeToContents);
    table->horizontalHeader()->setSectionResizeMode(4, QHeaderView::Stretch);
    table->setSelectionBehavior(QAbstractItemView::SelectRows);
    table->setSelectionMode(QAbstractItemView::SingleSelection);
    table->setEditTriggers(QAbstractItemView::NoEditTriggers);
    table->verticalHeader()->setVisible(false);
    local_layout->addWidget(table, 1);

    auto *buttons = new QHBoxLayout();
    auto *add = new QPushButton(QStringLiteral("add service..."), local);
    auto *edit = new QPushButton(QStringLiteral("edit..."), local);
    auto *remove = new QPushButton(QStringLiteral("delete"), local);
    auto *specifier = new QPushButton(QStringLiteral("service specifier..."), local);
    auto *tag_filter = new QPushButton(QStringLiteral("tag filter / purge..."), local);
    buttons->addWidget(add);
    buttons->addWidget(edit);
    buttons->addWidget(remove);
    buttons->addWidget(specifier);
    buttons->addWidget(tag_filter);
    buttons->addStretch(1);
    local_layout->addLayout(buttons);

    connect(add, &QPushButton::clicked, this, [this]() { add_service(); });
    connect(edit, &QPushButton::clicked, this, [this]() { edit_selected(); });
    connect(remove, &QPushButton::clicked, this, [this]() { delete_selected(); });
    connect(specifier, &QPushButton::clicked, this, [this]() { show_specifier(); });
    connect(tag_filter, &QPushButton::clicked, this, [this]() {
        const QString key = selected_key();

        if (!key.isEmpty()) {
            main->notify(QStringLiteral("tag_filter_open"), QString(), key);
        }
    });
    connect(table, &QTableWidget::cellDoubleClicked, this, [this](int, int) { edit_selected(); });

    tabs->addTab(local, QStringLiteral("services"));

    // Remote server ---------------------------------------------------------
    auto *server = new QWidget(tabs);
    auto *server_layout = new QVBoxLayout(server);

    auto *connection = new QHBoxLayout();
    admin_address = new QLineEdit(server);
    admin_address->setPlaceholderText(QStringLiteral("https://127.0.0.1:45870"));
    admin_key = new QLineEdit(server);
    admin_key->setPlaceholderText(QStringLiteral("administration access key"));
    auto *connect_button = new QPushButton(QStringLiteral("connect"), server);
    connection->addWidget(new QLabel(QStringLiteral("address"), server));
    connection->addWidget(admin_address, 2);
    connection->addWidget(new QLabel(QStringLiteral("key"), server));
    connection->addWidget(admin_key, 2);
    connection->addWidget(connect_button);
    server_layout->addLayout(connection);

    admin_status = new QLabel(server);
    admin_status->setWordWrap(true);
    server_layout->addWidget(admin_status);

    server_layout->addWidget(new QLabel(QStringLiteral("server services"), server));
    remote_services = new QTableWidget(server);
    remote_services->setColumnCount(5);
    remote_services->setHorizontalHeaderLabels({
        QStringLiteral("name"),
        QStringLiteral("type"),
        QStringLiteral("port"),
        QStringLiteral("service key"),
        QStringLiteral("access key")
    });
    remote_services->horizontalHeader()->setSectionResizeMode(0, QHeaderView::Stretch);
    remote_services->horizontalHeader()->setSectionResizeMode(1, QHeaderView::ResizeToContents);
    remote_services->horizontalHeader()->setSectionResizeMode(2, QHeaderView::ResizeToContents);
    remote_services->horizontalHeader()->setSectionResizeMode(3, QHeaderView::Stretch);
    remote_services->horizontalHeader()->setSectionResizeMode(4, QHeaderView::Stretch);
    remote_services->setSelectionBehavior(QAbstractItemView::SelectRows);
    remote_services->setSelectionMode(QAbstractItemView::SingleSelection);
    remote_services->setEditTriggers(QAbstractItemView::NoEditTriggers);
    remote_services->verticalHeader()->setVisible(false);
    server_layout->addWidget(remote_services, 1);

    auto *remote_buttons = new QHBoxLayout();
    remote_key = new QLineEdit(server);
    remote_key->setPlaceholderText(QStringLiteral("selected service access key"));
    auto *use = new QPushButton(QStringLiteral("review accounts"), server);
    auto *add_remote = new QPushButton(QStringLiteral("add service..."), server);
    auto *edit_remote = new QPushButton(QStringLiteral("edit service..."), server);
    auto *remove_remote = new QPushButton(QStringLiteral("delete service"), server);
    auto *lock = new QPushButton(QStringLiteral("lock"), server);
    auto *unlock = new QPushButton(QStringLiteral("unlock"), server);
    auto *regen = new QPushButton(QStringLiteral("regen info"), server);
    auto *info = new QPushButton(QStringLiteral("service info..."), server);
    auto *remote_filter = new QPushButton(QStringLiteral("tag filter..."), server);
    auto *account_types = new QPushButton(QStringLiteral("account types..."), server);
    auto *update_period = new QPushButton(QStringLiteral("update period..."), server);
    auto *nullification_period = new QPushButton(QStringLiteral("nullification..."), server);
    remote_buttons->addWidget(remote_key, 1);
    remote_buttons->addWidget(use);
    remote_buttons->addWidget(add_remote);
    remote_buttons->addWidget(edit_remote);
    remote_buttons->addWidget(remove_remote);
    server_layout->addLayout(remote_buttons);

    auto *period_buttons = new QHBoxLayout();
    period_buttons->addWidget(lock);
    period_buttons->addWidget(unlock);
    period_buttons->addWidget(regen);
    period_buttons->addWidget(info);
    period_buttons->addWidget(remote_filter);
    period_buttons->addWidget(account_types);
    period_buttons->addWidget(update_period);
    period_buttons->addWidget(nullification_period);
    period_buttons->addStretch(1);
    server_layout->addLayout(period_buttons);

    server_layout->addWidget(new QLabel(QStringLiteral("accounts on the selected service"), server));
    accounts = new QTableWidget(server);
    accounts->setColumnCount(6);
    accounts->setHorizontalHeaderLabels({
        QStringLiteral("account key"),
        QStringLiteral("type"),
        QStringLiteral("created"),
        QStringLiteral("expires"),
        QStringLiteral("banned"),
        QStringLiteral("message")
    });
    accounts->horizontalHeader()->setSectionResizeMode(0, QHeaderView::Stretch);
    accounts->horizontalHeader()->setSectionResizeMode(1, QHeaderView::ResizeToContents);
    accounts->horizontalHeader()->setSectionResizeMode(2, QHeaderView::ResizeToContents);
    accounts->horizontalHeader()->setSectionResizeMode(3, QHeaderView::ResizeToContents);
    accounts->horizontalHeader()->setSectionResizeMode(4, QHeaderView::ResizeToContents);
    accounts->horizontalHeader()->setSectionResizeMode(5, QHeaderView::Stretch);
    accounts->setSelectionBehavior(QAbstractItemView::SelectRows);
    accounts->setSelectionMode(QAbstractItemView::SingleSelection);
    accounts->setEditTriggers(QAbstractItemView::NoEditTriggers);
    accounts->verticalHeader()->setVisible(false);
    server_layout->addWidget(accounts, 1);

    auto *account_buttons = new QHBoxLayout();
    auto *ban = new QPushButton(QStringLiteral("ban..."), server);
    auto *unban = new QPushButton(QStringLiteral("unban"), server);
    auto *expires = new QPushButton(QStringLiteral("expires..."), server);
    auto *message = new QPushButton(QStringLiteral("message..."), server);
    auto *delete_content = new QPushButton(QStringLiteral("delete content"), server);
    auto *account_type = new QPushButton(QStringLiteral("account type..."), server);
    account_buttons->addWidget(ban);
    account_buttons->addWidget(unban);
    account_buttons->addWidget(expires);
    account_buttons->addWidget(message);
    account_buttons->addWidget(delete_content);
    account_buttons->addWidget(account_type);
    account_buttons->addStretch(1);
    server_layout->addLayout(account_buttons);

    connect(connect_button, &QPushButton::clicked, this, [this]() { connect_admin(); });
    connect(use, &QPushButton::clicked, this, [this]() { select_remote_service(); });
    connect(add_remote, &QPushButton::clicked, this, [this]() { add_remote_service(); });
    connect(edit_remote, &QPushButton::clicked, this, [this]() { edit_remote_service(); });
    connect(remove_remote, &QPushButton::clicked, this, [this]() { remove_remote_service(); });
    connect(lock, &QPushButton::clicked, this, [this]() { remote_service_action(); });
    connect(unlock, &QPushButton::clicked, this, [this]() { remote_service_action(); });
    connect(regen, &QPushButton::clicked, this, [this]() { remote_service_action(); });
    connect(update_period, &QPushButton::clicked, this, [this]() { remote_service_action(); });
    connect(nullification_period, &QPushButton::clicked, this, [this]() { remote_service_action(); });
    connect(info, &QPushButton::clicked, this, [this]() {
        const QJsonObject admin = data.value(QStringLiteral("admin")).toObject();
        QDialog dialog(this);
        dialog.setWindowTitle(QStringLiteral("service review"));
        dialog.resize(720, 520);
        auto *layout = new QVBoxLayout(&dialog);
        auto *text = new QTextEdit(&dialog);
        text->setReadOnly(true);

        QJsonObject review;
        review.insert(QStringLiteral("options"), admin.value(QStringLiteral("options")));
        review.insert(QStringLiteral("service_info"), admin.value(QStringLiteral("service_info")));
        text->setPlainText(QString::fromUtf8(
            QJsonDocument(review).toJson(QJsonDocument::Indented)));

        layout->addWidget(text, 1);
        auto *buttons = new QDialogButtonBox(QDialogButtonBox::Close, &dialog);
        layout->addWidget(buttons);
        connect(buttons, &QDialogButtonBox::rejected, &dialog, &QDialog::reject);
        dialog.exec();
    });
    connect(remote_filter, &QPushButton::clicked, this, [this]() {
        const QString key = selected_remote_key();

        if (!key.isEmpty()) {
            main->notify(QStringLiteral("tag_filter_open"), QString(), key);
        }
    });
    connect(account_types, &QPushButton::clicked, this, [this]() {
        if (!selected_remote_key().isEmpty()) {
            main->notify(QStringLiteral("account_types_open"), QString(), selected_remote_key());
        }
    });
    connect(ban, &QPushButton::clicked, this, [this]() { account_action(); });
    connect(unban, &QPushButton::clicked, this, [this]() { account_action(); });
    connect(expires, &QPushButton::clicked, this, [this]() { account_action(); });
    connect(message, &QPushButton::clicked, this, [this]() { account_action(); });
    connect(delete_content, &QPushButton::clicked, this, [this]() { account_action(); });
    connect(account_type, &QPushButton::clicked, this, [this]() { account_action(); });
    connect(remote_services, &QTableWidget::itemSelectionChanged, this, [this]() {
        const int row = remote_services->currentRow();

        if (row >= 0) {
            remote_key->setText(cell_value(remote_services, row, 4));
        }
    });

    tabs->addTab(server, QStringLiteral("server"));

    // Credentials -----------------------------------------------------------
    auto *credentials = new QWidget(tabs);
    auto *credentials_layout = new QVBoxLayout(credentials);
    logins = new QTableWidget(credentials);
    logins->setColumnCount(5);
    logins->setHorizontalHeaderLabels({
        QStringLiteral("domain"),
        QStringLiteral("username"),
        QStringLiteral("password"),
        QStringLiteral("access key"),
        QStringLiteral("expires")
    });
    logins->horizontalHeader()->setSectionResizeMode(0, QHeaderView::Stretch);
    logins->horizontalHeader()->setSectionResizeMode(1, QHeaderView::ResizeToContents);
    logins->horizontalHeader()->setSectionResizeMode(2, QHeaderView::ResizeToContents);
    logins->horizontalHeader()->setSectionResizeMode(3, QHeaderView::Stretch);
    logins->horizontalHeader()->setSectionResizeMode(4, QHeaderView::ResizeToContents);
    logins->setSelectionBehavior(QAbstractItemView::SelectRows);
    logins->setSelectionMode(QAbstractItemView::SingleSelection);
    logins->setEditTriggers(QAbstractItemView::NoEditTriggers);
    logins->verticalHeader()->setVisible(false);
    credentials_layout->addWidget(logins, 1);

    auto *login_buttons = new QHBoxLayout();
    auto *add_login = new QPushButton(QStringLiteral("add credentials..."), credentials);
    auto *edit_login_button = new QPushButton(QStringLiteral("edit..."), credentials);
    auto *remove_login_button = new QPushButton(QStringLiteral("remove"), credentials);
    login_buttons->addWidget(add_login);
    login_buttons->addWidget(edit_login_button);
    login_buttons->addWidget(remove_login_button);
    login_buttons->addStretch(1);
    credentials_layout->addLayout(login_buttons);
    connect(add_login, &QPushButton::clicked, this, [this]() { this->edit_login(); });
    connect(edit_login_button, &QPushButton::clicked, this, [this]() { this->edit_login(); });
    connect(remove_login_button, &QPushButton::clicked, this, [this]() { this->remove_login(); });
    connect(logins, &QTableWidget::cellDoubleClicked, this, [this](int, int) { this->edit_login(); });

    tabs->addTab(credentials, QStringLiteral("logins"));

    auto *close = new QPushButton(QStringLiteral("close"), this);
    auto *footer = new QHBoxLayout();
    footer->addStretch(1);
    footer->addWidget(close);
    layout->addLayout(footer);
    connect(close, &QPushButton::clicked, this, &QDialog::close);

    set_state(state);

    const QString requested_tab = qEnvironmentVariable("HYDRUS_QT_SERVICES_TAB");

    if (!requested_tab.isEmpty()) {
        for (int index = 0; index < tabs->count(); ++index) {
            if (tabs->tabText(index) == requested_tab) {
                tabs->setCurrentIndex(index);
                break;
            }
        }
    }

    const QString edit_key = qEnvironmentVariable("HYDRUS_QT_SERVICES_EDIT");

    if (edit_key == QStringLiteral("new")) {
        add_service();
    } else if (!edit_key.isEmpty()) {
        for (int row = 0; row < table->rowCount(); ++row) {
            if (cell_key(table, row).compare(edit_key, Qt::CaseInsensitive) == 0) {
                table->selectRow(row);
                edit_selected();
                break;
            }
        }
    }

    if (qEnvironmentVariableIsSet("HYDRUS_QT_SERVICES_SPECIFIER")) {
        show_specifier();
    }
}

void HydrusServicesDialog::set_state(const QJsonObject &state) {
    data = state;

    const QJsonArray services = state.value(QStringLiteral("services")).toArray();
    table->setRowCount(services.size());

    for (int index = 0; index < services.size(); ++index) {
        const QJsonObject service = services.at(index).toObject();

        for (int column = 0; column < 5; ++column) {
            if (table->item(index, column) == nullptr) {
                table->setItem(index, column, new QTableWidgetItem());
            }
        }

        const QJsonObject dictionary = service.value(QStringLiteral("dictionary")).toObject();

        auto *name_item = table->item(index, 0);
        name_item->setText(service.value(QStringLiteral("name")).toString());
        name_item->setData(Qt::UserRole, service);
        name_item->setData(Qt::UserRole + 1, service.value(QStringLiteral("key")).toString());

        if (service.value(QStringLiteral("builtin")).toBool()) {
            name_item->setForeground(QColor(120, 150, 190));
        } else {
            name_item->setForeground(QBrush());
        }

        table->item(index, 1)->setText(service.value(QStringLiteral("type_short")).toString());
        table->item(index, 2)->setText(service.value(QStringLiteral("key")).toString());
        table->item(index, 3)->setText(dictionary.contains(QStringLiteral("port"))
                                            ? QString::number(dictionary.value(QStringLiteral("port")).toInt())
                                            : QStringLiteral("-"));

        QString summary = QStringLiteral("-");

        if (!dictionary.isEmpty()) {
            summary = QStringLiteral("%1:%2")
                          .arg(dictionary.value(QStringLiteral("host")).toString())
                          .arg(dictionary.value(QStringLiteral("port")).toInt());

            if (dictionary.contains(QStringLiteral("update_period"))) {
                summary += QStringLiteral(" | update every %1 s")
                               .arg(dictionary.value(QStringLiteral("update_period")).toInt());
            }

            if (dictionary.contains(QStringLiteral("nullification_period"))) {
                summary += QStringLiteral(" | nullify every %1 s")
                               .arg(dictionary.value(QStringLiteral("nullification_period")).toInt());
            }
        }

        table->item(index, 4)->setText(summary);
    }

    table->resizeRowsToContents();

    const QJsonObject admin = state.value(QStringLiteral("admin")).toObject();
    admin_address->setText(admin.value(QStringLiteral("address")).toString());
    admin_key->setText(admin.value(QStringLiteral("access_key")).toString());
    admin_status->setText(admin.value(QStringLiteral("status")).toString());

    const QJsonArray remote = admin.value(QStringLiteral("services")).toArray();
    remote_services->setRowCount(remote.size());

    for (int index = 0; index < remote.size(); ++index) {
        const QJsonObject service = remote.at(index).toObject();

        for (int column = 0; column < 5; ++column) {
            if (remote_services->item(index, column) == nullptr) {
                remote_services->setItem(index, column, new QTableWidgetItem());
            }
        }

        auto *name_item = remote_services->item(index, 0);
        name_item->setText(service.value(QStringLiteral("name")).toString());
        name_item->setData(Qt::UserRole, service);
        remote_services->item(index, 1)->setText(service.value(QStringLiteral("type")).toString());
        remote_services->item(index, 2)->setText(QString::number(service.value(QStringLiteral("port")).toInt()));
        remote_services->item(index, 3)->setText(service.value(QStringLiteral("key")).toString());
        remote_services->item(index, 4)->setText(service.value(QStringLiteral("access_key")).toString());

        if (service.value(QStringLiteral("selected")).toBool()) {
            remote_services->selectRow(index);
        }
    }

    remote_services->resizeRowsToContents();

    if (!opened_account_types
        && qEnvironmentVariableIsSet("HYDRUS_QT_SERVICES_ACCOUNT_TYPES")
        && remote_services->rowCount() > 0) {
        opened_account_types = true;
        const QJsonObject first = remote_services->item(0, 0)->data(Qt::UserRole).toJsonObject();
        const QString key = first.value(QStringLiteral("key")).toString();
        remote_services->selectRow(0);

        QJsonObject select;
        select.insert(QStringLiteral("action"), QStringLiteral("select"));
        select.insert(QStringLiteral("service_key"), key);
        select.insert(QStringLiteral("access_key"),
                      first.value(QStringLiteral("access_key")).toString().isEmpty()
                          ? admin_key->text().trimmed()
                          : first.value(QStringLiteral("access_key")).toString());
        main->notify(QStringLiteral("admin_service_action"), QString(), json_payload(select));
        main->notify(QStringLiteral("account_types_open"), QString(), key);
    }

    const QJsonArray account_list = admin.value(QStringLiteral("accounts")).toArray();
    accounts->setRowCount(account_list.size());

    for (int index = 0; index < account_list.size(); ++index) {
        const QJsonObject account = account_list.at(index).toObject();

        for (int column = 0; column < 6; ++column) {
            if (accounts->item(index, column) == nullptr) {
                accounts->setItem(index, column, new QTableWidgetItem());
            }
        }

        auto *key_item = accounts->item(index, 0);
        key_item->setText(account.value(QStringLiteral("key")).toString());
        key_item->setData(Qt::UserRole, account);

        const QJsonValue expires = account.value(QStringLiteral("expires"));
        accounts->item(index, 1)->setText(account.value(QStringLiteral("type")).toString());
        accounts->item(index, 2)->setText(QString::number(account.value(QStringLiteral("created")).toInt()));
        accounts->item(index, 3)->setText(expires.isNull() ? QStringLiteral("never")
                                                           : QString::number(expires.toInt()));
        accounts->item(index, 4)->setText(account.value(QStringLiteral("banned")).toBool()
                                              ? account.value(QStringLiteral("banned_reason")).toString()
                                              : QStringLiteral("-"));
        accounts->item(index, 5)->setText(account.value(QStringLiteral("message")).toString());
    }

    accounts->resizeRowsToContents();

    const QJsonArray login_list = state.value(QStringLiteral("logins")).toArray();
    logins->setRowCount(login_list.size());

    for (int index = 0; index < login_list.size(); ++index) {
        const QJsonObject login = login_list.at(index).toObject();

        for (int column = 0; column < 5; ++column) {
            if (logins->item(index, column) == nullptr) {
                logins->setItem(index, column, new QTableWidgetItem());
            }
        }

        auto *domain_item = logins->item(index, 0);
        domain_item->setText(login.value(QStringLiteral("domain")).toString());
        domain_item->setData(Qt::UserRole, login);
        logins->item(index, 1)->setText(login.value(QStringLiteral("username")).toString());
        logins->item(index, 2)->setText(login.value(QStringLiteral("password")).toString().isEmpty()
                                            ? QStringLiteral("-")
                                            : QStringLiteral("********"));
        logins->item(index, 3)->setText(login.value(QStringLiteral("access_key")).toString());
        const QJsonValue expires = login.value(QStringLiteral("expires"));
        logins->item(index, 4)->setText(expires.isNull() ? QStringLiteral("never")
                                                         : QString::number(expires.toInt()));
    }

    logins->resizeRowsToContents();
}

QString HydrusServicesDialog::selected_key() const {
    const int row = table->currentRow();

    if (row < 0) {
        return {};
    }

    return cell_key(table, row);
}

QJsonObject HydrusServicesDialog::selected_remote_service() const {
    const int row = remote_services->currentRow();

    if (row < 0 || remote_services->item(row, 0) == nullptr) {
        return {};
    }

    return remote_services->item(row, 0)->data(Qt::UserRole).toJsonObject();
}

QString HydrusServicesDialog::selected_remote_key() const {
    const QJsonObject service = selected_remote_service();

    return service.value(QStringLiteral("key")).toString();
}

void HydrusServicesDialog::add_service() {
    QJsonObject service;
    service.insert(QStringLiteral("name"), QString());
    service.insert(QStringLiteral("type_int"), 2);
    service.insert(QStringLiteral("dictionary"), QJsonObject());

    build_service_editor(main, this, data, service);
}

void HydrusServicesDialog::edit_selected() {
    const int row = table->currentRow();

    if (row < 0 || table->item(row, 0) == nullptr) {
        return;
    }

    QJsonObject service = table->item(row, 0)->data(Qt::UserRole).toJsonObject();
    service.insert(QStringLiteral("original"), selected_key());
    build_service_editor(main, this, data, service);
}

void HydrusServicesDialog::delete_selected() {
    const QString key = selected_key();

    if (key.isEmpty()) {
        return;
    }

    if (QMessageBox::question(
            this,
            QStringLiteral("delete service"),
            QStringLiteral("Delete this service? Only empty custom services can be removed."))
        != QMessageBox::Yes) {
        return;
    }

    QJsonObject payload;
    payload.insert(QStringLiteral("key"), key);

    main->notify(QStringLiteral("service_delete"), QString(), json_payload(payload));
}

void HydrusServicesDialog::show_specifier() {
    QDialog dialog(this);
    dialog.setWindowTitle(QStringLiteral("service specifier"));
    dialog.resize(460, 360);

    auto *layout = new QVBoxLayout(&dialog);
    auto *type = new QComboBox(&dialog);
    type->addItem(QStringLiteral("file domains"), QStringLiteral("local file domain"));
    type->addItem(QStringLiteral("tag domains"), QStringLiteral("local tag domain"));
    type->addItem(QStringLiteral("notes"), QStringLiteral("notes"));
    type->addItem(QStringLiteral("ratings"), QStringLiteral("ratings"));
    type->addItem(QStringLiteral("tag repositories"), QStringLiteral("tag repository"));
    type->addItem(QStringLiteral("file repositories"), QStringLiteral("file repository"));
    layout->addWidget(type);

    auto *list = new QListWidget(&dialog);
    layout->addWidget(list, 1);

    auto *status = new QLabel(&dialog);
    status->setWordWrap(true);
    layout->addWidget(status);

    auto refresh = [this, list, type, status]() {
        list->clear();
        const QString wanted = type->currentData().toString();

        for (const QJsonValue &entry : data.value(QStringLiteral("services")).toArray()) {
            const QJsonObject service = entry.toObject();
            const QString short_type = service.value(QStringLiteral("type_short")).toString();

            const bool matches = wanted == QStringLiteral("ratings")
                ? short_type.contains(QStringLiteral("rating"))
                : short_type == wanted;

            if (matches) {
                auto *item = new QListWidgetItem(service.value(QStringLiteral("name")).toString(), list);
                item->setData(Qt::UserRole, service.value(QStringLiteral("key")).toString());
            }
        }

        status->setText(list->count() == 0 ? QStringLiteral("no services of that type")
                                           : QStringLiteral("%1 service(s) available").arg(list->count()));
    };

    connect(type, &QComboBox::currentIndexChanged, &dialog, [refresh](int) { refresh(); });
    refresh();

    auto *buttons = new QDialogButtonBox(QDialogButtonBox::Close, &dialog);
    auto *copy = new QPushButton(QStringLiteral("copy key"), &dialog);
    buttons->addButton(copy, QDialogButtonBox::ActionRole);
    layout->addWidget(buttons);
    connect(copy, &QPushButton::clicked, &dialog, [list, status]() {
        QListWidgetItem *item = list->currentItem();

        if (item == nullptr) {
            status->setText(QStringLiteral("select a service first"));
            return;
        }

        QApplication::clipboard()->setText(item->data(Qt::UserRole).toString());
        status->setText(QStringLiteral("service key copied to the clipboard"));
    });
    QObject::connect(buttons, &QDialogButtonBox::rejected, &dialog, &QDialog::reject);

    dialog.exec();
}

void HydrusServicesDialog::edit_login() {
    QJsonObject existing;
    const int row = logins->currentRow();

    if (row >= 0 && logins->item(row, 0) != nullptr) {
        existing = logins->item(row, 0)->data(Qt::UserRole).toJsonObject();
    }

    QDialog dialog(this);
    dialog.setWindowTitle(existing.isEmpty() ? QStringLiteral("add credentials")
                                             : QStringLiteral("edit credentials"));
    auto *layout = new QVBoxLayout(&dialog);
    auto *form = new QFormLayout();

    auto *domain = new QLineEdit(existing.value(QStringLiteral("domain")).toString(), &dialog);
    auto *username = new QLineEdit(existing.value(QStringLiteral("username")).toString(), &dialog);
    auto *password = new QLineEdit(existing.value(QStringLiteral("password")).toString(), &dialog);
    password->setEchoMode(QLineEdit::Password);
    auto *access_key = new QLineEdit(existing.value(QStringLiteral("access_key")).toString(), &dialog);
    auto *expires = new QSpinBox(&dialog);
    expires->setRange(0, 3650);
    expires->setSpecialValueText(QStringLiteral("never"));
    expires->setSuffix(QStringLiteral(" days"));

    form->addRow(QStringLiteral("domain"), domain);
    form->addRow(QStringLiteral("username"), username);
    form->addRow(QStringLiteral("password"), password);
    form->addRow(QStringLiteral("access key"), access_key);
    form->addRow(QStringLiteral("expires"), expires);
    layout->addLayout(form);

    auto *buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, &dialog);
    layout->addWidget(buttons);
    connect(buttons, &QDialogButtonBox::accepted, &dialog, [&]() {
        if (domain->text().trimmed().isEmpty()) {
            QMessageBox::warning(&dialog, QStringLiteral("credentials"),
                                 QStringLiteral("Enter a domain."));
            return;
        }

        dialog.accept();
    });
    QObject::connect(buttons, &QDialogButtonBox::rejected, &dialog, &QDialog::reject);

    if (dialog.exec() != QDialog::Accepted) {
        return;
    }

    QJsonObject payload;
    payload.insert(QStringLiteral("domain"), domain->text().trimmed());
    payload.insert(QStringLiteral("username"), username->text());
    payload.insert(QStringLiteral("password"), password->text());
    payload.insert(QStringLiteral("access_key"), access_key->text().trimmed());

    if (expires->value() > 0) {
        payload.insert(QStringLiteral("expires"),
                       QDateTime::currentSecsSinceEpoch() + static_cast<qint64>(expires->value()) * 86400);
    }

    main->notify(QStringLiteral("login_save"), QString(), json_payload(payload));
}

void HydrusServicesDialog::remove_login() {
    const int row = logins->currentRow();

    if (row < 0) {
        return;
    }

    const QString domain = cell_value(logins, row, 0);

    if (domain.isEmpty()) {
        return;
    }

    main->notify(QStringLiteral("login_remove"), QString(), domain);
}

void HydrusServicesDialog::connect_admin() {
    QJsonObject payload;
    payload.insert(QStringLiteral("address"), admin_address->text().trimmed());
    payload.insert(QStringLiteral("access_key"), admin_key->text().trimmed());

    main->notify(QStringLiteral("admin_connect"), QString(), json_payload(payload));
}

void HydrusServicesDialog::select_remote_service() {
    const QString key = selected_remote_key();

    if (key.isEmpty()) {
        return;
    }

    QJsonObject payload;
    payload.insert(QStringLiteral("service_key"), key);
    payload.insert(QStringLiteral("access_key"), remote_key->text().trimmed());

    main->notify(QStringLiteral("admin_service_action"), QString(),
                 json_payload([&]() {
                     QJsonObject with_action = payload;
                     with_action.insert(QStringLiteral("action"), QStringLiteral("select"));
                     return with_action;
                 }()));
}

void HydrusServicesDialog::add_remote_service() {
    QDialog dialog(this);
    dialog.setWindowTitle(QStringLiteral("add server service"));
    auto *layout = new QVBoxLayout(&dialog);
    auto *form = new QFormLayout();

    auto *name = new QLineEdit(&dialog);
    auto *type = new QComboBox(&dialog);
    type->addItem(QStringLiteral("tag repository"), 0);
    type->addItem(QStringLiteral("file repository"), 1);
    auto *port = new QSpinBox(&dialog);
    port->setRange(1, 65535);
    port->setValue(45871);

    form->addRow(QStringLiteral("name"), name);
    form->addRow(QStringLiteral("type"), type);
    form->addRow(QStringLiteral("port"), port);
    layout->addLayout(form);

    auto *note = new QLabel(
        QStringLiteral("The server returns a new access key for every service it creates; "
                       "it is stored here and used for account and option management."),
        &dialog);
    note->setWordWrap(true);
    layout->addWidget(note);

    auto *buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, &dialog);
    layout->addWidget(buttons);
    connect(buttons, &QDialogButtonBox::accepted, &dialog, [&]() {
        if (name->text().trimmed().isEmpty()) {
            QMessageBox::warning(&dialog, QStringLiteral("service"),
                                 QStringLiteral("Enter a service name."));
            return;
        }

        dialog.accept();
    });
    QObject::connect(buttons, &QDialogButtonBox::rejected, &dialog, &QDialog::reject);

    if (dialog.exec() != QDialog::Accepted) {
        return;
    }

    QJsonObject payload;
    payload.insert(QStringLiteral("name"), name->text().trimmed());
    payload.insert(QStringLiteral("type_int"), type->currentData().toInt());
    payload.insert(QStringLiteral("port"), port->value());

    main->notify(QStringLiteral("admin_add_service"), QString(), json_payload(payload));
}

void HydrusServicesDialog::edit_remote_service() {
    const QJsonObject service = selected_remote_service();

    if (service.isEmpty()) {
        return;
    }

    QDialog dialog(this);
    dialog.setWindowTitle(QStringLiteral("edit server service"));
    auto *layout = new QVBoxLayout(&dialog);
    auto *form = new QFormLayout();
    auto *name = new QLineEdit(service.value(QStringLiteral("name")).toString(), &dialog);
    auto *port = new QSpinBox(&dialog);
    port->setRange(1, 65535);
    port->setValue(service.value(QStringLiteral("port")).toInt());
    form->addRow(QStringLiteral("name"), name);
    form->addRow(QStringLiteral("port"), port);
    layout->addLayout(form);

    auto *buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, &dialog);
    layout->addWidget(buttons);
    connect(buttons, &QDialogButtonBox::accepted, &dialog, [&]() {
        if (name->text().trimmed().isEmpty()) {
            QMessageBox::warning(&dialog, QStringLiteral("service"),
                                 QStringLiteral("Enter a service name."));
            return;
        }

        dialog.accept();
    });
    connect(buttons, &QDialogButtonBox::rejected, &dialog, &QDialog::reject);

    if (dialog.exec() != QDialog::Accepted) {
        return;
    }

    QJsonObject payload;
    payload.insert(QStringLiteral("service_key"), service.value(QStringLiteral("key")).toString());
    payload.insert(QStringLiteral("name"), name->text().trimmed());
    payload.insert(QStringLiteral("port"), port->value());

    main->notify(QStringLiteral("admin_edit_service"), QString(), json_payload(payload));
}

void HydrusServicesDialog::remove_remote_service() {
    const QString key = selected_remote_key();

    if (key.isEmpty()) {
        return;
    }

    if (QMessageBox::question(this, QStringLiteral("delete service"),
                              QStringLiteral("Delete this service from the server? All of its "
                                             "content will be removed."))
        != QMessageBox::Yes) {
        return;
    }

    main->notify(QStringLiteral("admin_remove_service"), QString(), key);
}

void HydrusServicesDialog::remote_service_action() {
    const QString key = selected_remote_key();

    if (key.isEmpty()) {
        return;
    }

    auto *button = qobject_cast<QPushButton *>(sender());

    if (button == nullptr) {
        return;
    }

    QString action;
    const QString text = button->text();

    if (text.startsWith(QStringLiteral("lock"))) {
        action = QStringLiteral("lock_on");
    } else if (text.startsWith(QStringLiteral("unlock"))) {
        action = QStringLiteral("lock_off");
    } else if (text.startsWith(QStringLiteral("regen"))) {
        action = QStringLiteral("regen_info");
    } else if (text.startsWith(QStringLiteral("update"))) {
        action = QStringLiteral("update_period");
    } else if (text.startsWith(QStringLiteral("nullification"))) {
        action = QStringLiteral("nullification_period");
    }

    if (action.isEmpty()) {
        return;
    }

    QJsonObject payload;
    payload.insert(QStringLiteral("action"), action);
    payload.insert(QStringLiteral("service_key"), key);
    payload.insert(QStringLiteral("access_key"), remote_key->text().trimmed());

    if (action.endsWith(QStringLiteral("period"))) {
        bool ok = false;
        const int seconds = QInputDialog::getInt(
            this, QStringLiteral("service period"), QStringLiteral("seconds"),
            action.startsWith(QStringLiteral("update")) ? 86400 : 90 * 86400, 0,
            365 * 86400, 60, &ok);

        if (!ok) {
            return;
        }

        payload.insert(QStringLiteral("period"), seconds);
    }

    main->notify(QStringLiteral("admin_service_action"), QString(), json_payload(payload));
}

void HydrusServicesDialog::account_action() {
    const int row = accounts->currentRow();

    if (row < 0 || accounts->item(row, 0) == nullptr) {
        return;
    }

    auto *button = qobject_cast<QPushButton *>(sender());

    if (button == nullptr) {
        return;
    }

    const QJsonObject account = accounts->item(row, 0)->data(Qt::UserRole).toJsonObject();
    const QString account_key = account.value(QStringLiteral("key")).toString();
    const QString text = button->text();
    QString action;

    if (text.startsWith(QStringLiteral("ban"))) {
        action = QStringLiteral("ban");
    } else if (text.startsWith(QStringLiteral("unban"))) {
        action = QStringLiteral("unban");
    } else if (text.startsWith(QStringLiteral("expires"))) {
        action = QStringLiteral("expires");
    } else if (text.startsWith(QStringLiteral("message"))) {
        action = QStringLiteral("message");
    } else if (text.startsWith(QStringLiteral("delete"))) {
        if (QMessageBox::question(this, QStringLiteral("delete content"),
                                  QStringLiteral("Delete all content this account uploaded?"))
            != QMessageBox::Yes) {
            return;
        }

        action = QStringLiteral("delete_all_content");
    } else if (text.startsWith(QStringLiteral("account type"))) {
        action = QStringLiteral("account_type");
    }

    if (action.isEmpty()) {
        return;
    }

    QJsonObject payload;
    payload.insert(QStringLiteral("action"), action);
    payload.insert(QStringLiteral("subject_account_key"), account_key);

    if (action == QStringLiteral("ban")) {
        bool ok = false;
        const QString reason = QInputDialog::getText(
            this, QStringLiteral("ban account"), QStringLiteral("reason"),
            QLineEdit::Normal, QStringLiteral("banned by admin"), &ok);

        if (!ok) {
            return;
        }

        payload.insert(QStringLiteral("reason"), reason);
    } else if (action == QStringLiteral("expires")) {
        bool ok = false;
        const int days = QInputDialog::getInt(
            this, QStringLiteral("account expiry"),
            QStringLiteral("days from now (0 = never)"), 30, 0, 36500, 1, &ok);

        if (!ok) {
            return;
        }

        if (days > 0) {
            payload.insert(QStringLiteral("expires"),
                           QDateTime::currentSecsSinceEpoch() + static_cast<qint64>(days) * 86400);
        } else {
            payload.insert(QStringLiteral("expires"), QJsonValue::Null);
        }
    } else if (action == QStringLiteral("message")) {
        bool ok = false;
        const QString message = QInputDialog::getText(
            this, QStringLiteral("account message"), QStringLiteral("message"),
            QLineEdit::Normal, account.value(QStringLiteral("message")).toString(), &ok);

        if (!ok) {
            return;
        }

        payload.insert(QStringLiteral("message"), message);
    } else if (action == QStringLiteral("account_type")) {
        const QJsonArray types = data.value(QStringLiteral("admin"))
                                     .toObject()
                                     .value(QStringLiteral("account_types"))
                                     .toArray();
        QStringList titles;

        for (const QJsonValue &entry : types) {
            titles.append(entry.toObject().value(QStringLiteral("title")).toString());
        }

        if (titles.isEmpty()) {
            QMessageBox::information(this, QStringLiteral("account type"),
                                     QStringLiteral("Review the service first."));
            return;
        }

        bool ok = false;
        const QString title = QInputDialog::getItem(
            this, QStringLiteral("account type"), QStringLiteral("type"), titles,
            titles.indexOf(account.value(QStringLiteral("type")).toString()), false, &ok);

        if (!ok) {
            return;
        }

        for (const QJsonValue &entry : types) {
            const QJsonObject object = entry.toObject();

            if (object.value(QStringLiteral("title")).toString() == title) {
                payload.insert(QStringLiteral("account_type_key"),
                               object.value(QStringLiteral("key")).toString());
                break;
            }
        }
    }

    main->notify(QStringLiteral("admin_account_action"), QString(), json_payload(payload));
}

HydrusNetworkDialog::HydrusNetworkDialog(HydrusMainWindow *main_window, const QJsonObject &state, QWidget *parent)
    : QDialog(parent), main(main_window) {
    setWindowTitle(QStringLiteral("manage network"));
    setObjectName(QStringLiteral("hydrus_network_dialog"));
    setAttribute(Qt::WA_DeleteOnClose, true);
    resize(780, 620);

    auto *layout = new QVBoxLayout(this);
    auto *tabs = new QTabWidget(this);
    layout->addWidget(tabs, 1);

    auto *connection = new QWidget(tabs);
    auto *connection_layout = new QVBoxLayout(connection);
    auto *connection_form = new QFormLayout();

    http_proxy = new QLineEdit(connection);
    https_proxy = new QLineEdit(connection);
    no_proxy = new QLineEdit(connection);
    timeout = new QSpinBox(connection);
    timeout->setRange(3, 600);
    max_jobs = new QSpinBox(connection);
    max_jobs->setRange(1, 500);
    max_jobs_domain = new QSpinBox(connection);
    max_jobs_domain->setRange(1, 100);

    connection_form->addRow(QStringLiteral("http proxy"), http_proxy);
    connection_form->addRow(QStringLiteral("https proxy"), https_proxy);
    connection_form->addRow(QStringLiteral("no proxy"), no_proxy);
    connection_form->addRow(QStringLiteral("timeout (seconds)"), timeout);
    connection_form->addRow(QStringLiteral("max network jobs"), max_jobs);
    connection_form->addRow(QStringLiteral("max jobs per domain"), max_jobs_domain);
    connection_layout->addLayout(connection_form);

    paused = new QCheckBox(QStringLiteral("pause all new network traffic"), connection);
    connection_layout->addWidget(paused);
    connection_layout->addStretch(1);
    tabs->addTab(connection, QStringLiteral("connection"));

    auto *api = new QWidget(tabs);
    auto *api_layout = new QVBoxLayout(api);
    auto *api_form = new QFormLayout();
    api_enabled = new QCheckBox(QStringLiteral("run the local Client API"), api);
    api_port = new QSpinBox(api);
    api_port->setRange(1, 65535);
    api_key = new QLineEdit(api);
    api_key->setPlaceholderText(QStringLiteral("64 hex characters; generated when empty"));
    api_status = new QLabel(api);
    api_status->setTextInteractionFlags(Qt::TextSelectableByMouse);

    api_form->addRow(api_enabled);
    api_form->addRow(QStringLiteral("port"), api_port);
    api_form->addRow(QStringLiteral("access key"), api_key);
    api_form->addRow(QStringLiteral("status"), api_status);
    api_layout->addLayout(api_form);
    api_layout->addStretch(1);
    tabs->addTab(api, QStringLiteral("client api"));

    auto *headers_page = new QWidget(tabs);
    auto *headers_layout = new QVBoxLayout(headers_page);
    headers = new QTableWidget(headers_page);
    headers->setColumnCount(2);
    headers->setHorizontalHeaderLabels({QStringLiteral("name"), QStringLiteral("value")});
    headers->horizontalHeader()->setStretchLastSection(true);
    headers->horizontalHeader()->setSectionResizeMode(0, QHeaderView::ResizeToContents);
    headers->setSelectionBehavior(QAbstractItemView::SelectRows);
    headers->setEditTriggers(QAbstractItemView::NoEditTriggers);
    headers->verticalHeader()->setVisible(false);
    headers_layout->addWidget(headers, 1);

    auto *header_buttons = new QHBoxLayout();
    auto *add_header_button = new QPushButton(QStringLiteral("add header..."), headers_page);
    auto *remove_header_button = new QPushButton(QStringLiteral("remove"), headers_page);
    header_buttons->addWidget(add_header_button);
    header_buttons->addWidget(remove_header_button);
    header_buttons->addStretch(1);
    headers_layout->addLayout(header_buttons);
    tabs->addTab(headers_page, QStringLiteral("custom headers"));

    auto *classes_page = new QWidget(tabs);
    auto *classes_layout = new QVBoxLayout(classes_page);
    url_classes = new QTableWidget(classes_page);
    url_classes->setColumnCount(4);
    url_classes->setHorizontalHeaderLabels({QStringLiteral("name"), QStringLiteral("type"), QStringLiteral("domain"), QStringLiteral("path")});
    url_classes->horizontalHeader()->setStretchLastSection(true);
    url_classes->horizontalHeader()->setSectionResizeMode(0, QHeaderView::ResizeToContents);
    url_classes->horizontalHeader()->setSectionResizeMode(1, QHeaderView::ResizeToContents);
    url_classes->setSelectionBehavior(QAbstractItemView::SelectRows);
    url_classes->setEditTriggers(QAbstractItemView::NoEditTriggers);
    url_classes->verticalHeader()->setVisible(false);
    classes_layout->addWidget(url_classes, 1);

    auto *class_buttons = new QHBoxLayout();
    auto *add_class = new QPushButton(QStringLiteral("add url class..."), classes_page);
    auto *edit_class = new QPushButton(QStringLiteral("edit..."), classes_page);
    auto *remove_class = new QPushButton(QStringLiteral("remove"), classes_page);
    class_buttons->addWidget(add_class);
    class_buttons->addWidget(edit_class);
    class_buttons->addWidget(remove_class);
    class_buttons->addStretch(1);
    classes_layout->addLayout(class_buttons);
    tabs->addTab(classes_page, QStringLiteral("url classes"));

    auto *pools_page = new QWidget(tabs);
    auto *pools_layout = new QVBoxLayout(pools_page);
    pools = new QTableWidget(pools_page);
    pools->setColumnCount(2);
    pools->setHorizontalHeaderLabels({QStringLiteral("domain"), QStringLiteral("simultaneous downloads")});
    pools->horizontalHeader()->setStretchLastSection(true);
    pools->setSelectionBehavior(QAbstractItemView::SelectRows);
    pools->setEditTriggers(QAbstractItemView::NoEditTriggers);
    pools->verticalHeader()->setVisible(false);
    pools_layout->addWidget(pools, 1);

    auto *pool_buttons = new QHBoxLayout();
    auto *add_pool_button = new QPushButton(QStringLiteral("add domain limit..."), pools_page);
    auto *remove_pool_button = new QPushButton(QStringLiteral("remove"), pools_page);
    pool_buttons->addWidget(add_pool_button);
    pool_buttons->addWidget(remove_pool_button);
    pool_buttons->addStretch(1);
    pools_layout->addLayout(pool_buttons);
    tabs->addTab(pools_page, QStringLiteral("download pools"));

    auto *bandwidth_page = new QWidget(tabs);
    auto *bandwidth_layout = new QVBoxLayout(bandwidth_page);
    bandwidth_rules = new QTableWidget(bandwidth_page);
    bandwidth_rules->setColumnCount(5);
    bandwidth_rules->setHorizontalHeaderLabels({
        QStringLiteral("context"),
        QStringLiteral("domain"),
        QStringLiteral("type"),
        QStringLiteral("period"),
        QStringLiteral("maximum")
    });
    bandwidth_rules->horizontalHeader()->setStretchLastSection(true);
    bandwidth_rules->setSelectionBehavior(QAbstractItemView::SelectRows);
    bandwidth_rules->setEditTriggers(QAbstractItemView::NoEditTriggers);
    bandwidth_rules->verticalHeader()->setVisible(false);
    bandwidth_layout->addWidget(bandwidth_rules, 1);
    bandwidth_layout->addWidget(new QLabel(
        QStringLiteral("Data limits are in bytes. A zero period means the calendar month."),
        bandwidth_page));

    auto *bandwidth_buttons = new QHBoxLayout();
    auto *add_rule_button = new QPushButton(QStringLiteral("add rule..."), bandwidth_page);
    auto *remove_rule_button = new QPushButton(QStringLiteral("remove"), bandwidth_page);
    bandwidth_buttons->addWidget(add_rule_button);
    bandwidth_buttons->addWidget(remove_rule_button);
    bandwidth_buttons->addStretch(1);
    bandwidth_layout->addLayout(bandwidth_buttons);
    tabs->addTab(bandwidth_page, QStringLiteral("bandwidth"));

    auto *buttons = new QDialogButtonBox(QDialogButtonBox::Save | QDialogButtonBox::Close, this);
    connect(buttons, &QDialogButtonBox::accepted, this, [this]() { save(); });
    connect(buttons, &QDialogButtonBox::rejected, this, &QDialog::close);
    layout->addWidget(buttons);

    connect(paused, &QCheckBox::clicked, this, [this](bool) {
        main->notify(QStringLiteral("network_pause"));
    });
    connect(add_header_button, &QPushButton::clicked, this, [this]() { add_header(); });
    connect(remove_header_button, &QPushButton::clicked, this, [this]() { remove_header(); });
    connect(add_class, &QPushButton::clicked, this, [this]() { add_url_class(); });
    connect(edit_class, &QPushButton::clicked, this, [this]() { edit_url_class(); });
    connect(remove_class, &QPushButton::clicked, this, [this]() { remove_url_class(); });
    connect(add_pool_button, &QPushButton::clicked, this, [this]() { add_pool(); });
    connect(remove_pool_button, &QPushButton::clicked, this, [this]() { remove_pool(); });
    connect(add_rule_button, &QPushButton::clicked, this, [this]() { add_bandwidth_rule(); });
    connect(remove_rule_button, &QPushButton::clicked, this, [this]() { remove_bandwidth_rule(); });

    set_state(state);
}

void HydrusNetworkDialog::set_state(const QJsonObject &state) {
    data = state;
    const QJsonObject options = state.value(QStringLiteral("options")).toObject();

    http_proxy->setText(options.value(QStringLiteral("http_proxy")).toString());
    https_proxy->setText(options.value(QStringLiteral("https_proxy")).toString());
    no_proxy->setText(options.value(QStringLiteral("no_proxy")).toString());
    timeout->setValue(options.value(QStringLiteral("network_timeout")).toInt(10));
    max_jobs->setValue(options.value(QStringLiteral("max_network_jobs")).toInt(15));
    max_jobs_domain->setValue(options.value(QStringLiteral("max_network_jobs_per_domain")).toInt(3));

    api_enabled->setChecked(options.value(QStringLiteral("api_enabled")).toBool());
    api_port->setValue(options.value(QStringLiteral("api_port")).toInt(45869));
    api_key->setText(options.value(QStringLiteral("api_access_key")).toString());
    api_status->setText(state.value(QStringLiteral("api_status")).toString());

    QSignalBlocker blocker(paused);
    paused->setChecked(state.value(QStringLiteral("paused")).toBool());

    const QJsonArray header_values = state.value(QStringLiteral("headers")).toArray();
    headers->setRowCount(header_values.size());

    for (int index = 0; index < header_values.size(); ++index) {
        const QJsonObject header = header_values.at(index).toObject();

        if (headers->item(index, 0) == nullptr) {
            headers->setItem(index, 0, new QTableWidgetItem());
            headers->setItem(index, 1, new QTableWidgetItem());
        }

        headers->item(index, 0)->setText(header.value(QStringLiteral("name")).toString());
        headers->item(index, 1)->setText(header.value(QStringLiteral("value")).toString());
    }

    const QJsonArray classes = state.value(QStringLiteral("url_classes")).toArray();
    url_classes->setRowCount(classes.size());

    for (int index = 0; index < classes.size(); ++index) {
        const QJsonObject entry = classes.at(index).toObject();

        for (int column = 0; column < 4; ++column) {
            if (url_classes->item(index, column) == nullptr) {
                url_classes->setItem(index, column, new QTableWidgetItem());
            }
        }

        url_classes->item(index, 0)->setText(entry.value(QStringLiteral("name")).toString());
        url_classes->item(index, 0)->setData(Qt::UserRole, entry.value(QStringLiteral("name")).toString());
        url_classes->item(index, 1)->setText(entry.value(QStringLiteral("url_type_name")).toString());
        url_classes->item(index, 2)->setText(entry.value(QStringLiteral("netloc")).toString());
        url_classes->item(index, 3)->setText(json_to_strings(entry.value(QStringLiteral("components")).toArray()).join(QStringLiteral("/")));
    }

    const QJsonArray pool_values = state.value(QStringLiteral("download_pools")).toArray();
    pools->setRowCount(pool_values.size());

    for (int index = 0; index < pool_values.size(); ++index) {
        const QJsonObject entry = pool_values.at(index).toObject();

        for (int column = 0; column < 2; ++column) {
            if (pools->item(index, column) == nullptr) {
                pools->setItem(index, column, new QTableWidgetItem());
            }
        }

        pools->item(index, 0)->setText(entry.value(QStringLiteral("domain")).toString());
        pools->item(index, 0)->setData(Qt::UserRole, entry.value(QStringLiteral("domain")).toString());
        pools->item(index, 1)->setText(QString::number(entry.value(QStringLiteral("limit")).toInt()));
    }

    const QJsonArray rules = state.value(QStringLiteral("bandwidth_rules")).toArray();
    bandwidth_rules->setRowCount(rules.size());

    for (int index = 0; index < rules.size(); ++index) {
        const QJsonObject entry = rules.at(index).toObject();

        for (int column = 0; column < 5; ++column) {
            if (bandwidth_rules->item(index, column) == nullptr) {
                bandwidth_rules->setItem(index, column, new QTableWidgetItem());
            }
        }

        const QString context_type = entry.value(QStringLiteral("context_type")).toString();
        const QString context = entry.value(QStringLiteral("context")).toString();
        const QString bandwidth_type = entry.value(QStringLiteral("bandwidth_type")).toString();
        const QJsonValue period_value = entry.value(QStringLiteral("time_delta"));
        const qint64 max_value = entry.value(QStringLiteral("max_allowed")).toVariant().toLongLong();

        bandwidth_rules->item(index, 0)->setText(context_type);
        bandwidth_rules->item(index, 0)->setData(Qt::UserRole, context_type);
        bandwidth_rules->item(index, 1)->setText(context);
        bandwidth_rules->item(index, 1)->setData(Qt::UserRole, context);
        bandwidth_rules->item(index, 2)->setText(bandwidth_type);
        bandwidth_rules->item(index, 2)->setData(Qt::UserRole, bandwidth_type);
        bandwidth_rules->item(index, 3)->setText(
            period_value.isNull() || period_value.toInt() <= 0
                ? QStringLiteral("month")
                : QStringLiteral("%1 s").arg(period_value.toInt()));
        bandwidth_rules->item(index, 4)->setText(
            bandwidth_type == QStringLiteral("data")
                ? QStringLiteral("%1 MB").arg(max_value / (1024 * 1024))
                : QString::number(max_value));
    }

    headers->resizeRowsToContents();
    url_classes->resizeRowsToContents();
    pools->resizeRowsToContents();
    bandwidth_rules->resizeRowsToContents();
}

void HydrusNetworkDialog::save() {
    QJsonObject booleans;
    booleans.insert(QStringLiteral("api_enabled"), api_enabled->isChecked());

    QJsonObject integers;
    integers.insert(QStringLiteral("network_timeout"), timeout->value());
    integers.insert(QStringLiteral("max_network_jobs"), max_jobs->value());
    integers.insert(QStringLiteral("max_network_jobs_per_domain"), max_jobs_domain->value());
    integers.insert(QStringLiteral("api_port"), api_port->value());

    QJsonObject strings;
    strings.insert(QStringLiteral("api_access_key"), api_key->text());

    QJsonObject noneable_strings;
    noneable_strings.insert(QStringLiteral("http_proxy"), http_proxy->text());
    noneable_strings.insert(QStringLiteral("https_proxy"), https_proxy->text());
    noneable_strings.insert(QStringLiteral("no_proxy"), no_proxy->text());

    QJsonObject payload;
    payload.insert(QStringLiteral("booleans"), booleans);
    payload.insert(QStringLiteral("integers"), integers);
    payload.insert(QStringLiteral("strings"), strings);
    payload.insert(QStringLiteral("noneable_strings"), noneable_strings);

    main->notify(QStringLiteral("network_save"), QString(), json_payload(payload));
}

void HydrusNetworkDialog::add_header() {
    QDialog dialog(this);
    dialog.setWindowTitle(QStringLiteral("add custom header"));
    auto *layout = new QVBoxLayout(&dialog);
    auto *form = new QFormLayout();
    auto *name = new QLineEdit(&dialog);
    auto *value = new QLineEdit(&dialog);
    form->addRow(QStringLiteral("name"), name);
    form->addRow(QStringLiteral("value"), value);
    layout->addLayout(form);
    auto *buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, &dialog);
    connect(buttons, &QDialogButtonBox::accepted, &dialog, &QDialog::accept);
    QObject::connect(buttons, &QDialogButtonBox::rejected, &dialog, &QDialog::reject);
    layout->addWidget(buttons);

    if (dialog.exec() != QDialog::Accepted || name->text().trimmed().isEmpty()) {
        return;
    }

    QJsonObject payload;
    payload.insert(QStringLiteral("name"), name->text().trimmed());
    payload.insert(QStringLiteral("value"), value->text());

    main->notify(QStringLiteral("network_header_add"), QString(), json_payload(payload));
}

void HydrusNetworkDialog::remove_header() {
    const int row = headers->currentRow();

    if (row < 0 || headers->item(row, 0) == nullptr) {
        return;
    }

    QJsonObject payload;
    payload.insert(QStringLiteral("name"), headers->item(row, 0)->text());

    main->notify(QStringLiteral("network_header_remove"), QString(), json_payload(payload));
}

void HydrusNetworkDialog::add_url_class() {
    HydrusUrlClassDialog dialog(QJsonObject(), data.value(QStringLiteral("url_types")).toArray(), this);

    if (dialog.exec() != QDialog::Accepted) {
        return;
    }

    QJsonObject payload = dialog.payload();

    if (payload.value(QStringLiteral("netloc")).toString().isEmpty()) {
        return;
    }

    main->notify(QStringLiteral("network_url_class_add"), QString(), json_payload(payload));
}

void HydrusNetworkDialog::edit_url_class() {
    const int row = url_classes->currentRow();
    const QJsonArray classes = data.value(QStringLiteral("url_classes")).toArray();

    if (row < 0 || row >= classes.size()) {
        return;
    }

    HydrusUrlClassDialog dialog(
        classes.at(row).toObject(),
        data.value(QStringLiteral("url_types")).toArray(),
        this);

    if (dialog.exec() != QDialog::Accepted) {
        return;
    }

    main->notify(
        QStringLiteral("network_url_class_save"),
        QString(),
        json_payload(dialog.payload()));
}

void HydrusNetworkDialog::remove_url_class() {
    const int row = url_classes->currentRow();

    if (row < 0 || url_classes->item(row, 0) == nullptr) {
        return;
    }

    QJsonObject payload;
    payload.insert(QStringLiteral("name"), url_classes->item(row, 0)->text());

    main->notify(QStringLiteral("network_url_class_remove"), QString(), json_payload(payload));
}

void HydrusNetworkDialog::add_pool() {
    QDialog dialog(this);
    dialog.setWindowTitle(QStringLiteral("domain download pool"));
    auto *layout = new QVBoxLayout(&dialog);
    auto *form = new QFormLayout();
    auto *domain = new QLineEdit(&dialog);
    domain->setPlaceholderText(QStringLiteral("example.com"));
    auto *limit = new QSpinBox(&dialog);
    limit->setRange(1, 64);
    limit->setValue(2);
    form->addRow(QStringLiteral("domain"), domain);
    form->addRow(QStringLiteral("simultaneous downloads"), limit);
    layout->addLayout(form);
    auto *buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, &dialog);
    connect(buttons, &QDialogButtonBox::accepted, &dialog, &QDialog::accept);
    QObject::connect(buttons, &QDialogButtonBox::rejected, &dialog, &QDialog::reject);
    layout->addWidget(buttons);

    if (dialog.exec() != QDialog::Accepted || domain->text().trimmed().isEmpty()) {
        return;
    }

    QJsonObject payload;
    payload.insert(QStringLiteral("domain"), domain->text().trimmed());
    payload.insert(QStringLiteral("limit"), limit->value());
    main->notify(QStringLiteral("network_pool_set"), QString(), json_payload(payload));
}

void HydrusNetworkDialog::remove_pool() {
    const int row = pools->currentRow();

    if (row < 0 || pools->item(row, 0) == nullptr) {
        return;
    }

    QJsonObject payload;
    payload.insert(QStringLiteral("domain"), pools->item(row, 0)->data(Qt::UserRole).toString());
    main->notify(QStringLiteral("network_pool_remove"), QString(), json_payload(payload));
}

void HydrusNetworkDialog::add_bandwidth_rule() {
    QDialog dialog(this);
    dialog.setWindowTitle(QStringLiteral("bandwidth rule"));
    auto *layout = new QVBoxLayout(&dialog);
    auto *form = new QFormLayout();
    auto *context_type = new QComboBox(&dialog);
    context_type->addItem(QStringLiteral("global"), QStringLiteral("global"));
    context_type->addItem(QStringLiteral("domain"), QStringLiteral("domain"));
    context_type->addItem(QStringLiteral("hydrus"), QStringLiteral("hydrus"));
    auto *context = new QLineEdit(&dialog);
    context->setPlaceholderText(QStringLiteral("domain name (domain rules only)"));
    auto *bandwidth_type = new QComboBox(&dialog);
    bandwidth_type->addItem(QStringLiteral("requests"), QStringLiteral("requests"));
    bandwidth_type->addItem(QStringLiteral("data"), QStringLiteral("data"));
    auto *period = new QSpinBox(&dialog);
    period->setRange(0, 30 * 86400);
    period->setValue(1);
    period->setSpecialValueText(QStringLiteral("calendar month"));
    auto *maximum = new QSpinBox(&dialog);
    maximum->setRange(0, 2'000'000'000);
    maximum->setValue(1);
    auto *hint = new QLabel(QStringLiteral("Data maximums are in MB; request maximums are counts."), &dialog);
    hint->setWordWrap(true);
    form->addRow(QStringLiteral("context"), context_type);
    form->addRow(QStringLiteral("domain"), context);
    form->addRow(QStringLiteral("type"), bandwidth_type);
    form->addRow(QStringLiteral("period (seconds)"), period);
    form->addRow(QStringLiteral("maximum"), maximum);
    layout->addLayout(form);
    layout->addWidget(hint);
    auto *buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, &dialog);
    connect(buttons, &QDialogButtonBox::accepted, &dialog, &QDialog::accept);
    QObject::connect(buttons, &QDialogButtonBox::rejected, &dialog, &QDialog::reject);
    layout->addWidget(buttons);

    connect(bandwidth_type, QOverload<int>::of(&QComboBox::currentIndexChanged), &dialog, [maximum, bandwidth_type](int) {
        if (bandwidth_type->currentData().toString() == QStringLiteral("data")) {
            maximum->setRange(0, 2'000'000);
            maximum->setValue(1024);
        } else {
            maximum->setRange(0, 2'000'000'000);
            maximum->setValue(1);
        }
    });

    if (dialog.exec() != QDialog::Accepted) {
        return;
    }

    QJsonObject payload;
    payload.insert(QStringLiteral("context_type"), context_type->currentData().toString());
    payload.insert(QStringLiteral("context"), context->text().trimmed());
    payload.insert(QStringLiteral("bandwidth_type"), bandwidth_type->currentData().toString());

    if (period->value() <= 0) {
        payload.insert(QStringLiteral("time_delta"), QJsonValue::Null);
    } else {
        payload.insert(QStringLiteral("time_delta"), period->value());
    }

    payload.insert(
        QStringLiteral("max_allowed"),
        bandwidth_type->currentData().toString() == QStringLiteral("data")
            ? static_cast<qint64>(maximum->value()) * 1024 * 1024
            : static_cast<qint64>(maximum->value()));

    main->notify(QStringLiteral("network_rule_set"), QString(), json_payload(payload));
}

void HydrusNetworkDialog::remove_bandwidth_rule() {
    const int row = bandwidth_rules->currentRow();

    if (row < 0 || bandwidth_rules->item(row, 0) == nullptr) {
        return;
    }

    QJsonObject payload;
    payload.insert(QStringLiteral("context_type"), bandwidth_rules->item(row, 0)->data(Qt::UserRole).toString());
    payload.insert(QStringLiteral("context"), bandwidth_rules->item(row, 1)->data(Qt::UserRole).toString());
    payload.insert(QStringLiteral("bandwidth_type"), bandwidth_rules->item(row, 2)->data(Qt::UserRole).toString());

    main->notify(QStringLiteral("network_rule_remove"), QString(), json_payload(payload));
}

// ---------------------------------------------------------------------------
// HydrusTagRelationsDialog
// ---------------------------------------------------------------------------

HydrusTagRelationsDialog::HydrusTagRelationsDialog(HydrusMainWindow *main_window, const QJsonObject &state, QWidget *parent)
    : QDialog(parent), main(main_window) {
    setWindowTitle(QStringLiteral("tag siblings and parents"));
    setAttribute(Qt::WA_DeleteOnClose, true);
    resize(720, 520);

    auto *layout = new QVBoxLayout(this);

    auto *service_row = new QHBoxLayout();
    service_row->addWidget(new QLabel(QStringLiteral("service"), this));
    services = new QComboBox(this);
    service_row->addWidget(services, 1);
    layout->addLayout(service_row);

    auto *help = new QLabel(
        QStringLiteral("Siblings redirect a bad tag onto its preferred spelling. Parents describe a "
                       "child -> parent hierarchy. The left column is the tag being changed; the right "
                       "column is what it points to."),
        this);
    help->setWordWrap(true);
    layout->addWidget(help);

    tabs = new QTabWidget(this);

    auto make_table = [this]() {
        auto *table = new QTableWidget(this);
        table->setColumnCount(2);
        table->setHorizontalHeaderLabels({QStringLiteral("tag"), QStringLiteral("points to")});
        table->horizontalHeader()->setStretchLastSection(true);
        table->horizontalHeader()->setSectionResizeMode(0, QHeaderView::Stretch);
        table->setSelectionBehavior(QAbstractItemView::SelectRows);
        table->setSelectionMode(QAbstractItemView::SingleSelection);
        table->setEditTriggers(QAbstractItemView::NoEditTriggers);
        table->verticalHeader()->setVisible(false);
        return table;
    };

    siblings = make_table();
    parents = make_table();
    tabs->addTab(siblings, QStringLiteral("siblings"));
    tabs->addTab(parents, QStringLiteral("parents"));
    layout->addWidget(tabs, 1);

    auto *buttons = new QHBoxLayout();
    auto *add = new QPushButton(QStringLiteral("add pair..."), this);
    auto *remove = new QPushButton(QStringLiteral("remove"), this);
    auto *close_button = new QPushButton(QStringLiteral("close"), this);
    buttons->addWidget(add);
    buttons->addWidget(remove);
    buttons->addStretch(1);
    buttons->addWidget(close_button);
    layout->addLayout(buttons);

    connect(services, QOverload<int>::of(&QComboBox::currentIndexChanged), this, [this](int) {
        rebuild();
    });
    connect(add, &QPushButton::clicked, this, [this]() { add_relation(); });
    connect(remove, &QPushButton::clicked, this, [this]() { remove_selected(); });
    connect(close_button, &QPushButton::clicked, this, &QDialog::close);

    set_state(state);
}

void HydrusTagRelationsDialog::set_state(const QJsonObject &state) {
    const bool first_load = data.isEmpty();
    data = state;

    const QString previous = services->currentData().toString();
    QSignalBlocker blocker(services);
    services->clear();

    for (const QJsonValue &value : state.value(QStringLiteral("services")).toArray()) {
        const QJsonObject service = value.toObject();
        services->addItem(service.value(QStringLiteral("name")).toString(),
                          service.value(QStringLiteral("key")).toString());
    }

    if (services->count() == 0) {
        services->addItem(QStringLiteral("no local tag service"), QString());
    }

    if (!previous.isEmpty()) {
        const int index = services->findData(previous);

        if (index >= 0) {
            services->setCurrentIndex(index);
        }
    }

    blocker.unblock();

    if (first_load) {
        const QString tab = state.value(QStringLiteral("tab")).toString();
        tabs->setCurrentIndex(tab == QStringLiteral("parents") ? 1 : 0);
    }

    rebuild();
}

void HydrusTagRelationsDialog::rebuild() {
    siblings->setRowCount(0);
    parents->setRowCount(0);

    const QString key = services->currentData().toString();

    for (const QJsonValue &value : data.value(QStringLiteral("relations")).toArray()) {
        const QJsonObject relation = value.toObject();

        if (relation.value(QStringLiteral("service")).toString() != key) {
            continue;
        }

        const bool is_parent = relation.value(QStringLiteral("relation")).toString() == QStringLiteral("parents");
        QTableWidget *table = is_parent ? parents : siblings;
        const int row = table->rowCount();
        table->setRowCount(row + 1);

        auto *first = new QTableWidgetItem(relation.value(QStringLiteral("first")).toString());
        first->setData(Qt::UserRole, relation.value(QStringLiteral("first")).toString());
        auto *second = new QTableWidgetItem(relation.value(QStringLiteral("second")).toString());
        second->setData(Qt::UserRole, relation.value(QStringLiteral("second")).toString());

        table->setItem(row, 0, first);
        table->setItem(row, 1, second);
    }

    for (QTableWidget *table : {siblings, parents}) {
        table->resizeRowsToContents();
    }
}

QTableWidget *HydrusTagRelationsDialog::current_table() const {
    return tabs->currentIndex() == 1 ? parents : siblings;
}

QString HydrusTagRelationsDialog::current_relation() const {
    return tabs->currentIndex() == 1 ? QStringLiteral("parents") : QStringLiteral("siblings");
}

void HydrusTagRelationsDialog::add_relation() {
    const QString relation = current_relation();
    const bool is_parent = relation == QStringLiteral("parents");

    QDialog dialog(this);
    dialog.setWindowTitle(is_parent ? QStringLiteral("add tag parent") : QStringLiteral("add tag sibling"));
    auto *layout = new QVBoxLayout(&dialog);
    auto *form = new QFormLayout();

    auto *first = new QLineEdit(&dialog);
    auto *second = new QLineEdit(&dialog);
    first->setPlaceholderText(is_parent ? QStringLiteral("child tag") : QStringLiteral("bad tag"));
    second->setPlaceholderText(is_parent ? QStringLiteral("parent tag") : QStringLiteral("good tag"));
    form->addRow(is_parent ? QStringLiteral("child") : QStringLiteral("bad tag"), first);
    form->addRow(is_parent ? QStringLiteral("parent") : QStringLiteral("good tag"), second);
    layout->addLayout(form);

    auto *buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, &dialog);
    connect(buttons, &QDialogButtonBox::accepted, &dialog, &QDialog::accept);
    QObject::connect(buttons, &QDialogButtonBox::rejected, &dialog, &QDialog::reject);
    layout->addWidget(buttons);

    if (dialog.exec() != QDialog::Accepted) {
        return;
    }

    if (first->text().trimmed().isEmpty() || second->text().trimmed().isEmpty()) {
        return;
    }

    QJsonObject payload;
    payload.insert(QStringLiteral("relation"), relation);
    payload.insert(QStringLiteral("service"), services->currentData().toString());
    payload.insert(QStringLiteral("first"), first->text().trimmed());
    payload.insert(QStringLiteral("second"), second->text().trimmed());

    main->notify(QStringLiteral("tag_relations_add"), QString(), json_payload(payload));
}

void HydrusTagRelationsDialog::remove_selected() {
    QTableWidget *table = current_table();
    const int row = table->currentRow();

    if (row < 0 || table->item(row, 0) == nullptr || table->item(row, 1) == nullptr) {
        return;
    }

    QJsonObject payload;
    payload.insert(QStringLiteral("relation"), current_relation());
    payload.insert(QStringLiteral("service"), services->currentData().toString());
    payload.insert(QStringLiteral("first"), table->item(row, 0)->data(Qt::UserRole).toString());
    payload.insert(QStringLiteral("second"), table->item(row, 1)->data(Qt::UserRole).toString());

    main->notify(QStringLiteral("tag_relations_remove"), QString(), json_payload(payload));
}

// ---------------------------------------------------------------------------
// HydrusImportFoldersPage
// ---------------------------------------------------------------------------

HydrusImportFoldersPage::HydrusImportFoldersPage(HydrusMainWindow *main_window, const QString &page_id, const QJsonObject &state)
    : QWidget(), main(main_window), id(page_id) {
    auto *layout = new QVBoxLayout(this);
    layout->setContentsMargins(6, 6, 6, 6);

    status = new QLabel(this);
    status->setWordWrap(true);
    layout->addWidget(status);

    table = new QTableWidget(this);
    table->setColumnCount(7);
    table->setHorizontalHeaderLabels({
        QStringLiteral("name"),
        QStringLiteral("path"),
        QStringLiteral("check every"),
        QStringLiteral("after import"),
        QStringLiteral("filters"),
        QStringLiteral("status"),
        QStringLiteral("last queued")
    });
    table->horizontalHeader()->setStretchLastSection(true);
    table->horizontalHeader()->setSectionResizeMode(1, QHeaderView::Stretch);
    table->horizontalHeader()->setSectionResizeMode(4, QHeaderView::Stretch);
    table->setSelectionBehavior(QAbstractItemView::SelectRows);
    table->setSelectionMode(QAbstractItemView::SingleSelection);
    table->setEditTriggers(QAbstractItemView::NoEditTriggers);
    table->verticalHeader()->setVisible(false);
    layout->addWidget(table, 1);

    auto *buttons = new QHBoxLayout();
    auto *add = new QPushButton(QStringLiteral("add folder..."), this);
    auto *edit = new QPushButton(QStringLiteral("edit..."), this);
    auto *check = new QPushButton(QStringLiteral("check now"), this);
    auto *toggle = new QPushButton(QStringLiteral("pause/resume"), this);
    auto *remove = new QPushButton(QStringLiteral("remove"), this);
    buttons->addWidget(add);
    buttons->addWidget(edit);
    buttons->addWidget(check);
    buttons->addWidget(toggle);
    buttons->addWidget(remove);
    buttons->addStretch(1);
    layout->addLayout(buttons);

    connect(add, &QPushButton::clicked, this, [this]() {
        main->notify(QStringLiteral("import_folder_edit"), id);
    });
    connect(edit, &QPushButton::clicked, this, [this]() { with_selected(QStringLiteral("import_folder_edit")); });
    connect(check, &QPushButton::clicked, this, [this]() { with_selected(QStringLiteral("import_folder_check")); });
    connect(toggle, &QPushButton::clicked, this, [this]() { with_selected(QStringLiteral("import_folder_toggle")); });
    connect(remove, &QPushButton::clicked, this, [this]() { with_selected(QStringLiteral("import_folder_remove")); });

    update_state(state);
}

const QString &HydrusImportFoldersPage::page_id() const {
    return id;
}

void HydrusImportFoldersPage::with_selected(const QString &action) {
    const int row = table->currentRow();

    if (row < 0 || table->item(row, 0) == nullptr) {
        return;
    }

    main->notify(action, id, table->item(row, 0)->data(Qt::UserRole).toString());
}

void HydrusImportFoldersPage::update_state(const QJsonObject &state) {
    status->setText(state.value(QStringLiteral("status")).toString());

    const QJsonArray folders = state.value(QStringLiteral("folders")).toArray();
    table->setRowCount(folders.size());

    for (int index = 0; index < folders.size(); ++index) {
        const QJsonObject folder = folders.at(index).toObject();

        for (int column = 0; column < 7; ++column) {
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

        const QString action = folder.value(QStringLiteral("action")).toString();
        QString action_text = action;

        if (action == QStringLiteral("move")) {
            action_text = QStringLiteral("move -> %1").arg(
                folder.value(QStringLiteral("action_path")).toString());
        }

        QStringList filters;
        const QString extensions = folder.value(QStringLiteral("extensions")).toString().trimmed();
        const qint64 min_size = folder.value(QStringLiteral("min_size")).toVariant().toLongLong();
        const qint64 max_size = folder.value(QStringLiteral("max_size")).toVariant().toLongLong();
        const qint64 skip = folder.value(QStringLiteral("last_modified_skip")).toVariant().toLongLong();

        if (!extensions.isEmpty()) {
            filters.append(QStringLiteral("ext: %1").arg(extensions));
        }

        if (min_size > 0 || max_size > 0) {
            filters.append(QStringLiteral("%1 - %2 bytes").arg(min_size).arg(
                max_size > 0 ? QString::number(max_size) : QStringLiteral("any")));
        }

        if (skip > 0) {
            filters.append(QStringLiteral("skip files newer than %1 s").arg(skip));
        }

        table->item(index, 1)->setText(folder.value(QStringLiteral("path")).toString());
        table->item(index, 2)->setText(
            QStringLiteral("%1 s").arg(folder.value(QStringLiteral("period")).toInt()));
        table->item(index, 3)->setText(action_text);
        table->item(index, 4)->setText(filters.isEmpty() ? QStringLiteral("any") : filters.join(QStringLiteral("; ")));
        table->item(index, 5)->setText(folder.value(QStringLiteral("status")).toString());
        table->item(index, 6)->setText(QString::number(folder.value(QStringLiteral("last_imported")).toInt()));
    }

    table->resizeRowsToContents();
}

// ---------------------------------------------------------------------------
// HydrusExportFoldersPage
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// HydrusDuplicatesRulesDialog and the auto-resolution editors
// ---------------------------------------------------------------------------

namespace {

QComboBox *catalog_combo(const QJsonObject &catalog, const QString &key, QWidget *parent) {
    auto *combo = new QComboBox(parent);

    for (const QJsonValue &entry : catalog.value(key).toArray()) {
        const QJsonObject object = entry.toObject();
        combo->addItem(object.value(QStringLiteral("label")).toString(),
                       object.value(QStringLiteral("value")).toString());
    }

    return combo;
}

void select_combo(QComboBox *combo, const QString &value) {
    const int index = combo->findData(value);

    if (index >= 0) {
        combo->setCurrentIndex(index);
    }
}

QString comparator_pretty(const QJsonObject &comparator) {
    const QString kind = comparator.value(QStringLiteral("kind")).toString();

    if (kind == QLatin1String("group")) {
        const QString mode = comparator.value(QStringLiteral("mode")).toString();
        const int count = comparator.value(QStringLiteral("comparators")).toArray().size();

        return QStringLiteral("%1 group (%2 condition(s))")
            .arg(mode == QLatin1String("any") ? QStringLiteral("OR") : QStringLiteral("AND"))
            .arg(count);
    }

    if (kind == QLatin1String("filetype")) {
        return comparator.value(QStringLiteral("same")).toBool(true)
                   ? QStringLiteral("A and B have the same filetype")
                   : QStringLiteral("A and B have different filetypes");
    }

    const QString property = comparator.value(QStringLiteral("property")).toString();
    const QString op = comparator.value(QStringLiteral("operator")).toString(QStringLiteral(">"));

    if (kind == QLatin1String("relative")) {
        const double multiplier = comparator.value(QStringLiteral("multiplier")).toDouble(1.0);
        const double delta = comparator.value(QStringLiteral("delta")).toDouble(0.0);
        QString right = QStringLiteral("B");

        if (qAbs(multiplier - 1.0) > 0.000001) {
            right = QStringLiteral("%1x %2").arg(multiplier).arg(right);
        }

        if (delta > 0.0) {
            right += QStringLiteral(" +%1").arg(delta);
        } else if (delta < 0.0) {
            right += QStringLiteral(" -%1").arg(-delta);
        }

        return QStringLiteral("A %1 %2 %3").arg(property, op, right);
    }

    const QString target = comparator.value(QStringLiteral("looking_at")).toString(QStringLiteral("a"));
    const QString value = property == QLatin1String("mime")
                              ? comparator.value(QStringLiteral("text")).toString()
                              : QString::number(comparator.value(QStringLiteral("value")).toDouble());

    return QStringLiteral("%1 %2 %3 %4")
        .arg(target == QLatin1String("b") ? QStringLiteral("B")
             : target == QLatin1String("either") ? QStringLiteral("either")
                                                 : QStringLiteral("A"),
             property,
             op,
             value);
}

QJsonObject edit_group_dialog(QWidget *parent, const QJsonObject &initial, const QJsonObject &catalog);

QJsonObject edit_condition_dialog(QWidget *parent, const QJsonObject &initial, const QJsonObject &catalog) {
    QDialog dialog(parent);
    dialog.setWindowTitle(QStringLiteral("edit comparison"));
    dialog.resize(460, 300);

    auto *layout = new QVBoxLayout(&dialog);

    auto *kind = new QComboBox(&dialog);
    kind->addItem(QStringLiteral("one file condition"), QStringLiteral("one_file"));
    kind->addItem(QStringLiteral("compare A and B"), QStringLiteral("relative"));
    kind->addItem(QStringLiteral("filetype check"), QStringLiteral("filetype"));
    select_combo(kind, initial.value(QStringLiteral("kind")).toString(QStringLiteral("one_file")));
    layout->addWidget(kind);

    auto *one_file = new QWidget(&dialog);
    auto *one_form = new QFormLayout(one_file);
    auto *target = catalog_combo(catalog, QStringLiteral("looking_at"), one_file);
    auto *property = catalog_combo(catalog, QStringLiteral("properties"), one_file);
    auto *operator_combo = catalog_combo(catalog, QStringLiteral("operators"), one_file);
    auto *value = new QLineEdit(one_file);
    auto *text = new QLineEdit(one_file);
    text->setPlaceholderText(QStringLiteral("image, video, png, image/jpeg ..."));
    one_form->addRow(QStringLiteral("test"), target);
    one_form->addRow(QStringLiteral("property"), property);
    one_form->addRow(QStringLiteral("operator"), operator_combo);
    one_form->addRow(QStringLiteral("value"), value);
    one_form->addRow(QStringLiteral("mime / filetype"), text);
    layout->addWidget(one_file);

    auto *relative = new QWidget(&dialog);
    auto *relative_form = new QFormLayout(relative);
    auto *relative_property = catalog_combo(catalog, QStringLiteral("properties"), relative);
    auto *relative_operator = catalog_combo(catalog, QStringLiteral("operators"), relative);
    auto *multiplier = new QDoubleSpinBox(relative);
    multiplier->setRange(0.0, 1000.0);
    multiplier->setDecimals(3);
    multiplier->setValue(1.0);
    auto *delta = new QDoubleSpinBox(relative);
    delta->setRange(-1000000000.0, 1000000000.0);
    delta->setDecimals(2);
    relative_form->addRow(QStringLiteral("property"), relative_property);
    relative_form->addRow(QStringLiteral("operator"), relative_operator);
    relative_form->addRow(QStringLiteral("B multiplier"), multiplier);
    relative_form->addRow(QStringLiteral("B delta"), delta);
    layout->addWidget(relative);

    auto *filetype = new QWidget(&dialog);
    auto *filetype_form = new QFormLayout(filetype);
    auto *same = new QCheckBox(QStringLiteral("A and B have the same filetype"), filetype);
    same->setChecked(initial.value(QStringLiteral("same")).toBool(true));
    filetype_form->addRow(same);
    layout->addWidget(filetype);

    auto update_kind = [kind, one_file, relative, filetype]() {
        const QString current = kind->currentData().toString();
        one_file->setVisible(current == QLatin1String("one_file"));
        relative->setVisible(current == QLatin1String("relative"));
        filetype->setVisible(current == QLatin1String("filetype"));
    };
    QObject::connect(kind, &QComboBox::currentIndexChanged, &dialog, [update_kind](int) { update_kind(); });

    select_combo(target, initial.value(QStringLiteral("looking_at")).toString(QStringLiteral("a")));
    select_combo(property, initial.value(QStringLiteral("property")).toString(QStringLiteral("size")));
    select_combo(relative_property, initial.value(QStringLiteral("property")).toString(QStringLiteral("size")));
    select_combo(operator_combo, initial.value(QStringLiteral("operator")).toString(QStringLiteral(">")));
    select_combo(relative_operator, initial.value(QStringLiteral("operator")).toString(QStringLiteral(">")));
    value->setText(QString::number(initial.value(QStringLiteral("value")).toDouble()));
    text->setText(initial.value(QStringLiteral("text")).toString());
    multiplier->setValue(initial.value(QStringLiteral("multiplier")).toDouble(1.0));
    delta->setValue(initial.value(QStringLiteral("delta")).toDouble(0.0));

    update_kind();

    auto *buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, &dialog);
    QObject::connect(buttons, &QDialogButtonBox::accepted, &dialog, &QDialog::accept);
    QObject::connect(buttons, &QDialogButtonBox::rejected, &dialog, &QDialog::reject);
    layout->addWidget(buttons);

    if (dialog.exec() != QDialog::Accepted) {
        return {};
    }

    const QString current = kind->currentData().toString();
    QJsonObject result;
    result.insert(QStringLiteral("kind"), current);

    if (current == QLatin1String("one_file")) {
        result.insert(QStringLiteral("looking_at"), target->currentData().toString());
        result.insert(QStringLiteral("property"), property->currentData().toString());
        result.insert(QStringLiteral("operator"), operator_combo->currentData().toString());
        result.insert(QStringLiteral("value"), value->text().trimmed().toDouble());
        result.insert(QStringLiteral("text"), text->text().trimmed());
    } else if (current == QLatin1String("relative")) {
        result.insert(QStringLiteral("property"), relative_property->currentData().toString());
        result.insert(QStringLiteral("operator"), relative_operator->currentData().toString());
        result.insert(QStringLiteral("multiplier"), multiplier->value());
        result.insert(QStringLiteral("delta"), delta->value());
    } else {
        result.insert(QStringLiteral("same"), same->isChecked());
    }

    return result;
}

QJsonObject edit_group_dialog(QWidget *parent, const QJsonObject &initial, const QJsonObject &catalog) {
    QDialog dialog(parent);
    dialog.setWindowTitle(QStringLiteral("edit comparison group"));
    dialog.resize(560, 420);

    auto *layout = new QVBoxLayout(&dialog);
    auto *form = new QFormLayout();
    auto *mode = catalog_combo(catalog, QStringLiteral("group_modes"), &dialog);
    select_combo(mode, initial.value(QStringLiteral("mode")).toString(QStringLiteral("all")));
    form->addRow(QStringLiteral("group mode"), mode);
    layout->addLayout(form);

    auto *list = new QListWidget(&dialog);
    layout->addWidget(list, 1);

    QJsonArray comparators = initial.value(QStringLiteral("comparators")).toArray();

    auto refresh = [list, &comparators]() {
        list->clear();

        for (const QJsonValue &value : comparators) {
            auto *item = new QListWidgetItem(comparator_pretty(value.toObject()), list);
            item->setData(Qt::UserRole, QString::fromUtf8(
                                           QJsonDocument(value.toObject()).toJson(QJsonDocument::Compact)));
        }
    };

    auto *buttons_row = new QHBoxLayout();
    auto *add = new QPushButton(QStringLiteral("add condition"), &dialog);
    auto *edit = new QPushButton(QStringLiteral("edit"), &dialog);
    auto *remove = new QPushButton(QStringLiteral("remove"), &dialog);
    buttons_row->addWidget(add);
    buttons_row->addWidget(edit);
    buttons_row->addWidget(remove);
    buttons_row->addStretch(1);
    layout->addLayout(buttons_row);

    QObject::connect(add, &QPushButton::clicked, &dialog, [&]() {
        const QJsonObject condition = edit_condition_dialog(&dialog, QJsonObject(), catalog);

        if (!condition.isEmpty()) {
            comparators.append(condition);
            refresh();
        }
    });

    QObject::connect(edit, &QPushButton::clicked, &dialog, [&]() {
        const int row = list->currentRow();

        if (row < 0) {
            return;
        }

        const QJsonObject current = comparators.at(row).toObject();
        const QJsonObject changed = current.value(QStringLiteral("kind")).toString() == QLatin1String("group")
                                        ? edit_group_dialog(&dialog, current, catalog)
                                        : edit_condition_dialog(&dialog, current, catalog);

        if (!changed.isEmpty()) {
            comparators.replace(row, changed);
            refresh();
        }
    });

    QObject::connect(remove, &QPushButton::clicked, &dialog, [&]() {
        const int row = list->currentRow();

        if (row >= 0) {
            comparators.removeAt(row);
            refresh();
        }
    });

    auto *buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, &dialog);
    QObject::connect(buttons, &QDialogButtonBox::accepted, &dialog, &QDialog::accept);
    QObject::connect(buttons, &QDialogButtonBox::rejected, &dialog, &QDialog::reject);
    layout->addWidget(buttons);

    refresh();

    if (dialog.exec() != QDialog::Accepted || comparators.isEmpty()) {
        return {};
    }

    QJsonObject result;
    result.insert(QStringLiteral("kind"), QStringLiteral("group"));
    result.insert(QStringLiteral("mode"), mode->currentData().toString());
    result.insert(QStringLiteral("comparators"), comparators);

    return result;
}

} // namespace

HydrusAutoResolutionRuleDialog::HydrusAutoResolutionRuleDialog(
    HydrusMainWindow *main_window,
    HydrusDuplicatesRulesDialog *rules,
    const QJsonObject &rule,
    const QJsonObject &rule_catalog,
    QWidget *parent)
    : QDialog(parent), main(main_window), rules_dialog(rules), catalog(rule_catalog) {
    setWindowTitle(QStringLiteral("edit auto-resolution rule"));
    setObjectName(QStringLiteral("hydrus_auto_resolution_rule_dialog"));
    setAttribute(Qt::WA_DeleteOnClose, true);
    resize(820, 680);

    original_name = rule.value(QStringLiteral("name")).toString();
    selector = rule.value(QStringLiteral("pair_selector")).toArray();

    auto *layout = new QVBoxLayout(this);

    auto *warning = new QLabel(
        QStringLiteral("Rules test both ways around. Every comparator in the comparison list must match "
                       "(AND); add a group to mix AND/OR conditions. Preview before enabling automatic actions."),
        this);
    warning->setWordWrap(true);
    layout->addWidget(warning);

    auto *form = new QFormLayout();
    name = new QLineEdit(rule.value(QStringLiteral("name")).toString(), this);
    paused = new QCheckBox(QStringLiteral("paused"), this);
    paused->setChecked(rule.value(QStringLiteral("paused")).toBool());
    operation = catalog_combo(catalog, QStringLiteral("operation_modes"), this);
    select_combo(operation, rule.value(QStringLiteral("operation_mode")).toString(QStringLiteral("semi-automatic")));
    max_pending = new QSpinBox(this);
    max_pending->setRange(1, 10000);
    max_pending->setValue(rule.value(QStringLiteral("max_pending_pairs")).toInt(500));
    max_pending->setToolTip(QStringLiteral("semi-automatic queue cap for this rule"));
    form->addRow(QStringLiteral("name"), name);
    form->addRow(QStringLiteral("paused"), paused);
    form->addRow(QStringLiteral("operation"), operation);
    form->addRow(QStringLiteral("max pending pairs"), max_pending);
    layout->addLayout(form);

    auto *tabs = new QTabWidget(this);

    auto *search_tab = new QWidget(tabs);
    auto *search_form = new QFormLayout(search_tab);
    search_query = new QLineEdit(rule.value(QStringLiteral("search_query")).toString(), search_tab);
    search_query->setPlaceholderText(QStringLiteral("e.g. system:filetype = image, or leave empty for every pair"));
    search_mode = catalog_combo(catalog, QStringLiteral("search_modes"), search_tab);
    if (search_mode->count() == 0) {
        search_mode->addItem(QStringLiteral("both files match"), QStringLiteral("both"));
        search_mode->addItem(QStringLiteral("either file matches"), QStringLiteral("either"));
    }
    select_combo(search_mode, rule.value(QStringLiteral("search_mode")).toString(QStringLiteral("both")));
    max_distance = new QSpinBox(search_tab);
    max_distance->setRange(0, 1000);
    max_distance->setValue(rule.value(QStringLiteral("max_distance")).toInt(0));
    max_distance->setSpecialValueText(QStringLiteral("use the page/global distance"));
    search_form->addRow(QStringLiteral("search predicate"), search_query);
    search_form->addRow(QStringLiteral("files must"), search_mode);
    search_form->addRow(QStringLiteral("maximum distance"), max_distance);
    tabs->addTab(search_tab, QStringLiteral("search"));

    auto *comparison_tab = new QWidget(tabs);
    auto *comparison_layout = new QVBoxLayout(comparison_tab);
    auto *comparison_help = new QLabel(
        QStringLiteral("The pair must satisfy every row. A single row may be a one-file condition, "
                       "an A/B comparison or an AND/OR group."),
        comparison_tab);
    comparison_help->setWordWrap(true);
    comparison_layout->addWidget(comparison_help);
    comparators = new QListWidget(comparison_tab);
    comparison_layout->addWidget(comparators, 1);
    auto *comparison_buttons = new QHBoxLayout();
    auto *add_condition = new QPushButton(QStringLiteral("add condition..."), comparison_tab);
    auto *add_group = new QPushButton(QStringLiteral("add group..."), comparison_tab);
    auto *edit_condition = new QPushButton(QStringLiteral("edit"), comparison_tab);
    auto *remove_condition = new QPushButton(QStringLiteral("remove"), comparison_tab);
    comparison_buttons->addWidget(add_condition);
    comparison_buttons->addWidget(add_group);
    comparison_buttons->addWidget(edit_condition);
    comparison_buttons->addWidget(remove_condition);
    comparison_buttons->addStretch(1);
    comparison_layout->addLayout(comparison_buttons);
    tabs->addTab(comparison_tab, QStringLiteral("comparison"));

    auto *action_tab = new QWidget(tabs);
    auto *action_form = new QFormLayout(action_tab);
    action = catalog_combo(catalog, QStringLiteral("actions"), action_tab);
    select_combo(action, rule.value(QStringLiteral("action")).toString(QStringLiteral("better")));
    delete_a = new QCheckBox(QStringLiteral("also delete A after the action"), action_tab);
    delete_b = new QCheckBox(QStringLiteral("also delete B after the action"), action_tab);
    delete_a->setChecked(rule.value(QStringLiteral("delete_a")).toBool());
    delete_b->setChecked(rule.value(QStringLiteral("delete_b")).toBool());
    action_form->addRow(QStringLiteral("action"), action);
    action_form->addRow(delete_a);
    action_form->addRow(delete_b);
    tabs->addTab(action_tab, QStringLiteral("action"));

    auto *preview_tab = new QWidget(tabs);
    auto *preview_layout = new QVBoxLayout(preview_tab);
    auto *preview_help = new QLabel(
        QStringLiteral("Test this rule against the current duplicate queue. Nothing is changed."),
        preview_tab);
    preview_help->setWordWrap(true);
    preview_layout->addWidget(preview_help);
    preview_label = new QLabel(QStringLiteral("not tested yet"), preview_tab);
    preview_label->setWordWrap(true);
    preview_label->setTextInteractionFlags(Qt::TextSelectableByMouse);
    preview_layout->addWidget(preview_label);
    auto *run_preview = new QPushButton(QStringLiteral("test against the queue"), preview_tab);
    preview_layout->addWidget(run_preview, 0, Qt::AlignLeft);
    preview_layout->addStretch(1);
    tabs->addTab(preview_tab, QStringLiteral("preview"));

    layout->addWidget(tabs, 1);

    auto *footer = new QHBoxLayout();
    auto *cancel = new QPushButton(QStringLiteral("cancel"), this);
    auto *save = new QPushButton(QStringLiteral("save rule"), this);
    footer->addStretch(1);
    footer->addWidget(cancel);
    footer->addWidget(save);
    layout->addLayout(footer);

    QObject::connect(add_condition, &QPushButton::clicked, this, [this]() { add_comparator(); });
    QObject::connect(add_group, &QPushButton::clicked, this, [this]() { this->add_group(); });
    QObject::connect(edit_condition, &QPushButton::clicked, this, [this]() { edit_selected_comparator(); });
    QObject::connect(remove_condition, &QPushButton::clicked, this, [this]() { remove_selected_comparator(); });
    QObject::connect(comparators, &QListWidget::itemDoubleClicked, this, [this](QListWidgetItem *) {
        edit_selected_comparator();
    });
    QObject::connect(run_preview, &QPushButton::clicked, this, [this]() { this->run_preview(); });
    QObject::connect(save, &QPushButton::clicked, this, [this]() { this->save(); });
    QObject::connect(cancel, &QPushButton::clicked, this, &QDialog::close);
    QObject::connect(operation, &QComboBox::currentIndexChanged, this, [this](int) {
        max_pending->setEnabled(operation->currentData().toString() != QLatin1String("fully automatic"));
    });

    max_pending->setEnabled(operation->currentData().toString() != QLatin1String("fully automatic"));
    refresh_comparators();

    if (qEnvironmentVariableIsSet("HYDRUS_QT_DUPLICATES_RULE_PREVIEW")) {
        tabs->setCurrentIndex(3);
        QTimer::singleShot(250, this, [this]() { this->run_preview(); });
    }

    const QString tab = qEnvironmentVariable("HYDRUS_QT_DUPLICATES_RULE_TAB");
    if (tab == QStringLiteral("comparison")) {
        tabs->setCurrentIndex(1);
    } else if (tab == QStringLiteral("action")) {
        tabs->setCurrentIndex(2);
    } else if (tab == QStringLiteral("preview")) {
        tabs->setCurrentIndex(3);
    }
}

void HydrusAutoResolutionRuleDialog::refresh_comparators() {
    comparators->clear();

    for (const QJsonValue &value : selector) {
        comparators->addItem(comparator_pretty(value.toObject()));
    }
}

void HydrusAutoResolutionRuleDialog::add_comparator() {
    QJsonObject initial;
    initial.insert(QStringLiteral("kind"), QStringLiteral("one_file"));
    initial.insert(QStringLiteral("looking_at"), QStringLiteral("a"));
    initial.insert(QStringLiteral("property"), QStringLiteral("size"));
    initial.insert(QStringLiteral("operator"), QStringLiteral(">"));
    initial.insert(QStringLiteral("value"), 0.0);

    const QJsonObject result = edit_condition_dialog(this, initial, catalog);

    if (!result.isEmpty()) {
        selector.append(result);
        refresh_comparators();
    }
}

void HydrusAutoResolutionRuleDialog::add_group() {
    const QJsonObject result = edit_group_dialog(this, QJsonObject(), catalog);

    if (!result.isEmpty()) {
        selector.append(result);
        refresh_comparators();
    }
}

void HydrusAutoResolutionRuleDialog::edit_selected_comparator() {
    const int row = comparators->currentRow();

    if (row < 0 || row >= selector.size()) {
        return;
    }

    const QJsonObject current = selector.at(row).toObject();
    const QJsonObject result =
        current.value(QStringLiteral("kind")).toString() == QLatin1String("group")
            ? edit_group_dialog(this, current, catalog)
            : edit_condition_dialog(this, current, catalog);

    if (!result.isEmpty()) {
        selector.replace(row, result);
        refresh_comparators();
    }
}

void HydrusAutoResolutionRuleDialog::remove_selected_comparator() {
    const int row = comparators->currentRow();

    if (row >= 0 && row < selector.size()) {
        selector.removeAt(row);
        refresh_comparators();
    }
}

QJsonObject HydrusAutoResolutionRuleDialog::rule_json() const {
    QJsonObject result;
    result.insert(QStringLiteral("name"), name->text().trimmed());
    result.insert(QStringLiteral("paused"), paused->isChecked());
    result.insert(QStringLiteral("operation_mode"), operation->currentData().toString());
    result.insert(QStringLiteral("max_pending_pairs"), max_pending->value());
    result.insert(QStringLiteral("search_query"), search_query->text().trimmed());
    result.insert(QStringLiteral("search_mode"), search_mode->currentData().toString());
    result.insert(QStringLiteral("max_distance"), max_distance->value());
    result.insert(QStringLiteral("pair_selector"), selector);
    result.insert(QStringLiteral("action"), action->currentData().toString());
    result.insert(QStringLiteral("delete_a"), delete_a->isChecked());
    result.insert(QStringLiteral("delete_b"), delete_b->isChecked());

    return result;
}

void HydrusAutoResolutionRuleDialog::run_preview() {
    if (name->text().trimmed().isEmpty()) {
        QMessageBox::warning(this, QStringLiteral("auto-resolution"), QStringLiteral("Give the rule a name first."));
        return;
    }

    preview_label->setText(QStringLiteral("testing..."));

    QJsonObject payload = rule_json();
    payload.insert(QStringLiteral("original_name"), original_name);
    main->notify(QStringLiteral("duplicates_rule_preview"), QString(), json_payload(payload));
}

void HydrusAutoResolutionRuleDialog::set_preview(const QJsonObject &preview) {
    const QString status = preview.value(QStringLiteral("status")).toString();

    if (status.isEmpty()) {
        preview_label->setText(QStringLiteral("no preview result"));
        return;
    }

    QStringList lines;
    lines.append(status);

    const QString selector = preview.value(QStringLiteral("selector")).toString();
    if (!selector.isEmpty()) {
        lines.append(QStringLiteral("selector: %1").arg(selector));
    }

    const QString search = preview.value(QStringLiteral("search")).toString();
    if (!search.isEmpty()) {
        lines.append(QStringLiteral("search: %1").arg(search));
    }

    for (const QJsonValue &value : preview.value(QStringLiteral("samples")).toArray()) {
        const QJsonObject sample = value.toObject();
        lines.append(QStringLiteral("%1  A=%2  B=%3  -> %4")
                         .arg(sample.value(QStringLiteral("distance")).toInt())
                         .arg(sample.value(QStringLiteral("a")).toString().left(16))
                         .arg(sample.value(QStringLiteral("b")).toString().left(16))
                         .arg(sample.value(QStringLiteral("decision")).toString()));
    }

    preview_label->setText(lines.join(QLatin1Char('\n')));
}

void HydrusAutoResolutionRuleDialog::save() {
    if (name->text().trimmed().isEmpty()) {
        QMessageBox::warning(this, QStringLiteral("auto-resolution"), QStringLiteral("Give the rule a name first."));
        return;
    }

    QJsonObject payload = rule_json();
    payload.insert(QStringLiteral("original_name"), original_name);
    main->notify(QStringLiteral("duplicates_rule_save"), QString(), json_payload(payload));
    close();
}

HydrusDuplicatesRulesDialog::HydrusDuplicatesRulesDialog(HydrusMainWindow *main_window, const QJsonObject &state, QWidget *parent)
    : QDialog(parent), main(main_window) {
    setWindowTitle(QStringLiteral("duplicate auto-resolution"));
    setObjectName(QStringLiteral("hydrus_duplicates_rules_dialog"));
    setAttribute(Qt::WA_DeleteOnClose, true);
    resize(980, 540);

    auto *layout = new QVBoxLayout(this);

    enabled = new QCheckBox(QStringLiteral("apply these rules automatically when a duplicates page refreshes"), this);
    layout->addWidget(enabled);

    auto *help = new QLabel(
        QStringLiteral("Rules are tested in alphabetical order; the first matching rule decides the pair. "
                       "Semi-automatic rules queue matches for review. Fully automatic rules apply immediately. "
                       "Run a preview before trusting a new rule."),
        this);
    help->setWordWrap(true);
    layout->addWidget(help);

    table = new QTableWidget(this);
    table->setColumnCount(5);
    table->setHorizontalHeaderLabels({
        QStringLiteral("name"),
        QStringLiteral("operation"),
        QStringLiteral("search"),
        QStringLiteral("comparison"),
        QStringLiteral("action")
    });
    table->horizontalHeader()->setSectionResizeMode(0, QHeaderView::ResizeToContents);
    table->horizontalHeader()->setSectionResizeMode(1, QHeaderView::ResizeToContents);
    table->horizontalHeader()->setSectionResizeMode(2, QHeaderView::ResizeToContents);
    table->horizontalHeader()->setSectionResizeMode(3, QHeaderView::Stretch);
    table->horizontalHeader()->setSectionResizeMode(4, QHeaderView::ResizeToContents);
    table->setSelectionBehavior(QAbstractItemView::SelectRows);
    table->setSelectionMode(QAbstractItemView::SingleSelection);
    table->setEditTriggers(QAbstractItemView::NoEditTriggers);
    table->verticalHeader()->setVisible(false);
    layout->addWidget(table, 1);

    auto *buttons = new QHBoxLayout();
    auto *add = new QPushButton(QStringLiteral("add rule..."), this);
    auto *edit = new QPushButton(QStringLiteral("edit..."), this);
    auto *duplicate = new QPushButton(QStringLiteral("duplicate"), this);
    auto *remove = new QPushButton(QStringLiteral("remove"), this);
    auto *review = new QPushButton(QStringLiteral("review pending..."), this);
    auto *close = new QPushButton(QStringLiteral("close"), this);
    buttons->addWidget(add);
    buttons->addWidget(edit);
    buttons->addWidget(duplicate);
    buttons->addWidget(remove);
    buttons->addWidget(review);
    buttons->addStretch(1);
    buttons->addWidget(close);
    layout->addLayout(buttons);

    QObject::connect(enabled, &QCheckBox::clicked, this, [this](bool) {
        main->notify(QStringLiteral("duplicates_auto_toggle"));
    });
    QObject::connect(add, &QPushButton::clicked, this, [this]() { add_rule(); });
    QObject::connect(edit, &QPushButton::clicked, this, [this]() { edit_selected(); });
    QObject::connect(duplicate, &QPushButton::clicked, this, [this]() { duplicate_selected(); });
    QObject::connect(remove, &QPushButton::clicked, this, [this]() { remove_selected(); });
    QObject::connect(review, &QPushButton::clicked, this, [this]() { open_review(); });
    QObject::connect(close, &QPushButton::clicked, this, &QDialog::close);
    QObject::connect(table, &QTableWidget::cellDoubleClicked, this, [this](int, int) { edit_selected(); });

    set_state(state);

    const QString edit_key = qEnvironmentVariable("HYDRUS_QT_DUPLICATES_RULE_EDIT");

    if (!edit_key.isEmpty()) {
        const QString key = edit_key;
        QTimer::singleShot(150, this, [this, key]() {
            if (key == QStringLiteral("new")) {
                add_rule();
                return;
            }

            for (int row = 0; row < table->rowCount(); ++row) {
                if (table->item(row, 0) != nullptr
                    && table->item(row, 0)->text().compare(key, Qt::CaseInsensitive) == 0) {
                    table->selectRow(row);
                    edit_selected();
                    break;
                }
            }
        });
    }
}

void HydrusDuplicatesRulesDialog::set_state(const QJsonObject &state) {
    data = state;

    const QJsonArray rules = state.value(QStringLiteral("rules")).toArray();
    const int previous_row = table->currentRow();
    table->setRowCount(rules.size());

    for (int index = 0; index < rules.size(); ++index) {
        const QJsonObject rule = rules.at(index).toObject();

        for (int column = 0; column < 5; ++column) {
            if (table->item(index, column) == nullptr) {
                table->setItem(index, column, new QTableWidgetItem());
            }
        }

        QString operation_label = rule.value(QStringLiteral("operation_label")).toString();
        if (rule.value(QStringLiteral("paused")).toBool()) {
            operation_label += QStringLiteral(" - paused");
        }

        table->item(index, 0)->setText(rule.value(QStringLiteral("name")).toString());
        table->item(index, 0)->setData(
            Qt::UserRole,
            QString::fromUtf8(QJsonDocument(rule).toJson(QJsonDocument::Compact)));
        table->item(index, 1)->setText(operation_label);
        table->item(index, 2)->setText(rule.value(QStringLiteral("search_summary")).toString());
        table->item(index, 3)->setText(rule.value(QStringLiteral("selector_summary")).toString());
        table->item(index, 4)->setText(rule.value(QStringLiteral("action_summary")).toString());
    }

    if (previous_row >= 0 && previous_row < table->rowCount()) {
        table->selectRow(previous_row);
    }

    QSignalBlocker blocker(enabled);
    enabled->setChecked(state.value(QStringLiteral("enabled")).toBool());

    const int pending = state.value(QStringLiteral("pending")).toInt(0);
    setToolTip(pending > 0 ? QStringLiteral("%1 pair(s) are waiting for review").arg(pending) : QString());

    table->resizeRowsToContents();
}

void HydrusDuplicatesRulesDialog::set_preview(const QJsonObject &preview) {
    if (rule_editor != nullptr) {
        rule_editor->set_preview(preview);
    }
}

void HydrusDuplicatesRulesDialog::add_rule() {
    QJsonObject rule;
    rule.insert(QStringLiteral("name"), QStringLiteral("new rule"));
    rule.insert(QStringLiteral("paused"), false);
    rule.insert(QStringLiteral("operation_mode"), QStringLiteral("semi-automatic"));
    rule.insert(QStringLiteral("max_pending_pairs"), 500);
    rule.insert(QStringLiteral("search_query"), QString());
    rule.insert(QStringLiteral("search_mode"), QStringLiteral("both"));
    rule.insert(QStringLiteral("max_distance"), 0);
    rule.insert(QStringLiteral("action"), QStringLiteral("better"));
    rule.insert(QStringLiteral("delete_a"), false);
    rule.insert(QStringLiteral("delete_b"), false);

    QJsonObject relative;
    relative.insert(QStringLiteral("kind"), QStringLiteral("relative"));
    relative.insert(QStringLiteral("property"), QStringLiteral("num_pixels"));
    relative.insert(QStringLiteral("operator"), QStringLiteral(">"));
    relative.insert(QStringLiteral("multiplier"), 1.0);
    relative.insert(QStringLiteral("delta"), 0.0);

    QJsonArray selector;
    selector.append(relative);
    rule.insert(QStringLiteral("pair_selector"), selector);

    edit_rule(rule);
}

void HydrusDuplicatesRulesDialog::edit_selected() {
    const int row = table->currentRow();

    if (row < 0 || table->item(row, 0) == nullptr) {
        return;
    }

    QJsonParseError error{};
    const QJsonDocument document = QJsonDocument::fromJson(
        table->item(row, 0)->data(Qt::UserRole).toString().toUtf8(), &error);

    if (error.error == QJsonParseError::NoError && document.isObject()) {
        edit_rule(document.object());
    }
}

void HydrusDuplicatesRulesDialog::duplicate_selected() {
    const int row = table->currentRow();

    if (row < 0 || table->item(row, 0) == nullptr) {
        return;
    }

    QJsonParseError error{};
    const QJsonDocument document = QJsonDocument::fromJson(
        table->item(row, 0)->data(Qt::UserRole).toString().toUtf8(), &error);

    if (error.error != QJsonParseError::NoError || !document.isObject()) {
        return;
    }

    QJsonObject copy = document.object();
    copy.insert(QStringLiteral("name"), copy.value(QStringLiteral("name")).toString() + QStringLiteral(" copy"));
    copy.insert(QStringLiteral("paused"), true);
    edit_rule(copy);
}

void HydrusDuplicatesRulesDialog::remove_selected() {
    const int row = table->currentRow();

    if (row < 0 || table->item(row, 0) == nullptr) {
        return;
    }

    const QString name = table->item(row, 0)->text();

    if (QMessageBox::question(this, QStringLiteral("delete auto-resolution rule"),
                              QStringLiteral("Delete rule \"%1\" and its pending review pairs?").arg(name))
        != QMessageBox::Yes) {
        return;
    }

    main->notify(QStringLiteral("duplicates_rule_remove"), QString(), name);
}

void HydrusDuplicatesRulesDialog::open_review() {
    main->notify(QStringLiteral("duplicates_review"));
}

void HydrusDuplicatesRulesDialog::edit_rule(const QJsonObject &rule) {
    if (rule_editor != nullptr) {
        rule_editor->close();
        rule_editor = nullptr;
    }

    rule_editor = new HydrusAutoResolutionRuleDialog(main, this, rule, data, this);
    QObject::connect(rule_editor, &QObject::destroyed, this, [this]() { rule_editor = nullptr; });
    rule_editor->show();
    rule_editor->raise();
    rule_editor->activateWindow();
}

HydrusDuplicateReviewDialog::HydrusDuplicateReviewDialog(HydrusMainWindow *main_window, const QJsonObject &state, QWidget *parent)
    : QDialog(parent), main(main_window) {
    setWindowTitle(QStringLiteral("duplicate auto-resolution review"));
    setObjectName(QStringLiteral("hydrus_duplicate_review_dialog"));
    setAttribute(Qt::WA_DeleteOnClose, true);
    resize(900, 520);

    auto *layout = new QVBoxLayout(this);

    status = new QLabel(this);
    status->setWordWrap(true);
    layout->addWidget(status);

    tabs = new QTabWidget(this);

    auto make_table = [this](const QStringList &headers) {
        auto *table = new QTableWidget(this);
        table->setColumnCount(headers.size());
        table->setHorizontalHeaderLabels(headers);
        table->setSelectionBehavior(QAbstractItemView::SelectRows);
        table->setSelectionMode(QAbstractItemView::ExtendedSelection);
        table->setEditTriggers(QAbstractItemView::NoEditTriggers);
        table->verticalHeader()->setVisible(false);
        table->horizontalHeader()->setStretchLastSection(true);
        return table;
    };

    pending = make_table({
        QStringLiteral("rule"),
        QStringLiteral("distance"),
        QStringLiteral("A"),
        QStringLiteral("B"),
        QStringLiteral("decision")
    });
    denied = make_table({QStringLiteral("rule"), QStringLiteral("A"), QStringLiteral("B")});
    actioned = make_table({
        QStringLiteral("rule"),
        QStringLiteral("A"),
        QStringLiteral("B"),
        QStringLiteral("decision"),
        QStringLiteral("denied")
    });

    tabs->addTab(pending, QStringLiteral("pending"));
    tabs->addTab(denied, QStringLiteral("denied"));
    tabs->addTab(actioned, QStringLiteral("taken"));
    layout->addWidget(tabs, 1);

    auto *buttons = new QHBoxLayout();
    auto *approve_selected = new QPushButton(QStringLiteral("approve selected"), this);
    auto *deny_selected = new QPushButton(QStringLiteral("deny selected"), this);
    auto *approve_all = new QPushButton(QStringLiteral("approve all"), this);
    auto *deny_all = new QPushButton(QStringLiteral("deny all"), this);
    auto *close = new QPushButton(QStringLiteral("close"), this);
    buttons->addWidget(approve_selected);
    buttons->addWidget(deny_selected);
    buttons->addWidget(approve_all);
    buttons->addWidget(deny_all);
    buttons->addStretch(1);
    buttons->addWidget(close);
    layout->addLayout(buttons);

    QObject::connect(approve_selected, &QPushButton::clicked, this, [this]() { apply_selected(true); });
    QObject::connect(deny_selected, &QPushButton::clicked, this, [this]() { apply_selected(false); });
    QObject::connect(approve_all, &QPushButton::clicked, this, [this]() { apply_all(true); });
    QObject::connect(deny_all, &QPushButton::clicked, this, [this]() { apply_all(false); });
    QObject::connect(close, &QPushButton::clicked, this, &QDialog::close);

    set_state(state);
}

void HydrusDuplicateReviewDialog::set_state(const QJsonObject &state) {
    status->setText(state.value(QStringLiteral("status")).toString());

    const QJsonArray pending_rows = state.value(QStringLiteral("pending")).toArray();
    pending->setRowCount(pending_rows.size());

    for (int index = 0; index < pending_rows.size(); ++index) {
        const QJsonObject entry = pending_rows.at(index).toObject();

        for (int column = 0; column < 5; ++column) {
            if (pending->item(index, column) == nullptr) {
                pending->setItem(index, column, new QTableWidgetItem());
            }
        }

        pending->item(index, 0)->setText(entry.value(QStringLiteral("rule")).toString());
        pending->item(index, 1)->setText(QString::number(entry.value(QStringLiteral("distance")).toInt()));
        pending->item(index, 2)->setText(entry.value(QStringLiteral("a")).toString().left(16));
        pending->item(index, 2)->setToolTip(entry.value(QStringLiteral("a")).toString());
        pending->item(index, 3)->setText(entry.value(QStringLiteral("b")).toString().left(16));
        pending->item(index, 3)->setToolTip(entry.value(QStringLiteral("b")).toString());
        pending->item(index, 4)->setText(entry.value(QStringLiteral("decision_label")).toString());
    }

    const QJsonArray denied_rows = state.value(QStringLiteral("denied")).toArray();
    denied->setRowCount(denied_rows.size());

    for (int index = 0; index < denied_rows.size(); ++index) {
        const QJsonObject entry = denied_rows.at(index).toObject();

        for (int column = 0; column < 3; ++column) {
            if (denied->item(index, column) == nullptr) {
                denied->setItem(index, column, new QTableWidgetItem());
            }
        }

        denied->item(index, 0)->setText(entry.value(QStringLiteral("rule")).toString());
        denied->item(index, 1)->setText(entry.value(QStringLiteral("a")).toString().left(16));
        denied->item(index, 2)->setText(entry.value(QStringLiteral("b")).toString().left(16));
    }

    const QJsonArray actioned_rows = state.value(QStringLiteral("actioned")).toArray();
    actioned->setRowCount(actioned_rows.size());

    for (int index = 0; index < actioned_rows.size(); ++index) {
        const QJsonObject entry = actioned_rows.at(index).toObject();

        for (int column = 0; column < 5; ++column) {
            if (actioned->item(index, column) == nullptr) {
                actioned->setItem(index, column, new QTableWidgetItem());
            }
        }

        actioned->item(index, 0)->setText(entry.value(QStringLiteral("rule")).toString());
        actioned->item(index, 1)->setText(entry.value(QStringLiteral("a")).toString().left(16));
        actioned->item(index, 2)->setText(entry.value(QStringLiteral("b")).toString().left(16));
        actioned->item(index, 3)->setText(entry.value(QStringLiteral("decision_label")).toString());
        actioned->item(index, 4)->setText(entry.value(QStringLiteral("banned")).toBool() ? QStringLiteral("yes")
                                                                                          : QString());
    }

    pending->resizeRowsToContents();
    denied->resizeRowsToContents();
    actioned->resizeRowsToContents();
}

QList<int> HydrusDuplicateReviewDialog::selected_pending_rows() const {
    QList<int> rows;

    for (const QModelIndex &index : pending->selectionModel()->selectedRows()) {
        if (!rows.contains(index.row())) {
            rows.append(index.row());
        }
    }

    std::sort(rows.begin(), rows.end());

    return rows;
}

void HydrusDuplicateReviewDialog::apply_selected(bool approve) {
    const QList<int> rows = selected_pending_rows();

    if (rows.isEmpty()) {
        QMessageBox::information(this, QStringLiteral("duplicate review"),
                                 QStringLiteral("Select at least one pending pair first."));
        return;
    }

    QJsonArray indices;
    for (int row : rows) {
        indices.append(row);
    }

    QJsonObject payload;
    payload.insert(QStringLiteral("indices"), indices);
    payload.insert(QStringLiteral("approve"), approve);
    main->notify(QStringLiteral("duplicates_review_apply"), QString(), json_payload(payload));
}

void HydrusDuplicateReviewDialog::apply_all(bool approve) {
    QJsonObject payload;
    payload.insert(QStringLiteral("all"), true);
    payload.insert(QStringLiteral("approve"), approve);
    main->notify(QStringLiteral("duplicates_review_apply"), QString(), json_payload(payload));
}

HydrusDuplicateMergeOptionsDialog::HydrusDuplicateMergeOptionsDialog(
    HydrusMainWindow *main_window, const QJsonObject &state, QWidget *parent)
    : QDialog(parent), main(main_window) {
    setWindowTitle(QStringLiteral("duplicate merge options"));
    setObjectName(QStringLiteral("hydrus_duplicate_merge_options_dialog"));
    setAttribute(Qt::WA_DeleteOnClose, true);
    resize(760, 620);

    data = state;

    auto *layout = new QVBoxLayout(this);

    auto *help = new QLabel(
        QStringLiteral("Choose what a duplicate merge copies from the worse file. Services that are not "
                       "listed are left untouched. \"This is better\" keeps one file and can move or copy "
                       "metadata; \"same quality\" merges both files in the chosen direction."),
        this);
    help->setWordWrap(true);
    layout->addWidget(help);

    auto *tabs = new QTabWidget(this);
    build_page(tabs, QStringLiteral("better"), QStringLiteral("this is better"), better);
    build_page(tabs, QStringLiteral("same"), QStringLiteral("same quality"), same);
    layout->addWidget(tabs, 1);

    auto *buttons = new QHBoxLayout();
    auto *save_button = new QPushButton(QStringLiteral("save"), this);
    auto *close_button = new QPushButton(QStringLiteral("close"), this);
    buttons->addStretch(1);
    buttons->addWidget(save_button);
    buttons->addWidget(close_button);
    layout->addLayout(buttons);

    connect(save_button, &QPushButton::clicked, this, [this]() { save(); });
    connect(close_button, &QPushButton::clicked, this, &QDialog::close);

    set_state(state);

    if (qEnvironmentVariable("HYDRUS_QT_DUPLICATES_MERGE_TAB") == QStringLiteral("same")) {
        tabs->setCurrentIndex(1);
    }
}

void HydrusDuplicateMergeOptionsDialog::set_state(const QJsonObject &state) {
    data = state;

    fill_page(better, data.value(QStringLiteral("better")).toObject());
    fill_page(same, data.value(QStringLiteral("same")).toObject());
}

void HydrusDuplicateMergeOptionsDialog::build_page(
    QTabWidget *tabs, const QString &key, const QString &title, Page &page) {
    Q_UNUSED(key);

    auto *widget = new QWidget(tabs);
    auto *layout = new QVBoxLayout(widget);

    auto *form = new QFormLayout();

    page.archive = new QComboBox(widget);
    page.modified_date = new QComboBox(widget);
    page.urls = new QComboBox(widget);
    page.notes = new QComboBox(widget);

    for (const QJsonValue &action : data.value(QStringLiteral("archive_actions")).toArray()) {
        const QJsonObject entry = action.toObject();
        page.archive->addItem(entry.value(QStringLiteral("label")).toString(), entry.value(QStringLiteral("value")).toString());
    }

    for (QComboBox *combo : {page.modified_date, page.urls, page.notes}) {
        for (const QJsonValue &action : data.value(QStringLiteral("actions")).toArray()) {
            const QJsonObject entry = action.toObject();
            const QString value = entry.value(QStringLiteral("value")).toString();

            if (combo == page.modified_date && (value == QStringLiteral("move") || value == QStringLiteral("add_move"))) {
                continue;
            }

            combo->addItem(entry.value(QStringLiteral("label")).toString(), value);
        }
    }

    form->addRow(QStringLiteral("sync archived status:"), page.archive);
    form->addRow(QStringLiteral("sync file modified time:"), page.modified_date);
    form->addRow(QStringLiteral("sync known urls:"), page.urls);
    form->addRow(QStringLiteral("sync notes:"), page.notes);
    layout->addLayout(form);

    page.tags = new QTableWidget(widget);
    page.tags->setColumnCount(2);
    page.tags->setHorizontalHeaderLabels({QStringLiteral("tag service"), QStringLiteral("action")});
    page.tags->horizontalHeader()->setSectionResizeMode(0, QHeaderView::Stretch);
    page.tags->horizontalHeader()->setSectionResizeMode(1, QHeaderView::ResizeToContents);
    page.tags->verticalHeader()->setVisible(false);
    page.tags->setEditTriggers(QAbstractItemView::NoEditTriggers);
    page.tags->setSelectionMode(QAbstractItemView::NoSelection);

    page.ratings = new QTableWidget(widget);
    page.ratings->setColumnCount(2);
    page.ratings->setHorizontalHeaderLabels({QStringLiteral("rating service"), QStringLiteral("action")});
    page.ratings->horizontalHeader()->setSectionResizeMode(0, QHeaderView::Stretch);
    page.ratings->horizontalHeader()->setSectionResizeMode(1, QHeaderView::ResizeToContents);
    page.ratings->verticalHeader()->setVisible(false);
    page.ratings->setEditTriggers(QAbstractItemView::NoEditTriggers);
    page.ratings->setSelectionMode(QAbstractItemView::NoSelection);

    layout->addWidget(new QLabel(QStringLiteral("tag services"), widget));
    layout->addWidget(page.tags, 1);
    layout->addWidget(new QLabel(QStringLiteral("rating services"), widget));
    layout->addWidget(page.ratings, 1);

    tabs->addTab(widget, title);
}

void HydrusDuplicateMergeOptionsDialog::fill_page(Page &page, const QJsonObject &options) {
    auto select_value = [](QComboBox *combo, const QString &value) {
        const int index = combo->findData(value);

        if (index >= 0) {
            combo->setCurrentIndex(index);
        }
    };

    select_value(page.archive, options.value(QStringLiteral("sync_archive_action")).toString());
    select_value(page.modified_date, options.value(QStringLiteral("sync_file_modified_date_action")).toString());
    select_value(page.urls, options.value(QStringLiteral("sync_urls_action")).toString());
    select_value(page.notes, options.value(QStringLiteral("sync_notes_action")).toString());

    auto fill_table = [this](QTableWidget *table, const QJsonArray &services, const QJsonObject &actions) {
        table->setRowCount(0);

        for (const QJsonValue &service_value : services) {
            const QString service = service_value.toString();
            const int row = table->rowCount();
            table->insertRow(row);
            table->setItem(row, 0, new QTableWidgetItem(service));

            auto *combo = new QComboBox(table);

            for (const QJsonValue &action : data.value(QStringLiteral("actions")).toArray()) {
                const QJsonObject entry = action.toObject();
                combo->addItem(entry.value(QStringLiteral("label")).toString(), entry.value(QStringLiteral("value")).toString());
            }

            const int index = combo->findData(actions.value(service).toString());

            if (index >= 0) {
                combo->setCurrentIndex(index);
            } else {
                const int none_index = combo->findData(QStringLiteral("none"));

                if (none_index >= 0) {
                    combo->setCurrentIndex(none_index);
                }
            }

            table->setCellWidget(row, 1, combo);
        }
    };

    fill_table(
        page.tags,
        data.value(QStringLiteral("tag_services")).toArray(),
        options.value(QStringLiteral("tag_service_actions")).toObject());
    fill_table(
        page.ratings,
        data.value(QStringLiteral("rating_services")).toArray(),
        options.value(QStringLiteral("rating_service_actions")).toObject());
}

void HydrusDuplicateMergeOptionsDialog::save() {
    auto read_table = [](QTableWidget *table) {
        QJsonObject values;

        for (int row = 0; row < table->rowCount(); ++row) {
            const QTableWidgetItem *service = table->item(row, 0);
            auto *combo = qobject_cast<QComboBox *>(table->cellWidget(row, 1));

            if (service == nullptr || combo == nullptr) {
                continue;
            }

            const QString action = combo->currentData().toString();

            if (action != QStringLiteral("none")) {
                values.insert(service->text(), action);
            }
        }

        return values;
    };

    auto page_json = [&](const Page &page) {
        QJsonObject options;
        options.insert(QStringLiteral("sync_archive_action"), page.archive->currentData().toString());
        options.insert(QStringLiteral("sync_file_modified_date_action"), page.modified_date->currentData().toString());
        options.insert(QStringLiteral("sync_urls_action"), page.urls->currentData().toString());
        options.insert(QStringLiteral("sync_notes_action"), page.notes->currentData().toString());
        options.insert(QStringLiteral("tag_service_actions"), read_table(page.tags));
        options.insert(QStringLiteral("rating_service_actions"), read_table(page.ratings));

        return options;
    };

    for (const auto &[key, page] : std::initializer_list<std::pair<QString, const Page *>>{
             {QStringLiteral("better"), &better}, {QStringLiteral("same"), &same}}) {
        QJsonObject payload;
        payload.insert(QStringLiteral("action"), key);
        payload.insert(QStringLiteral("options"), page_json(*page));
        main->notify(QStringLiteral("duplicates_merge_options"), QString(), json_payload(payload));
    }

    close();
}

void HydrusMainWindow::show_duplicates_rules(const QJsonObject &state) {
    if (duplicates_rules_dialog == nullptr) {
        duplicates_rules_dialog = new HydrusDuplicatesRulesDialog(this, state, this);
        connect(duplicates_rules_dialog, &QObject::destroyed, this, [this]() { duplicates_rules_dialog = nullptr; });
    } else {
        duplicates_rules_dialog->set_state(state);
        duplicates_rules_dialog->raise();
        duplicates_rules_dialog->activateWindow();
    }

    duplicates_rules_dialog->show();
}

void HydrusMainWindow::show_duplicate_merge_options(const QJsonObject &state) {
    if (merge_options_dialog == nullptr) {
        merge_options_dialog = new HydrusDuplicateMergeOptionsDialog(this, state, this);
        connect(merge_options_dialog, &QObject::destroyed, this, [this]() { merge_options_dialog = nullptr; });
    } else {
        merge_options_dialog->set_state(state);
        merge_options_dialog->raise();
        merge_options_dialog->activateWindow();
    }

    merge_options_dialog->show();
}

void HydrusMainWindow::update_auto_resolution_preview(const QJsonObject &state) {
    if (duplicates_rules_dialog != nullptr) {
        duplicates_rules_dialog->set_preview(state);
    }
}

void HydrusMainWindow::show_duplicate_review(const QJsonObject &state) {
    if (duplicate_review_dialog == nullptr) {
        duplicate_review_dialog = new HydrusDuplicateReviewDialog(this, state, this);
        connect(duplicate_review_dialog, &QObject::destroyed, this, [this]() { duplicate_review_dialog = nullptr; });
        duplicate_review_dialog->show();
    } else {
        duplicate_review_dialog->set_state(state);
        duplicate_review_dialog->raise();
        duplicate_review_dialog->activateWindow();
    }
}

void HydrusMainWindow::update_duplicate_review(const QJsonObject &state) {
    if (duplicate_review_dialog != nullptr) {
        duplicate_review_dialog->set_state(state);
    }
}

// ---------------------------------------------------------------------------
// HydrusMainWindow
// ---------------------------------------------------------------------------


HydrusMainWindow::HydrusMainWindow(QWidget *parent) : QMainWindow(parent) {
    active_notebook = QStringLiteral("root");
    setAcceptDrops(true);
}

void HydrusMainWindow::dragEnterEvent(QDragEnterEvent *event) {
    if (event->mimeData()->hasUrls()) {
        event->acceptProposedAction();
    }
}

void HydrusMainWindow::dropEvent(QDropEvent *event) {
    QStringList paths;

    for (const QUrl &url : event->mimeData()->urls()) {
        if (url.isLocalFile()) {
            paths.append(url.toLocalFile());
        }
    }

    if (paths.isEmpty()) {
        return;
    }

    QJsonArray array;

    for (const QString &path : paths) {
        array.append(path);
    }

    notify(QStringLiteral("import_paths"), QString(), QString::fromUtf8(QJsonDocument(array).toJson(QJsonDocument::Compact)));
    event->acceptProposedAction();
}

void HydrusMainWindow::closeEvent(QCloseEvent *event) {
    if (tick_timer != nullptr) {
        tick_timer->stop();
    }

    QSettings settings;
    settings.setValue(QStringLiteral("main/geometry"), saveGeometry());
    settings.setValue(QStringLiteral("main/state"), saveState());
    settings.sync();

    if (tray != nullptr) {
        tray->hide();
    }

    if (services_dialog != nullptr) {
        services_dialog->close();
    }

    if (network_dialog != nullptr) {
        network_dialog->close();
    }

    QMainWindow::closeEvent(event);
}

void HydrusMainWindow::notify(const QString &action, const QString &target, const QString &payload) {
    QByteArray action_bytes = action.toUtf8();
    QByteArray target_bytes = target.toUtf8();
    QByteArray payload_bytes = payload.toUtf8();

    app_action(
        *this,
        rust::Str(action_bytes.constData(), action_bytes.size()),
        rust::Str(target_bytes.constData(), target_bytes.size()),
        rust::Str(payload_bytes.constData(), payload_bytes.size()));
}

void HydrusMainWindow::build_shell() {
    if (root != nullptr) {
        return;
    }

    setStyleSheet(QStringLiteral(
        "QTabWidget::pane { border: 1px solid #c8c8c8; }"
        "QTabBar::tab { padding: 4px 14px; }"
        "QStatusBar QLabel { padding: 0px 8px; border-left: 1px solid #b0b0b0; }"
        "QSplitter::handle { background: #c8c8c8; }"));

    root = new HydrusNotebook(this, QStringLiteral("root"), this);
    setCentralWidget(root);

    status_message = new QLabel(QStringLiteral("ready"), this);
    status_media = new QLabel(this);
    status_idle = new QLabel(this);
    status_db = new QLabel(this);

    statusBar()->addWidget(status_message, 1);
    statusBar()->addPermanentWidget(status_media);
    statusBar()->addPermanentWidget(status_idle);
    statusBar()->addPermanentWidget(status_db);

    if (QSystemTrayIcon::isSystemTrayAvailable()) {
        tray = new QSystemTrayIcon(style()->standardIcon(QStyle::SP_ComputerIcon), this);
        tray->setToolTip(QStringLiteral("Hydrus Client"));

        auto *tray_menu = new QMenu(this);
        auto *show_action = tray_menu->addAction(QStringLiteral("show/hide"));
        auto *tray_exit = tray_menu->addAction(QStringLiteral("exit"));

        connect(show_action, &QAction::triggered, this, [this]() {
            if (isVisible()) {
                hide();
            } else {
                show();
                raise();
                activateWindow();
            }
        });
        connect(tray_exit, &QAction::triggered, this, [this]() { notify(QStringLiteral("exit")); });
        connect(tray, &QSystemTrayIcon::activated, this, [this](QSystemTrayIcon::ActivationReason reason) {
            if (reason == QSystemTrayIcon::Trigger) {
                if (isVisible()) {
                    hide();
                } else {
                    show();
                    raise();
                    activateWindow();
                }
            }
        });

        tray->setContextMenu(tray_menu);
        tray->show();
    }

    build_shortcuts();

    resize(1280, 800);
    setWindowTitle(QStringLiteral("Hydrus Client"));

    QSettings settings;
    const QByteArray geometry = settings.value(QStringLiteral("main/geometry")).toByteArray();
    const QByteArray state = settings.value(QStringLiteral("main/state")).toByteArray();

    if (!geometry.isEmpty()) {
        restoreGeometry(geometry);
    }

    if (!state.isEmpty()) {
        restoreState(state);
    }
}

void show_command_palette(HydrusMainWindow *main);

void HydrusMainWindow::build_shortcuts() {
    auto *new_page = new QShortcut(QKeySequence(QStringLiteral("Ctrl+T")), this);
    connect(new_page, &QShortcut::activated, this, [this]() {
        show_page_chooser(active_notebook.isEmpty() ? QStringLiteral("root") : active_notebook);
    });

    auto *close_page = new QShortcut(QKeySequence(QStringLiteral("Ctrl+W")), this);
    connect(close_page, &QShortcut::activated, this, [this]() {
        if (!active_page.isEmpty()) {
            notify(QStringLiteral("close_page"), active_page);
        }
    });

    auto *refresh = new QShortcut(QKeySequence(QStringLiteral("Ctrl+R")), this);
    connect(refresh, &QShortcut::activated, this, [this]() {
        if (!active_page.isEmpty()) {
            notify(QStringLiteral("refresh_page"), active_page);
        }
    });

    auto *undo = new QShortcut(QKeySequence(QStringLiteral("Ctrl+Z")), this);
    connect(undo, &QShortcut::activated, this, [this]() { notify(QStringLiteral("undo")); });

    auto *back = new QShortcut(QKeySequence(QStringLiteral("Alt+Left")), this);
    connect(back, &QShortcut::activated, this, [this]() { notify(QStringLiteral("history_back")); });

    auto *forward = new QShortcut(QKeySequence(QStringLiteral("Alt+Right")), this);
    connect(forward, &QShortcut::activated, this, [this]() { notify(QStringLiteral("history_forward")); });

    auto *page_manager = new QShortcut(QKeySequence(QStringLiteral("Ctrl+Shift+P")), this);
    connect(page_manager, &QShortcut::activated, this, [this]() { notify(QStringLiteral("page_manager")); });

    auto *palette = new QShortcut(QKeySequence(QStringLiteral("Ctrl+K")), this);
    connect(palette, &QShortcut::activated, this, [this]() {
        show_command_palette(this);
    });

    auto *duplicate = new QShortcut(QKeySequence(QStringLiteral("Ctrl+Shift+T")), this);
    connect(duplicate, &QShortcut::activated, this, [this]() {
        if (!active_page.isEmpty()) {
            notify(QStringLiteral("duplicate_page"), active_page);
        }
    });

    auto *import = new QShortcut(QKeySequence(QStringLiteral("Ctrl+I")), this);
    connect(import, &QShortcut::activated, this, [this]() { notify(QStringLiteral("import_files")); });

    auto *import_url = new QShortcut(QKeySequence(QStringLiteral("Ctrl+U")), this);
    connect(import_url, &QShortcut::activated, this, [this]() { notify(QStringLiteral("import_url")); });

    auto *import_folders_shortcut = new QShortcut(QKeySequence(QStringLiteral("Ctrl+Shift+I")), this);
    connect(import_folders_shortcut, &QShortcut::activated, this, [this]() {
        notify(QStringLiteral("new_import_folders"));
    });

    auto *options = new QShortcut(QKeySequence(QStringLiteral("Ctrl+P")), this);
    connect(options, &QShortcut::activated, this, [this]() { notify(QStringLiteral("options")); });

    auto *manage_tags = new QShortcut(QKeySequence(QStringLiteral("Ctrl+M")), this);
    manage_tags->setContext(Qt::ApplicationShortcut);
    connect(manage_tags, &QShortcut::activated, this, [this]() {
        notify(QStringLiteral("manage_tags_active"));
    });

    auto *archive = new QShortcut(QKeySequence(QStringLiteral("F7")), this);
    connect(archive, &QShortcut::activated, this, [this]() { notify(QStringLiteral("archive_active")); });

    auto *inbox = new QShortcut(QKeySequence(QStringLiteral("F8")), this);
    connect(inbox, &QShortcut::activated, this, [this]() { notify(QStringLiteral("inbox_active")); });

    auto *refresh_f5 = new QShortcut(QKeySequence(QStringLiteral("F5")), this);
    connect(refresh_f5, &QShortcut::activated, this, [this]() {
        if (!active_page.isEmpty()) {
            notify(QStringLiteral("refresh_page"), active_page);
        }
    });

    auto *quit = new QShortcut(QKeySequence(QStringLiteral("Ctrl+Q")), this);
    connect(quit, &QShortcut::activated, this, [this]() { notify(QStringLiteral("exit")); });
}

void HydrusMainWindow::set_status(const QJsonObject &status) {
    status_message->setText(status.value(QStringLiteral("message")).toString());
    status_media->setText(status.value(QStringLiteral("media")).toString());
    status_idle->setText(status.value(QStringLiteral("idle")).toString());
    status_db->setText(status.value(QStringLiteral("db")).toString());
}

void HydrusMainWindow::set_menus(const QJsonArray &menus) {
    menuBar()->clear();

    for (const QJsonValue &value : menus) {
        const QJsonObject menu_object = value.toObject();
        auto *menu = menuBar()->addMenu(menu_object.value(QStringLiteral("title")).toString());
        menu->setToolTipsVisible(true);

        std::function<void(QMenu *, const QJsonArray &)> add_items;

        add_items = [this, &add_items](QMenu *menu_target, const QJsonArray &items) {
            for (const QJsonValue &item_value : items) {
                const QJsonObject item = item_value.toObject();

                if (item.value(QStringLiteral("separator")).toBool()) {
                    menu_target->addSeparator();
                    continue;
                }

                const QJsonArray children = item.value(QStringLiteral("items")).toArray();

                if (!children.isEmpty()) {
                    auto *submenu = menu_target->addMenu(item.value(QStringLiteral("label")).toString());
                    submenu->setToolTipsVisible(true);
                    add_items(submenu, children);
                    continue;
                }

                const QString label = item.value(QStringLiteral("label")).toString();
                const QString action_id = item.value(QStringLiteral("action")).toString();
                const QString item_target = item.value(QStringLiteral("target")).toString();
                const QString payload = item.value(QStringLiteral("payload")).toString();

                if (label.isEmpty()) {
                    continue;
                }

                auto *action = menu_target->addAction(label);
                action->setEnabled(item.value(QStringLiteral("enabled")).toBool(true));

                if (item.value(QStringLiteral("checkable")).toBool(false)) {
                    action->setCheckable(true);
                    action->setChecked(item.value(QStringLiteral("checked")).toBool(false));
                }

                const QString tooltip = item.value(QStringLiteral("description")).toString();

                if (!tooltip.isEmpty()) {
                    action->setToolTip(tooltip);
                }

                connect(action, &QAction::triggered, this, [this, action_id, item_target, payload]() {
                    if (!action_id.isEmpty()) {
                        notify(action_id, item_target, payload);
                    }
                });
            }
        };

        add_items(menu, menu_object.value(QStringLiteral("items")).toArray());
    }
}

void HydrusMainWindow::clear_notebook(HydrusNotebook *notebook) {
    notebook->clear_pages();
}

void HydrusMainWindow::set_tag_suggestions(const QStringList &suggestions) {
    tag_suggestions = suggestions;

    for (HydrusQueryPage *page : pages) {
        page->set_suggestions(tag_suggestions);
    }
}

const QStringList &HydrusMainWindow::suggestions() const {
    return tag_suggestions;
}

void HydrusMainWindow::show_autocomplete(const QJsonObject &state) {
    const QString page_id = state.value(QStringLiteral("page_id")).toString();
    HydrusQueryPage *page = pages.value(page_id, nullptr);

    if (page != nullptr) {
        page->show_autocomplete(state);
    }
}

void HydrusMainWindow::build_pages(const QJsonObject &object) {
    if (root == nullptr) {
        return;
    }

    building = true;

    pages.clear();
    downloaders.clear();
    subscriptions.clear();
    duplicates.clear();
    import_folders.clear();
    export_folders.clear();
    notebooks.clear();
    notebooks.insert(QStringLiteral("root"), root);

    clear_notebook(root);

    const QJsonObject root_object = object.value(QStringLiteral("root")).toObject();
    populate_notebook(root, root_object);

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

void HydrusMainWindow::populate_notebook(HydrusNotebook *notebook, const QJsonObject &object) {
    const QJsonArray children = object.value(QStringLiteral("children")).toArray();

    for (const QJsonValue &value : children) {
        const QJsonObject child = value.toObject();
        const QString kind = child.value(QStringLiteral("kind")).toString();
        const QString page_id = child.value(QStringLiteral("id")).toString();
        const QString title = child.value(QStringLiteral("title")).toString();

        if (kind == QStringLiteral("notebook")) {
            auto *nested = new HydrusNotebook(this, page_id, notebook);
            notebooks.insert(page_id, nested);
            notebook->add_page_widget(page_id, title.isEmpty() ? QStringLiteral("page of pages") : title, nested);
            populate_notebook(nested, child);
        } else if (kind == QStringLiteral("query")) {
            auto *page = new HydrusQueryPage(this, page_id, child.value(QStringLiteral("state")).toObject());
            pages.insert(page_id, page);
            notebook->add_page_widget(page_id, title.isEmpty() ? QStringLiteral("files") : title, page);
        } else if (kind == QStringLiteral("downloader")) {
            auto *page = new HydrusDownloaderPage(this, page_id, child.value(QStringLiteral("state")).toObject());
            downloaders.insert(page_id, page);
            notebook->add_page_widget(page_id, title.isEmpty() ? QStringLiteral("download") : title, page);
        } else if (kind == QStringLiteral("subscriptions")) {
            auto *page = new HydrusSubscriptionsPage(this, page_id, child.value(QStringLiteral("state")).toObject());
            subscriptions.insert(page_id, page);
            notebook->add_page_widget(page_id, title.isEmpty() ? QStringLiteral("subscriptions") : title, page);
        } else if (kind == QStringLiteral("duplicates")) {
            auto *page = new HydrusDuplicatesPage(this, page_id, child.value(QStringLiteral("state")).toObject());
            duplicates.insert(page_id, page);
            notebook->add_page_widget(page_id, title.isEmpty() ? QStringLiteral("duplicates") : title, page);
        } else if (kind == QStringLiteral("import_folders")) {
            auto *page = new HydrusImportFoldersPage(this, page_id, child.value(QStringLiteral("state")).toObject());
            import_folders.insert(page_id, page);
            notebook->add_page_widget(page_id, title.isEmpty() ? QStringLiteral("import folders") : title, page);
        } else if (kind == QStringLiteral("export_folders")) {
            auto *page = new HydrusExportFoldersPage(this, page_id, child.value(QStringLiteral("state")).toObject());
            export_folders.insert(page_id, page);
            notebook->add_page_widget(page_id, title.isEmpty() ? QStringLiteral("export folders") : title, page);
        } else {
            const QString message = child.value(QStringLiteral("state")).toObject().value(QStringLiteral("message")).toString();
            auto *placeholder = new QWidget(notebook);
            auto *layout = new QVBoxLayout(placeholder);
            auto *label = new QLabel(message.isEmpty() ? QStringLiteral("Unknown page type.") : message, placeholder);
            label->setAlignment(Qt::AlignCenter);
            label->setWordWrap(true);
            layout->addWidget(label);
            notebook->add_page_widget(page_id, title.isEmpty() ? QStringLiteral("page") : title, placeholder);
        }
    }

    notebook->finalize_notebook(object.value(QStringLiteral("current")).toString());
}

HydrusQueryPage *HydrusMainWindow::find_page(const QString &page_id) const {
    return pages.value(page_id, nullptr);
}

void HydrusMainWindow::update_page(const QString &page_id, const QJsonObject &state) {
    if (HydrusQueryPage *page = find_page(page_id)) {
        page->update_state(state);
    }
}

void HydrusMainWindow::update_page_progress(const QString &page_id, const QJsonObject &state) {
    if (HydrusQueryPage *page = find_page(page_id)) {
        page->update_progress(state);
    }
}

void HydrusMainWindow::update_downloader(const QString &page_id, const QJsonObject &state) {
    if (HydrusDownloaderPage *page = downloaders.value(page_id, nullptr)) {
        page->update_state(state);
    }
}

void HydrusMainWindow::update_subscriptions(const QString &page_id, const QJsonObject &state) {
    if (HydrusSubscriptionsPage *page = subscriptions.value(page_id, nullptr)) {
        page->update_state(state);
    }
}

void HydrusMainWindow::update_duplicates(const QString &page_id, const QJsonObject &state) {
    if (HydrusDuplicatesPage *page = duplicates.value(page_id, nullptr)) {
        page->update_state(state);
    }
}

void HydrusMainWindow::update_import_folders(const QString &page_id, const QJsonObject &state) {
    if (HydrusImportFoldersPage *page = import_folders.value(page_id, nullptr)) {
        page->update_state(state);
    }
}

void HydrusMainWindow::update_export_folders(const QString &page_id, const QJsonObject &state) {
    if (HydrusExportFoldersPage *page = export_folders.value(page_id, nullptr)) {
        page->update_state(state);
    }
}

void HydrusMainWindow::set_selection(
    const QString &page_id,
    const QString &hash,
    const QStringList &tags,
    const QString &preview_path,
    const QJsonObject &metadata) {
    if (HydrusQueryPage *page = find_page(page_id)) {
        page->set_selection(hash, tags, preview_path, metadata);
    }
}

// ---------------------------------------------------------------------------
// Command palette
// ---------------------------------------------------------------------------

class HydrusCommandPalette : public QDialog {
public:
    explicit HydrusCommandPalette(HydrusMainWindow *main_window, QWidget *parent = nullptr)
        : QDialog(parent), main(main_window) {
        setWindowTitle(QStringLiteral("command palette"));
        resize(520, 420);

        auto *layout = new QVBoxLayout(this);
        filter = new QLineEdit(this);
        filter->setPlaceholderText(QStringLiteral("type a command..."));
        layout->addWidget(filter);

        list = new QListWidget(this);
        list->setSelectionMode(QAbstractItemView::SingleSelection);
        layout->addWidget(list, 1);

        const QList<QPair<QString, QString>> commands = {
            {QStringLiteral("new page..."), QStringLiteral("new_page")},
            {QStringLiteral("manage pages..."), QStringLiteral("page_manager")},
            {QStringLiteral("duplicate current page"), QStringLiteral("duplicate_page")},
            {QStringLiteral("back"), QStringLiteral("history_back")},
            {QStringLiteral("forward"), QStringLiteral("history_forward")},
            {QStringLiteral("refresh current page"), QStringLiteral("refresh_page")},
            {QStringLiteral("options..."), QStringLiteral("options")},
            {QStringLiteral("manage tags..."), QStringLiteral("manage_tags_active")},
            {QStringLiteral("import files..."), QStringLiteral("import_files")},
            {QStringLiteral("import url..."), QStringLiteral("import_url")},
            {QStringLiteral("import folders..."), QStringLiteral("new_import_folders")},
            {QStringLiteral("export folders..."), QStringLiteral("new_export_folders")},
            {QStringLiteral("run all export folders"), QStringLiteral("export_folder_run_all")},
            {QStringLiteral("check all import folders"), QStringLiteral("import_folder_check_all")},
            {QStringLiteral("manage services..."), QStringLiteral("manage_services")},
            {QStringLiteral("client api permissions..."), QStringLiteral("api_permissions")},
            {QStringLiteral("clear orphan files..."), QStringLiteral("orphan_files_scan")},
            {QStringLiteral("review duplicate pending decisions..."), QStringLiteral("duplicates_review")},
            {QStringLiteral("back up database..."), QStringLiteral("db_backup_open")},
            {QStringLiteral("manage network..."), QStringLiteral("options_network")},
            {QStringLiteral("save session as..."), QStringLiteral("session_save_as")},
            {QStringLiteral("backup session"), QStringLiteral("session_backup")},
            {QStringLiteral("check database"), QStringLiteral("db_check")},
            {QStringLiteral("analyze database"), QStringLiteral("db_analyze")},
            {QStringLiteral("vacuum database"), QStringLiteral("db_vacuum")},
            {QStringLiteral("database statistics"), QStringLiteral("db_stats")},
            {QStringLiteral("undo closed page"), QStringLiteral("undo")},
            {QStringLiteral("documentation"), QStringLiteral("docs")},
            {QStringLiteral("about"), QStringLiteral("about")},
            {QStringLiteral("exit"), QStringLiteral("exit")},
        };

        for (const auto &[label, action] : commands) {
            auto *item = new QListWidgetItem(label, list);
            item->setData(Qt::UserRole, action);
        }

        connect(filter, &QLineEdit::textChanged, this, [this](const QString &text) {
            for (int index = 0; index < list->count(); ++index) {
                QListWidgetItem *item = list->item(index);
                item->setHidden(!text.isEmpty()
                    && !item->text().contains(text, Qt::CaseInsensitive));
            }

            if (list->count() > 0) {
                for (int index = 0; index < list->count(); ++index) {
                    if (!list->item(index)->isHidden()) {
                        list->setCurrentRow(index);
                        break;
                    }
                }
            }
        });
        connect(filter, &QLineEdit::returnPressed, this, [this]() { run_current(); });
        connect(list, &QListWidget::itemDoubleClicked, this, [this](QListWidgetItem *) {
            run_current();
        });

        if (list->count() > 0) {
            list->setCurrentRow(0);
        }

        filter->setFocus();
    }

private:
    void run_current() {
        QListWidgetItem *item = list->currentItem();

        if (item == nullptr) {
            return;
        }

        const QString action = item->data(Qt::UserRole).toString();
        accept();

        if (!action.isEmpty()) {
            if (action == QStringLiteral("duplicate_page")
                || action == QStringLiteral("refresh_page")) {
                main->notify(action, main->active_page);
            } else {
                main->notify(action);
            }
        }
    }

    HydrusMainWindow *main = nullptr;
    QLineEdit *filter = nullptr;
    QListWidget *list = nullptr;
};

void show_command_palette(HydrusMainWindow *main) {
    HydrusCommandPalette palette(main);
    palette.exec();
}

// ---------------------------------------------------------------------------
// OR predicate editor
// ---------------------------------------------------------------------------

class HydrusOrDialog : public QDialog {
public:
    HydrusOrDialog(const QString &initial, QWidget *parent = nullptr)
        : QDialog(parent) {
        setWindowTitle(QStringLiteral("OR predicates"));
        resize(520, 380);

        auto *layout = new QVBoxLayout(this);
        auto *help = new QLabel(
            QStringLiteral("One predicate per line. A file matches if any line matches.\n"
                           "Tags, wildcards, namespaces and system: predicates are supported."),
            this);
        help->setWordWrap(true);
        layout->addWidget(help);

        editor = new QPlainTextEdit(this);
        editor->setPlainText(initial);
        editor->setPlaceholderText(QStringLiteral("blue eyes\nspecies:wolf\nsystem:width > 1000"));
        layout->addWidget(editor, 1);

        auto *buttons = new QDialogButtonBox(
            QDialogButtonBox::Ok | QDialogButtonBox::Cancel,
            this);
        connect(buttons, &QDialogButtonBox::accepted, this, &QDialog::accept);
        connect(buttons, &QDialogButtonBox::rejected, this, &QDialog::reject);
        layout->addWidget(buttons);
    }

    QString expression() const {
        QStringList parts;

        for (const QString &line : editor->toPlainText().split(QLatin1Char('\n'))) {
            const QString trimmed = line.trimmed();

            if (!trimmed.isEmpty()) {
                parts.append(trimmed);
            }
        }

        if (parts.isEmpty()) {
            return {};
        }

        return QStringLiteral("{%1}").arg(parts.join(QStringLiteral(" | ")));
    }

private:
    QPlainTextEdit *editor = nullptr;
};

void show_or_predicate_dialog(HydrusMainWindow &main, const QString &page_id, const QString &initial) {
    HydrusOrDialog dialog(initial, &main);

    if (dialog.exec() == QDialog::Accepted) {
        const QString expression = dialog.expression();

        if (!expression.isEmpty()) {
            main.notify(QStringLiteral("or_insert"), page_id, expression);
        }
    }
}

void hydrus_show_or_dialog(HydrusMainWindow &window, rust::Str page_id, rust::Str initial) {
    show_or_predicate_dialog(window, to_qstring(page_id), to_qstring(initial));
}

// ---------------------------------------------------------------------------
// Page manager
// ---------------------------------------------------------------------------

class HydrusPageManager : public QDialog {
public:
    HydrusPageManager(HydrusMainWindow *main_window, const QJsonObject &state, QWidget *parent = nullptr)
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

private:
    QIcon icon_for(const QString &kind) const {
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

    void build(const QJsonObject &root) {
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

    QTreeWidgetItem *add_node(QTreeWidgetItem *parent, const QJsonObject &node) {
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

    bool filter_item(QTreeWidgetItem *item, const QString &needle) {
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

    void apply_filter(const QString &needle) {
        for (int index = 0; index < tree->topLevelItemCount(); ++index) {
            filter_item(tree->topLevelItem(index), needle);
        }
    }

    QString id_of(QTreeWidgetItem *item) const {
        return item == nullptr ? QString() : item->data(0, Qt::UserRole).toString();
    }

    void activate_current() {
        const QString id = id_of(tree->currentItem());

        if (id.isEmpty()) {
            return;
        }

        main->notify(QStringLiteral("page_manager_activate"), id);
        accept();
    }

    void close_selected_pages() {
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

    void close_all_pages() {
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

    HydrusMainWindow *main = nullptr;
    QLineEdit *filter = nullptr;
    QTreeWidget *tree = nullptr;
    QStringList top_levels;
};

void HydrusMainWindow::show_page_chooser(const QString &notebook_id) {
    HydrusPageChooser chooser(notebook_id, this);

    if (chooser.exec() == QDialog::Accepted && !chooser.result.isEmpty()) {
        notify(QStringLiteral("create_page"), notebook_id, chooser.result);
    }
}

void hydrus_show_page_manager(HydrusMainWindow &window, rust::Str json) {
    HydrusPageManager manager(&window, parse_object(json), &window);
    manager.exec();
}

void hydrus_show_command_palette(HydrusMainWindow &window) {
    show_command_palette(&window);
}

void hydrus_prompt_text(
    HydrusMainWindow &window,
    rust::Str action,
    rust::Str title,
    rust::Str label,
    rust::Str initial) {
    bool ok = false;
    const QString text = QInputDialog::getText(
        &window,
        from_rust(title),
        from_rust(label),
        QLineEdit::Normal,
        from_rust(initial),
        &ok);

    if (ok) {
        window.notify(from_rust(action), QString(), text);
    }
}

void hydrus_confirm_action(
    HydrusMainWindow &window,
    rust::Str action,
    rust::Str target,
    rust::Str title,
    rust::Str text) {
    const QMessageBox::StandardButton answer = QMessageBox::question(
        &window,
        from_rust(title),
        from_rust(text),
        QMessageBox::Yes | QMessageBox::No,
        QMessageBox::No);

    if (answer == QMessageBox::Yes) {
        window.notify(from_rust(action), from_rust(target));
    }
}

void HydrusMainWindow::prompt_url() {
    bool ok = false;
    const QString url = QInputDialog::getText(
        this,
        QStringLiteral("import url"),
        QStringLiteral("file url"),
        QLineEdit::Normal,
        QString(),
        &ok);

    if (ok && !url.isEmpty()) {
        notify(QStringLiteral("download_url"), QString(), url);
    }
}

void HydrusMainWindow::open_viewer(const QJsonObject &state) {
    const QString viewer_id = state.value(QStringLiteral("viewer_id")).toString();

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

void HydrusMainWindow::update_viewer(const QString &viewer_id, const QJsonObject &state) {
    if (HydrusViewerWindow *viewer = viewers.value(viewer_id, nullptr)) {
        viewer->update_state(state);
    }
}

void HydrusMainWindow::close_viewer(const QString &viewer_id) {
    if (HydrusViewerWindow *viewer = viewers.value(viewer_id, nullptr)) {
        viewers.remove(viewer_id);
        viewer->close();
    }
}

class HydrusManageTagsDialog : public QDialog {
public:
    HydrusManageTagsDialog(HydrusMainWindow *main_window, const QJsonObject &state, QWidget *parent = nullptr)
        : QDialog(parent), main(main_window), data(state) {
        setWindowTitle(QStringLiteral("manage tags"));
        resize(460, 600);

        auto *layout = new QVBoxLayout(this);

        services = new QComboBox(this);
        layout->addWidget(services);

        summary = new QLabel(this);
        layout->addWidget(summary);

        tags = new QListWidget(this);
        tags->setSelectionMode(QAbstractItemView::ExtendedSelection);
        layout->addWidget(tags, 1);

        auto *input_row = new QHBoxLayout();
        input = new QLineEdit(this);
        input->setPlaceholderText(QStringLiteral("tag1, tag2, namespace:tag"));
        input_row->addWidget(input, 1);

        auto *add = new QPushButton(QStringLiteral("add"), this);
        auto *remove = new QPushButton(QStringLiteral("remove"), this);
        input_row->addWidget(add);
        input_row->addWidget(remove);
        layout->addLayout(input_row);

        auto *close_box = new QDialogButtonBox(QDialogButtonBox::Close, this);
        connect(close_box, &QDialogButtonBox::rejected, this, &QDialog::accept);
        layout->addWidget(close_box);

        for (const QJsonValue &value : data.value(QStringLiteral("services")).toArray()) {
            const QString name = value.toObject().value(QStringLiteral("name")).toString();

            if (!name.isEmpty()) {
                services->addItem(name);
            }
        }

        summary->setText(QStringLiteral("%1 file(s) selected")
            .arg(data.value(QStringLiteral("hashes")).toArray().size()));

        connect(services, &QComboBox::currentTextChanged, this, [this](const QString &) {
            rebuild();
        });
        connect(add, &QPushButton::clicked, this, [this]() { add_tags(); });
        connect(remove, &QPushButton::clicked, this, [this]() { remove_tags(); });
        connect(input, &QLineEdit::returnPressed, this, [this]() { add_tags(); });

        rebuild();
    }

private:
    QJsonObject current_service() const {
        const QString name = services->currentText();

        for (const QJsonValue &value : data.value(QStringLiteral("services")).toArray()) {
            const QJsonObject service = value.toObject();

            if (service.value(QStringLiteral("name")).toString() == name) {
                return service;
            }
        }

        return {};
    }

    void rebuild() {
        tags->clear();

        for (const QJsonValue &value : current_service().value(QStringLiteral("tags")).toArray()) {
            const QJsonObject tag = value.toObject();
            const QString name = tag.value(QStringLiteral("tag")).toString();
            const int count = tag.value(QStringLiteral("count")).toInt();
            auto *item = new QListWidgetItem(
                count > 1 ? QStringLiteral("%1 (%2)").arg(name).arg(count) : name,
                tags);
            item->setData(Qt::UserRole, name);
        }
    }

    void send(const QStringList &values, const QString &operation) {
        if (values.isEmpty() || services->currentText().isEmpty()) {
            return;
        }

        QJsonArray hashes = data.value(QStringLiteral("hashes")).toArray();
        QJsonArray array;

        for (const QString &value : values) {
            array.append(value);
        }

        QJsonObject payload;
        payload.insert(QStringLiteral("hashes"), hashes);
        payload.insert(QStringLiteral("service"), services->currentText());
        payload.insert(operation, array);
        main->notify(
            QStringLiteral("apply_tags"),
            QString(),
            QString::fromUtf8(QJsonDocument(payload).toJson(QJsonDocument::Compact)));
    }

    void add_tags() {
        QStringList values;

        for (const QString &part : input->text().split(QRegularExpression(QStringLiteral("[,;\\n]")))) {
            const QString tag = part.trimmed();

            if (!tag.isEmpty()) {
                values.append(tag);
            }
        }

        if (values.isEmpty()) {
            return;
        }

        send(values, QStringLiteral("add"));

        for (const QString &tag : values) {
            bool exists = false;

            for (int index = 0; index < tags->count(); ++index) {
                if (tags->item(index)->data(Qt::UserRole).toString() == tag) {
                    exists = true;
                    break;
                }
            }

            if (!exists) {
                auto *item = new QListWidgetItem(tag, tags);
                item->setData(Qt::UserRole, tag);
            }
        }

        input->clear();
    }

    void remove_tags() {
        QStringList values;

        for (QListWidgetItem *item : tags->selectedItems()) {
            values.append(item->data(Qt::UserRole).toString());
        }

        if (values.isEmpty()) {
            return;
        }

        send(values, QStringLiteral("remove"));

        for (QListWidgetItem *item : tags->selectedItems()) {
            delete item;
        }
    }

    HydrusMainWindow *main = nullptr;
    QJsonObject data;
    QComboBox *services = nullptr;
    QLabel *summary = nullptr;
    QListWidget *tags = nullptr;
    QLineEdit *input = nullptr;
};

class HydrusTimestampsDialog : public QDialog {
public:
    HydrusTimestampsDialog(HydrusMainWindow *main_window, const QJsonObject &state, QWidget *parent = nullptr)
        : QDialog(parent), main(main_window), hash(state.value(QStringLiteral("hash")).toString()) {
        setWindowTitle(QStringLiteral("edit timestamps"));
        resize(420, 220);

        auto *layout = new QVBoxLayout(this);
        auto *form = new QFormLayout();

        archived_enabled = new QCheckBox(QStringLiteral("archived time"), this);
        archived = new QDateTimeEdit(this);
        archived->setDisplayFormat(QStringLiteral("yyyy-MM-dd HH:mm:ss"));
        archived->setCalendarPopup(true);
        form->addRow(archived_enabled, archived);

        modified_enabled = new QCheckBox(QStringLiteral("file modified time"), this);
        modified = new QDateTimeEdit(this);
        modified->setDisplayFormat(QStringLiteral("yyyy-MM-dd HH:mm:ss"));
        modified->setCalendarPopup(true);
        form->addRow(modified_enabled, modified);

        layout->addLayout(form);

        set_time(archived_enabled, archived, state.value(QStringLiteral("archived")));
        set_time(modified_enabled, modified, state.value(QStringLiteral("modified")));

        auto *buttons = new QDialogButtonBox(
            QDialogButtonBox::Save | QDialogButtonBox::Cancel,
            this);
        connect(buttons, &QDialogButtonBox::accepted, this, [this]() { save(); });
        connect(buttons, &QDialogButtonBox::rejected, this, &QDialog::reject);
        layout->addWidget(buttons);
    }

private:
    void set_time(QCheckBox *enabled, QDateTimeEdit *editor, const QJsonValue &value) {
        if (value.isDouble()) {
            enabled->setChecked(true);
            editor->setDateTime(QDateTime::fromSecsSinceEpoch(value.toInteger()));
        } else {
            enabled->setChecked(false);
            editor->setDateTime(QDateTime::currentDateTime());
        }
    }

    void save() {
        QJsonObject payload;
        payload.insert(QStringLiteral("hash"), hash);
        payload.insert(
            QStringLiteral("archived"),
            archived_enabled->isChecked()
                ? QJsonValue(archived->dateTime().toSecsSinceEpoch())
                : QJsonValue(QJsonValue::Null));
        payload.insert(
            QStringLiteral("modified"),
            modified_enabled->isChecked()
                ? QJsonValue(modified->dateTime().toSecsSinceEpoch())
                : QJsonValue(QJsonValue::Null));

        main->notify(
            QStringLiteral("timestamps_save"),
            QString(),
            QString::fromUtf8(QJsonDocument(payload).toJson(QJsonDocument::Compact)));
        accept();
    }

    HydrusMainWindow *main = nullptr;
    QString hash;
    QCheckBox *archived_enabled = nullptr;
    QCheckBox *modified_enabled = nullptr;
    QDateTimeEdit *archived = nullptr;
    QDateTimeEdit *modified = nullptr;
};

// ---------------------------------------------------------------------------
// Popup messages
// ---------------------------------------------------------------------------

class HydrusNotificationsDialog : public QDialog {
public:
    HydrusNotificationsDialog(HydrusMainWindow *main_window, const QJsonObject &state, QWidget *parent = nullptr)
        : QDialog(parent), main(main_window) {
        setWindowTitle(QStringLiteral("popup messages"));
        resize(720, 460);

        auto *layout = new QVBoxLayout(this);

        summary = new QLabel(
            QStringLiteral("%1 message(s)").arg(state.value(QStringLiteral("count")).toInt()),
            this);
        layout->addWidget(summary);

        table = new QTableWidget(this);
        table->setColumnCount(3);
        table->setHorizontalHeaderLabels({
            QStringLiteral("time"),
            QStringLiteral("level"),
            QStringLiteral("message"),
        });
        table->verticalHeader()->setVisible(false);
        table->setEditTriggers(QAbstractItemView::NoEditTriggers);
        table->setSelectionBehavior(QAbstractItemView::SelectRows);
        table->horizontalHeader()->setSectionResizeMode(0, QHeaderView::ResizeToContents);
        table->horizontalHeader()->setSectionResizeMode(1, QHeaderView::ResizeToContents);
        table->horizontalHeader()->setSectionResizeMode(2, QHeaderView::Stretch);
        layout->addWidget(table, 1);

        rebuild(state.value(QStringLiteral("entries")).toArray());

        auto *buttons = new QDialogButtonBox(QDialogButtonBox::Close, this);
        auto *clear = new QPushButton(QStringLiteral("clear"), this);
        buttons->addButton(clear, QDialogButtonBox::ActionRole);
        connect(clear, &QPushButton::clicked, this, [this]() {
            table->setRowCount(0);
            summary->setText(QStringLiteral("0 message(s)"));
            main->notify(QStringLiteral("notifications_clear"));
        });
        connect(buttons, &QDialogButtonBox::rejected, this, &QDialog::accept);
        layout->addWidget(buttons);
    }

private:
    void rebuild(const QJsonArray &entries) {
        table->setRowCount(entries.size());

        for (int row = 0; row < entries.size(); ++row) {
            const QJsonObject entry = entries.at(row).toObject();

            table->setItem(row, 0, new QTableWidgetItem(entry.value(QStringLiteral("time")).toString()));
            table->setItem(row, 1, new QTableWidgetItem(entry.value(QStringLiteral("level")).toString()));
            table->setItem(row, 2, new QTableWidgetItem(entry.value(QStringLiteral("text")).toString()));
        }
    }

    HydrusMainWindow *main = nullptr;
    QLabel *summary = nullptr;
    QTableWidget *table = nullptr;
};

void HydrusMainWindow::show_viewing_stats(const QJsonObject &state) {
    QDialog dialog(this);
    dialog.setWindowTitle(QStringLiteral("file viewing statistics"));
    dialog.resize(560, 260);

    auto *layout = new QVBoxLayout(&dialog);
    auto *hash = new QLabel(
        QStringLiteral("file: %1").arg(state.value(QStringLiteral("hash")).toString()),
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

    const QJsonArray rows = state.value(QStringLiteral("rows")).toArray();
    table->setRowCount(rows.size());

    for (int row = 0; row < rows.size(); ++row) {
        const QJsonObject entry = rows.at(row).toObject();
        table->setItem(row, 0, new QTableWidgetItem(entry.value(QStringLiteral("canvas")).toString()));
        table->setItem(row, 1, new QTableWidgetItem(QString::number(entry.value(QStringLiteral("views")).toInt())));
        table->setItem(row, 2, new QTableWidgetItem(QString::number(entry.value(QStringLiteral("viewtime")).toDouble())));
        table->setItem(row, 3, new QTableWidgetItem(entry.value(QStringLiteral("last_viewed")).toString()));
    }

    layout->addWidget(table, 1);

    auto *buttons = new QDialogButtonBox(QDialogButtonBox::Close, &dialog);
    connect(buttons, &QDialogButtonBox::rejected, &dialog, &QDialog::accept);
    layout->addWidget(buttons);

    dialog.exec();
}

void HydrusMainWindow::show_about(const QJsonObject &state) {
    QDialog dialog(this);
    dialog.setWindowTitle(QStringLiteral("about"));
    dialog.resize(680, 520);

    auto *layout = new QVBoxLayout(&dialog);

    auto *title = new QLabel(QStringLiteral("Hydrus Network"), &dialog);
    title->setStyleSheet(QStringLiteral("font-size: 20px; font-weight: bold;"));
    layout->addWidget(title);

    auto *subtitle = new QLabel(
        QStringLiteral("Rust/Qt client Р В Р’В Р вЂ™Р’В Р В РІР‚в„ўР вЂ™Р’В Р В Р’В Р В РІР‚В Р В Р’В Р Р†Р вЂљРЎв„ўР В Р вЂ Р Р†Р вЂљРЎвЂєР РЋРЎвЂєР В Р’В Р вЂ™Р’В Р В Р вЂ Р В РІР‚С™Р Р†РІР‚С›РЎС›Р В Р’В Р Р†Р вЂљРІвЂћСћР В РІР‚в„ўР вЂ™Р’В· software v%1 Р В Р’В Р вЂ™Р’В Р В РІР‚в„ўР вЂ™Р’В Р В Р’В Р В РІР‚В Р В Р’В Р Р†Р вЂљРЎв„ўР В Р вЂ Р Р†Р вЂљРЎвЂєР РЋРЎвЂєР В Р’В Р вЂ™Р’В Р В Р вЂ Р В РІР‚С™Р Р†РІР‚С›РЎС›Р В Р’В Р Р†Р вЂљРІвЂћСћР В РІР‚в„ўР вЂ™Р’В· network v%2")
            .arg(state.value(QStringLiteral("software_version")).toInt())
            .arg(state.value(QStringLiteral("network_version")).toInt()),
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

    add_row(QStringLiteral("software version"), QString::number(state.value(QStringLiteral("software_version")).toInt()));
    add_row(QStringLiteral("network version"), QString::number(state.value(QStringLiteral("network_version")).toInt()));
    add_row(QStringLiteral("client api version"), QString::number(state.value(QStringLiteral("client_api_version")).toInt()));
    add_row(QStringLiteral("db version"), QString::number(state.value(QStringLiteral("db_version")).toInt()));
    add_row(QStringLiteral("client api"), state.value(QStringLiteral("api")).toString());
    add_row(QStringLiteral("sqlite"), state.value(QStringLiteral("sqlite")).toString());
    add_row(QStringLiteral("rust crate"), state.value(QStringLiteral("rust")).toString());
    add_row(QStringLiteral("Qt"), QString::fromLatin1(qVersion()));
    add_row(QStringLiteral("services"), QString::number(state.value(QStringLiteral("services")).toInt()));
    add_row(QStringLiteral("file locations"), QString::number(state.value(QStringLiteral("locations")).toInt()));
    add_row(QStringLiteral("shortcuts"), QString::number(state.value(QStringLiteral("shortcuts")).toInt()));
    add_row(QStringLiteral("db dir"), state.value(QStringLiteral("db_dir")).toString());
    add_row(QStringLiteral("install dir"), state.value(QStringLiteral("install_dir")).toString());
    tabs->addTab(overview, QStringLiteral("overview"));

    auto *license = new QPlainTextEdit(tabs);
    license->setReadOnly(true);
    license->setPlainText(
        state.value(QStringLiteral("license")).toString() + QStringLiteral("\n\n")
        + state.value(QStringLiteral("site")).toString());
    tabs->addTab(license, QStringLiteral("license"));

    auto *credits = new QPlainTextEdit(tabs);
    credits->setReadOnly(true);
    credits->setPlainText(state.value(QStringLiteral("credits")).toString());
    tabs->addTab(credits, QStringLiteral("credits"));

    auto *buttons = new QDialogButtonBox(QDialogButtonBox::Close, &dialog);
    auto *site = new QPushButton(QStringLiteral("open site"), &dialog);
    buttons->addButton(site, QDialogButtonBox::ActionRole);
    connect(site, &QPushButton::clicked, &dialog, [&state]() {
        QDesktopServices::openUrl(QUrl(state.value(QStringLiteral("site")).toString()));
    });
    connect(buttons, &QDialogButtonBox::rejected, &dialog, &QDialog::accept);
    layout->addWidget(buttons);

    dialog.exec();
}

void HydrusMainWindow::show_notifications(const QJsonObject &state) {
    HydrusNotificationsDialog dialog(this, state, this);
    dialog.exec();
}

void HydrusMainWindow::show_tag_dialog(const QJsonObject &state) {
    QDialog dialog(this);
    dialog.setWindowTitle(QStringLiteral("manage metadata"));
    dialog.resize(460, 620);

    auto *layout = new QVBoxLayout(&dialog);
    auto *panel = new HydrusMetadataPanel(this, &dialog);
    panel->set_suggestions(suggestions());
    panel->set_state(state.value(QStringLiteral("metadata")).toObject());
    layout->addWidget(panel, 1);

    auto *buttons = new QDialogButtonBox(QDialogButtonBox::Close, &dialog);
    connect(buttons, &QDialogButtonBox::rejected, &dialog, &QDialog::accept);
    layout->addWidget(buttons);

    dialog.exec();
}

void hydrus_show_manage_tags(HydrusMainWindow &window, rust::Str json) {
    HydrusManageTagsDialog dialog(&window, parse_object(json), &window);
    dialog.exec();
}

void hydrus_show_timestamps(HydrusMainWindow &window, rust::Str json) {
    HydrusTimestampsDialog dialog(&window, parse_object(json), &window);
    dialog.exec();
}

void HydrusMainWindow::show_notes(const QJsonObject &state) {
    const QString hash = state.value(QStringLiteral("hash")).toString();

    QDialog dialog(this);
    dialog.setWindowTitle(QStringLiteral("notes"));
    dialog.resize(620, 460);

    auto *layout = new QVBoxLayout(&dialog);
    auto *info = new QLabel(hash.left(16) + QStringLiteral("..."), &dialog);
    info->setTextInteractionFlags(Qt::TextSelectableByMouse);
    layout->addWidget(info);

    auto *table = new QTableWidget(&dialog);
    table->setColumnCount(2);
    table->setHorizontalHeaderLabels({QStringLiteral("name"), QStringLiteral("note")});
    table->horizontalHeader()->setStretchLastSection(true);
    table->horizontalHeader()->setSectionResizeMode(0, QHeaderView::ResizeToContents);
    table->verticalHeader()->setVisible(false);

    const QJsonArray notes = state.value(QStringLiteral("notes")).toArray();
    table->setRowCount(notes.size());

    for (int index = 0; index < notes.size(); ++index) {
        const QJsonObject note = notes.at(index).toObject();
        table->setItem(index, 0, new QTableWidgetItem(note.value(QStringLiteral("name")).toString()));
        table->setItem(index, 1, new QTableWidgetItem(note.value(QStringLiteral("text")).toString()));
    }

    table->resizeRowsToContents();
    layout->addWidget(table, 1);

    auto *buttons = new QHBoxLayout();
    auto *add = new QPushButton(QStringLiteral("add note"), &dialog);
    auto *remove = new QPushButton(QStringLiteral("remove"), &dialog);
    buttons->addWidget(add);
    buttons->addWidget(remove);
    buttons->addStretch(1);
    auto *box = new QDialogButtonBox(QDialogButtonBox::Save | QDialogButtonBox::Cancel, &dialog);
    buttons->addWidget(box);
    layout->addLayout(buttons);

    connect(add, &QPushButton::clicked, &dialog, [table]() {
        const int row = table->rowCount();
        table->insertRow(row);
        table->setItem(row, 0, new QTableWidgetItem());
        table->setItem(row, 1, new QTableWidgetItem());
        table->editItem(table->item(row, 0));
    });
    connect(remove, &QPushButton::clicked, &dialog, [table]() {
        const int row = table->currentRow();

        if (row >= 0) {
            table->removeRow(row);
        }
    });
    connect(box, &QDialogButtonBox::accepted, &dialog, &QDialog::accept);
    connect(box, &QDialogButtonBox::rejected, &dialog, &QDialog::reject);

    if (dialog.exec() != QDialog::Accepted) {
        return;
    }

    QJsonArray values;

    for (int row = 0; row < table->rowCount(); ++row) {
        const QString name = table->item(row, 0) == nullptr
            ? QString()
            : table->item(row, 0)->text().trimmed();
        const QString text = table->item(row, 1) == nullptr
            ? QString()
            : table->item(row, 1)->text();

        if (name.isEmpty() && text.trimmed().isEmpty()) {
            continue;
        }

        QJsonObject note;
        note.insert(QStringLiteral("name"), name);
        note.insert(QStringLiteral("text"), text);
        values.append(note);
    }

    QJsonObject payload;
    payload.insert(QStringLiteral("hash"), hash);
    payload.insert(QStringLiteral("notes"), values);

    notify(QStringLiteral("notes_save"), QString(), json_payload(payload));
}

void HydrusMainWindow::show_tag_relations(const QJsonObject &state) {
    if (tag_relations_dialog == nullptr) {
        tag_relations_dialog = new HydrusTagRelationsDialog(this, state, this);
        connect(tag_relations_dialog, &QObject::destroyed, this, [this]() {
            tag_relations_dialog = nullptr;
        });
        tag_relations_dialog->show();
    } else {
        tag_relations_dialog->set_state(state);
    }

    tag_relations_dialog->raise();
    tag_relations_dialog->activateWindow();
}

void HydrusMainWindow::show_import_folder(const QJsonObject &state) {
    const QJsonObject folder = state.value(QStringLiteral("folder")).toObject();
    const QString original = folder.value(QStringLiteral("name")).toString();

    QDialog dialog(this);
    dialog.setWindowTitle(original.isEmpty() ? QStringLiteral("add import folder")
                                             : QStringLiteral("edit import folder '%1'").arg(original));
    dialog.resize(620, 480);

    auto *layout = new QVBoxLayout(&dialog);
    auto *form = new QFormLayout();

    auto *name = new QLineEdit(folder.value(QStringLiteral("name")).toString(), &dialog);
    auto *path_row = new QWidget(&dialog);
    auto *path_layout = new QHBoxLayout(path_row);
    path_layout->setContentsMargins(0, 0, 0, 0);
    auto *path = new QLineEdit(folder.value(QStringLiteral("path")).toString(), path_row);
    auto *browse = new QPushButton(QStringLiteral("browse..."), path_row);
    path_layout->addWidget(path, 1);
    path_layout->addWidget(browse);

    auto *period = new QSpinBox(&dialog);
    period->setRange(60, 30 * 86400);
    period->setSingleStep(60);
    period->setValue(folder.value(QStringLiteral("period")).toInt(300));
    period->setSuffix(QStringLiteral(" s"));

    auto *regular = new QCheckBox(QStringLiteral("check this folder regularly"), &dialog);
    regular->setChecked(folder.value(QStringLiteral("check_regularly")).toBool(true));
    auto *subdirectories = new QCheckBox(QStringLiteral("search subdirectories"), &dialog);
    subdirectories->setChecked(folder.value(QStringLiteral("search_subdirectories")).toBool(true));

    auto *skip = new QSpinBox(&dialog);
    skip->setRange(0, 30 * 86400);
    skip->setValue(folder.value(QStringLiteral("last_modified_skip")).toVariant().toInt());
    skip->setSuffix(QStringLiteral(" s"));
    skip->setToolTip(QStringLiteral("ignore files modified more recently than this"));

    auto *action = new QComboBox(&dialog);
    action->addItem(QStringLiteral("leave the file alone"), QStringLiteral("ignore"));
    action->addItem(QStringLiteral("move it to a folder"), QStringLiteral("move"));
    action->addItem(QStringLiteral("delete it"), QStringLiteral("delete"));
    const QString action_value = folder.value(QStringLiteral("action")).toString();
    const int action_index = action->findData(action_value);
    action->setCurrentIndex(action_index >= 0 ? action_index : 0);

    auto *action_row = new QWidget(&dialog);
    auto *action_layout = new QHBoxLayout(action_row);
    action_layout->setContentsMargins(0, 0, 0, 0);
    auto *action_path = new QLineEdit(folder.value(QStringLiteral("action_path")).toString(), action_row);
    auto *action_browse = new QPushButton(QStringLiteral("browse..."), action_row);
    action_layout->addWidget(action_path, 1);
    action_layout->addWidget(action_browse);

    auto *min_size = new QSpinBox(&dialog);
    min_size->setRange(0, 2'000'000'000);
    min_size->setValue(folder.value(QStringLiteral("min_size")).toVariant().toInt());
    min_size->setSuffix(QStringLiteral(" bytes"));
    auto *max_size = new QSpinBox(&dialog);
    max_size->setRange(0, 2'000'000'000);
    max_size->setValue(folder.value(QStringLiteral("max_size")).toVariant().toInt());
    max_size->setSuffix(QStringLiteral(" bytes"));
    auto *extensions = new QLineEdit(folder.value(QStringLiteral("extensions")).toString(), &dialog);
    extensions->setPlaceholderText(QStringLiteral("jpg, png, webp (empty accepts everything)"));

    form->addRow(QStringLiteral("name"), name);
    form->addRow(QStringLiteral("folder"), path_row);
    form->addRow(QStringLiteral("check every"), period);
    form->addRow(regular);
    form->addRow(subdirectories);
    form->addRow(QStringLiteral("skip recent files"), skip);
    form->addRow(QStringLiteral("after import"), action);
    form->addRow(QStringLiteral("move to"), action_row);
    form->addRow(QStringLiteral("minimum size"), min_size);
    form->addRow(QStringLiteral("maximum size"), max_size);
    form->addRow(QStringLiteral("extensions"), extensions);
    layout->addLayout(form);

    auto *note = new QLabel(
        QStringLiteral("Filters are applied before a file is queued. Move/delete runs only for "
                       "files that imported successfully."),
        &dialog);
    note->setWordWrap(true);
    layout->addWidget(note);

    auto *buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, &dialog);
    layout->addWidget(buttons);
    QObject::connect(buttons, &QDialogButtonBox::rejected, &dialog, &QDialog::reject);
    connect(buttons, &QDialogButtonBox::accepted, &dialog, [&dialog, path, action, action_path]() {
        if (path->text().trimmed().isEmpty()) {
            QMessageBox::warning(&dialog, QStringLiteral("import folder"), QStringLiteral("Choose a folder to import from."));
            return;
        }

        if (action->currentData().toString() == QStringLiteral("move")
            && action_path->text().trimmed().isEmpty()) {
            QMessageBox::warning(&dialog, QStringLiteral("import folder"), QStringLiteral("Choose a destination folder for moved files."));
            return;
        }

        dialog.accept();
    });

    auto sync_action = [form, action, action_path, action_browse, action_row](int) {
        const bool move = action->currentData().toString() == QStringLiteral("move");
        action_path->setEnabled(move);
        action_browse->setEnabled(move);
        form->setRowVisible(action_row, move);
    };

    connect(action, QOverload<int>::of(&QComboBox::currentIndexChanged), &dialog, sync_action);
    sync_action(0);

    connect(browse, &QPushButton::clicked, &dialog, [this, &dialog, path]() {
        const QString chosen = QFileDialog::getExistingDirectory(&dialog, QStringLiteral("choose an import folder"), path->text());

        if (!chosen.isEmpty()) {
            path->setText(chosen);
        }
    });

    connect(action_browse, &QPushButton::clicked, &dialog, [this, &dialog, action_path]() {
        const QString chosen = QFileDialog::getExistingDirectory(&dialog, QStringLiteral("choose the destination folder"), action_path->text());

        if (!chosen.isEmpty()) {
            action_path->setText(chosen);
        }
    });

    if (dialog.exec() != QDialog::Accepted) {
        return;
    }

    QJsonObject payload;
    payload.insert(QStringLiteral("original"), original);
    payload.insert(QStringLiteral("name"), name->text().trimmed());
    payload.insert(QStringLiteral("path"), path->text().trimmed());
    payload.insert(QStringLiteral("period"), period->value());
    payload.insert(QStringLiteral("check_regularly"), regular->isChecked());
    payload.insert(QStringLiteral("search_subdirectories"), subdirectories->isChecked());
    payload.insert(QStringLiteral("last_modified_skip"), skip->value());
    payload.insert(QStringLiteral("action"), action->currentData().toString());
    payload.insert(QStringLiteral("action_path"), action_path->text().trimmed());
    payload.insert(QStringLiteral("min_size"), min_size->value());
    payload.insert(QStringLiteral("max_size"), max_size->value());
    payload.insert(QStringLiteral("extensions"), extensions->text().trimmed());

    notify(QStringLiteral("import_folder_save"), active_page, json_payload(payload));
}

void HydrusMainWindow::show_export_folder(const QJsonObject &state) {
    const QJsonObject folder = state.value(QStringLiteral("folder")).toObject();
    const QString original = folder.value(QStringLiteral("name")).toString();

    QDialog dialog(this);
    dialog.setWindowTitle(original.isEmpty() ? QStringLiteral("add export folder")
                                             : QStringLiteral("edit export folder '%1'").arg(original));
    dialog.resize(680, 520);

    auto *layout = new QVBoxLayout(&dialog);
    auto *form = new QFormLayout();

    auto *name = new QLineEdit(folder.value(QStringLiteral("name")).toString(), &dialog);

    auto *path_row = new QWidget(&dialog);
    auto *path_layout = new QHBoxLayout(path_row);
    path_layout->setContentsMargins(0, 0, 0, 0);
    auto *path = new QLineEdit(folder.value(QStringLiteral("path")).toString(), path_row);
    auto *browse = new QPushButton(QStringLiteral("browse..."), path_row);
    path_layout->addWidget(path, 1);
    path_layout->addWidget(browse);

    auto *query = new QLineEdit(folder.value(QStringLiteral("query")).toString(), &dialog);
    query->setPlaceholderText(QStringLiteral("tags or system:everything (empty exports every file)"));

    auto *phrase = new QLineEdit(folder.value(QStringLiteral("phrase")).toString(), &dialog);
    phrase->setPlaceholderText(QStringLiteral("{hash}, [creator], {tags}, {nn tags}, (tag)"));

    auto *export_type = new QComboBox(&dialog);
    export_type->addItem(QStringLiteral("copy files"), QStringLiteral("copy"));
    export_type->addItem(QStringLiteral("synchronise folder"), QStringLiteral("synchronise"));
    const int type_index = export_type->findData(
        folder.value(QStringLiteral("export_type")).toString());
    export_type->setCurrentIndex(type_index >= 0 ? type_index : 0);

    auto *sidecar = new QComboBox(&dialog);
    sidecar->addItem(QStringLiteral("no sidecar"), QStringLiteral("none"));
    sidecar->addItem(QStringLiteral("tags sidecar (.tags.json)"), QStringLiteral("json"));
    sidecar->addItem(QStringLiteral("tags sidecar (.tags.txt)"), QStringLiteral("text"));
    const int sidecar_index = sidecar->findData(
        folder.value(QStringLiteral("sidecar_format")).toString());
    sidecar->setCurrentIndex(sidecar_index >= 0 ? sidecar_index : 0);

    auto *overwrite_sidecars = new QCheckBox(
        QStringLiteral("overwrite existing sidecars"), &dialog);
    overwrite_sidecars->setChecked(
        folder.value(QStringLiteral("overwrite_sidecars")).toBool(false));

    auto *delete_from_client = new QCheckBox(
        QStringLiteral("delete files from the client after export"), &dialog);
    delete_from_client->setChecked(
        folder.value(QStringLiteral("delete_from_client")).toBool(false));

    auto *regular = new QCheckBox(QStringLiteral("run this folder regularly"), &dialog);
    regular->setChecked(folder.value(QStringLiteral("run_regularly")).toBool(true));

    auto *period = new QSpinBox(&dialog);
    period->setRange(60, 30 * 86400);
    period->setSingleStep(60);
    period->setValue(folder.value(QStringLiteral("period")).toInt(3600));
    period->setSuffix(QStringLiteral(" s"));

    form->addRow(QStringLiteral("name"), name);
    form->addRow(QStringLiteral("destination"), path_row);
    form->addRow(QStringLiteral("search"), query);
    form->addRow(QStringLiteral("filename"), phrase);
    form->addRow(QStringLiteral("mode"), export_type);
    form->addRow(QStringLiteral("metadata"), sidecar);
    form->addRow(overwrite_sidecars);
    form->addRow(delete_from_client);
    form->addRow(regular);
    form->addRow(QStringLiteral("run every"), period);
    layout->addLayout(form);

    auto *note = new QLabel(
        QStringLiteral("Phrase terms: {hash}, {file_id}, {#}, {tags}, {nn tags}, "
                       "[namespace] and (tag). Tags are cleaned before they become filenames.\n"
                       "Synchronise deletes files in the destination that are no longer in the "
                       "search result. Delete from client moves exported files to the trash."),
        &dialog);
    note->setWordWrap(true);
    layout->addWidget(note);

    auto *buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, &dialog);
    layout->addWidget(buttons);
    QObject::connect(buttons, &QDialogButtonBox::rejected, &dialog, &QDialog::reject);
    connect(buttons, &QDialogButtonBox::accepted, &dialog, [&dialog, path, phrase]() {
        if (path->text().trimmed().isEmpty()) {
            QMessageBox::warning(&dialog, QStringLiteral("export folder"),
                                 QStringLiteral("Choose a destination folder."));
            return;
        }

        if (phrase->text().trimmed().isEmpty()) {
            QMessageBox::warning(&dialog, QStringLiteral("export folder"),
                                 QStringLiteral("Enter a filename phrase."));
            return;
        }

        dialog.accept();
    });

    auto sync_options = [regular, period](int) {
        period->setEnabled(regular->isChecked());
    };
    connect(regular, &QCheckBox::toggled, &dialog, sync_options);
    sync_options(0);

    connect(browse, &QPushButton::clicked, &dialog, [this, &dialog, path]() {
        const QString chosen = QFileDialog::getExistingDirectory(
            &dialog, QStringLiteral("choose an export folder"), path->text());

        if (!chosen.isEmpty()) {
            path->setText(chosen);
        }
    });

    if (dialog.exec() != QDialog::Accepted) {
        return;
    }

    QJsonObject payload;
    payload.insert(QStringLiteral("original"), original);
    payload.insert(QStringLiteral("name"), name->text().trimmed());
    payload.insert(QStringLiteral("path"), path->text().trimmed());
    payload.insert(QStringLiteral("query"), query->text().trimmed());
    payload.insert(QStringLiteral("phrase"), phrase->text().trimmed());
    payload.insert(QStringLiteral("export_type"), export_type->currentData().toString());
    payload.insert(QStringLiteral("sidecar_format"), sidecar->currentData().toString());
    payload.insert(QStringLiteral("overwrite_sidecars"), overwrite_sidecars->isChecked());
    payload.insert(QStringLiteral("delete_from_client"), delete_from_client->isChecked());
    payload.insert(QStringLiteral("run_regularly"), regular->isChecked());
    payload.insert(QStringLiteral("period"), period->value());

    notify(QStringLiteral("export_folder_save"), active_page, json_payload(payload));
}

void HydrusMainWindow::show_export_review(const QJsonObject &state) {
    const QString name = state.value(QStringLiteral("name")).toString();
    const QJsonArray files = state.value(QStringLiteral("files")).toArray();

    QDialog dialog(this);
    dialog.setWindowTitle(QStringLiteral("review export folder '%1'").arg(name));
    dialog.resize(860, 520);

    auto *layout = new QVBoxLayout(&dialog);

    auto *summary = new QLabel(
        QStringLiteral("%1 -> %2\n%3 files match the search. Uncheck anything you do not want "
                       "to export this run.")
            .arg(name, state.value(QStringLiteral("destination")).toString())
            .arg(files.size()),
        &dialog);
    summary->setWordWrap(true);
    layout->addWidget(summary);

    auto *table = new QTableWidget(&dialog);
    table->setColumnCount(5);
    table->setHorizontalHeaderLabels({
        QStringLiteral("export"),
        QStringLiteral("file"),
        QStringLiteral("tags"),
        QStringLiteral("destination"),
        QStringLiteral("state")
    });
    table->horizontalHeader()->setStretchLastSection(false);
    table->horizontalHeader()->setSectionResizeMode(0, QHeaderView::ResizeToContents);
    table->horizontalHeader()->setSectionResizeMode(1, QHeaderView::ResizeToContents);
    table->horizontalHeader()->setSectionResizeMode(2, QHeaderView::Stretch);
    table->horizontalHeader()->setSectionResizeMode(3, QHeaderView::Stretch);
    table->horizontalHeader()->setSectionResizeMode(4, QHeaderView::ResizeToContents);
    table->setSelectionBehavior(QAbstractItemView::SelectRows);
    table->setSelectionMode(QAbstractItemView::SingleSelection);
    table->setEditTriggers(QAbstractItemView::NoEditTriggers);
    table->setTextElideMode(Qt::ElideMiddle);
    table->verticalHeader()->setVisible(false);
    table->setRowCount(files.size());

    for (int index = 0; index < files.size(); ++index) {
        const QJsonObject file = files.at(index).toObject();
        const QString hash = file.value(QStringLiteral("hash")).toString();
        const bool safe = file.value(QStringLiteral("safe")).toBool();
        const bool source_exists = file.value(QStringLiteral("source_exists")).toBool(true);
        const bool exists = file.value(QStringLiteral("exists")).toBool();
        const QString destination = file.value(QStringLiteral("destination")).toString();
        const QString root_dir = state.value(QStringLiteral("destination")).toString();
        const QString relative = destination.startsWith(root_dir)
            ? destination.mid(root_dir.size()).replace(QLatin1Char('\\'), QLatin1Char('/'))
            : destination;

        for (int column = 0; column < 5; ++column) {
            table->setItem(index, column, new QTableWidgetItem());
        }

        auto *check = table->item(index, 0);
        check->setFlags(Qt::ItemIsEnabled | Qt::ItemIsUserCheckable | Qt::ItemIsSelectable);
        check->setCheckState(safe && source_exists ? Qt::Checked : Qt::Unchecked);
        check->setData(Qt::UserRole, hash);

        QStringList tags;

        for (const QJsonValue &tag : file.value(QStringLiteral("tags")).toArray()) {
            tags.append(tag.toString());
        }

        QString note = QStringLiteral("ready");

        if (!source_exists) {
            note = QStringLiteral("source missing");
        } else if (!safe) {
            note = QStringLiteral("unsafe filename");
        } else if (exists) {
            note = QStringLiteral("will be updated");
        }

        table->item(index, 1)->setText(hash);
        table->item(index, 2)->setText(tags.join(QStringLiteral(", ")));
        table->item(index, 3)->setText(relative);
        table->item(index, 3)->setToolTip(destination);
        table->item(index, 4)->setText(note);
    }

    layout->addWidget(table, 1);

    auto *buttons = new QHBoxLayout();
    auto *check_all = new QPushButton(QStringLiteral("check all"), &dialog);
    auto *check_none = new QPushButton(QStringLiteral("check none"), &dialog);
    auto *export_checked = new QPushButton(QStringLiteral("export checked"), &dialog);
    auto *close = new QPushButton(QStringLiteral("close"), &dialog);
    buttons->addWidget(check_all);
    buttons->addWidget(check_none);
    buttons->addStretch(1);
    buttons->addWidget(export_checked);
    buttons->addWidget(close);
    layout->addLayout(buttons);

    auto set_all = [table](Qt::CheckState value) {
        for (int index = 0; index < table->rowCount(); ++index) {
            QTableWidgetItem *item = table->item(index, 0);

            if (item != nullptr && (item->flags() & Qt::ItemIsUserCheckable)) {
                item->setCheckState(value);
            }
        }
    };

    connect(check_all, &QPushButton::clicked, &dialog, [set_all]() { set_all(Qt::Checked); });
    connect(check_none, &QPushButton::clicked, &dialog, [set_all]() { set_all(Qt::Unchecked); });
    connect(close, &QPushButton::clicked, &dialog, &QDialog::reject);
    connect(export_checked, &QPushButton::clicked, &dialog, [this, &dialog, table, name]() {
        QJsonArray hashes;

        for (int index = 0; index < table->rowCount(); ++index) {
            QTableWidgetItem *item = table->item(index, 0);

            if (item != nullptr && item->checkState() == Qt::Checked) {
                hashes.append(item->data(Qt::UserRole).toString());
            }
        }

        if (hashes.isEmpty()) {
            QMessageBox::information(&dialog, QStringLiteral("export folder"),
                                     QStringLiteral("No files are checked."));
            return;
        }

        QJsonObject payload;
        payload.insert(QStringLiteral("name"), name);
        payload.insert(QStringLiteral("hashes"), hashes);
        notify(QStringLiteral("export_folder_run"), active_page, json_payload(payload));
        dialog.accept();
    });

    dialog.exec();
}

void HydrusMainWindow::show_options(const QJsonObject &state) {
    HydrusOptionsDialog dialog(this, state, this);
    dialog.exec();
}

void HydrusMainWindow::show_services(const QJsonObject &state) {
    if (services_dialog == nullptr) {
        services_dialog = new HydrusServicesDialog(this, state, this);
        connect(services_dialog, &QObject::destroyed, this, [this]() { services_dialog = nullptr; });
        services_dialog->show();
    } else {
        services_dialog->set_state(state);
        services_dialog->raise();
        services_dialog->activateWindow();
    }
}

void HydrusMainWindow::update_services(const QJsonObject &state) {
    if (services_dialog != nullptr) {
        services_dialog->set_state(state);
    }
}

void HydrusMainWindow::show_tag_filter(const QJsonObject &state) {
    const QString service_key = state.value(QStringLiteral("service_key")).toString();
    const QString name = state.value(QStringLiteral("name")).toString();
    const bool can_purge = state.value(QStringLiteral("can_purge")).toBool(false);

    QDialog dialog(this);
    dialog.setWindowTitle(QStringLiteral("tag filter: ") + name);
    dialog.setObjectName(QStringLiteral("hydrus_tag_filter_dialog"));
    dialog.resize(760, 640);

    auto *layout = new QVBoxLayout(&dialog);

    auto *summary = new QLabel(
        QStringLiteral("current filter: %1").arg(state.value(QStringLiteral("filter_string")).toString()),
        &dialog);
    summary->setWordWrap(true);
    layout->addWidget(summary);

    const QString note = state.value(QStringLiteral("note")).toString();

    if (!note.isEmpty()) {
        auto *note_label = new QLabel(note, &dialog);
        note_label->setWordWrap(true);
        layout->addWidget(note_label);
    }

    auto *explanation = new QLabel(
        QStringLiteral("One tag slice per line. Bullet points are namespaces (\"character\" or \"character:\"); "
                       "\"*\" matches all unnamespaced tags and \"*:*\" matches all namespaced tags. "
                       "Whitelist rules are exceptions to blacklist rules."),
        &dialog);
    explanation->setWordWrap(true);
    layout->addWidget(explanation);

    auto *tabs = new QTabWidget(&dialog);
    auto *whitelist = new QPlainTextEdit(tabs);
    whitelist->setObjectName(QStringLiteral("tag_filter_whitelist"));
    whitelist->setPlaceholderText(QStringLiteral("character\ncreator:someone"));
    auto *blacklist = new QPlainTextEdit(tabs);
    blacklist->setObjectName(QStringLiteral("tag_filter_blacklist"));
    blacklist->setPlaceholderText(QStringLiteral("*\n*:*\nspecies:chat"));
    tabs->addTab(whitelist, QStringLiteral("whitelist"));
    tabs->addTab(blacklist, QStringLiteral("blacklist"));
    layout->addWidget(tabs, 1);

    QStringList whitelist_lines;
    QStringList blacklist_lines;

    for (const QJsonValue &entry : state.value(QStringLiteral("rules")).toArray()) {
        const QJsonObject rule = entry.toObject();
        const QString display = rule.value(QStringLiteral("display")).toString();
        const QString kind = rule.value(QStringLiteral("rule")).toString();

        if (kind == QStringLiteral("whitelist")) {
            whitelist_lines.append(display);
        } else {
            blacklist_lines.append(display);
        }
    }

    whitelist->setPlainText(whitelist_lines.join(QLatin1Char('\n')));
    blacklist->setPlainText(blacklist_lines.join(QLatin1Char('\n')));

    auto collect_rules = [](QPlainTextEdit *edit, const QString &rule) {
        QJsonArray rules;

        for (const QString &line : edit->toPlainText().split(QLatin1Char('\n'))) {
            const QString trimmed = line.trimmed();

            if (trimmed.isEmpty()) {
                continue;
            }

            QString slice = trimmed;

            if (trimmed == QStringLiteral("*")) {
                slice = QString();
            } else if (trimmed == QStringLiteral("*:*") || trimmed == QStringLiteral("*:")) {
                slice = QStringLiteral(":");
            }

            QJsonObject entry;
            entry.insert(QStringLiteral("slice"), slice);
            entry.insert(QStringLiteral("rule"), rule);
            rules.append(entry);
        }

        return rules;
    };

    auto build_payload = [&]() {
        QJsonArray rules = collect_rules(whitelist, QStringLiteral("whitelist"));

        for (const QJsonValue &entry : collect_rules(blacklist, QStringLiteral("blacklist"))) {
            rules.append(entry);
        }

        QJsonObject payload;
        payload.insert(QStringLiteral("service_key"), service_key);
        payload.insert(QStringLiteral("rules"), rules);

        return payload;
    };

    auto *test_row = new QHBoxLayout();
    auto *test_input = new QLineEdit(&dialog);
    test_input->setPlaceholderText(QStringLiteral("character:samus"));
    auto *test_button = new QPushButton(QStringLiteral("test tag"), &dialog);
    test_row->addWidget(new QLabel(QStringLiteral("test:"), &dialog));
    test_row->addWidget(test_input, 1);
    test_row->addWidget(test_button);
    layout->addLayout(test_row);

    connect(test_button, &QPushButton::clicked, this, [this, test_input, build_payload]() {
        QJsonObject payload = build_payload();
        payload.insert(QStringLiteral("tag"), test_input->text().trimmed());
        notify(QStringLiteral("tag_filter_test"), QString(), json_payload(payload));
    });
    connect(test_input, &QLineEdit::returnPressed, test_button, &QPushButton::click);

    auto *purge_tab = new QWidget(&dialog);
    auto *purge_layout = new QVBoxLayout(purge_tab);
    auto *purge_note = new QLabel(
        can_purge
            ? QStringLiteral("Deletes every mapping of these tags from the local tag service.")
            : QStringLiteral("Purging is only available for local tag services in this build."),
        purge_tab);
    purge_note->setWordWrap(true);
    purge_layout->addWidget(purge_note);

    auto *purge_tags = new QPlainTextEdit(purge_tab);
    purge_tags->setPlaceholderText(QStringLiteral("character:samus\ncreator:someone"));
    purge_tags->setEnabled(can_purge);
    purge_layout->addWidget(purge_tags, 1);

    auto *reason_row = new QHBoxLayout();
    auto *purge_reason = new QLineEdit(purge_tab);
    purge_reason->setPlaceholderText(QStringLiteral("Tags purged by janitor."));
    purge_reason->setEnabled(can_purge);
    reason_row->addWidget(new QLabel(QStringLiteral("reason:"), purge_tab));
    reason_row->addWidget(purge_reason, 1);
    purge_layout->addLayout(reason_row);

    auto *purge_button = new QPushButton(QStringLiteral("purge tags"), purge_tab);
    purge_button->setEnabled(can_purge);
    auto *purge_footer = new QHBoxLayout();
    purge_footer->addStretch(1);
    purge_footer->addWidget(purge_button);
    purge_layout->addLayout(purge_footer);
    tabs->addTab(purge_tab, QStringLiteral("purge"));

    connect(purge_button, &QPushButton::clicked, this, [this, service_key, purge_tags, purge_reason, purge_button]() {
        const QStringList tags = purge_tags->toPlainText().split(QLatin1Char('\n'), Qt::SkipEmptyParts);

        if (tags.isEmpty()) {
            QMessageBox::information(this, QStringLiteral("purge tags"), QStringLiteral("Enter tags first."));
            return;
        }

        if (QMessageBox::question(this, QStringLiteral("purge tags"),
                                  QStringLiteral("Purge %1 tag(s) from this service?").arg(tags.size()))
            != QMessageBox::Yes) {
            return;
        }

        QJsonArray list;

        for (const QString &tag : tags) {
            if (!tag.trimmed().isEmpty()) {
                list.append(tag.trimmed());
            }
        }

        QJsonObject payload;
        payload.insert(QStringLiteral("service_key"), service_key);
        payload.insert(QStringLiteral("tags"), list);
        payload.insert(QStringLiteral("reason"), purge_reason->text().trimmed());

        notify(QStringLiteral("purge_tags"), QString(), json_payload(payload));
        purge_tags->clear();
        purge_button->setEnabled(false);
    });

    auto *buttons = new QDialogButtonBox(QDialogButtonBox::Save | QDialogButtonBox::Close, &dialog);
    layout->addWidget(buttons);
    connect(buttons, &QDialogButtonBox::rejected, &dialog, &QDialog::reject);
    connect(buttons, &QDialogButtonBox::accepted, this, [&]() {
        notify(QStringLiteral("tag_filter_save"), QString(), json_payload(build_payload()));
        dialog.accept();
    });

    const QString requested_tab = qEnvironmentVariable("HYDRUS_QT_TAG_FILTER_TAB");

    if (!requested_tab.isEmpty()) {
        for (int index = 0; index < tabs->count(); ++index) {
            if (tabs->tabText(index) == requested_tab) {
                tabs->setCurrentIndex(index);
                break;
            }
        }
    }

    dialog.exec();
}

QString api_permission_slice_display(const QString &slice) {
    if (slice.isEmpty()) {
        return QStringLiteral("*");
    }

    if (slice == QStringLiteral(":")) {
        return QStringLiteral("*:*");
    }

    return slice;
}

void show_api_permission_editor(HydrusMainWindow *main, QWidget *parent, const QJsonObject &data, const QJsonObject &entry) {
    const QString original = entry.value(QStringLiteral("access_key")).toString();

    QDialog dialog(parent);
    dialog.setWindowTitle(original.isEmpty() ? QStringLiteral("add api permissions")
                                             : QStringLiteral("edit api permissions"));
    dialog.setObjectName(QStringLiteral("hydrus_api_permission_editor"));
    dialog.resize(680, 620);

    auto *layout = new QVBoxLayout(&dialog);
    auto *form = new QFormLayout();

    auto *name = new QLineEdit(entry.value(QStringLiteral("name")).toString(), &dialog);
    auto *access_key = new QLineEdit(original, &dialog);
    access_key->setReadOnly(true);
    auto *generate = new QPushButton(QStringLiteral("generate new key"), &dialog);

    auto *key_row = new QHBoxLayout();
    key_row->addWidget(access_key, 1);
    key_row->addWidget(generate);

    form->addRow(QStringLiteral("name"), name);
    form->addRow(QStringLiteral("access key"), key_row);
    layout->addLayout(form);

    QObject::connect(generate, &QPushButton::clicked, &dialog, [access_key]() {
        QString key;

        for (int index = 0; index < 4; ++index) {
            key += QStringLiteral("%1").arg(QRandomGenerator::system()->generate64(), 16, 16, QLatin1Char('0'));
        }

        access_key->setText(key);
    });

    auto *permits_everything = new QCheckBox(QStringLiteral("permits everything"), &dialog);
    permits_everything->setChecked(entry.value(QStringLiteral("permits_everything")).toBool(true));
    layout->addWidget(permits_everything);

    auto *permission_list = new QListWidget(&dialog);
    permission_list->setObjectName(QStringLiteral("api_permission_list"));

    QJsonArray existing_permissions = entry.value(QStringLiteral("basic_permissions")).toArray();
    QSet<int> checked_permissions;

    for (const QJsonValue &value : existing_permissions) {
        checked_permissions.insert(value.toInt());
    }

    for (const QJsonValue &value : data.value(QStringLiteral("available_permissions")).toArray()) {
        const QJsonObject permission = value.toObject();
        auto *item = new QListWidgetItem(permission.value(QStringLiteral("label")).toString(), permission_list);
        item->setData(Qt::UserRole, permission.value(QStringLiteral("value")).toInt());
        item->setFlags(item->flags() | Qt::ItemIsUserCheckable);
        item->setCheckState(checked_permissions.contains(permission.value(QStringLiteral("value")).toInt())
                                ? Qt::Checked
                                : Qt::Unchecked);
    }

    layout->addWidget(new QLabel(QStringLiteral("basic permissions"), &dialog));
    layout->addWidget(permission_list, 1);

    auto *tabs = new QTabWidget(&dialog);
    auto *whitelist = new QPlainTextEdit(tabs);
    whitelist->setPlaceholderText(QStringLiteral("character:"));
    auto *blacklist = new QPlainTextEdit(tabs);
    blacklist->setPlaceholderText(QStringLiteral("*\n*:*"));

    QStringList whitelist_lines;
    QStringList blacklist_lines;

    for (const QJsonValue &value : entry.value(QStringLiteral("search_tag_filter")).toArray()) {
        const QJsonArray pair = value.toArray();

        if (pair.size() != 2) {
            continue;
        }

        const QString display = api_permission_slice_display(pair.at(0).toString());

        if (pair.at(1).toString() == QStringLiteral("whitelist")) {
            whitelist_lines.append(display);
        } else {
            blacklist_lines.append(display);
        }
    }

    whitelist->setPlainText(whitelist_lines.join(QLatin1Char('\n')));
    blacklist->setPlainText(blacklist_lines.join(QLatin1Char('\n')));
    tabs->addTab(whitelist, QStringLiteral("allowed search tags"));
    tabs->addTab(blacklist, QStringLiteral("denied search tags"));
    layout->addWidget(tabs, 1);

    auto collect_rules = [](QPlainTextEdit *edit, const QString &rule) {
        QJsonArray rules;

        for (const QString &line : edit->toPlainText().split(QLatin1Char('\n'))) {
            const QString trimmed = line.trimmed();

            if (trimmed.isEmpty()) {
                continue;
            }

            QString slice = trimmed;

            if (trimmed == QStringLiteral("*")) {
                slice = QString();
            } else if (trimmed == QStringLiteral("*:*") || trimmed == QStringLiteral("*:")) {
                slice = QStringLiteral(":");
            }

            rules.append(QJsonArray{slice, rule});
        }

        return rules;
    };

    auto update_enabled = [permits_everything, permission_list]() {
        permission_list->setEnabled(!permits_everything->isChecked());
    };
    QObject::connect(permits_everything, &QCheckBox::clicked, &dialog, [update_enabled](bool) { update_enabled(); });
    update_enabled();

    auto *buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, &dialog);
    layout->addWidget(buttons);
    QObject::connect(buttons, &QDialogButtonBox::rejected, &dialog, &QDialog::reject);
    QObject::connect(buttons, &QDialogButtonBox::accepted, &dialog, [&]() {
        if (name->text().trimmed().isEmpty()) {
            QMessageBox::warning(&dialog, QStringLiteral("api permissions"),
                                 QStringLiteral("Enter a name for this access key."));
            return;
        }

        if (access_key->text().trimmed().size() != 64) {
            QMessageBox::warning(&dialog, QStringLiteral("api permissions"),
                                 QStringLiteral("The access key must be 64 hex characters."));
            return;
        }

        dialog.accept();
    });

    if (dialog.exec() != QDialog::Accepted) {
        return;
    }

    QJsonArray permissions;

    for (int index = 0; index < permission_list->count(); ++index) {
        QListWidgetItem *item = permission_list->item(index);

        if (item->checkState() == Qt::Checked) {
            permissions.append(item->data(Qt::UserRole).toInt());
        }
    }

    QJsonArray rules = collect_rules(whitelist, QStringLiteral("whitelist"));

    for (const QJsonValue &rule : collect_rules(blacklist, QStringLiteral("blacklist"))) {
        rules.append(rule);
    }

    QJsonObject payload;
    payload.insert(QStringLiteral("name"), name->text().trimmed());
    payload.insert(QStringLiteral("access_key"), access_key->text().trimmed().toLower());
    payload.insert(QStringLiteral("original_access_key"), original);
    payload.insert(QStringLiteral("permits_everything"), permits_everything->isChecked());
    payload.insert(QStringLiteral("basic_permissions"), permissions);
    payload.insert(QStringLiteral("rules"), rules);

    main->notify(QStringLiteral("api_permissions_save"), QString(), json_payload(payload));
}

HydrusApiPermissionsDialog::HydrusApiPermissionsDialog(HydrusMainWindow *main_window, const QJsonObject &state, QWidget *parent)
    : QDialog(parent), main(main_window) {
    setWindowTitle(QStringLiteral("client api permissions"));
    setObjectName(QStringLiteral("hydrus_api_permissions_dialog"));
    setAttribute(Qt::WA_DeleteOnClose, true);
    resize(860, 480);

    auto *layout = new QVBoxLayout(this);

    status = new QLabel(this);
    status->setWordWrap(true);
    layout->addWidget(status);

    table = new QTableWidget(this);
    table->setColumnCount(3);
    table->setHorizontalHeaderLabels({
        QStringLiteral("name"),
        QStringLiteral("access key"),
        QStringLiteral("permissions")
    });
    table->horizontalHeader()->setSectionResizeMode(0, QHeaderView::ResizeToContents);
    table->horizontalHeader()->setSectionResizeMode(1, QHeaderView::Stretch);
    table->horizontalHeader()->setSectionResizeMode(2, QHeaderView::Stretch);
    table->setSelectionBehavior(QAbstractItemView::SelectRows);
    table->setSelectionMode(QAbstractItemView::SingleSelection);
    table->setEditTriggers(QAbstractItemView::NoEditTriggers);
    table->verticalHeader()->setVisible(false);
    layout->addWidget(table, 1);

    auto *buttons = new QHBoxLayout();
    auto *add = new QPushButton(QStringLiteral("add access key..."), this);
    auto *edit = new QPushButton(QStringLiteral("edit..."), this);
    auto *remove = new QPushButton(QStringLiteral("delete"), this);
    buttons->addWidget(add);
    buttons->addWidget(edit);
    buttons->addWidget(remove);
    buttons->addStretch(1);
    layout->addLayout(buttons);

    connect(add, &QPushButton::clicked, this, [this]() { add_entry(); });
    connect(edit, &QPushButton::clicked, this, [this]() { edit_selected(); });
    connect(remove, &QPushButton::clicked, this, [this]() { delete_selected(); });
    connect(table, &QTableWidget::cellDoubleClicked, this, [this](int, int) { edit_selected(); });

    auto *close = new QPushButton(QStringLiteral("close"), this);
    auto *footer = new QHBoxLayout();
    footer->addStretch(1);
    footer->addWidget(close);
    layout->addLayout(footer);
    connect(close, &QPushButton::clicked, this, &QDialog::close);

    set_state(state);

    const QString edit_key = qEnvironmentVariable("HYDRUS_QT_API_EDIT");

    if (edit_key == QStringLiteral("new")) {
        add_entry();
    } else if (!edit_key.isEmpty()) {
        for (int row = 0; row < table->rowCount(); ++row) {
            if (table->item(row, 1) != nullptr
                && table->item(row, 1)->text().compare(edit_key, Qt::CaseInsensitive) == 0) {
                table->selectRow(row);
                edit_selected();
                break;
            }
        }
    }
}

void HydrusApiPermissionsDialog::set_state(const QJsonObject &state) {
    data = state;
    status->setText(QStringLiteral("api: %1").arg(state.value(QStringLiteral("api_status")).toString()));

    const QJsonArray permissions = state.value(QStringLiteral("permissions")).toArray();
    table->setRowCount(permissions.size());

    for (int index = 0; index < permissions.size(); ++index) {
        const QJsonObject entry = permissions.at(index).toObject();

        for (int column = 0; column < 3; ++column) {
            if (table->item(index, column) == nullptr) {
                table->setItem(index, column, new QTableWidgetItem());
            }
        }

        auto *name_item = table->item(index, 0);
        name_item->setText(entry.value(QStringLiteral("name")).toString());
        name_item->setData(Qt::UserRole, entry);
        table->item(index, 1)->setText(entry.value(QStringLiteral("access_key")).toString());
        table->item(index, 2)->setText(entry.value(QStringLiteral("summary")).toString());
    }

    table->resizeRowsToContents();
}

QJsonObject HydrusApiPermissionsDialog::selected_entry() const {
    const int row = table->currentRow();

    if (row < 0 || table->item(row, 0) == nullptr) {
        return {};
    }

    return table->item(row, 0)->data(Qt::UserRole).toJsonObject();
}

void HydrusApiPermissionsDialog::add_entry() {
    show_api_permission_editor(main, this, data, QJsonObject());
}

void HydrusApiPermissionsDialog::edit_selected() {
    const QJsonObject entry = selected_entry();

    if (!entry.isEmpty()) {
        show_api_permission_editor(main, this, data, entry);
    }
}

void HydrusApiPermissionsDialog::delete_selected() {
    const QJsonObject entry = selected_entry();

    if (entry.isEmpty()) {
        return;
    }

    if (QMessageBox::question(
            this,
            QStringLiteral("delete access key"),
            QStringLiteral("Delete '%1'? Anything using that key will lose access.")
                .arg(entry.value(QStringLiteral("name")).toString()))
        != QMessageBox::Yes) {
        return;
    }

    main->notify(QStringLiteral("api_permissions_delete"), QString(),
                 entry.value(QStringLiteral("access_key")).toString());
}

void HydrusMainWindow::show_api_permissions(const QJsonObject &state) {
    if (api_permissions_dialog == nullptr) {
        api_permissions_dialog = new HydrusApiPermissionsDialog(this, state, this);
        connect(api_permissions_dialog, &QObject::destroyed, this, [this]() { api_permissions_dialog = nullptr; });
        api_permissions_dialog->show();
    } else {
        api_permissions_dialog->set_state(state);
        api_permissions_dialog->raise();
        api_permissions_dialog->activateWindow();
    }
}

void HydrusMainWindow::update_api_permissions(const QJsonObject &state) {
    if (api_permissions_dialog != nullptr) {
        api_permissions_dialog->set_state(state);
    }
}

void show_account_type_editor(HydrusMainWindow *main, QWidget *parent, const QJsonObject &data, const QJsonObject &entry) {
    const bool adding = entry.isEmpty();
    const QString key = entry.value(QStringLiteral("key")).toString();

    QDialog dialog(parent);
    dialog.setWindowTitle(adding ? QStringLiteral("add account type") : QStringLiteral("edit account type"));
    dialog.setObjectName(QStringLiteral("hydrus_account_type_editor"));
    dialog.resize(640, 620);

    auto *layout = new QVBoxLayout(&dialog);
    auto *form = new QFormLayout();

    auto *title = new QLineEdit(entry.value(QStringLiteral("title")).toString(), &dialog);
    form->addRow(QStringLiteral("title"), title);

    auto *accounts = new QSpinBox(&dialog);
    accounts->setRange(0, 1000000);
    accounts->setValue(entry.value(QStringLiteral("auto_create_accounts")).toInt(0));
    auto *time_delta = new QSpinBox(&dialog);
    time_delta->setRange(60, 365 * 86400);
    time_delta->setValue(entry.value(QStringLiteral("auto_create_time_delta")).toInt(86400));
    time_delta->setSuffix(QStringLiteral(" s"));

    auto *velocity_row = new QHBoxLayout();
    velocity_row->addWidget(accounts);
    velocity_row->addWidget(new QLabel(QStringLiteral("account(s) per"), &dialog));
    velocity_row->addWidget(time_delta);
    form->addRow(QStringLiteral("auto-creation"), velocity_row);
    layout->addLayout(form);

    QHash<int, QComboBox *> permission_controls;
    QJsonArray entry_permissions = entry.value(QStringLiteral("permissions")).toArray();
    QHash<int, int> current_actions;

    for (const QJsonValue &value : entry_permissions) {
        const QJsonArray pair = value.toArray();

        if (pair.size() == 2) {
            current_actions.insert(pair.at(0).toInt(), pair.at(1).isNull() ? -1 : pair.at(1).toInt());
        }
    }

    auto *permissions_box = new QGroupBox(QStringLiteral("permissions"), &dialog);
    auto *permissions_form = new QFormLayout(permissions_box);

    for (const QJsonValue &value : data.value(QStringLiteral("possible")).toArray()) {
        const QJsonObject content_type = value.toObject();
        auto *combo = new QComboBox(permissions_box);
        combo->setObjectName(QStringLiteral("account_type_permission_%1")
                                 .arg(content_type.value(QStringLiteral("content_type")).toInt()));

        for (const QJsonValue &action_value : content_type.value(QStringLiteral("actions")).toArray()) {
            const QJsonObject action = action_value.toObject();
            const QJsonValue action_number = action.value(QStringLiteral("value"));

            combo->addItem(action.value(QStringLiteral("label")).toString(),
                           action_number.isNull() ? -1 : action_number.toInt());
        }

        const int wanted = current_actions.value(content_type.value(QStringLiteral("content_type")).toInt(), -1);
        const int index = combo->findData(wanted);
        combo->setCurrentIndex(index >= 0 ? index : 0);
        permissions_form->addRow(content_type.value(QStringLiteral("label")).toString(), combo);
        permission_controls.insert(content_type.value(QStringLiteral("content_type")).toInt(), combo);
    }

    layout->addWidget(permissions_box, 1);

    auto *note = new QLabel(
        QStringLiteral("Bandwidth rules and auto-creation history are preserved. "
                       "Permissions without an entry mean no permission."),
        &dialog);
    note->setWordWrap(true);
    layout->addWidget(note);

    auto *buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, &dialog);
    layout->addWidget(buttons);
    QObject::connect(buttons, &QDialogButtonBox::rejected, &dialog, &QDialog::reject);
    QObject::connect(buttons, &QDialogButtonBox::accepted, &dialog, [&]() {
        if (title->text().trimmed().isEmpty()) {
            QMessageBox::warning(&dialog, QStringLiteral("account type"),
                                 QStringLiteral("Enter a title."));
            return;
        }

        dialog.accept();
    });

    if (dialog.exec() != QDialog::Accepted) {
        return;
    }

    QJsonArray permissions;

    for (auto iterator = permission_controls.constBegin(); iterator != permission_controls.constEnd(); ++iterator) {
        const int action = iterator.value()->currentData().toInt();

        if (action < 0) {
            continue;
        }

        permissions.append(QJsonArray{iterator.key(), action});
    }

    QJsonObject payload;
    payload.insert(QStringLiteral("account_type_key"), key);
    payload.insert(QStringLiteral("title"), title->text().trimmed());
    payload.insert(QStringLiteral("permissions"), permissions);
    payload.insert(QStringLiteral("auto_create_accounts"), accounts->value());
    payload.insert(QStringLiteral("auto_create_time_delta"), time_delta->value());

    main->notify(QStringLiteral("account_type_save"), QString(), json_payload(payload));
}

HydrusAccountTypesDialog::HydrusAccountTypesDialog(HydrusMainWindow *main_window, const QJsonObject &state, QWidget *parent)
    : QDialog(parent), main(main_window) {
    setWindowTitle(QStringLiteral("account types"));
    setObjectName(QStringLiteral("hydrus_account_types_dialog"));
    setAttribute(Qt::WA_DeleteOnClose, true);
    resize(820, 480);

    auto *layout = new QVBoxLayout(this);

    table = new QTableWidget(this);
    table->setColumnCount(3);
    table->setHorizontalHeaderLabels({
        QStringLiteral("title"),
        QStringLiteral("permissions"),
        QStringLiteral("auto-creation")
    });
    table->horizontalHeader()->setSectionResizeMode(0, QHeaderView::ResizeToContents);
    table->horizontalHeader()->setSectionResizeMode(1, QHeaderView::Stretch);
    table->horizontalHeader()->setSectionResizeMode(2, QHeaderView::ResizeToContents);
    table->setSelectionBehavior(QAbstractItemView::SelectRows);
    table->setSelectionMode(QAbstractItemView::SingleSelection);
    table->setEditTriggers(QAbstractItemView::NoEditTriggers);
    table->verticalHeader()->setVisible(false);
    layout->addWidget(table, 1);

    auto *buttons = new QHBoxLayout();
    auto *add = new QPushButton(QStringLiteral("add account type..."), this);
    auto *edit = new QPushButton(QStringLiteral("edit..."), this);
    auto *remove = new QPushButton(QStringLiteral("delete"), this);
    buttons->addWidget(add);
    buttons->addWidget(edit);
    buttons->addWidget(remove);
    buttons->addStretch(1);
    layout->addLayout(buttons);

    connect(add, &QPushButton::clicked, this, [this]() { add_entry(); });
    connect(edit, &QPushButton::clicked, this, [this]() { edit_selected(); });
    connect(remove, &QPushButton::clicked, this, [this]() { delete_selected(); });
    connect(table, &QTableWidget::cellDoubleClicked, this, [this](int, int) { edit_selected(); });

    auto *close = new QPushButton(QStringLiteral("close"), this);
    auto *footer = new QHBoxLayout();
    footer->addStretch(1);
    footer->addWidget(close);
    layout->addLayout(footer);
    connect(close, &QPushButton::clicked, this, &QDialog::close);

    set_state(state);

    const QString edit_key = qEnvironmentVariable("HYDRUS_QT_ACCOUNT_TYPE_EDIT");

    if (edit_key == QStringLiteral("new")) {
        add_entry();
    } else if (!edit_key.isEmpty()) {
        for (int row = 0; row < table->rowCount(); ++row) {
            if (table->item(row, 0) != nullptr
                && table->item(row, 0)->data(Qt::UserRole).toJsonObject().value(QStringLiteral("key")).toString()
                       == edit_key) {
                table->selectRow(row);
                edit_selected();
                break;
            }
        }
    }
}

void HydrusAccountTypesDialog::set_state(const QJsonObject &state) {
    data = state;

    const QJsonArray types = state.value(QStringLiteral("types")).toArray();
    table->setRowCount(types.size());

    for (int index = 0; index < types.size(); ++index) {
        const QJsonObject entry = types.at(index).toObject();

        for (int column = 0; column < 3; ++column) {
            if (table->item(index, column) == nullptr) {
                table->setItem(index, column, new QTableWidgetItem());
            }
        }

        auto *title_item = table->item(index, 0);
        title_item->setText(entry.value(QStringLiteral("title")).toString());
        title_item->setData(Qt::UserRole, entry);

        if (entry.value(QStringLiteral("is_null")).toBool()) {
            title_item->setForeground(QColor(120, 150, 190));
        }

        table->item(index, 1)->setText(entry.value(QStringLiteral("summary")).toString());

        const int accounts = entry.value(QStringLiteral("auto_create_accounts")).toInt();
        table->item(index, 2)->setText(accounts == 0
                                           ? QStringLiteral("disabled")
                                           : QStringLiteral("%1 / %2 s")
                                                 .arg(accounts)
                                                 .arg(entry.value(QStringLiteral("auto_create_time_delta")).toInt()));
    }

    table->resizeRowsToContents();
}

QJsonObject HydrusAccountTypesDialog::selected_entry() const {
    const int row = table->currentRow();

    if (row < 0 || table->item(row, 0) == nullptr) {
        return {};
    }

    return table->item(row, 0)->data(Qt::UserRole).toJsonObject();
}

void HydrusAccountTypesDialog::add_entry() {
    show_account_type_editor(main, this, data, QJsonObject());
}

void HydrusAccountTypesDialog::edit_selected() {
    const QJsonObject entry = selected_entry();

    if (!entry.isEmpty()) {
        if (entry.value(QStringLiteral("is_null")).toBool()) {
            QMessageBox::information(this, QStringLiteral("account type"),
                                     QStringLiteral("The null account type cannot be edited."));
            return;
        }

        show_account_type_editor(main, this, data, entry);
    }
}

void HydrusAccountTypesDialog::delete_selected() {
    const QJsonObject entry = selected_entry();

    if (entry.isEmpty()) {
        return;
    }

    if (entry.value(QStringLiteral("is_null")).toBool()) {
        QMessageBox::information(this, QStringLiteral("account type"),
                                 QStringLiteral("The null account type cannot be deleted."));
        return;
    }

    QStringList titles;
    QStringList keys;

    for (const QJsonValue &value : data.value(QStringLiteral("types")).toArray()) {
        const QJsonObject other = value.toObject();

        if (other.value(QStringLiteral("key")).toString() == entry.value(QStringLiteral("key")).toString()
            || other.value(QStringLiteral("is_null")).toBool()) {
            continue;
        }

        titles.append(other.value(QStringLiteral("title")).toString());
        keys.append(other.value(QStringLiteral("key")).toString());
    }

    if (titles.isEmpty()) {
        QMessageBox::information(this, QStringLiteral("account type"),
                                 QStringLiteral("There is no other account type to move accounts to."));
        return;
    }

    bool ok = false;
    const QString title = QInputDialog::getItem(
        this, QStringLiteral("delete account type"),
        QStringLiteral("Move its accounts to:"), titles, 0, false, &ok);

    if (!ok) {
        return;
    }

    QJsonObject payload;
    payload.insert(QStringLiteral("account_type_key"), entry.value(QStringLiteral("key")));
    payload.insert(QStringLiteral("replacement_account_type_key"), keys.at(titles.indexOf(title)));

    main->notify(QStringLiteral("account_type_delete"), QString(), json_payload(payload));
}

void HydrusMainWindow::show_account_types(const QJsonObject &state) {
    if (account_types_dialog == nullptr) {
        account_types_dialog = new HydrusAccountTypesDialog(this, state, this);
        connect(account_types_dialog, &QObject::destroyed, this, [this]() { account_types_dialog = nullptr; });
        account_types_dialog->show();
    } else {
        account_types_dialog->set_state(state);
        account_types_dialog->raise();
        account_types_dialog->activateWindow();
    }
}

void HydrusMainWindow::update_account_types(const QJsonObject &state) {
    if (account_types_dialog != nullptr) {
        account_types_dialog->set_state(state);
    }
}

HydrusOrphanFilesDialog::HydrusOrphanFilesDialog(HydrusMainWindow *main_window, const QJsonObject &state, QWidget *parent)
    : QDialog(parent), main(main_window) {
    setWindowTitle(QStringLiteral("clear orphan files"));
    setObjectName(QStringLiteral("hydrus_orphan_files_dialog"));
    setAttribute(Qt::WA_DeleteOnClose, true);
    resize(880, 520);

    auto *layout = new QVBoxLayout(this);

    auto *help = new QLabel(
        QStringLiteral("Files in the media storage whose hash is not in the database. "
                       "Thumbnails are included. Anything that is not named after a hash is left alone."),
        this);
    help->setWordWrap(true);
    layout->addWidget(help);

    status = new QLabel(this);
    status->setObjectName(QStringLiteral("orphan_files_status"));
    layout->addWidget(status);

    table = new QTableWidget(this);
    table->setColumnCount(4);
    table->setHorizontalHeaderLabels({
        QStringLiteral("kind"),
        QStringLiteral("size"),
        QStringLiteral("hash"),
        QStringLiteral("path")
    });
    table->horizontalHeader()->setSectionResizeMode(0, QHeaderView::ResizeToContents);
    table->horizontalHeader()->setSectionResizeMode(1, QHeaderView::ResizeToContents);
    table->horizontalHeader()->setSectionResizeMode(2, QHeaderView::Interactive);
    table->setColumnWidth(2, 260);
    table->horizontalHeader()->setSectionResizeMode(3, QHeaderView::Stretch);
    table->setSelectionBehavior(QAbstractItemView::SelectRows);
    table->setSelectionMode(QAbstractItemView::ExtendedSelection);
    table->setEditTriggers(QAbstractItemView::NoEditTriggers);
    table->setWordWrap(false);
    table->verticalHeader()->setVisible(false);
    layout->addWidget(table, 1);

    auto *buttons = new QHBoxLayout();
    auto *rescan = new QPushButton(QStringLiteral("scan again"), this);
    auto *delete_selected_button = new QPushButton(QStringLiteral("delete selected"), this);
    auto *delete_all_button = new QPushButton(QStringLiteral("delete all"), this);
    auto *move_selected_button = new QPushButton(QStringLiteral("move selected to..."), this);
    auto *move_all_button = new QPushButton(QStringLiteral("move all to..."), this);
    buttons->addWidget(rescan);
    buttons->addWidget(delete_selected_button);
    buttons->addWidget(delete_all_button);
    buttons->addWidget(move_selected_button);
    buttons->addWidget(move_all_button);
    buttons->addStretch(1);
    layout->addLayout(buttons);

    connect(rescan, &QPushButton::clicked, this, [this]() {
        main->notify(QStringLiteral("orphan_files_scan"));
    });
    connect(delete_selected_button, &QPushButton::clicked, this, [this]() { delete_selected(false); });
    connect(delete_all_button, &QPushButton::clicked, this, [this]() { delete_selected(true); });
    connect(move_selected_button, &QPushButton::clicked, this, [this]() { move_selected(false); });
    connect(move_all_button, &QPushButton::clicked, this, [this]() { move_selected(true); });

    auto *close = new QPushButton(QStringLiteral("close"), this);
    auto *footer = new QHBoxLayout();
    footer->addStretch(1);
    footer->addWidget(close);
    layout->addLayout(footer);
    connect(close, &QPushButton::clicked, this, &QDialog::close);

    set_state(state);
}

void HydrusOrphanFilesDialog::set_state(const QJsonObject &state) {
    data = state;

    const QJsonArray orphans = state.value(QStringLiteral("orphans")).toArray();
    const int previous_row = table->currentRow();
    table->setRowCount(orphans.size());

    for (int index = 0; index < orphans.size(); ++index) {
        const QJsonObject entry = orphans.at(index).toObject();

        for (int column = 0; column < 4; ++column) {
            if (table->item(index, column) == nullptr) {
                table->setItem(index, column, new QTableWidgetItem());
            }
        }

        auto *kind = table->item(index, 0);
        kind->setText(entry.value(QStringLiteral("kind")).toString());
        kind->setData(Qt::UserRole, entry.value(QStringLiteral("path")).toString());
        table->item(index, 1)->setText(
            entry.value(QStringLiteral("size_display")).toString());
        table->item(index, 2)->setText(entry.value(QStringLiteral("hash")).toString());
        table->item(index, 3)->setText(entry.value(QStringLiteral("path")).toString());
    }

    if (previous_row >= 0 && previous_row < table->rowCount()) {
        table->selectRow(previous_row);
    }

    table->resizeRowsToContents();
    status->setText(QStringLiteral("%1 — %2 file(s), %3 thumbnail(s), %4")
                        .arg(state.value(QStringLiteral("status")).toString())
                        .arg(state.value(QStringLiteral("files")).toInt())
                        .arg(state.value(QStringLiteral("thumbnails")).toInt())
                        .arg(state.value(QStringLiteral("bytes_display")).toString()));
}

QStringList HydrusOrphanFilesDialog::selected_paths(bool all) const {
    QStringList paths;

    if (all) {
        for (int row = 0; row < table->rowCount(); ++row) {
            paths.append(table->item(row, 0)->data(Qt::UserRole).toString());
        }

        return paths;
    }

    for (const QModelIndex &index : table->selectionModel()->selectedRows()) {
        paths.append(table->item(index.row(), 0)->data(Qt::UserRole).toString());
    }

    return paths;
}

void HydrusOrphanFilesDialog::delete_selected(bool all) {
    const QStringList paths = selected_paths(all);

    if (paths.isEmpty()) {
        QMessageBox::information(this, QStringLiteral("clear orphan files"),
                                 QStringLiteral("Select the orphan files to delete first."));
        return;
    }

    if (QMessageBox::question(this, QStringLiteral("delete orphan files"),
                              QStringLiteral("Permanently delete %1 orphan file(s)? This cannot be undone.")
                                  .arg(paths.size()))
        != QMessageBox::Yes) {
        return;
    }

    QJsonObject payload;
    payload.insert(QStringLiteral("paths"), QJsonArray::fromStringList(paths));
    main->notify(QStringLiteral("orphan_files_delete"), QString(), json_payload(payload));
}

void HydrusOrphanFilesDialog::move_selected(bool all) {
    const QStringList paths = selected_paths(all);

    if (paths.isEmpty()) {
        QMessageBox::information(this, QStringLiteral("clear orphan files"),
                                 QStringLiteral("Select the orphan files to move first."));
        return;
    }

    const QString directory = QFileDialog::getExistingDirectory(
        this, QStringLiteral("move orphan files to"),
        QStandardPaths::writableLocation(QStandardPaths::DownloadLocation));

    if (directory.isEmpty()) {
        return;
    }

    QJsonObject payload;
    payload.insert(QStringLiteral("paths"), QJsonArray::fromStringList(paths));
    payload.insert(QStringLiteral("target"), directory);
    main->notify(QStringLiteral("orphan_files_move"), QString(), json_payload(payload));
}

void HydrusMainWindow::show_orphan_files(const QJsonObject &state) {
    if (orphan_files_dialog == nullptr) {
        orphan_files_dialog = new HydrusOrphanFilesDialog(this, state, this);
        connect(orphan_files_dialog, &QObject::destroyed, this, [this]() { orphan_files_dialog = nullptr; });
        orphan_files_dialog->show();
    } else {
        orphan_files_dialog->set_state(state);
        orphan_files_dialog->raise();
        orphan_files_dialog->activateWindow();
    }
}

void HydrusMainWindow::update_orphan_files(const QJsonObject &state) {
    if (orphan_files_dialog != nullptr) {
        orphan_files_dialog->set_state(state);
    }
}

HydrusDatabaseBackupDialog::HydrusDatabaseBackupDialog(HydrusMainWindow *main_window, const QJsonObject &state, QWidget *parent)
    : QDialog(parent), main(main_window) {
    setWindowTitle(QStringLiteral("database backup"));
    setObjectName(QStringLiteral("hydrus_database_backup_dialog"));
    setAttribute(Qt::WA_DeleteOnClose, true);
    resize(640, 260);

    auto *layout = new QVBoxLayout(this);

    auto *help = new QLabel(
        QStringLiteral("A consistent copy of client.db is written as client.db in the backup directory. "
                       "The existing copy is replaced."),
        this);
    help->setWordWrap(true);
    layout->addWidget(help);

    auto *form = new QFormLayout();
    auto *path_row = new QHBoxLayout();
    path = new QLineEdit(this);
    path->setReadOnly(true);
    auto *change = new QPushButton(QStringLiteral("change path..."), this);
    path_row->addWidget(path, 1);
    path_row->addWidget(change);
    form->addRow(QStringLiteral("backup directory"), path_row);
    last_backup = new QLabel(this);
    form->addRow(QStringLiteral("last backup"), last_backup);
    layout->addLayout(form);

    status = new QLabel(this);
    status->setWordWrap(true);
    layout->addWidget(status);
    layout->addStretch(1);

    auto *buttons = new QHBoxLayout();
    auto *run = new QPushButton(QStringLiteral("back up now"), this);
    auto *close = new QPushButton(QStringLiteral("close"), this);
    buttons->addWidget(run);
    buttons->addStretch(1);
    buttons->addWidget(close);
    layout->addLayout(buttons);

    connect(change, &QPushButton::clicked, this, [this]() { choose_path(); });
    connect(run, &QPushButton::clicked, this, [this]() {
        if (QMessageBox::question(this, QStringLiteral("database backup"),
                                  QStringLiteral("Back up the database now?"))
            == QMessageBox::Yes) {
            main->notify(QStringLiteral("db_backup_run"));
        }
    });
    connect(close, &QPushButton::clicked, this, &QDialog::close);

    set_state(state);
}

void HydrusDatabaseBackupDialog::set_state(const QJsonObject &state) {
    const QString directory = state.value(QStringLiteral("path")).toString();
    path->setText(directory.isEmpty() ? QStringLiteral("(not set)") : directory);
    path->setCursorPosition(0);
    last_backup->setText(state.value(QStringLiteral("last_backup")).toString());

    const QString target = state.value(QStringLiteral("target")).toString();
    status->setText(target.isEmpty()
                        ? QStringLiteral("Set a backup directory to enable backups.")
                        : QStringLiteral("target: %1").arg(target));
}

void HydrusDatabaseBackupDialog::choose_path() {
    const QString directory = QFileDialog::getExistingDirectory(
        this, QStringLiteral("backup directory"),
        QStandardPaths::writableLocation(QStandardPaths::DocumentsLocation));

    if (!directory.isEmpty()) {
        main->notify(QStringLiteral("db_backup_set_path"), QString(), directory);
    }
}

void HydrusMainWindow::show_database_backup(const QJsonObject &state) {
    if (database_backup_dialog == nullptr) {
        database_backup_dialog = new HydrusDatabaseBackupDialog(this, state, this);
        connect(database_backup_dialog, &QObject::destroyed, this, [this]() { database_backup_dialog = nullptr; });
        database_backup_dialog->show();
    } else {
        database_backup_dialog->set_state(state);
        database_backup_dialog->raise();
        database_backup_dialog->activateWindow();
    }
}

void HydrusMainWindow::update_database_backup(const QJsonObject &state) {
    if (database_backup_dialog != nullptr) {
        database_backup_dialog->set_state(state);
    }
}

void HydrusMainWindow::show_network(const QJsonObject &state) {
    if (network_dialog == nullptr) {
        network_dialog = new HydrusNetworkDialog(this, state, this);
        connect(network_dialog, &QObject::destroyed, this, [this]() { network_dialog = nullptr; });
        network_dialog->show();
    } else {
        network_dialog->set_state(state);
        network_dialog->raise();
        network_dialog->activateWindow();
    }
}

void HydrusMainWindow::update_network(const QJsonObject &state) {
    if (network_dialog != nullptr) {
        network_dialog->set_state(state);
    }
}

void HydrusMainWindow::start_tick() {
    if (tick_timer != nullptr) {
        return;
    }

    tick_timer = new QTimer(this);
    tick_timer->setInterval(120);
    connect(tick_timer, &QTimer::timeout, this, [this]() { app_tick(*this); });
    tick_timer->start();
}

void HydrusMainWindow::show_message(const QString &title, const QString &text) {
    QMessageBox::information(this, title, text);
}

void HydrusMainWindow::notify_message(const QString &message) {
    if (status_message != nullptr) {
        status_message->setText(message);
    }

    if (tray != nullptr && tray->isVisible()) {
        tray->showMessage(
            QStringLiteral("Hydrus Client"),
            message,
            QSystemTrayIcon::Information,
            3000);
    }
}

void HydrusMainWindow::schedule_screenshot(const QString &path) {
    int delay = qEnvironmentVariableIntValue("HYDRUS_QT_SCREENSHOT_DELAY_MS");

    if (delay <= 0) {
        delay = 1400;
    }

    QTimer::singleShot(delay, this, [this, path]() {
        QWidget *target = QApplication::activeWindow();

        if (target == nullptr) {
            target = this;
        }

        for (HydrusViewerWindow *viewer : viewers) {
            if (viewer != nullptr && viewer->isVisible()) {
                target = viewer;
                break;
            }
        }

        const bool ok = target->grab().save(path);
        QCoreApplication::quit();

        if (!ok) {
            qWarning("could not save screenshot to %s", path.toUtf8().constData());
        }
    });
}

// ---------------------------------------------------------------------------
// Free C++ bridge functions
// ---------------------------------------------------------------------------

QApplication *new_application_impl() {
    static int argc = 1;
    static char name[] = "hydrus_client";
    static char *argv[] = {name, nullptr};
    return new QApplication(argc, argv);
}

std::unique_ptr<QApplication> new_application() {
    return std::unique_ptr<QApplication>(new_application_impl());
}

int exec_application(QApplication &application) {
    return application.exec();
}

void quit_application(QApplication &application) {
    application.quit();
}

void qapp_process_events() {
    QApplication::processEvents();
}

std::unique_ptr<QSplashScreen> new_hydrus_splash() {
    QPixmap pixmap(620, 340);
    pixmap.fill(QColor(36, 36, 36));

    QPainter painter(&pixmap);
    painter.setRenderHint(QPainter::Antialiasing);

    QFont title_font = painter.font();
    title_font.setPointSize(30);
    title_font.setBold(true);
    painter.setFont(title_font);
    painter.setPen(QColor(238, 238, 238));
    painter.drawText(QRect(36, 48, 548, 64), Qt::AlignLeft | Qt::AlignVCenter, QStringLiteral("Hydrus Network"));

    QFont subtitle_font = painter.font();
    subtitle_font.setPointSize(13);
    subtitle_font.setBold(false);
    painter.setFont(subtitle_font);
    painter.setPen(QColor(170, 170, 170));
    painter.drawText(
        QRect(38, 118, 548, 30),
        Qt::AlignLeft | Qt::AlignVCenter,
        QStringLiteral("Rust + Qt 6 client"));

    painter.setPen(QColor(120, 120, 120));
    painter.drawLine(38, 168, 582, 168);

    painter.setPen(QColor(150, 150, 150));
    painter.drawText(
        QRect(38, 186, 548, 80),
        Qt::AlignLeft | Qt::AlignTop,
        QStringLiteral("A media management application for the hydrus network.\n"
                       "Opening the client database and preparing the session..."));
    painter.end();

    auto *splash = new QSplashScreen(pixmap);
    splash->setWindowFlag(Qt::WindowStaysOnTopHint, true);
    splash->setWindowFlag(Qt::FramelessWindowHint, true);
    return std::unique_ptr<QSplashScreen>(splash);
}

void hydrus_splash_show(QSplashScreen &splash) {
    splash.show();
    QApplication::processEvents();
}

void hydrus_splash_message(QSplashScreen &splash, rust::Str message) {
    splash.showMessage(
        to_qstring(message),
        Qt::AlignLeft | Qt::AlignBottom,
        QColor(225, 225, 225));
    QApplication::processEvents();
}

void hydrus_splash_finish_later(QSplashScreen &splash, int delay_ms) {
    QTimer::singleShot(delay_ms, &splash, [&splash]() { splash.close(); });
}

void hydrus_splash_screenshot(QSplashScreen &splash, rust::Str path, int delay_ms) {
    const QString target = to_qstring(path);

    QTimer::singleShot(delay_ms, &splash, [&splash, target]() {
        splash.grab().save(target);
    });
}

void hydrus_splash_finish(QSplashScreen &splash) {
    splash.close();
}

std::unique_ptr<HydrusMainWindow> new_hydrus_main_window() {
    return std::make_unique<HydrusMainWindow>();
}

void build_hydrus_shell(HydrusMainWindow &window) {
    window.build_shell();
}

void show_hydrus_main_window(HydrusMainWindow &window) {
    window.show();
}

void hydrus_set_title(HydrusMainWindow &window, rust::Str title) {
    window.setWindowTitle(to_qstring(title));
}

void hydrus_resize_window(HydrusMainWindow &window, int width, int height) {
    window.resize(width, height);
}

void hydrus_set_menus(HydrusMainWindow &window, rust::Str json) {
    window.set_menus(parse_array(json));
}

void hydrus_set_tag_suggestions(HydrusMainWindow &window, rust::Str json) {
    window.set_tag_suggestions(json_to_strings(parse_array(json)));
}

void hydrus_show_autocomplete(HydrusMainWindow &window, rust::Str json) {
    window.show_autocomplete(parse_object(json));
}

void hydrus_set_status(HydrusMainWindow &window, rust::Str json) {
    window.set_status(parse_object(json));
}

void hydrus_build_pages(HydrusMainWindow &window, rust::Str json) {
    window.build_pages(parse_object(json));
}

void hydrus_update_page(HydrusMainWindow &window, rust::Str page_id, rust::Str json) {
    window.update_page(to_qstring(page_id), parse_object(json));
}

void hydrus_update_page_progress(HydrusMainWindow &window, rust::Str page_id, rust::Str json) {
    window.update_page_progress(to_qstring(page_id), parse_object(json));
}

void hydrus_update_downloader(HydrusMainWindow &window, rust::Str page_id, rust::Str json) {
    window.update_downloader(to_qstring(page_id), parse_object(json));
}

void hydrus_update_subscriptions(HydrusMainWindow &window, rust::Str page_id, rust::Str json) {
    window.update_subscriptions(to_qstring(page_id), parse_object(json));
}

void hydrus_update_duplicates(HydrusMainWindow &window, rust::Str page_id, rust::Str json) {
    window.update_duplicates(to_qstring(page_id), parse_object(json));
}

void hydrus_update_import_folders(HydrusMainWindow &window, rust::Str page_id, rust::Str json) {
    window.update_import_folders(to_qstring(page_id), parse_object(json));
}

void hydrus_update_export_folders(HydrusMainWindow &window, rust::Str page_id, rust::Str json) {
    window.update_export_folders(to_qstring(page_id), parse_object(json));
}

void hydrus_show_duplicates_rules(HydrusMainWindow &window, rust::Str json) {
    window.show_duplicates_rules(parse_object(json));
}

void hydrus_show_duplicate_merge_options(HydrusMainWindow &window, rust::Str json) {
    window.show_duplicate_merge_options(parse_object(json));
}

void hydrus_update_auto_resolution_preview(HydrusMainWindow &window, rust::Str json) {
    window.update_auto_resolution_preview(parse_object(json));
}

void hydrus_show_duplicate_review(HydrusMainWindow &window, rust::Str json) {
    window.show_duplicate_review(parse_object(json));
}

void hydrus_update_duplicate_review(HydrusMainWindow &window, rust::Str json) {
    window.update_duplicate_review(parse_object(json));
}

void hydrus_show_tag_relations(HydrusMainWindow &window, rust::Str json) {
    window.show_tag_relations(parse_object(json));
}

void hydrus_show_import_folder(HydrusMainWindow &window, rust::Str json) {
    window.show_import_folder(parse_object(json));
}

void hydrus_show_export_folder(HydrusMainWindow &window, rust::Str json) {
    window.show_export_folder(parse_object(json));
}

void hydrus_show_export_review(HydrusMainWindow &window, rust::Str json) {
    window.show_export_review(parse_object(json));
}

void hydrus_show_notes(HydrusMainWindow &window, rust::Str json) {
    window.show_notes(parse_object(json));
}

void hydrus_set_selection(
    HydrusMainWindow &window,
    rust::Str page_id,
    rust::Str hash,
    rust::Str tags_json,
    rust::Str preview_path,
    rust::Str metadata_json) {
    window.set_selection(
        to_qstring(page_id),
        to_qstring(hash),
        json_to_strings(parse_array(tags_json)),
        to_qstring(preview_path),
        parse_object(metadata_json));
}

void hydrus_show_page_chooser(HydrusMainWindow &window, rust::Str notebook_id) {
    window.show_page_chooser(to_qstring(notebook_id));
}

void hydrus_pick_files(HydrusMainWindow &window) {
    const QStringList paths = QFileDialog::getOpenFileNames(
        &window,
        QStringLiteral("import files"),
        QString(),
        QStringLiteral("all files (*.*)"));

    if (paths.isEmpty()) {
        return;
    }

    QJsonArray array;

    for (const QString &path : paths) {
        array.append(path);
    }

    window.notify(QStringLiteral("import_paths"), QString(), QString::fromUtf8(QJsonDocument(array).toJson(QJsonDocument::Compact)));
}

void hydrus_prompt_url(HydrusMainWindow &window) {
    window.prompt_url();
}

void hydrus_open_viewer(HydrusMainWindow &window, rust::Str json) {
    window.open_viewer(parse_object(json));
}

void hydrus_update_viewer(HydrusMainWindow &window, rust::Str viewer_id, rust::Str json) {
    window.update_viewer(to_qstring(viewer_id), parse_object(json));
}

void hydrus_close_viewer(HydrusMainWindow &window, rust::Str viewer_id) {
    window.close_viewer(to_qstring(viewer_id));
}

void hydrus_show_tag_dialog(HydrusMainWindow &window, rust::Str json) {
    window.show_tag_dialog(parse_object(json));
}

void hydrus_show_notifications(HydrusMainWindow &window, rust::Str json) {
    window.show_notifications(parse_object(json));
}

void hydrus_show_viewing_stats(HydrusMainWindow &window, rust::Str json) {
    window.show_viewing_stats(parse_object(json));
}

void hydrus_show_about(HydrusMainWindow &window, rust::Str json) {
    window.show_about(parse_object(json));
}

void hydrus_export_downloader(HydrusMainWindow &window, rust::Str page_id) {
    const QString path = QFileDialog::getSaveFileName(
        &window,
        QStringLiteral("export downloader definition"),
        QStringLiteral("downloader.json"),
        QStringLiteral("JSON (*.json)"));

    if (!path.isEmpty()) {
        window.notify(QStringLiteral("downloader_export_to"), to_qstring(page_id), path);
    }
}

void hydrus_import_downloader(HydrusMainWindow &window, rust::Str target) {
    const QString path = QFileDialog::getOpenFileName(
        &window,
        QStringLiteral("import downloader definition"),
        QString(),
        QStringLiteral("JSON (*.json)"));

    if (!path.isEmpty()) {
        window.notify(QStringLiteral("downloader_import_from"), to_qstring(target), path);
    }
}

void hydrus_show_options(HydrusMainWindow &window, rust::Str json) {
    window.show_options(parse_object(json));
}

void hydrus_show_services(HydrusMainWindow &window, rust::Str json) {
    window.show_services(parse_object(json));
}

void hydrus_update_services(HydrusMainWindow &window, rust::Str json) {
    window.update_services(parse_object(json));
}

void hydrus_show_tag_filter(HydrusMainWindow &window, rust::Str json) {
    window.show_tag_filter(parse_object(json));
}

void hydrus_show_api_permissions(HydrusMainWindow &window, rust::Str json) {
    window.show_api_permissions(parse_object(json));
}

void hydrus_update_api_permissions(HydrusMainWindow &window, rust::Str json) {
    window.update_api_permissions(parse_object(json));
}

void hydrus_show_account_types(HydrusMainWindow &window, rust::Str json) {
    window.show_account_types(parse_object(json));
}

void hydrus_update_account_types(HydrusMainWindow &window, rust::Str json) {
    window.update_account_types(parse_object(json));
}

void hydrus_show_orphan_files(HydrusMainWindow &window, rust::Str json) {
    window.show_orphan_files(parse_object(json));
}

void hydrus_update_orphan_files(HydrusMainWindow &window, rust::Str json) {
    window.update_orphan_files(parse_object(json));
}

void hydrus_show_database_backup(HydrusMainWindow &window, rust::Str json) {
    window.show_database_backup(parse_object(json));
}

void hydrus_update_database_backup(HydrusMainWindow &window, rust::Str json) {
    window.update_database_backup(parse_object(json));
}

void hydrus_show_network(HydrusMainWindow &window, rust::Str json) {
    window.show_network(parse_object(json));
}

void hydrus_update_network(HydrusMainWindow &window, rust::Str json) {
    window.update_network(parse_object(json));
}

void hydrus_start_tick(HydrusMainWindow &window) {
    window.start_tick();
}

void hydrus_defer_action(
    HydrusMainWindow &window,
    rust::Str action,
    rust::Str target,
    rust::Str payload,
    int delay_ms) {
    const QString action_text = to_qstring(action);
    const QString target_text = to_qstring(target);
    const QString payload_text = to_qstring(payload);

    QTimer::singleShot(delay_ms, &window, [&window, action_text, target_text, payload_text]() {
        QByteArray action_bytes = action_text.toUtf8();
        QByteArray target_bytes = target_text.toUtf8();
        QByteArray payload_bytes = payload_text.toUtf8();

        app_action(
            window,
            rust::Str(action_bytes.constData(), action_bytes.size()),
            rust::Str(target_bytes.constData(), target_bytes.size()),
            rust::Str(payload_bytes.constData(), payload_bytes.size()));
    });
}

void hydrus_show_message(HydrusMainWindow &window, rust::Str title, rust::Str text) {
    window.show_message(to_qstring(title), to_qstring(text));
}

void hydrus_notify_message(HydrusMainWindow &window, rust::Str message) {
    window.notify_message(to_qstring(message));
}

void hydrus_set_clipboard(HydrusMainWindow &window, rust::Str text) {
    Q_UNUSED(window);
    QApplication::clipboard()->setText(to_qstring(text));
}

void hydrus_open_path(rust::Str path) {
    QDesktopServices::openUrl(QUrl::fromLocalFile(to_qstring(path)));
}

void hydrus_open_url(rust::Str url) {
    QDesktopServices::openUrl(QUrl(to_qstring(url)));
}

void hydrus_schedule_screenshot(HydrusMainWindow &window, rust::Str path) {
    window.schedule_screenshot(to_qstring(path));
}

void qapp_set_application_name(rust::Str name) {
    QApplication::setApplicationName(to_qstring(name));
}

void qapp_set_application_display_name(rust::Str name) {
    QApplication::setApplicationDisplayName(to_qstring(name));
}

void qapp_set_organization_name(rust::Str name) {
    QApplication::setOrganizationName(to_qstring(name));
}

void qapp_set_style(rust::Str name) {
    if (QStyle *style = QStyleFactory::create(to_qstring(name))) {
        QApplication::setStyle(style);
    }
}

void qapp_set_stylesheet(rust::Str stylesheet) {
    qApp->setStyleSheet(to_qstring(stylesheet));
}

void qapp_quit() {
    QCoreApplication::quit();
}

