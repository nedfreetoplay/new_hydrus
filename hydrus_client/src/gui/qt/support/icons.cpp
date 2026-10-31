#include "icons.hpp"

#include "../bridge.hpp"

#include <QBrush>
#include <QColor>
#include <QDir>
#include <QFileInfo>
#include <QHash>
#include <QIcon>
#include <QPainter>
#include <QPen>
#include <QPixmap>
#include <QPolygonF>

#include <cmath>

QString hydrus_small_icon_file_name(const QString &kind) {
    static const QHash<QString, QString> mapping = {
        {QStringLiteral("paste"), QStringLiteral("paste.png")},
        {QStringLiteral("star"), QStringLiteral("star.svg")},
        {QStringLiteral("pause"), QStringLiteral("pause.png")},
        {QStringLiteral("play"), QStringLiteral("play.png")},
        {QStringLiteral("stop"), QStringLiteral("stop.png")},
        {QStringLiteral("refresh"), QStringLiteral("refresh.png")},
        {QStringLiteral("gear"), QStringLiteral("cog.svg")},
        {QStringLiteral("trash"), QStringLiteral("trash.png")},
        {QStringLiteral("highlight"), QStringLiteral("highlight.png")},
        {QStringLiteral("unhighlight"), QStringLiteral("clear_highlight.png")},
        {QStringLiteral("clear_highlight"), QStringLiteral("clear_highlight.png")},
        {QStringLiteral("help"), QStringLiteral("help.png")},
        {QStringLiteral("lock"), QStringLiteral("lock.png")},
        {QStringLiteral("lock_open"), QStringLiteral("lock_open.png")},
        {QStringLiteral("zoom_in"), QStringLiteral("zoom_in.png")},
        {QStringLiteral("zoom_out"), QStringLiteral("zoom_out.png")},
        {QStringLiteral("zoom_switch"), QStringLiteral("zoom_switch.png")},
        {QStringLiteral("fullscreen_switch"), QStringLiteral("fullscreen_switch.svg")},
        {QStringLiteral("file_pause"), QStringLiteral("file_pause.png")},
        {QStringLiteral("file_play"), QStringLiteral("file_play.png")},
        {QStringLiteral("gallery_pause"), QStringLiteral("gallery_pause.png")},
        {QStringLiteral("gallery_play"), QStringLiteral("gallery_play.png")},
        {QStringLiteral("retry"), QStringLiteral("retry.svg")},
        {QStringLiteral("link"), QStringLiteral("link.png")},
        {QStringLiteral("notes"), QStringLiteral("notes.png")},
        {QStringLiteral("ipfs"), QStringLiteral("ipfs.png")},
        {QStringLiteral("inbox"), QStringLiteral("inbox.png")},
        {QStringLiteral("archive"), QStringLiteral("archive.png")},
        {QStringLiteral("to_inbox"), QStringLiteral("to_inbox.png")},
        {QStringLiteral("undelete"), QStringLiteral("undelete.png")},
        {QStringLiteral("move"), QStringLiteral("move.png")},
        {QStringLiteral("open_externally"), QStringLiteral("open_externally.png")},
        {QStringLiteral("check"), QStringLiteral("check.png")},
        {QStringLiteral("eye"), QStringLiteral("eye.svg")},
        {QStringLiteral("eye_closed"), QStringLiteral("eye_closed.svg")},
        {QStringLiteral("position_first"), QStringLiteral("position_first.svg")},
        {QStringLiteral("position_previous"), QStringLiteral("position_previous.svg")},
        {QStringLiteral("position_next"), QStringLiteral("position_next.svg")},
        {QStringLiteral("position_last"), QStringLiteral("position_last.svg")},
        {QStringLiteral("position_random"), QStringLiteral("position_random.svg")},
        {QStringLiteral("page_of_pages"), QStringLiteral("page_of_pages.png")},
        {QStringLiteral("thumbnails"), QStringLiteral("thumbnails.png")},
        {QStringLiteral("listctrl"), QStringLiteral("listctrl.png")},
        {QStringLiteral("import_options_stack_chart"), QStringLiteral("import_options_stack_chart.svg")},
        {QStringLiteral("copy"), QStringLiteral("copy.png")},
        {QStringLiteral("hydrus"), QStringLiteral("hydrus.png")},
        {QStringLiteral("hydrus_32"), QStringLiteral("hydrus_32.png")},
    };
    return mapping.value(kind);
}

