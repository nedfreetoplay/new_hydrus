#pragma once

// WS-15.1.3c1: page manager moved from bridge.cpp (move-only split; inline
// methods outlined into page_manager.cpp, no logic change).
// WS-15.2.2a: dialog contents arrive as a typed PageTree (see typed_seams.cpp).

#include <QDialog>
#include <QIcon>
#include <QString>
#include <QStringList>

class QLineEdit;
class QTreeWidget;
class QTreeWidgetItem;
class HydrusMainWindow;

struct PageTree;

class HydrusPageManager : public QDialog {
public:
    HydrusPageManager(
        HydrusMainWindow *main_window,
        const PageTree &page_tree,
        QWidget *parent = nullptr);

private:
    QIcon icon_for(const QString &kind) const;
    void build(const PageTree &page_tree);
    bool filter_item(QTreeWidgetItem *item, const QString &needle);
    void apply_filter(const QString &needle);
    QString id_of(QTreeWidgetItem *item) const;
    void activate_current();
    void close_selected_pages();
    void close_all_pages();

    HydrusMainWindow *main = nullptr;
    QLineEdit *filter = nullptr;
    QTreeWidget *tree = nullptr;
    QStringList top_levels;
};
