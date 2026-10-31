#pragma once

#include <QJsonArray>
#include <QPixmap>
#include <QString>
#include <QStringList>

class HydrusMainWindow;
class QTableWidget;
class QTableWidgetItem;

// WS-15.1.1: thumbnail pair-list helpers moved verbatim from bridge.cpp.
// Default sizes live on this declaration; the definition must not repeat them.
QPixmap thumbnail_pixmap(const QString &path, int width = 150, int height = 125);
QTableWidgetItem *pair_hash_item(const QString &hash, const QString &thumb_path);
void show_selected_pair_rows(QTableWidget *table, HydrusMainWindow *main);
void install_pair_list_actions(QTableWidget *table, HydrusMainWindow *main);
void fill_pair_table(
    QTableWidget *table,
    const QJsonArray &rows,
    int hash_columns,
    const QStringList &text_columns);
