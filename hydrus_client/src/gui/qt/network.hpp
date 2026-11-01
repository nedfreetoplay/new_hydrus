#pragma once

// WS-15.1.3a: network manager moved verbatim from bridge.hpp/bridge.cpp (move-only split).

#include <QDialog>
#include <QJsonObject>
#include <QString>

#include <vector>

class QCheckBox;
class QLabel;
class QLineEdit;
class QSpinBox;
class QTableWidget;
class HydrusMainWindow;

struct NetworkDialogState;
struct NetworkUrlClassRow;
struct UrlTypeRow;

struct NetworkUrlClassInfo {
    QString name;
    int url_type = 0;
    QString preferred_scheme;
    QString netloc;
    QString components_text;
    QString parameters_text;
    bool match_subdomains = false;
    bool keep_matched_subdomains = false;
    bool can_produce_multiple_files = false;
    bool should_be_associated_with_files = true;
    QString example_url;
};

struct NetworkUrlTypeInfo {
    int value = 0;
    QString label;
};

class HydrusNetworkDialog : public QDialog {
public:
    explicit HydrusNetworkDialog(HydrusMainWindow *main, const NetworkDialogState &state, QWidget *parent = nullptr);

    void set_state(const NetworkDialogState &state);

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
    std::vector<NetworkUrlClassInfo> url_class_infos;
    std::vector<NetworkUrlTypeInfo> url_type_infos;
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
