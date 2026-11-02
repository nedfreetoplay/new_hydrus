#pragma once

// WS-13.9: QPainter replacement for the QtCharts `FileHistory` chart
// (QtCharts is not in this build). White background, grey grid, axes on the
// left/bottom, `yyyy-MM-dd` tick labels rotated 90 degrees, legend on top
// with colour markers, 2px series lines. All values come from Rust
// (`gui/app/charts.rs`); this widget only draws them.

#include <QColor>
#include <QVector>
#include <QWidget>

struct FileHistoryPoint;
struct FileHistorySeries;

class HydrusFileHistoryChart : public QWidget {
public:
    struct DrawSeries {
        QString name;
        QColor color;
        bool visible = true;
        QVector<QPointF> points; // x = unix seconds, y = file count
    };

    explicit HydrusFileHistoryChart(QWidget *parent = nullptr);

    void setSeries(const QVector<DrawSeries> &series);
    void setYRange(qint64 min_y, qint64 max_y);
    void setXRange(qint64 start_secs, qint64 end_secs);

protected:
    void paintEvent(QPaintEvent *event) override;

private:
    QVector<DrawSeries> series_;
    qint64 min_y_ = 0;
    qint64 max_y_ = 1;
    qint64 start_secs_ = 0;
    qint64 end_secs_ = 0;
    bool has_range_ = false;
};
