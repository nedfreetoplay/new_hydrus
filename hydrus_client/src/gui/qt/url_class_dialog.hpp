#pragma once

// WS-15.1.3c1: url class editor moved from network_tables.cpp (move-only split;
// inline methods outlined into url_class_dialog.cpp, no logic change).

#include <QDialog>
#include <QJsonArray>
#include <QJsonObject>

class QCheckBox;
class QComboBox;
class QLabel;
class QLineEdit;
class QPlainTextEdit;

class HydrusUrlClassDialog : public QDialog {
public:
    HydrusUrlClassDialog(const QJsonObject &entry, const QJsonArray &types, QWidget *parent = nullptr);

    QJsonObject payload() const;

private:
    void run_test();

    QJsonObject data;
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
