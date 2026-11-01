#pragma once

// WS-15.1.3c2: account types dialog moved from bridge.hpp/bridge.cpp (move-only split).

#include <QDialog>
#include <QJsonObject>
#include <QString>

#include <vector>

class QTableWidget;
class HydrusMainWindow;

// Non-modal account type manager for the selected server service.
// WS-15.2.4b: typed seam; editors read the stored vectors, never JSON.

struct AccountTypesDialogState;
struct AccountTypeRow;

struct AccountPermissionInfo {
    int content_type = 0;
    int action = -1;
    bool has_action = false;
};

struct AccountPermissionActionInfo {
    int value = -1;
    bool has_value = false;
    QString label;
};

struct AccountPermissionGroupInfo {
    int content_type = 0;
    QString label;
    std::vector<AccountPermissionActionInfo> actions;
};

struct AccountTypeInfo {
    QString key;
    QString title;
    QString summary;
    QString auto_creation_text;
    bool is_null = false;
    int auto_create_accounts = 0;
    int auto_create_time_delta = 86400;
    std::vector<AccountPermissionInfo> permissions;
};

class HydrusAccountTypesDialog : public QDialog {
public:
    explicit HydrusAccountTypesDialog(HydrusMainWindow *main, const AccountTypesDialogState &state, QWidget *parent = nullptr);

    void set_state(const AccountTypesDialogState &state);

private:
    void add_entry();
    void edit_selected();
    void delete_selected();
    int selected_index() const;

    HydrusMainWindow *main = nullptr;
    std::vector<AccountTypeInfo> type_infos;
    std::vector<AccountPermissionGroupInfo> possible_infos;
    QTableWidget *table = nullptr;
};
