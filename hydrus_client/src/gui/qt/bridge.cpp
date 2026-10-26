#include "bridge.hpp"

#include <QAction>
#include <QApplication>
#include <QCheckBox>
#include <QComboBox>
#include <QCompleter>
#include <QCoreApplication>
#include <QDebug>
#include <QDateTime>
#include <QDesktopServices>
#include <QDialog>
#include <QDialogButtonBox>
#include <QDir>
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
#include <QMouseEvent>
#include <QPainter>
#include <QPlainTextEdit>
#include <QPixmap>
#include <QPushButton>
#include <QRegularExpression>
#include <QScreen>
#include <QScrollArea>
#include <QScrollBar>
#include <QShortcut>
#include <QSignalBlocker>
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
    queue->setColumnCount(3);
    queue->setHorizontalHeaderLabels({QStringLiteral("url"), QStringLiteral("status"), QStringLiteral("note")});
    queue->horizontalHeader()->setStretchLastSection(true);
    queue->horizontalHeader()->setSectionResizeMode(0, QHeaderView::Stretch);
    queue->horizontalHeader()->setSectionResizeMode(1, QHeaderView::ResizeToContents);
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

    if (queue->rowCount() != items.size() || queue->columnCount() != 3) {
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
        }

        url_item->setText(item_url);
        url_item->setData(Qt::UserRole, item_url);
        url_item->setToolTip(item_url);

        QTableWidgetItem *status_item = queue->item(index, 1);
        status_item->setText(item_status);

        if (item_status == QStringLiteral("error")) {
            status_item->setForeground(QColor(220, 110, 110));
        } else if (item_status == QStringLiteral("done")) {
            status_item->setForeground(QColor(120, 190, 120));
        } else if (item_status == QStringLiteral("downloading")) {
            status_item->setForeground(QColor(220, 200, 120));
        }

        queue->item(index, 2)->setText(item.value(QStringLiteral("note")).toString());
        queue->item(index, 2)->setToolTip(item.value(QStringLiteral("note")).toString());
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
    auto *filter = new QLineEdit(&dialog);
    auto *period = new QSpinBox(&dialog);
    period->setRange(60, 31536000);
    period->setValue(86400);

    url->setPlaceholderText(QStringLiteral("https://example.com/search?q={query}"));
    query->setPlaceholderText(QStringLiteral("search terms for {query}"));
    form->addRow(QStringLiteral("name"), name);
    form->addRow(QStringLiteral("gallery url"), url);
    form->addRow(QStringLiteral("search terms"), query);
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
    payload.insert(QStringLiteral("filter"), filter->text());
    payload.insert(QStringLiteral("period"), period->value());

    main->notify(QStringLiteral("subscription_add"), id, json_payload(payload));
}

void HydrusSubscriptionsPage::update_state(const QJsonObject &state) {
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
    header->addWidget(status, 1);
    header->addWidget(distance);
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
        add_button(QStringLiteral("subscriptions"), QStringLiteral("subscriptions"));
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
    url_classes->setColumnCount(3);
    url_classes->setHorizontalHeaderLabels({QStringLiteral("name"), QStringLiteral("domain"), QStringLiteral("path")});
    url_classes->horizontalHeader()->setStretchLastSection(true);
    url_classes->horizontalHeader()->setSectionResizeMode(0, QHeaderView::ResizeToContents);
    url_classes->setSelectionBehavior(QAbstractItemView::SelectRows);
    url_classes->setEditTriggers(QAbstractItemView::NoEditTriggers);
    url_classes->verticalHeader()->setVisible(false);
    classes_layout->addWidget(url_classes, 1);

    auto *class_buttons = new QHBoxLayout();
    auto *add_class = new QPushButton(QStringLiteral("add url class..."), classes_page);
    auto *remove_class = new QPushButton(QStringLiteral("remove"), classes_page);
    class_buttons->addWidget(add_class);
    class_buttons->addWidget(remove_class);
    class_buttons->addStretch(1);
    classes_layout->addLayout(class_buttons);
    tabs->addTab(classes_page, QStringLiteral("url classes"));

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
    connect(remove_class, &QPushButton::clicked, this, [this]() { remove_url_class(); });

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

        for (int column = 0; column < 3; ++column) {
            if (url_classes->item(index, column) == nullptr) {
                url_classes->setItem(index, column, new QTableWidgetItem());
            }
        }

        url_classes->item(index, 0)->setText(entry.value(QStringLiteral("name")).toString());
        url_classes->item(index, 1)->setText(entry.value(QStringLiteral("netloc")).toString());
        url_classes->item(index, 2)->setText(json_to_strings(entry.value(QStringLiteral("components")).toArray()).join(QStringLiteral("/")));
    }

    headers->resizeRowsToContents();
    url_classes->resizeRowsToContents();
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
    QDialog dialog(this);
    dialog.setWindowTitle(QStringLiteral("add url class"));
    auto *layout = new QVBoxLayout(&dialog);
    auto *form = new QFormLayout();
    auto *name = new QLineEdit(&dialog);
    auto *netloc = new QLineEdit(&dialog);
    auto *path = new QLineEdit(&dialog);
    path->setPlaceholderText(QStringLiteral("post/{post_id}"));
    form->addRow(QStringLiteral("name"), name);
    form->addRow(QStringLiteral("domain"), netloc);
    form->addRow(QStringLiteral("path"), path);
    layout->addLayout(form);
    auto *buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, &dialog);
    connect(buttons, &QDialogButtonBox::accepted, &dialog, &QDialog::accept);
    connect(buttons, &QDialogButtonBox::rejected, &dialog, &QDialog::reject);
    layout->addWidget(buttons);

    if (dialog.exec() != QDialog::Accepted || netloc->text().trimmed().isEmpty()) {
        return;
    }

    QJsonObject payload;
    payload.insert(QStringLiteral("name"), name->text().trimmed());
    payload.insert(QStringLiteral("netloc"), netloc->text().trimmed());
    payload.insert(QStringLiteral("path"), path->text().trimmed());

    main->notify(QStringLiteral("network_url_class_add"), QString(), json_payload(payload));
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
    downloaders.clear();
    subscriptions.clear();
    duplicates.clear();
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
