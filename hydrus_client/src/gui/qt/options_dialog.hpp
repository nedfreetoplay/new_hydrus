#pragma once

// WS-15.1.3c1: options dialog moved from bridge.cpp (move-only split; inline
// methods outlined into options_dialog.cpp, no logic change).

#include <QCheckBox>
#include <QDialog>
#include <QJsonObject>
#include <QLineEdit>
#include <QSpinBox>
#include <QString>
#include <QVector>
#include <QWidget>

class HydrusMainWindow;

struct OptionField {
    QString key;
    QString label;
    QString type;
    bool noneable = false;
    int min_value = -2000000000;
    int max_value = 2000000000;
    QString none_phrase;
    QCheckBox *boolean = nullptr;
    QSpinBox *integer = nullptr;
    QLineEdit *text = nullptr;
    QWidget *noneable_container = nullptr;
    QSpinBox *noneable_integer = nullptr;
    QCheckBox *noneable_checkbox = nullptr;
    QWidget *page = nullptr;
    QWidget *label_widget = nullptr;
    QWidget *input_widget = nullptr;
};

class HydrusOptionsDialog : public QDialog {
public:
    HydrusOptionsDialog(HydrusMainWindow *main_window, const QJsonObject &state, QWidget *parent);

private:
    void apply();

    HydrusMainWindow *main = nullptr;
    QLineEdit *search = nullptr;
    QVector<OptionField> fields;
};
