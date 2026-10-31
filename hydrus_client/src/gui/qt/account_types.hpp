#pragma once

// WS-15.1.3c2: account types dialog moved from bridge.hpp/bridge.cpp (move-only split).

#include <QDialog>
#include <QJsonObject>

class QTableWidget;
class HydrusMainWindow;

// Non-modal account type manager for the selected server service.
class HydrusAccountTypesDialog : public QDialog {
public:
    explicit HydrusAccountTypesDialog(HydrusMainWindow *main, const QJsonObject &state, QWidget *parent = nullptr);

    void set_state(const QJsonObject &state);

private:
    void add_entry();
    void edit_selected();
    void delete_selected();
    QJsonObject selected_entry() const;

    HydrusMainWindow *main = nullptr;
    QJsonObject data;
    QTableWidget *table = nullptr;
};
