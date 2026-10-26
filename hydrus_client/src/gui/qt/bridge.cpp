#include "bridge.hpp"

#include <QAction>
#include <QApplication>
#include <QCompleter>
#include <QCoreApplication>
#include <QDebug>
#include <QDesktopServices>
#include <QDialog>
#include <QDialogButtonBox>
#include <QDir>
#include <QDragEnterEvent>
#include <QDropEvent>
#include <QFileDialog>
#include <QFileInfo>
#include <QFrame>
#include <QGridLayout>
#include <QHBoxLayout>
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
#include <QMouseEvent>
#include <QPainter>
#include <QPlainTextEdit>
#include <QPixmap>
#include <QPushButton>
#include <QRegularExpression>
#include <QScreen>
#include <QScrollBar>
#include <QShortcut>
#include <QSignalBlocker>
#include <QSplitter>
#include <QStringListModel>
#include <QStackedWidget>
#include <QStatusBar>
#include <QStyle>
#include <QStyleFactory>
#include <QTabBar>
#include <QTimer>
#include <QToolButton>
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
    scene->clear();
    pixmap_item = nullptr;
    resetTransform();
    fit_mode = true;
    rotation = 0;
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

void HydrusCanvas::resizeEvent(QResizeEvent *event) {
    QGraphicsView::resizeEvent(event);

    if (fit_mode) {
        fit_image();
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

    connect(this, &QListWidget::itemDoubleClicked, this, [this](QListWidgetItem *item) {
        if (main->building || item == nullptr) {
            return;
        }

        const QString hash = page_hash_from_item(item);

        if (!hash.isEmpty()) {
            QByteArray hash_bytes = hash.toUtf8();
            QByteArray page_bytes = page->page_id().toUtf8();
            app_action(*main, rust::Str("open_viewer", 11), rust::Str(page_bytes.constData(), page_bytes.size()), rust::Str(hash_bytes.constData(), hash_bytes.size()));
        }
    });
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
        item->setIcon(QIcon(thumbnail_pixmap(thumb)));
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
        action = QStringLiteral("archive_files");
    } else if (chosen == inbox) {
        action = QStringLiteral("inbox_files");
    } else if (chosen == remove) {
        action = QStringLiteral("delete_files");
    }

    if (action.isEmpty()) {
        return;
    }

    QByteArray action_bytes = action.toUtf8();
    QByteArray page_bytes = page->page_id().toUtf8();
    QByteArray hash_bytes = hash.toUtf8();
    app_action(*main, rust::Str(action_bytes.constData(), action_bytes.size()), rust::Str(page_bytes.constData(), page_bytes.size()), rust::Str(hash_bytes.constData(), hash_bytes.size()));
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

    status_label = new QLabel(sidebar);
    status_label->setWordWrap(true);
    sidebar_layout->addWidget(status_label);

    auto *tags_title = new QLabel(QStringLiteral("selection tags"), sidebar);
    tags_title->setStyleSheet(QStringLiteral("font-weight: bold;"));
    sidebar_layout->addWidget(tags_title);

    selection_tags = new QListWidget(sidebar);
    selection_tags->setSelectionMode(QAbstractItemView::NoSelection);
    sidebar_layout->addWidget(selection_tags, 1);

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

    QString status = state.value(QStringLiteral("status")).toString();
    const QString empty_text = state.value(QStringLiteral("empty_text")).toString();

    if (!empty_text.isEmpty()) {
        status += QStringLiteral("\n") + empty_text;
    }

    status_label->setText(status);

    const QStringList tags = json_to_strings(state.value(QStringLiteral("tags")).toArray());
    selection_tags->clear();
    selection_tags->addItems(tags);

    const QString focused = state.value(QStringLiteral("focused")).toString();

    grid->set_media(state.value(QStringLiteral("media")).toArray());

    if (!focused.isEmpty()) {
        grid->set_focused(focused);
    }
}

