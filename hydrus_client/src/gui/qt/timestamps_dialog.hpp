#pragma once

// WS-15.2.4b: typed edit-timestamps dialog (no QJsonObject).

#include <QDialog>
#include <QString>

class QCheckBox;
class QDateTimeEdit;
class HydrusMainWindow;
struct TimestampsDialogState;

class HydrusTimestampsDialog : public QDialog {
public:
    HydrusTimestampsDialog(HydrusMainWindow *main_window, const TimestampsDialogState &state, QWidget *parent = nullptr);

private:
    void set_time(QCheckBox *enabled, QDateTimeEdit *editor, bool has_value, long long secs);
    void save();

    HydrusMainWindow *main = nullptr;
    QString hash;
    QCheckBox *archived_enabled = nullptr;
    QCheckBox *modified_enabled = nullptr;
    QDateTimeEdit *archived = nullptr;
    QDateTimeEdit *modified = nullptr;
};
