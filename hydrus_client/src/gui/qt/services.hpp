#pragma once

// WS-15.1.3a: services manager moved verbatim from bridge.hpp/bridge.cpp (move-only split).

#include <QDialog>
#include <QJsonObject>
#include <QString>

#include <vector>

class QCheckBox;
class QComboBox;
class QLabel;
class QLineEdit;
class QTableWidget;
class QTabWidget;
class HydrusMainWindow;

// Non-modal service manager. It is updated by `ServicesDialog` commands.
//
// The seam is typed (WS-15.2.4b): the constructor and `set_state` take the
// cxx struct by reference (forward-declared here, as in subscriptions.hpp)
// and convert the fields the actions need into plain Qt storage. C++ never
// parses JSON here; the R1-exception server blobs stay `*_json` in Rust.
struct ServicesDialogState;
struct ServiceRow;
struct ServiceTypeRow;
struct RemoteServiceRow;
struct ServiceAccountRow;
struct LoginRow;

struct ServiceTypeInfo {
    QString label;
    int value = 0;
    QString family;
};

struct LocalServiceInfo {
    QString key;
    QString name;
    QString type_short;
    int type_int = 0;
    QString host;
    int port = 0;
    QString access_key;
    bool has_update_period = false;
    int update_period = 86400;
    bool has_nullify_period = false;
    int nullify_period = 90 * 86400;
    int max_storage = 0;
    bool log_uploader_ips = false;
};

struct RemoteServiceInfo {
    QString key;
    QString name;
    QString type_name;
    int port = 0;
    QString access_key;
};

struct ServiceAccountInfo {
    QString key;
    QString type_title;
    QString message;
};

struct ServiceLoginInfo {
    QString domain;
    QString username;
    QString password;
    QString access_key;
};

struct ServerAccountTypeInfo {
    QString key;
    QString title;
};

class HydrusServicesDialog : public QDialog {
public:
    explicit HydrusServicesDialog(HydrusMainWindow *main, const ServicesDialogState &state, QWidget *parent = nullptr);

    void set_state(const ServicesDialogState &state);

private:
    void add_service();
    void edit_selected();
    void delete_selected();
    void show_specifier();
    void edit_login();
    void remove_login();
    void connect_admin();
    void select_remote_service();
    void add_remote_service();
    void edit_remote_service();
    void remove_remote_service();
    void remote_service_action();
    void account_action();
    QString selected_key() const;
    int selected_remote_row() const;
    QString selected_remote_key() const;

    HydrusMainWindow *main = nullptr;
    std::vector<ServiceTypeInfo> type_infos;
    std::vector<LocalServiceInfo> local_services;
    std::vector<RemoteServiceInfo> remote_service_infos;
    std::vector<ServiceAccountInfo> account_infos;
    std::vector<ServiceLoginInfo> login_infos;
    std::vector<ServerAccountTypeInfo> server_account_types;
    QString admin_options_json;
    QString admin_service_info_json;
    QTabWidget *tabs = nullptr;
    QTableWidget *table = nullptr;
    QTableWidget *remote_services = nullptr;
    QTableWidget *accounts = nullptr;
    QTableWidget *logins = nullptr;
    QLabel *admin_status = nullptr;
    QLineEdit *admin_address = nullptr;
    QLineEdit *admin_key = nullptr;
    QLineEdit *remote_key = nullptr;
    bool opened_account_types = false;
};

// Shared helpers (services_helpers.cpp).
QString service_type_label(const std::vector<ServiceTypeInfo> &types, int value);
QString service_family(const std::vector<ServiceTypeInfo> &types, int value);
void build_service_editor(HydrusMainWindow *main, QWidget *parent, const std::vector<ServiceTypeInfo> &types, const LocalServiceInfo &service, const QString &original);
QString cell_value(QTableWidget *table, int row, int column);
QString cell_key(QTableWidget *table, int row);
QString cell_value(QTableWidget *table, int row, int column);
QString cell_key(QTableWidget *table, int row);
