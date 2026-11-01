
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
#include <QMenu>
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
        // WS-13.13: right-release opens the viewer context menu (Python
        // CanvasPanel.ShowMenuFromSignal order); composition comes from Rust.
        if (event->type() == QEvent::MouseButtonRelease) {
            const auto *mouse = static_cast<QMouseEvent *>(event);
            if (mouse->button() == Qt::RightButton
                && viewer_gesture_action(QStringLiteral("right release"))
                    == QStringLiteral("show_context_menu")) {
                show_context_menu(mouse->globalPosition().toPoint());
                return true;
            }
        }
    }

    return QWidget::eventFilter(watched, event);
}

void HydrusViewerWindow::show_context_menu(const QPoint &global_pos) {
    const QByteArray viewer_bytes = id.toUtf8();
    const MenuBar menu = hydrus_viewer_menu(
        rust::Str(viewer_bytes.constData(), viewer_bytes.size()));
    if (menu.entries.empty()) {
        return;
    }
    if (menu.entries.empty()) {
        return;
    }
    QMenu popup;
    popup.setToolTipsVisible(true);
    std::vector<QMenu *> stack;
    stack.push_back(&popup);
    for (const MenuEntry &entry : menu.entries) {
        if (entry.depth <= 0 && entry.is_submenu) {
            // Top-level submenu rows own the depth-1 rows that follow them.
            QMenu *submenu = stack.front()->addMenu(to_qstring(entry.label));
            submenu->setToolTipsVisible(true);
            if (stack.size() == 1) {
                stack.push_back(submenu);
            } else {
                stack.back() = submenu;
            }
            continue;
        }
        while (stack.size() > 1 && static_cast<int>(stack.size()) > entry.depth + 1) {
            stack.pop_back();
        }
        QMenu *parent = stack.back();
        if (entry.is_separator) {
            parent->addSeparator();
            continue;
        }
        if (entry.is_submenu) {
            QMenu *submenu = parent->addMenu(to_qstring(entry.label));
            submenu->setToolTipsVisible(true);
            stack.push_back(submenu);
            continue;
        }
        const QString label = to_qstring(entry.label);
        if (label.isEmpty()) {
            continue;
        }
        QAction *action = parent->addAction(label);
        action->setEnabled(entry.enabled);
        const QString tooltip = to_qstring(entry.tooltip);
        if (!tooltip.isEmpty()) {
            action->setToolTip(tooltip);
        }
        const UiAction action_id = entry.action;
        const QString item_target = to_qstring(entry.target);
        const QString payload = to_qstring(entry.payload);
        connect(action, &QAction::triggered, this, [this, action_id, item_target, payload]() {
            if (action_id == UiAction::NotesManage) {
                const QByteArray hash_bytes = payload.toUtf8();
                app_manage_notes(*main, rust::Str(hash_bytes.constData(), hash_bytes.size()));
            } else if (action_id != UiAction::None) {
                main->notify(action_id, item_target, payload);
            }
        });
    }
    popup.exec(global_pos);
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
