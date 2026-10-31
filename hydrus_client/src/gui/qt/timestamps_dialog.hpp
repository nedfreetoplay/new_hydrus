#pragma once

// WS-15.1.3c2: edit timestamps dialog moved from bridge.cpp (move-only split;
// inline methods outlined into timestamps_dialog.cpp, no logic change).

#include <QDialog>
#include <QJsonObject>
#include <QJsonValue>
#include <QString>

class QCheckBox;
class QDateTimeEdit;
class HydrusMainWindow;

class HydrusTimestampsDialog : public QDialog {
public:
    HydrusTimestampsDialog(HydrusMainWindow *main_window, const QJsonObject &state, QWidget *parent = nullptr);

private:
    void set_time(QCheckBox *enabled, QDateTimeEdit *editor, const QJsonValue &value);
    void save();

    HydrusMainWindow *main = nullptr;
    QString hash;
    QCheckBox *archived_enabled = nullptr;
    QCheckBox *modified_enabled = nullptr;
    QDateTimeEdit *archived = nullptr;
    QDateTimeEdit *modified = nullptr;
};
