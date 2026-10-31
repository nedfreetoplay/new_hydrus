#pragma once

// WS-15.1.3a: network manager moved verbatim from bridge.hpp/bridge.cpp (move-only split).

#include <QDialog>
#include <QJsonObject>
#include <QString>

class QCheckBox;
class QLabel;
class QLineEdit;
class QSpinBox;
class QTableWidget;
class HydrusMainWindow;

class HydrusNetworkDialog : public QDialog {
public:
    explicit HydrusNetworkDialog(HydrusMainWindow *main, const QJsonObject &state, QWidget *parent = nullptr);

    void set_state(const QJsonObject &state);

private:
    void save();
    void add_header();
    void remove_header();
    void add_url_class();
    void edit_url_class();
    void remove_url_class();
    void add_pool();
    void remove_pool();
    void add_bandwidth_rule();
    void remove_bandwidth_rule();

    HydrusMainWindow *main = nullptr;
    QJsonObject data;
    QLineEdit *http_proxy = nullptr;
    QLineEdit *https_proxy = nullptr;
    QLineEdit *no_proxy = nullptr;
    QSpinBox *timeout = nullptr;
    QSpinBox *max_jobs = nullptr;
    QSpinBox *max_jobs_domain = nullptr;
    QCheckBox *api_enabled = nullptr;
    QSpinBox *api_port = nullptr;
    QLineEdit *api_key = nullptr;
    QLabel *api_status = nullptr;
    QCheckBox *paused = nullptr;
    QTableWidget *headers = nullptr;
    QTableWidget *url_classes = nullptr;
    QTableWidget *pools = nullptr;
    QTableWidget *bandwidth_rules = nullptr;
};
