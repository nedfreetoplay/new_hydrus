#pragma once

// WS-15.1.3c2: client api permissions dialog moved from bridge.hpp/bridge.cpp (move-only split).

#include <QDialog>
#include <QJsonObject>
#include <QString>

#include <vector>

class QLabel;
class QTableWidget;
class HydrusMainWindow;

// Non-modal Client API permission manager.
// WS-15.2.4b: typed seam; editors read the stored vectors, never JSON.

struct ApiPermissionsDialogState;
struct ApiPermissionRow;

struct ApiPermissionRuleInfo {
    QString slice;
    QString display;
    QString rule;
};

struct ApiAvailablePermissionInfo {
    int value = 0;
    QString label;
};

struct ApiPermissionInfo {
    QString name;
    QString access_key;
    QString summary;
    bool permits_everything = true;
    std::vector<int> basic_permissions;
    std::vector<ApiPermissionRuleInfo> rules;
};

class HydrusApiPermissionsDialog : public QDialog {
public:
    explicit HydrusApiPermissionsDialog(HydrusMainWindow *main, const ApiPermissionsDialogState &state, QWidget *parent = nullptr);

    void set_state(const ApiPermissionsDialogState &state);

private:
    void add_entry();
    void edit_selected();
    void delete_selected();
    int selected_index() const;

    HydrusMainWindow *main = nullptr;
    std::vector<ApiPermissionInfo> permission_infos;
    std::vector<ApiAvailablePermissionInfo> available_infos;
    QTableWidget *table = nullptr;
    QLabel *status = nullptr;
};
