#pragma once

// WS-15.1.3c1: tag relations dialog moved verbatim from bridge.hpp/bridge.cpp (move-only split).

#include <QDialog>
#include <QString>

#include <vector>

struct TagRelationService;
struct TagRelationRow;
struct TagRelationsDialogState;

class QComboBox;
class QTabWidget;
class QTableWidget;
class HydrusMainWindow;

// Tag sibling/parent editor. One service at a time, like Hydrus' dialog.
class HydrusTagRelationsDialog : public QDialog {
public:
    HydrusTagRelationsDialog(HydrusMainWindow *main, const TagRelationsDialogState &state, QWidget *parent = nullptr);

    void set_state(const TagRelationsDialogState &state);

private:
    void rebuild();
    void add_relation();
    void remove_selected();
    QTableWidget *current_table() const;
    QString current_relation() const;

    HydrusMainWindow *main = nullptr;
    std::vector<TagRelationService> services_state;
    std::vector<TagRelationRow> relations_state;
    QComboBox *services = nullptr;
    QTabWidget *tabs = nullptr;
    QTableWidget *siblings = nullptr;
    QTableWidget *parents = nullptr;
};
