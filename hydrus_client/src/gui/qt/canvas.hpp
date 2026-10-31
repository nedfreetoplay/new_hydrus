#pragma once

// WS-15.1.2a: HydrusCanvas moved verbatim from bridge.cpp (move-only split).

#include <QGraphicsView>
#include <QString>

class QGraphicsPixmapItem;
class QGraphicsScene;
class QMovie;
class QResizeEvent;
class QWheelEvent;
class QWidget;

// A zoomable, pannable view for one image. Used both by the small preview in a
// page sidebar and by the media viewer window.
class HydrusCanvas : public QGraphicsView {
public:
    explicit HydrusCanvas(QWidget *parent = nullptr);

    bool load_image(const QString &path);
    void clear_image();
    void fit_image();
    void reset_zoom();
    void zoom_by(double factor);
    void zoom_to(double factor);
    void rotate_by(int degrees);
    void set_recenter_on_resize(bool value);
    void set_background_mode(const QString &mode);
    void set_pan_mode(bool enabled);
    void toggle_animation();
    bool is_animated() const;

protected:
    void resizeEvent(QResizeEvent *event) override;
    void wheelEvent(QWheelEvent *event) override;

private:
    QGraphicsScene *scene = nullptr;
    QGraphicsPixmapItem *pixmap_item = nullptr;
    QMovie *movie = nullptr;
    bool fit_mode = true;
    bool recenter_on_resize = true;
    int rotation = 0;
};
