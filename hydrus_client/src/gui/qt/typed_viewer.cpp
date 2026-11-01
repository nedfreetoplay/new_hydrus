// WS-15.2.2b: typed media-viewer seam bodies (Rust structs, no JSON).
// Covers HydrusViewerWindow; the thumbnail grid and metadata panel live in
// typed_media.cpp and typed_metadata.cpp (one file per screen, R3).
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

#include <QApplication>
#include <QByteArray>
#include <QClipboard>
#include <QComboBox>
#include <QDesktopServices>
#include <QHBoxLayout>
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

// The class definitions (members, other methods) come from the exported
// bridge.hpp chain included above; the typed declarations live in the topic
// headers next to forward declarations of the structs.

// --- HydrusViewerWindow ----------------------------------------------------

HydrusViewerWindow::HydrusViewerWindow(HydrusMainWindow *main_window, const ViewerState &state)
    : QWidget(nullptr), main(main_window) {
    id = to_qstring(state.viewer_id);
    page_id = to_qstring(state.page_id);
    hash = to_qstring(state.hash);

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

    const auto add_hover = [this, hover_layout](const QString &label, UiAction action) {
        auto *button = new QPushButton(label, hover_frame);
        button->setAutoDefault(false);
        connect(button, &QPushButton::clicked, this, [this, action]() {
            if (!hash.isEmpty()) {
                main->notify(action, page_id, hash);
            }
        });
        hover_layout->addWidget(button);
    };

    add_hover(QStringLiteral("archive"), UiAction::ArchiveFiles);
    add_hover(QStringLiteral("inbox"), UiAction::InboxFiles);
    add_hover(QStringLiteral("delete"), UiAction::DeleteFiles);
    add_hover(QStringLiteral("tags"), UiAction::ManageTags);

    auto *notes_button = new QPushButton(QStringLiteral("notes"), hover_frame);
    notes_button->setAutoDefault(false);
    connect(notes_button, &QPushButton::clicked, this, [this]() {
        if (hash.isEmpty()) {
            return;
        }

        const QByteArray hash_bytes = hash.toUtf8();
        app_manage_notes(*main, rust::Str(hash_bytes.constData(), hash_bytes.size()));
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
        app_action(*main, UiAction::ManageTags, rust::Str(page_bytes.constData(), page_bytes.size()), rust::Str(hash_bytes.constData(), hash_bytes.size()));
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
        app_action(*main, UiAction::ManageTags, rust::Str(page_bytes.constData(), page_bytes.size()), rust::Str(hash_bytes.constData(), hash_bytes.size()));
    });

    bind_shortcut(QStringLiteral("A"), [this]() {
        const QByteArray page_bytes = page_id.toUtf8();
        const QByteArray hash_bytes = hash.toUtf8();
        app_action(*main, UiAction::ArchiveFiles, rust::Str(page_bytes.constData(), page_bytes.size()), rust::Str(hash_bytes.constData(), hash_bytes.size()));
    });

    bind_shortcut(QStringLiteral("I"), [this]() {
        const QByteArray page_bytes = page_id.toUtf8();
        const QByteArray hash_bytes = hash.toUtf8();
        app_action(*main, UiAction::InboxFiles, rust::Str(page_bytes.constData(), page_bytes.size()), rust::Str(hash_bytes.constData(), hash_bytes.size()));
    });

    bind_shortcut(QStringLiteral("Delete"), [this]() {
        const QByteArray page_bytes = page_id.toUtf8();
        const QByteArray hash_bytes = hash.toUtf8();
        app_action(*main, UiAction::DeleteFiles, rust::Str(page_bytes.constData(), page_bytes.size()), rust::Str(hash_bytes.constData(), hash_bytes.size()));
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

void HydrusViewerWindow::update_state(const ViewerState &state) {
    page_id = to_qstring(state.page_id);
    hash = to_qstring(state.hash);
    file_path = to_qstring(state.file_path);
    url = to_qstring(state.url);

    const QString path = to_qstring(state.path);
    const QString title = to_qstring(state.title);
    const bool on_top = state.always_on_top;
    const bool borderless = state.frameless;

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

    const double step = static_cast<double>(state.zoom_step_percent);

    if (step >= 5.0) {
        zoom_step = 1.0 + step / 100.0;
    }

    canvas->set_recenter_on_resize(state.recenter);

    setWindowTitle(title.isEmpty() ? QStringLiteral("media viewer") : title);

    if (!path.isEmpty()) {
        canvas->load_image(path);
    } else {
        canvas->clear_image();
    }

    animation_button->setEnabled(canvas->is_animated());
    animation_button->setText(QStringLiteral("pause"));

    for (const QString &neighbour : {
             to_qstring(state.previous_path),
             to_qstring(state.next_path),
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

    metadata->set_state(state.metadata);
    metadata->set_suggestions(main->suggestions());

    status->setText(to_qstring(state.info));
}