void HydrusQueryPage::set_selection(const QString &hash, const QStringList &tags, const QString &preview_path) {
    selection_tags->clear();
    selection_tags->addItems(tags);

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
// HydrusViewerWindow
// ---------------------------------------------------------------------------

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
    auto *close_button = new QPushButton(QStringLiteral("close"), toolbar);

    toolbar_layout->addWidget(previous);
    toolbar_layout->addWidget(next);
    toolbar_layout->addSpacing(12);
    toolbar_layout->addWidget(fit);
    toolbar_layout->addWidget(zoom_out);
    toolbar_layout->addWidget(zoom_in);
    toolbar_layout->addStretch(1);
    toolbar_layout->addWidget(manage);
    toolbar_layout->addWidget(close_button);
    layout->addWidget(toolbar);

    auto *body = new QSplitter(Qt::Horizontal, this);

    tags = new QListWidget(body);
    tags->setMinimumWidth(180);
    tags->setMaximumWidth(320);
    body->addWidget(tags);

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
    connect(zoom_in, &QPushButton::clicked, this, [this]() { canvas->zoom_by(1.2); });
    connect(zoom_out, &QPushButton::clicked, this, [this]() { canvas->zoom_by(1.0 / 1.2); });
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
    bind_shortcut(QStringLiteral("+"), [this]() { canvas->zoom_by(1.2); });
    bind_shortcut(QStringLiteral("="), [this]() { canvas->zoom_by(1.2); });
    bind_shortcut(QStringLiteral("-"), [this]() { canvas->zoom_by(1.0 / 1.2); });
    bind_shortcut(QStringLiteral("0"), [this]() { canvas->reset_zoom(); });
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

    const QString path = state.value(QStringLiteral("path")).toString();
    const QString title = state.value(QStringLiteral("title")).toString();

    setWindowTitle(title.isEmpty() ? QStringLiteral("media viewer") : title);

    if (!path.isEmpty()) {
        canvas->load_image(path);
    } else {
        canvas->clear_image();
    }

    tags->clear();
    tags->addItems(json_to_strings(state.value(QStringLiteral("tags")).toArray()));

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
// HydrusNotebook
// ---------------------------------------------------------------------------

HydrusNotebook::HydrusNotebook(HydrusMainWindow *main_window, const QString &notebook_id, QWidget *parent)
    : QTabWidget(parent), main(main_window), id(notebook_id) {
    setTabsClosable(true);
    setMovable(false);
    setDocumentMode(true);

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
        add_nav(QStringLiteral("back"), 0);
        page_layout->addStretch(1);
    }

    void build_special() {
        make_page();
        page_layout->addWidget(new QLabel(QStringLiteral("special"), stack->currentWidget()));
        add_button(QStringLiteral("page of pages"), QStringLiteral("notebook"));
        add_button(QStringLiteral("duplicates processing"), QStringLiteral("duplicates"));
        add_nav(QStringLiteral("back"), 0);
        page_layout->addStretch(1);
    }

    QString notebook;
    QStackedWidget *stack = nullptr;
    QVBoxLayout *page_layout = nullptr;
};

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

    auto *import = new QShortcut(QKeySequence(QStringLiteral("Ctrl+I")), this);
    connect(import, &QShortcut::activated, this, [this]() { notify(QStringLiteral("import_files")); });
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

        add_items = [this, &add_items](QMenu *target, const QJsonArray &items) {
            for (const QJsonValue &item_value : items) {
                const QJsonObject item = item_value.toObject();

                if (item.value(QStringLiteral("separator")).toBool()) {
                    target->addSeparator();
                    continue;
                }

                const QJsonArray children = item.value(QStringLiteral("items")).toArray();

                if (!children.isEmpty()) {
                    auto *submenu = target->addMenu(item.value(QStringLiteral("label")).toString());
                    submenu->setToolTipsVisible(true);
                    add_items(submenu, children);
                    continue;
                }

                const QString label = item.value(QStringLiteral("label")).toString();
                const QString action_id = item.value(QStringLiteral("action")).toString();

                if (label.isEmpty()) {
                    continue;
                }

                auto *action = target->addAction(label);
                action->setEnabled(item.value(QStringLiteral("enabled")).toBool(true));

                if (item.value(QStringLiteral("checkable")).toBool(false)) {
                    action->setCheckable(true);
                    action->setChecked(item.value(QStringLiteral("checked")).toBool(false));
                }

                const QString tooltip = item.value(QStringLiteral("description")).toString();

                if (!tooltip.isEmpty()) {
                    action->setToolTip(tooltip);
                }

                connect(action, &QAction::triggered, this, [this, action_id]() {
                    if (!action_id.isEmpty()) {
                        notify(action_id);
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
        } else {
            const QString message = child.value(QStringLiteral("state")).toObject().value(QStringLiteral("message")).toString();
            auto *placeholder = new QWidget(notebook);
            auto *layout = new QVBoxLayout(placeholder);
            auto *label = new QLabel(message.isEmpty() ? QStringLiteral("This page type is not ported yet.") : message, placeholder);
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

void HydrusMainWindow::set_selection(
    const QString &page_id,
    const QString &hash,
    const QStringList &tags,
    const QString &preview_path) {
    if (HydrusQueryPage *page = find_page(page_id)) {
        page->set_selection(hash, tags, preview_path);
    }
}

void HydrusMainWindow::show_page_chooser(const QString &notebook_id) {
    HydrusPageChooser chooser(notebook_id, this);

    if (chooser.exec() == QDialog::Accepted && !chooser.result.isEmpty()) {
        notify(QStringLiteral("create_page"), notebook_id, chooser.result);
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
    const QString hash = state.value(QStringLiteral("hash")).toString();
    const QString service = state.value(QStringLiteral("service")).toString();
    const QStringList tags = json_to_strings(state.value(QStringLiteral("tags")).toArray());

    // NOTE: a small local dialog keeps the common "edit tags of one file" flow
    // synchronous. It mirrors Hydrus' manage tags dialog for the local service.
    class Dialog : public QDialog {
    public:
        Dialog(HydrusMainWindow *main_window, const QString &file_hash, const QString &service_name, const QStringList &initial, QWidget *parent)
            : QDialog(parent), main(main_window), hash(file_hash), before(initial) {
            setWindowTitle(QStringLiteral("manage tags"));
            resize(520, 480);

            auto *layout = new QVBoxLayout(this);
            auto *info = new QLabel(
                QStringLiteral("%1\nservice: %2\n\none tag per line; empty list removes every tag").arg(hash.left(16) + QStringLiteral("..."), service_name),
                this);
            info->setWordWrap(true);
            layout->addWidget(info);

            editor = new QPlainTextEdit(this);
            editor->setPlainText(initial.join(QStringLiteral("\n")));
            layout->addWidget(editor, 1);

            auto *buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, this);
            connect(buttons, &QDialogButtonBox::accepted, this, [this]() { apply(); accept(); });
            connect(buttons, &QDialogButtonBox::rejected, this, &QDialog::reject);
            layout->addWidget(buttons);
        }

    private:
        void apply() {
            QStringList after;

            for (const QString &line : editor->toPlainText().split(QRegularExpression(QStringLiteral("[\r\n]+")), Qt::SkipEmptyParts)) {
                const QString tag = line.trimmed();

                if (!tag.isEmpty() && !after.contains(tag)) {
                    after.append(tag);
                }
            }

            QJsonArray added;
            QJsonArray removed;

            for (const QString &tag : after) {
                if (!before.contains(tag)) {
                    added.append(tag);
                }
            }

            for (const QString &tag : before) {
                if (!after.contains(tag)) {
                    removed.append(tag);
                }
            }

            QJsonObject payload;
            payload.insert(QStringLiteral("hash"), hash);
            payload.insert(QStringLiteral("add"), added);
            payload.insert(QStringLiteral("remove"), removed);

            main->notify(QStringLiteral("apply_tags"), QString(), QString::fromUtf8(QJsonDocument(payload).toJson(QJsonDocument::Compact)));
        }

        HydrusMainWindow *main = nullptr;
        QString hash;
        QStringList before;
        QPlainTextEdit *editor = nullptr;
    };

    Dialog dialog(this, hash, service, tags, this);
    dialog.exec();
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
    QTimer::singleShot(1400, this, [this, path]() {
        QWidget *target = this;

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

void hydrus_set_selection(
    HydrusMainWindow &window,
    rust::Str page_id,
    rust::Str hash,
    rust::Str tags_json,
    rust::Str preview_path) {
    window.set_selection(
        to_qstring(page_id),
        to_qstring(hash),
        json_to_strings(parse_array(tags_json)),
        to_qstring(preview_path));
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
