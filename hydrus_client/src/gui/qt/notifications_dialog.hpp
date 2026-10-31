#pragma once

// WS-15.1.3c2: popup messages dialog moved from bridge.cpp (move-only split;
// inline methods outlined into notifications_dialog.cpp, no logic change).

#include <QDialog>
#include <QJsonArray>
#include <QJsonObject>

class QLabel;
class QTableWidget;
class HydrusMainWindow;

class HydrusNotificationsDialog : public QDialog {
public:
    HydrusNotificationsDialog(HydrusMainWindow *main_window, const QJsonObject &state, QWidget *parent = nullptr);

private:
    void rebuild(const QJsonArray &entries);

    HydrusMainWindow *main = nullptr;
    QLabel *summary = nullptr;
    QTableWidget *table = nullptr;
};
