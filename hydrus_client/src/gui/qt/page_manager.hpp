#pragma once

// WS-15.1.3c1: page manager moved from bridge.cpp (move-only split; inline
// methods outlined into page_manager.cpp, no logic change).

#include <QDialog>
#include <QIcon>
#include <QJsonObject>
#include <QString>
#include <QStringList>

class QLineEdit;
class QTreeWidget;
class QTreeWidgetItem;
class HydrusMainWindow;

class HydrusPageManager : public QDialog {
public:
    HydrusPageManager(HydrusMainWindow *main_window, const QJsonObject &state, QWidget *parent = nullptr);

private:
    QIcon icon_for(const QString &kind) const;
    void build(const QJsonObject &root);
    QTreeWidgetItem *add_node(QTreeWidgetItem *parent, const QJsonObject &node);
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
