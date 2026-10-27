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
#include <QDesktopServices>
#include <QDialog>
#include <QDialogButtonBox>
#include <QDir>
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
#include <QTabBar>
#include <QTableWidget>
#include <QTextEdit>
#include <QTimer>
#include <QToolButton>
#include <QTreeWidget>
#include <QUrl>
#include <QVBoxLayout>

#include <functional>

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

    QPixmap pixmap(path);

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

void HydrusThumbnailGrid::set_media(const QJsonArray &media) {
    const QString focused = focused_hash();

    QSignalBlocker blocker(this);

    clear();

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
        item->setSizeHint(QSize(158, 133));
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

    QAction *open = menu.addAction(QStringLiteral("open in media viewer"));
    QAction *tags = menu.addAction(QStringLiteral("manage tags..."));
    menu.addSeparator();
    QAction *archive = menu.addAction(QStringLiteral("archive"));
    QAction *inbox = menu.addAction(QStringLiteral("inbox"));
    QAction *remove = menu.addAction(QStringLiteral("delete..."));

    QAction *chosen = menu.exec(event->globalPos());

    if (chosen == nullptr) {
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

    completer = new QCompleter(main->suggestions(), this);
    completer->setCaseSensitivity(Qt::CaseInsensitive);
    completer->setFilterMode(Qt::MatchContains);
    completer->setCompletionMode(QCompleter::PopupCompletion);
    search->setCompleter(completer);

    connect(search, &QLineEdit::textEdited, this, [this](const QString &text) {
        text_before_completion = text;
    });

    connect(completer, QOverload<const QString &>::of(&QCompleter::activated), this, [this](const QString &completion) {
        const QString before = text_before_completion.trimmed();
        int keep = 0;

        // Find the longest suffix of what was typed that the completion starts
        // with. This replaces a partially typed multi-word tag while keeping
        // earlier predicates such as `series:foo blue ey`.
        for (int index = 0; index < before.size(); ++index) {
            if (completion.startsWith(before.mid(index), Qt::CaseInsensitive)) {
                keep = index;
                break;
            }
        }

        QString prefix = before.left(keep);

        if (!prefix.isEmpty() && !prefix.endsWith(QLatin1Char(' '))) {
            prefix += QLatin1Char(' ');
        }

        const QString text = prefix + completion + QStringLiteral(" ");

        search->setText(text);
        text_before_completion = text;
        search->setFocus();
    });

    search_button = new QPushButton(QStringLiteral("search"), sidebar);
    sidebar_layout->addWidget(search_button);

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

    grid = new HydrusThumbnailGrid(main, this);
    splitter->addWidget(grid);
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

    grid->set_media(state.value(QStringLiteral("media")).toArray());

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
    if (completer == nullptr) {
        return;
    }

    completer->setModel(new QStringListModel(suggestions, completer));
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
    connect(buttons, &QDialogButtonBox::rejected, &dialog, &QDialog::reject);
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

    auto *header = new QHBoxLayout();
    status = new QLabel(this);
    distance = new QLabel(this);
    auto *refresh_button = new QPushButton(QStringLiteral("refresh"), this);
    refresh_button->setObjectName(QStringLiteral("refresh_duplicates"));
    refresh = refresh_button;
    delete_both = new QPushButton(QStringLiteral("delete both"), this);
    rules = new QPushButton(QStringLiteral("auto-resolution..."), this);
    header->addWidget(status, 1);
    header->addWidget(distance);
    header->addWidget(rules);
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
    connect(rules, &QPushButton::clicked, this, [this]() {
        main->notify(QStringLiteral("duplicates_rules"));
    });
    connect(delete_both, &QPushButton::clicked, this, [this]() {
        if (QMessageBox::question(this, QStringLiteral("delete both"), QStringLiteral("Move both files to the trash?")) == QMessageBox::Yes) {
            act(QStringLiteral("duplicates_delete_both"));
        }
    });

    update_state(state);
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
    auto *zoom_in = new QPushButton(QStringLiteral("+"), toolbar);
    auto *zoom_out = new QPushButton(QStringLiteral("-"), toolbar);
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
    toolbar_layout->addWidget(zoom_out);
    toolbar_layout->addWidget(zoom_in);
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

    layout->addWidget(body, 1);

    status = new QLabel(this);
    status->setContentsMargins(6, 2, 6, 4);
    layout->addWidget(status);

    connect(previous, &QPushButton::clicked, this, [this]() { navigate(QStringLiteral("prev")); });
    connect(next, &QPushButton::clicked, this, [this]() { navigate(QStringLiteral("next")); });
    connect(fit, &QPushButton::clicked, this, [this]() { canvas->fit_image(); });
    connect(zoom_in, &QPushButton::clicked, this, [this]() { canvas->zoom_by(zoom_step); });
    connect(zoom_out, &QPushButton::clicked, this, [this]() { canvas->zoom_by(1.0 / zoom_step); });
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

    resize(1100, 820);
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

    metadata->set_state(state.value(QStringLiteral("metadata")).toObject());
    metadata->set_suggestions(main->suggestions());

    status->setText(state.value(QStringLiteral("info")).toString());
}

void HydrusViewerWindow::navigate(const QString &direction) {
    const QByteArray viewer_bytes = id.toUtf8();
    const QByteArray direction_bytes = direction.toUtf8();
    app_action(*main, rust::Str("viewer_nav", 10), rust::Str(viewer_bytes.constData(), viewer_bytes.size()), rust::Str(direction_bytes.constData(), direction_bytes.size()));
}

void HydrusViewerWindow::closeEvent(QCloseEvent *event) {
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

    const QString data = tabBar()->tabData(index).toString();

    if (data == QStringLiteral("__plus__")) {
        return {};
    }

    return data;
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
    QString type;
    bool noneable = false;
    QCheckBox *boolean = nullptr;
    QSpinBox *integer = nullptr;
    QLineEdit *text = nullptr;
};

class HydrusOptionsDialog : public QDialog {
public:
    HydrusOptionsDialog(HydrusMainWindow *main_window, const QJsonObject &state, QWidget *parent)
        : QDialog(parent), main(main_window) {
        setWindowTitle(QStringLiteral("options"));
        resize(720, 600);

        auto *layout = new QVBoxLayout(this);
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

                const QString label = field.value(QStringLiteral("label")).toString();
                const QJsonValue value = field.value(QStringLiteral("value"));

                if (entry.type == QStringLiteral("bool")) {
                    entry.boolean = new QCheckBox(page);
                    entry.boolean->setChecked(value.toBool());
                    form->addRow(label, entry.boolean);
                } else if (entry.type == QStringLiteral("int")) {
                    entry.integer = new QSpinBox(page);
                    entry.integer->setRange(-2'000'000'000, 2'000'000'000);
                    entry.integer->setValue(value.toInt());
                    form->addRow(label, entry.integer);
                } else if (entry.type == QStringLiteral("string")) {
                    entry.text = new QLineEdit(page);
                    entry.text->setText(value.toString());
                    form->addRow(label, entry.text);
                } else {
                    auto *readonly = new QLabel(value.toString(), page);
                    readonly->setTextInteractionFlags(Qt::TextSelectableByMouse);
                    form->addRow(label, readonly);
                }

                fields.append(entry);
            }

            tabs->addTab(page, tab.value(QStringLiteral("title")).toString());
        }

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

HydrusServicesDialog::HydrusServicesDialog(HydrusMainWindow *main_window, const QJsonObject &state, QWidget *parent)
    : QDialog(parent), main(main_window) {
    setWindowTitle(QStringLiteral("manage services"));
    setObjectName(QStringLiteral("hydrus_services_dialog"));
    setAttribute(Qt::WA_DeleteOnClose, true);
    resize(760, 480);

    auto *layout = new QVBoxLayout(this);

    table = new QTableWidget(this);
    table->setColumnCount(4);
    table->setHorizontalHeaderLabels({
        QStringLiteral("name"),
        QStringLiteral("type"),
        QStringLiteral("service key"),
        QStringLiteral("id")
    });
    table->horizontalHeader()->setStretchLastSection(true);
    table->horizontalHeader()->setSectionResizeMode(0, QHeaderView::Stretch);
    table->horizontalHeader()->setSectionResizeMode(1, QHeaderView::ResizeToContents);
    table->horizontalHeader()->setSectionResizeMode(2, QHeaderView::Stretch);
    table->setSelectionBehavior(QAbstractItemView::SelectRows);
    table->setSelectionMode(QAbstractItemView::SingleSelection);
    table->setEditTriggers(QAbstractItemView::NoEditTriggers);
    table->verticalHeader()->setVisible(false);
    layout->addWidget(table, 1);

    auto *buttons = new QHBoxLayout();
    auto *add = new QPushButton(QStringLiteral("add local service..."), this);
    auto *rename = new QPushButton(QStringLiteral("rename..."), this);
    auto *remove = new QPushButton(QStringLiteral("delete"), this);
    auto *close = new QPushButton(QStringLiteral("close"), this);
    buttons->addWidget(add);
    buttons->addWidget(rename);
    buttons->addWidget(remove);
    buttons->addStretch(1);
    buttons->addWidget(close);
    layout->addLayout(buttons);

    connect(add, &QPushButton::clicked, this, [this]() { add_service(); });
    connect(rename, &QPushButton::clicked, this, [this]() { rename_selected(); });
    connect(remove, &QPushButton::clicked, this, [this]() { delete_selected(); });
    connect(close, &QPushButton::clicked, this, &QDialog::close);

    set_state(state);
}

void HydrusServicesDialog::set_state(const QJsonObject &state) {
    data = state;

    const QJsonArray services = state.value(QStringLiteral("services")).toArray();
    table->setRowCount(services.size());

    for (int index = 0; index < services.size(); ++index) {
        const QJsonObject service = services.at(index).toObject();

        for (int column = 0; column < 4; ++column) {
            if (table->item(index, column) == nullptr) {
                table->setItem(index, column, new QTableWidgetItem());
            }
        }

        auto *name_item = table->item(index, 0);
        name_item->setText(service.value(QStringLiteral("name")).toString());
        name_item->setData(Qt::UserRole, service.value(QStringLiteral("key")).toString());

        if (service.value(QStringLiteral("builtin")).toBool()) {
            name_item->setForeground(QColor(120, 150, 190));
        }

        table->item(index, 1)->setText(service.value(QStringLiteral("type_short")).toString());
        table->item(index, 2)->setText(service.value(QStringLiteral("key")).toString());
        table->item(index, 3)->setText(QString::number(service.value(QStringLiteral("service_id")).toInt()));
    }

    table->resizeRowsToContents();
}

QString HydrusServicesDialog::selected_key() const {
    const int row = table->currentRow();

    if (row < 0) {
        return {};
    }

    QTableWidgetItem *item = table->item(row, 0);

    return item == nullptr ? QString() : item->data(Qt::UserRole).toString();
}

void HydrusServicesDialog::add_service() {
    QDialog dialog(this);
    dialog.setWindowTitle(QStringLiteral("add local service"));
    auto *layout = new QVBoxLayout(&dialog);
    auto *form = new QFormLayout();

    auto *name = new QLineEdit(&dialog);
    auto *type = new QComboBox(&dialog);

    for (const QJsonValue &value : data.value(QStringLiteral("types")).toArray()) {
        const QJsonObject entry = value.toObject();
        type->addItem(entry.value(QStringLiteral("label")).toString(), entry.value(QStringLiteral("value")).toInt());
    }

    form->addRow(QStringLiteral("name"), name);
    form->addRow(QStringLiteral("type"), type);
    layout->addLayout(form);

    auto *buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, &dialog);
    connect(buttons, &QDialogButtonBox::accepted, &dialog, &QDialog::accept);
    connect(buttons, &QDialogButtonBox::rejected, &dialog, &QDialog::reject);
    layout->addWidget(buttons);

    if (dialog.exec() != QDialog::Accepted || name->text().trimmed().isEmpty()) {
        return;
    }

    QJsonObject payload;
    payload.insert(QStringLiteral("name"), name->text().trimmed());
    payload.insert(QStringLiteral("type"), type->currentData().toInt());

    main->notify(QStringLiteral("service_add"), QString(), json_payload(payload));
}

