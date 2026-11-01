
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

// WS-15.2.2b: constructor and update_state moved to typed_media.cpp
// (typed ViewerState; the QJsonObject seam is gone).

const QString &HydrusViewerWindow::viewer_id() const {
    return id;
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
