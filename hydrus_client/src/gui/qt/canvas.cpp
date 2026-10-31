#include "canvas.hpp"

#include <QColor>
#include <QGraphicsPixmapItem>
#include <QGraphicsScene>
#include <QMovie>
#include <QPainter>
#include <QPixmap>
#include <QPixmapCache>
#include <QResizeEvent>
#include <QWheelEvent>

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

    if (mode == QStringLiteral("white")) {
        setBackgroundBrush(QColor(255, 255, 255));
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