void HydrusServicesDialog::rename_selected() {
    const QString key = selected_key();

    if (key.isEmpty()) {
        return;
    }

    const int row = table->currentRow();
    bool ok = false;
    const QString name = QInputDialog::getText(
        this,
        QStringLiteral("rename service"),
        QStringLiteral("name"),
        QLineEdit::Normal,
        table->item(row, 0)->text(),
        &ok);

    if (!ok || name.trimmed().isEmpty()) {
        return;
    }

    QJsonObject payload;
    payload.insert(QStringLiteral("key"), key);
    payload.insert(QStringLiteral("name"), name.trimmed());

    main->notify(QStringLiteral("service_rename"), QString(), json_payload(payload));
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
    connect(buttons, &QDialogButtonBox::rejected, &dialog, &QDialog::reject);
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
    connect(buttons, &QDialogButtonBox::rejected, &dialog, &QDialog::reject);
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
    connect(buttons, &QDialogButtonBox::rejected, &dialog, &QDialog::reject);
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
    connect(buttons, &QDialogButtonBox::rejected, &dialog, &QDialog::reject);
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
// HydrusDuplicatesRulesDialog
// ---------------------------------------------------------------------------

HydrusDuplicatesRulesDialog::HydrusDuplicatesRulesDialog(HydrusMainWindow *main_window, const QJsonObject &state, QWidget *parent)
    : QDialog(parent), main(main_window) {
    setWindowTitle(QStringLiteral("duplicate auto-resolution"));
    setAttribute(Qt::WA_DeleteOnClose, true);
    resize(700, 460);

    auto *layout = new QVBoxLayout(this);

    enabled = new QCheckBox(QStringLiteral("apply these rules automatically when a duplicates page refreshes"), this);
    layout->addWidget(enabled);

    auto *help = new QLabel(
        QStringLiteral("Rules are checked in order. The first matching rule resolves the pair.\n"
                       "An empty rule list leaves every pair for manual review."),
        this);
    help->setWordWrap(true);
    layout->addWidget(help);

    table = new QTableWidget(this);
    table->setColumnCount(3);
    table->setHorizontalHeaderLabels({
        QStringLiteral("name"),
        QStringLiteral("comparator"),
        QStringLiteral("action")
    });
    table->horizontalHeader()->setStretchLastSection(true);
    table->setSelectionBehavior(QAbstractItemView::SelectRows);
    table->setSelectionMode(QAbstractItemView::SingleSelection);
    table->setEditTriggers(QAbstractItemView::NoEditTriggers);
    table->verticalHeader()->setVisible(false);
    layout->addWidget(table, 1);

    auto *buttons = new QHBoxLayout();
    auto *add = new QPushButton(QStringLiteral("add rule..."), this);
    auto *remove = new QPushButton(QStringLiteral("remove"), this);
    auto *close = new QPushButton(QStringLiteral("close"), this);
    buttons->addWidget(add);
    buttons->addWidget(remove);
    buttons->addStretch(1);
    buttons->addWidget(close);
    layout->addLayout(buttons);

    connect(enabled, &QCheckBox::clicked, this, [this](bool) {
        main->notify(QStringLiteral("duplicates_auto_toggle"));
    });
    connect(add, &QPushButton::clicked, this, [this]() { add_rule(); });
    connect(remove, &QPushButton::clicked, this, [this]() { remove_selected(); });
    connect(close, &QPushButton::clicked, this, &QDialog::close);

    set_state(state);
}

void HydrusDuplicatesRulesDialog::set_state(const QJsonObject &state) {
    data = state;

    const QJsonArray rules = state.value(QStringLiteral("rules")).toArray();
    table->setRowCount(rules.size());

    for (int index = 0; index < rules.size(); ++index) {
        const QJsonObject rule = rules.at(index).toObject();

        for (int column = 0; column < 3; ++column) {
            if (table->item(index, column) == nullptr) {
                table->setItem(index, column, new QTableWidgetItem());
            }
        }

        table->item(index, 0)->setText(rule.value(QStringLiteral("name")).toString());
        table->item(index, 0)->setData(Qt::UserRole, rule.value(QStringLiteral("name")).toString());
        table->item(index, 1)->setText(rule.value(QStringLiteral("comparator")).toString());
        table->item(index, 2)->setText(rule.value(QStringLiteral("action")).toString());
    }

    QSignalBlocker blocker(enabled);
    enabled->setChecked(state.value(QStringLiteral("enabled")).toBool());

    table->resizeRowsToContents();
}

void HydrusDuplicatesRulesDialog::add_rule() {
    QDialog dialog(this);
    dialog.setWindowTitle(QStringLiteral("add auto-resolution rule"));
    auto *layout = new QVBoxLayout(&dialog);
    auto *form = new QFormLayout();
    auto *name = new QLineEdit(&dialog);
    auto *comparator = new QComboBox(&dialog);
    auto *action = new QComboBox(&dialog);

    for (const QJsonValue &value : data.value(QStringLiteral("comparators")).toArray()) {
        const QJsonObject entry = value.toObject();
        comparator->addItem(entry.value(QStringLiteral("label")).toString(), entry.value(QStringLiteral("value")).toString());
    }

    for (const QJsonValue &value : data.value(QStringLiteral("actions")).toArray()) {
        const QJsonObject entry = value.toObject();
        action->addItem(entry.value(QStringLiteral("label")).toString(), entry.value(QStringLiteral("value")).toString());
    }

    form->addRow(QStringLiteral("name"), name);
    form->addRow(QStringLiteral("comparator"), comparator);
    form->addRow(QStringLiteral("action"), action);
    layout->addLayout(form);

    auto *buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, &dialog);
    connect(buttons, &QDialogButtonBox::accepted, &dialog, &QDialog::accept);
    connect(buttons, &QDialogButtonBox::rejected, &dialog, &QDialog::reject);
    layout->addWidget(buttons);

    if (dialog.exec() != QDialog::Accepted || name->text().trimmed().isEmpty()) {
        return;
    }

    QJsonObject payload;
    payload.insert(QStringLiteral("name"), name->text().trimmed());
    payload.insert(QStringLiteral("comparator"), comparator->currentData().toString());
    payload.insert(QStringLiteral("action"), action->currentData().toString());

    main->notify(QStringLiteral("duplicates_rule_add"), QString(), json_payload(payload));
}

