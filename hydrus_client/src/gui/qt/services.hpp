#pragma once

// WS-15.1.3a: services manager moved verbatim from bridge.hpp/bridge.cpp (move-only split).

#include <QDialog>
#include <QJsonObject>
#include <QString>

class QCheckBox;
class QComboBox;
class QLabel;
class QLineEdit;
class QTableWidget;
class QTabWidget;
class HydrusMainWindow;

// Non-modal service manager. It is updated by `ServicesDialog` commands.
class HydrusServicesDialog : public QDialog {
public:
    explicit HydrusServicesDialog(HydrusMainWindow *main, const QJsonObject &state, QWidget *parent = nullptr);

    void set_state(const QJsonObject &state);

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
    QJsonObject selected_remote_service() const;
    QString selected_remote_key() const;

    HydrusMainWindow *main = nullptr;
    QJsonObject data;
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
QJsonObject service_type_entry(const QJsonObject &data, int value);
QString service_family(const QJsonObject &data, int value);
void build_service_editor(HydrusMainWindow *main, QWidget *parent, const QJsonObject &data, const QJsonObject &service);
QString cell_value(QTableWidget *table, int row, int column);
QString cell_key(QTableWidget *table, int row);
