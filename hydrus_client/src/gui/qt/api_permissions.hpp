#pragma once

// WS-15.1.3c2: client api permissions dialog moved from bridge.hpp/bridge.cpp (move-only split).

#include <QDialog>
#include <QJsonObject>

class QLabel;
class QTableWidget;
class HydrusMainWindow;

// Non-modal network manager.
// Non-modal Client API permission manager.
class HydrusApiPermissionsDialog : public QDialog {
public:
    explicit HydrusApiPermissionsDialog(HydrusMainWindow *main, const QJsonObject &state, QWidget *parent = nullptr);

    void set_state(const QJsonObject &state);

private:
    void add_entry();
    void edit_selected();
    void delete_selected();
    QJsonObject selected_entry() const;

    HydrusMainWindow *main = nullptr;
    QJsonObject data;
    QTableWidget *table = nullptr;
    QLabel *status = nullptr;
};