void HydrusDuplicatesRulesDialog::remove_selected() {
    const int row = table->currentRow();

    if (row < 0 || table->item(row, 0) == nullptr) {
        return;
    }

    main->notify(
        QStringLiteral("duplicates_rule_remove"),
        QString(),
        table->item(row, 0)->data(Qt::UserRole).toString());
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

    auto *import_folders = new QShortcut(QKeySequence(QStringLiteral("Ctrl+Shift+I")), this);
    connect(import_folders, &QShortcut::activated, this, [this]() {
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

void HydrusMainWindow::show_duplicates_rules(const QJsonObject &state) {
    if (duplicates_rules_dialog == nullptr) {
        duplicates_rules_dialog = new HydrusDuplicatesRulesDialog(this, state, this);
        connect(duplicates_rules_dialog, &QObject::destroyed, this, [this]() {
            duplicates_rules_dialog = nullptr;
        });
        duplicates_rules_dialog->show();
        duplicates_rules_dialog->raise();
        duplicates_rules_dialog->activateWindow();
        return;
    }

    duplicates_rules_dialog->set_state(state);
    duplicates_rules_dialog->raise();
    duplicates_rules_dialog->activateWindow();
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
    connect(buttons, &QDialogButtonBox::rejected, &dialog, &QDialog::reject);
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

void hydrus_set_status(HydrusMainWindow &window, rust::Str json) {
    window.set_status(parse_object(json));
}

void hydrus_build_pages(HydrusMainWindow &window, rust::Str json) {
    window.build_pages(parse_object(json));
}

void hydrus_update_page(HydrusMainWindow &window, rust::Str page_id, rust::Str json) {
    window.update_page(to_qstring(page_id), parse_object(json));
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

void hydrus_show_duplicates_rules(HydrusMainWindow &window, rust::Str json) {
    window.show_duplicates_rules(parse_object(json));
}

void hydrus_show_tag_relations(HydrusMainWindow &window, rust::Str json) {
    window.show_tag_relations(parse_object(json));
}

void hydrus_show_import_folder(HydrusMainWindow &window, rust::Str json) {
    window.show_import_folder(parse_object(json));
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

void hydrus_show_options(HydrusMainWindow &window, rust::Str json) {
    window.show_options(parse_object(json));
}

void hydrus_show_services(HydrusMainWindow &window, rust::Str json) {
    window.show_services(parse_object(json));
}

void hydrus_update_services(HydrusMainWindow &window, rust::Str json) {
    window.update_services(parse_object(json));
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
