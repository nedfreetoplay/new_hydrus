
// WS-13.9: QPainter file-history chart (QtCharts is not in this build).
// Layout mirrors `ClientGUICharts.FileHistory`: legend on top, plot area with
// a grey grid, y axis on the left (`%i` labels), x axis at the bottom with
// `yyyy-MM-dd` labels at 90 degrees, 2px lines in the QtCharts Light-theme
// pen order passed in from Rust.

#include <hydrus_client/src/gui/qt/api.cxx.h>
#include "support/support.hpp"
#include <QDateTime>
#include <QFontMetrics>
#include <QPainter>
#include <QPaintEvent>
#include <QPen>

namespace {

constexpr int kLeftMargin = 72;
constexpr int kRightMargin = 16;
constexpr int kTopMargin = 30;
constexpr int kBottomMargin = 96;
constexpr int kYTicks = 5;
constexpr int kXTicks = 8;

QString date_label(qint64 secs) {
    return QDateTime::fromSecsSinceEpoch(secs).toString(QStringLiteral("yyyy-MM-dd"));
}

} // namespace

HydrusFileHistoryChart::HydrusFileHistoryChart(QWidget *parent) : QWidget(parent) {
    setMinimumSize(720, 480);
    setAutoFillBackground(true);
    QPalette palette = this->palette();
    palette.setColor(QPalette::Window, Qt::white);
    setPalette(palette);
}

void HydrusFileHistoryChart::setSeries(const QVector<DrawSeries> &series) {
    series_ = series;
    update();
}

void HydrusFileHistoryChart::setYRange(qint64 min_y, qint64 max_y) {
    min_y_ = min_y;
    max_y_ = max_y <= min_y ? min_y + 1 : max_y;
    update();
}

void HydrusFileHistoryChart::setXRange(qint64 start_secs, qint64 end_secs) {
    start_secs_ = start_secs;
    end_secs_ = end_secs;
    has_range_ = end_secs > start_secs;
    update();
}

void HydrusFileHistoryChart::paintEvent(QPaintEvent *event) {
    Q_UNUSED(event);

    QPainter painter(this);
    painter.setRenderHint(QPainter::Antialiasing, true);
    painter.fillRect(rect(), Qt::white);

    const QRect plot(
        kLeftMargin,
        kTopMargin,
        width() - kLeftMargin - kRightMargin,
        height() - kTopMargin - kBottomMargin);
    if (plot.width() <= 0 || plot.height() <= 0) {
        return;
    }

    const qint64 y_span = max_y_ - min_y_ > 0 ? max_y_ - min_y_ : 1;
    const qint64 x_span = has_range_ ? (end_secs_ - start_secs_) : 1;

    auto to_plot = [&](qint64 x, qint64 y) {
        const double px = plot.left() + (double(x - start_secs_) / double(x_span)) * plot.width();
        const double py = plot.bottom() - (double(y - min_y_) / double(y_span)) * plot.height();
        return QPointF(px, py);
    };

    // Grid + y axis (`%i` labels).
    painter.setPen(QPen(QColor(0xdd, 0xdd, 0xdd), 1));
    QFontMetrics metrics = painter.fontMetrics();
    for (int i = 0; i <= kYTicks; ++i) {
        const qint64 value = min_y_ + y_span * i / kYTicks;
        const double y = plot.bottom() - double(i) / double(kYTicks) * plot.height();
        painter.drawLine(plot.left(), int(y), plot.right(), int(y));
        painter.setPen(Qt::black);
        painter.drawText(
            4, int(y) - metrics.height() / 2, kLeftMargin - 8, metrics.height(),
            Qt::AlignRight | Qt::AlignVCenter, QString::number(value));
        painter.setPen(QPen(QColor(0xdd, 0xdd, 0xdd), 1));
    }

    // Grid + x axis (`yyyy-MM-dd` labels at 90 degrees).
    for (int i = 0; i <= kXTicks; ++i) {
        const qint64 secs = has_range_ ? start_secs_ + x_span * i / kXTicks : start_secs_;
        const double x = plot.left() + double(i) / double(kXTicks) * plot.width();
        painter.drawLine(int(x), plot.top(), int(x), plot.bottom());
        painter.setPen(Qt::black);
        painter.save();
        painter.translate(int(x) + 4, plot.bottom() + 4);
        painter.rotate(90);
        painter.drawText(0, 0, date_label(secs));
        painter.restore();
        painter.setPen(QPen(QColor(0xdd, 0xdd, 0xdd), 1));
    }

    painter.setPen(Qt::black);
    painter.drawRect(plot);

    // Series, 2px lines in the Rust-provided colours.
    for (const DrawSeries &series : series_) {
        if (!series.visible || series.points.size() < 2) {
            continue;
        }

        QPen pen(series.color, 2);
        painter.setPen(pen);

        QPolygonF poly;
        poly.reserve(series.points.size());
        for (const QPointF &point : series.points) {
            poly << to_plot(qint64(point.x()), qint64(point.y()));
        }
        painter.drawPolyline(poly);
    }

    // Legend on top: colour marker plus name, hidden series greyed out.
    int legend_x = kLeftMargin;
    for (const DrawSeries &series : series_) {
        const QColor marker = series.visible ? series.color : QColor(0xbb, 0xbb, 0xbb);
        painter.fillRect(legend_x, 8, 18, 10, marker);
        painter.setPen(series.visible ? Qt::black : QColor(0x88, 0x88, 0x88));
        const int text_width = metrics.horizontalAdvance(series.name) + 8;
        painter.drawText(legend_x + 22, 8, text_width, metrics.height(), Qt::AlignLeft | Qt::AlignVCenter, series.name);
        legend_x += 22 + text_width + 18;
    }
}