QIcon hydrus_small_icon(const QString &kind) {
    const QString file_name = hydrus_small_icon_file_name(kind);
    if (!file_name.isEmpty()) {
        const rust::String dir_rust = hydrus_static_dir();
        const QString dir = QString::fromUtf8(dir_rust.data(), static_cast<int>(dir_rust.size()));
        if (!dir.isEmpty()) {
            const QString path = QDir(dir).filePath(file_name);
            if (QFileInfo::exists(path)) {
                static QHash<QString, QIcon> cache;
                auto it = cache.find(path);
                if (it != cache.end()) {
                    return it.value();
                }
                QIcon icon(path);
                if (!icon.isNull()) {
                    cache.insert(path, icon);
                    return icon;
                }
            }
        }
    }

    QPixmap pixmap(16, 16);
    pixmap.fill(Qt::transparent);

    QPainter painter(&pixmap);
    painter.setRenderHint(QPainter::Antialiasing, true);

    if (kind == QStringLiteral("paste")) {
        painter.setPen(QPen(QColor(150, 140, 120), 1));
        painter.setBrush(QColor(245, 240, 220));
        painter.drawRect(QRect(3, 3, 9, 11));
        painter.drawRect(QRect(5, 1, 5, 3));
        painter.setPen(QPen(QColor(110, 100, 80), 1));
        painter.drawLine(5, 6, 10, 6);
        painter.drawLine(5, 8, 10, 8);
        painter.drawLine(5, 10, 9, 10);
    } else if (kind == QStringLiteral("star")) {
        QPolygonF star;
        const QPointF centre(8.0, 8.0);
        for (int index = 0; index < 10; ++index) {
            const double radius = index % 2 == 0 ? 7.0 : 3.0;
            const double angle = -1.5707963267948966 + index * 0.6283185307179586;
            star << QPointF(
                centre.x() + radius * std::cos(angle),
                centre.y() + radius * std::sin(angle));
        }
        painter.setPen(QPen(QColor(140, 110, 10), 1));
        painter.setBrush(QColor(240, 190, 20));
        painter.drawPolygon(star);
    } else if (kind == QStringLiteral("clear")) {
        painter.setPen(QPen(QColor(45, 110, 200), 1.5));
        painter.setBrush(Qt::NoBrush);
        painter.drawRoundedRect(QRect(2, 2, 12, 12), 2, 2);
        painter.setBrush(QColor(45, 110, 200));
        painter.drawRect(QRect(5, 5, 6, 6));
    } else if (kind == QStringLiteral("pause")) {
        painter.setPen(Qt::NoPen);
        painter.setBrush(QColor(70, 70, 70));
        painter.drawRect(QRect(4, 3, 3, 10));
        painter.drawRect(QRect(9, 3, 3, 10));
    } else if (kind == QStringLiteral("play")) {
        painter.setPen(Qt::NoPen);
        painter.setBrush(QColor(70, 70, 70));
        QPolygonF triangle;
        triangle << QPointF(4.0, 3.0) << QPointF(13.0, 8.0) << QPointF(4.0, 13.0);
        painter.drawPolygon(triangle);
    } else if (kind == QStringLiteral("stop")) {
        painter.setPen(QPen(QColor(130, 130, 130), 1));
        painter.setBrush(QColor(225, 225, 225));
        painter.drawEllipse(QRectF(1.5, 1.5, 13.0, 13.0));
        painter.setPen(Qt::NoPen);
        painter.setBrush(QColor(120, 120, 120));
        painter.drawRect(QRectF(5.0, 5.0, 6.0, 6.0));
    } else if (kind == QStringLiteral("retry")) {
        painter.setPen(QPen(QColor(70, 70, 70), 1.6));
        painter.setBrush(Qt::NoBrush);
        painter.drawArc(QRect(2, 2, 12, 12), 40 * 16, 280 * 16);
        painter.setBrush(QColor(70, 70, 70));
        QPolygonF arrow;
        arrow << QPointF(11.0, 1.0) << QPointF(14.5, 4.5) << QPointF(10.5, 5.5);
        painter.drawPolygon(arrow);
    } else if (kind == QStringLiteral("trash")) {
        painter.setPen(QPen(QColor(90, 90, 90), 1.2));
        painter.setBrush(QColor(235, 235, 235));
        painter.drawRect(QRect(3, 4, 10, 10));
        painter.drawLine(2, 4, 14, 4);
        painter.drawLine(6, 2, 10, 2);
        painter.setPen(QPen(QColor(90, 90, 90), 1));
        painter.drawLine(6, 6, 6, 12);
        painter.drawLine(9, 6, 9, 12);
    } else if (kind == QStringLiteral("highlight")) {
        QPolygonF star;
        const QPointF centre(8.0, 8.0);
        for (int index = 0; index < 10; ++index) {
            const double radius = index % 2 == 0 ? 6.5 : 2.8;
            const double angle = -1.5707963267948966 + index * 0.6283185307179586;
            star << QPointF(
                centre.x() + radius * std::cos(angle),
                centre.y() + radius * std::sin(angle));
        }
        painter.setPen(QPen(QColor(90, 90, 90), 1));
        painter.setBrush(QColor(235, 235, 235));
        painter.drawPolygon(star);
    } else if (kind == QStringLiteral("unhighlight")) {
        QPolygonF star;
        const QPointF centre(8.0, 8.0);
        for (int index = 0; index < 10; ++index) {
            const double radius = index % 2 == 0 ? 6.5 : 2.8;
            const double angle = -1.5707963267948966 + index * 0.6283185307179586;
            star << QPointF(
                centre.x() + radius * std::cos(angle),
                centre.y() + radius * std::sin(angle));
        }
        painter.setPen(QPen(QColor(130, 130, 130), 1));
        painter.setBrush(Qt::NoBrush);
        painter.drawPolygon(star);
        painter.setPen(QPen(QColor(160, 60, 60), 1.6));
        painter.drawLine(3, 13, 13, 3);
    } else if (kind == QStringLiteral("gear")) {
        painter.setPen(QPen(QColor(70, 70, 70), 1.2));
        painter.setBrush(QColor(205, 205, 205));
        painter.drawEllipse(QRectF(3.5, 3.5, 9.0, 9.0));

        for (int index = 0; index < 8; ++index) {
            const double angle = index * 0.7853981633974483;
            painter.drawLine(
                QPointF(8.0 + 4.6 * std::cos(angle), 8.0 + 4.6 * std::sin(angle)),
                QPointF(8.0 + 6.8 * std::cos(angle), 8.0 + 6.8 * std::sin(angle)));
        }

        painter.setBrush(Qt::white);
        painter.drawEllipse(QRectF(6.0, 6.0, 4.0, 4.0));
    } else if (kind == QStringLiteral("copy")) {
        painter.setPen(QPen(QColor(95, 95, 95), 1));
        painter.setBrush(QColor(250, 250, 250));
        painter.drawRect(QRect(2, 4, 8, 10));
        painter.drawRect(QRect(5, 2, 9, 10));
        painter.setPen(QPen(QColor(150, 150, 150), 1));
        painter.drawLine(7, 5, 11, 5);
        painter.drawLine(7, 7, 11, 7);
        painter.drawLine(7, 9, 10, 9);
    } else if (kind == QStringLiteral("help")) {
        painter.setPen(QPen(QColor(20, 80, 160), 1));
        painter.setBrush(QColor(45, 120, 210));
        painter.drawEllipse(QRectF(2.0, 2.0, 12.0, 12.0));
        painter.setPen(QPen(Qt::white, 1.4));
        painter.drawArc(QRectF(6.0, 4.0, 5.0, 5.0), 60 * 16, 250 * 16);
        painter.drawLine(QPointF(8.4, 8.4), QPointF(8.4, 9.6));
        painter.setPen(Qt::NoPen);
        painter.setBrush(Qt::white);
        painter.drawEllipse(QRectF(7.8, 11.0, 1.6, 1.6));
    } else {
        painter.setPen(QPen(QColor(130, 105, 45), 1.5));
        painter.setBrush(QColor(220, 185, 95));
        painter.drawRoundedRect(QRect(4, 7, 9, 7), 1, 1);
        painter.setBrush(Qt::NoBrush);
        painter.drawArc(QRect(5, 2, 7, 9), 0, 180 * 16);
    }

    painter.end();

    return QIcon(pixmap);
}
