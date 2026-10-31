#include "pairs.hpp"

#include "../bridge.hpp"
#include "strings.hpp"

#include <QAction>
#include <QColor>
#include <QIcon>
#include <QJsonObject>
#include <QMenu>
#include <QObject>
#include <QPainter>
#include <QPixmap>
#include <QTableWidget>

QPixmap thumbnail_pixmap(const QString &path, int width, int height) {
    QPixmap box(width, height);
    box.fill(QColor(52, 52, 52));

    QPixmap source;

    if (!path.isEmpty()) {
        source.load(path);
    }

    QPainter painter(&box);

    if (source.isNull()) {
        painter.setPen(QColor(120, 128, 144));
        painter.drawRect(0, 0, width - 1, height - 1);
        painter.drawText(box.rect(), Qt::AlignCenter, QStringLiteral("no thumb"));
    } else {
        const QPixmap scaled = source.scaled(width, height, Qt::KeepAspectRatio, Qt::SmoothTransformation);
        painter.drawPixmap((width - scaled.width()) / 2, (height - scaled.height()) / 2, scaled);
    }

    painter.end();

    return box;
}

QTableWidgetItem *pair_hash_item(const QString &hash, const QString &thumb_path) {
    auto *item = new QTableWidgetItem(hash.left(16));
    item->setToolTip(hash);
    item->setData(Qt::UserRole, hash);

    const QPixmap pixmap = thumbnail_pixmap(thumb_path, 110, 90);
    item->setIcon(QIcon(pixmap));

    return item;
}

void show_selected_pair_rows(QTableWidget *table, HydrusMainWindow *main) {
    if (main == nullptr || table->selectionModel() == nullptr) {
        return;
    }

    QStringList hashes;

    for (const QModelIndex &index : table->selectionModel()->selectedRows()) {
        for (int column = 0; column < table->columnCount(); ++column) {
            const QTableWidgetItem *item = table->item(index.row(), column);
            const QString hash = item == nullptr ? QString() : item->data(Qt::UserRole).toString();

            if (!hash.isEmpty() && !hashes.contains(hash)) {
                hashes.append(hash);
            }
        }
    }

    if (hashes.isEmpty()) {
        return;
    }

    QJsonObject payload;
    payload.insert(QStringLiteral("hashes"), QJsonArray::fromStringList(hashes));
    main->notify(QStringLiteral("duplicates_show_pair"), QString(), json_payload(payload));
}

void install_pair_list_actions(QTableWidget *table, HydrusMainWindow *main) {
    table->setContextMenuPolicy(Qt::CustomContextMenu);

    QObject::connect(table, &QTableWidget::customContextMenuRequested, table, [table, main](const QPoint &pos) {
        if (!table->selectionModel()->hasSelection()) {
            return;
        }

        const int num_selected = table->selectionModel()->selectedRows().size();
        QMenu menu(table);
        const QString label = num_selected == 1
            ? QStringLiteral("show selected row in a new page")
            : QStringLiteral("show %1 rows in a new page").arg(num_selected);
        QAction *action = menu.addAction(label);
        action->setToolTip(QStringLiteral("Open a new page and show these pairs, even if deleted"));

        if (menu.exec(table->viewport()->mapToGlobal(pos)) == action) {
            show_selected_pair_rows(table, main);
        }
    });

    QObject::connect(table, &QTableWidget::cellActivated, table, [table, main](int, int) {
        show_selected_pair_rows(table, main);
    });
}

void fill_pair_table(
    QTableWidget *table,
    const QJsonArray &rows,
    int hash_columns,
    const QStringList &text_columns) {
    table->setRowCount(rows.size());

    for (int index = 0; index < rows.size(); ++index) {
        const QJsonObject entry = rows.at(index).toObject();
        const int text_column = hash_columns;
        const int column_count = hash_columns + text_columns.size();

        for (int column = 0; column < column_count; ++column) {
            if (table->item(index, column) == nullptr) {
                table->setItem(index, column, new QTableWidgetItem());
            }
        }

        if (hash_columns >= 1) {
            table->setItem(index, 0, pair_hash_item(
                entry.value(QStringLiteral("a")).toString(),
                entry.value(QStringLiteral("a_thumb")).toString()));
        }

        if (hash_columns >= 2) {
            table->setItem(index, 1, pair_hash_item(
                entry.value(QStringLiteral("b")).toString(),
                entry.value(QStringLiteral("b_thumb")).toString()));
        }

        for (int offset = 0; offset < text_columns.size(); ++offset) {
            const QString value = text_columns.at(offset);

            if (value == QStringLiteral("rule")) {
                table->item(index, text_column + offset)->setText(entry.value(QStringLiteral("rule")).toString());
            } else if (value == QStringLiteral("distance")) {
                table->item(index, text_column + offset)->setText(QString::number(entry.value(QStringLiteral("distance")).toInt()));
            } else if (value == QStringLiteral("decision")) {
                table->item(index, text_column + offset)->setText(entry.value(QStringLiteral("decision_label")).toString());
            } else if (value == QStringLiteral("banned")) {
                table->item(index, text_column + offset)->setText(entry.value(QStringLiteral("banned")).toBool()
                    ? QStringLiteral("yes")
                    : QString());
            }
        }

        table->setRowHeight(index, 96);
    }
}
