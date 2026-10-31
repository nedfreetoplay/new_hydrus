#pragma once

// WS-15.1.3c2: clear orphan files dialog moved from bridge.hpp/bridge.cpp (move-only split).

#include <QDialog>
#include <QJsonObject>
#include <QStringList>

class QLabel;
class QTableWidget;
class HydrusMainWindow;

// Non-modal surplus file cleaner for the media storage.
class HydrusOrphanFilesDialog : public QDialog {
public:
    explicit HydrusOrphanFilesDialog(HydrusMainWindow *main, const QJsonObject &state, QWidget *parent = nullptr);

    void set_state(const QJsonObject &state);

private:
    void delete_selected(bool all);
    void move_selected(bool all);
    QStringList selected_paths(bool all) const;

    HydrusMainWindow *main = nullptr;
    QJsonObject data;
    QTableWidget *table = nullptr;
    QLabel *status = nullptr;
};
