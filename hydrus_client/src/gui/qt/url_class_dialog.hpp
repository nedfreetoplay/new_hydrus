#pragma once

// WS-15.1.3c1: url class editor moved from network_tables.cpp (move-only split;
// inline methods outlined into url_class_dialog.cpp, no logic change).
// WS-15.2.4b: typed entries (NetworkUrlClassInfo from network.hpp); the dialog
// builds the save payload as before (reverse channel stays JSON for now).

#include "network.hpp"

#include <QDialog>
#include <QJsonObject>
#include <QString>

class QCheckBox;
class QComboBox;
class QLabel;
class QLineEdit;
class QPlainTextEdit;

class HydrusUrlClassDialog : public QDialog {
public:
    HydrusUrlClassDialog(const NetworkUrlClassInfo &entry, const std::vector<NetworkUrlTypeInfo> &types, QWidget *parent = nullptr);

    QJsonObject payload() const;

private:
    void run_test();

    QString original;
    QLineEdit *name = nullptr;
    QComboBox *type = nullptr;
    QLineEdit *scheme = nullptr;
    QLineEdit *netloc = nullptr;
    QLineEdit *path = nullptr;
    QPlainTextEdit *parameters = nullptr;
    QLineEdit *example = nullptr;
    QCheckBox *match_subdomains = nullptr;
    QCheckBox *keep_subdomains = nullptr;
    QCheckBox *multiple = nullptr;
    QCheckBox *associate = nullptr;
    QLineEdit *test = nullptr;
    QLabel *result = nullptr;
};
