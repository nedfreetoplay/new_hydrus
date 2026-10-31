#include "viewer.hpp"

#include "bridge.hpp"
#include "canvas.hpp"
#include "metadata_panel.hpp"
#include "support/support.hpp"

#include <QApplication>
#include <QClipboard>
#include <QComboBox>
#include <QDesktopServices>
#include <QHBoxLayout>
#include <QJsonArray>
#include <QJsonDocument>
#include <QKeySequence>
#include <QLabel>
#include <QMouseEvent>
#include <QPixmap>
#include <QPixmapCache>
#include <QPushButton>
#include <QSettings>
#include <QShortcut>
#include <QSplitter>
#include <QTimer>
#include <QUrl>
#include <QVBoxLayout>

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
