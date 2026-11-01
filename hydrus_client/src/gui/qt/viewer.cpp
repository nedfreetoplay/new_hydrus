
#include <hydrus_client/src/gui/qt/api.cxx.h>
#include "support/support.hpp"

#include <QApplication>
#include <QClipboard>
#include <QComboBox>
#include <QDesktopServices>
#include <QHBoxLayout>
#include <QJsonArray>
#include <QJsonDocument>
#include <QKeySequence>
#include <QKeyEvent>
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
#include <QWheelEvent>

#include "support/support.hpp"

// WS-15.2.2b: constructor and update_state moved to typed_media.cpp
// (typed ViewerState; the QJsonObject seam is gone).

const QString &HydrusViewerWindow::viewer_id() const {
    return id;
}

// WS-13.13: gesture/key resolution through Rust (media_viewer_browser).
static QString viewer_gesture_action(const QString &gesture) {
    const QByteArray context_bytes = QByteArrayLiteral("media_viewer_browser");
    const QByteArray gesture_bytes = gesture.toUtf8();
    return to_qstring(hydrus_mouse_gesture_action(
        rust::Str(context_bytes.constData(), context_bytes.size()),
        rust::Str(gesture_bytes.constData(), gesture_bytes.size())));
}

static QString viewer_key_action(const QString &combo) {
    const QByteArray context_bytes = QByteArrayLiteral("media_viewer_browser");
    const QByteArray combo_bytes = combo.toUtf8();
    return to_qstring(hydrus_keyboard_action(
        rust::Str(context_bytes.constData(), context_bytes.size()),
        rust::Str(combo_bytes.constData(), combo_bytes.size())));
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

        return QWidget::eventFilter(watched, event);
    }

    if (watched == canvas) {
        // WS-13.13: wheel and close gestures resolve through Rust
        // (media_viewer_browser context, Python ClientDefaults bindings).
        if (event->type() == QEvent::Wheel) {
            const auto *wheel = static_cast<QWheelEvent *>(event);
            QStringList parts;
            if (wheel->modifiers() & Qt::ControlModifier) {
                parts << QStringLiteral("ctrl");
            }
            parts << (wheel->angleDelta().y() > 0 ? QStringLiteral("scroll up")
                                                  : QStringLiteral("scroll down"));
            const QString gesture = parts.join(QStringLiteral("+")) + QStringLiteral(" press");
            const QString action = viewer_gesture_action(gesture);
            if (action == QStringLiteral("previous_file")) {
                navigate(QStringLiteral("prev"));
                return true;
            }
            if (action == QStringLiteral("next_file")) {
                navigate(QStringLiteral("next"));
                return true;
            }
            if (action == QStringLiteral("zoom_in")) {
                canvas->zoom_by(zoom_step);
                return true;
            }
            if (action == QStringLiteral("zoom_out")) {
                canvas->zoom_by(1.0 / zoom_step);
                return true;
            }
            return QWidget::eventFilter(watched, event);
        }
        if (event->type() == QEvent::MouseButtonDblClick) {
            const auto *mouse = static_cast<QMouseEvent *>(event);
            if (mouse->button() == Qt::LeftButton
                && viewer_gesture_action(QStringLiteral("left double"))
                    == QStringLiteral("close_viewer")) {
                close();
                return true;
            }
            return QWidget::eventFilter(watched, event);
        }
        if (event->type() == QEvent::MouseButtonPress) {
            const auto *mouse = static_cast<QMouseEvent *>(event);
            if (mouse->button() == Qt::MiddleButton
                && viewer_gesture_action(QStringLiteral("middle press"))
                    == QStringLiteral("close_viewer")) {
                close();
                return true;
            }
        }
    }

    return QWidget::eventFilter(watched, event);
}

void HydrusViewerWindow::keyPressEvent(QKeyEvent *event) {
    // WS-13.13: nav keys resolve through Rust so custom sets apply. Only the
    // keys Python binds in media_viewer_browser are handled here; the rest
    // keeps the hardcoded viewer shortcuts in typed_viewer.cpp.
    QString key;
    switch (event->key()) {
    case Qt::Key_Up: key = QStringLiteral("Up"); break;
    case Qt::Key_Down: key = QStringLiteral("Down"); break;
    case Qt::Key_Left: key = QStringLiteral("Left"); break;
    case Qt::Key_Right: key = QStringLiteral("Right"); break;
    case Qt::Key_PageUp: key = QStringLiteral("PageUp"); break;
    case Qt::Key_PageDown: key = QStringLiteral("PageDown"); break;
    case Qt::Key_Home: key = QStringLiteral("Home"); break;
    case Qt::Key_End: key = QStringLiteral("End"); break;
    default: break;
    }
    if (!key.isEmpty() && event->modifiers() == Qt::NoModifier) {
        const QString action = viewer_key_action(key);
        if (action == QStringLiteral("previous_file")) {
            navigate(QStringLiteral("prev"));
            event->accept();
            return;
        }
        if (action == QStringLiteral("next_file")) {
            navigate(QStringLiteral("next"));
            event->accept();
            return;
        }
        if (action == QStringLiteral("first_file")) {
            navigate(QStringLiteral("first"));
            event->accept();
            return;
        }
        if (action == QStringLiteral("last_file")) {
            navigate(QStringLiteral("last"));
            event->accept();
            return;
        }
    }
    QWidget::keyPressEvent(event);
}

void HydrusViewerWindow::navigate(const QString &direction) {
    const QByteArray viewer_bytes = id.toUtf8();
    const QByteArray direction_bytes = direction.toUtf8();
    app_action(*main, UiAction::ViewerNav, rust::Str(viewer_bytes.constData(), viewer_bytes.size()), rust::Str(direction_bytes.constData(), direction_bytes.size()));
}

void HydrusViewerWindow::closeEvent(QCloseEvent *event) {
    QSettings settings;
    settings.setValue(QStringLiteral("viewer/geometry"), saveGeometry());
    settings.sync();

    const QByteArray viewer_bytes = id.toUtf8();
    app_action(*main, UiAction::ViewerClose, rust::Str(viewer_bytes.constData(), viewer_bytes.size()), rust::Str());

    QWidget::closeEvent(event);
}
