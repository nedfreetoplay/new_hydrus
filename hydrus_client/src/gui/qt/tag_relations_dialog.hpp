#pragma once

// WS-15.1.3c1: tag relations dialog moved verbatim from bridge.hpp/bridge.cpp (move-only split).

#include <QDialog>
#include <QJsonObject>
#include <QString>

class QComboBox;
class QTabWidget;
class QTableWidget;
class HydrusMainWindow;

// Tag sibling/parent editor. One service at a time, like Hydrus' dialog.
class HydrusTagRelationsDialog : public QDialog {
public:
    HydrusTagRelationsDialog(HydrusMainWindow *main, const QJsonObject &state, QWidget *parent = nullptr);

    void set_state(const QJsonObject &state);

private:
    void rebuild();
    void add_relation();
    void remove_selected();
    QTableWidget *current_table() const;
    QString current_relation() const;

    HydrusMainWindow *main = nullptr;
    QJsonObject data;
    QComboBox *services = nullptr;
    QTabWidget *tabs = nullptr;
    QTableWidget *siblings = nullptr;
    QTableWidget *parents = nullptr;
};
