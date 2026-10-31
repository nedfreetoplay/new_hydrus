#pragma once

// WS-15.1.2b: HydrusThumbnailGrid moved verbatim from bridge.hpp/bridge.cpp (move-only split).

#include <QJsonArray>
#include <QListWidget>
#include <QString>
#include <QStringList>

class QContextMenuEvent;
class QDragEnterEvent;
class QDragMoveEvent;
class QDropEvent;
class QWheelEvent;
class HydrusDuplicatesPage;
class HydrusMainWindow;
class HydrusQueryPage;

// The thumbnail grid. Hydrus uses a custom graphics scene; a uniform-icon
// QListWidget gives the same grid behaviour with much less code.
class HydrusThumbnailGrid : public QListWidget {
public:
    HydrusThumbnailGrid(HydrusMainWindow *main, HydrusQueryPage *page);
    HydrusThumbnailGrid(HydrusMainWindow *main, HydrusDuplicatesPage *page);

    void set_media(const QJsonArray &media);
    void set_focused(const QString &hash);
    void set_layout(bool masonry, bool horizontal, int scale_percent);
    QString focused_hash() const;
    QStringList selected_hashes() const;

protected:
    void contextMenuEvent(QContextMenuEvent *event) override;
    void wheelEvent(QWheelEvent *event) override;
    void startDrag(Qt::DropActions supported_actions) override;
    void dragEnterEvent(QDragEnterEvent *event) override;
    void dragMoveEvent(QDragMoveEvent *event) override;
    void dropEvent(QDropEvent *event) override;

private:
    HydrusMainWindow *main = nullptr;
    HydrusQueryPage *page = nullptr;
    HydrusDuplicatesPage *dup_page = nullptr;
    QString page_id_value;
    bool masonry = false;
    bool horizontal = false;
    int scale_percent = 100;
};
