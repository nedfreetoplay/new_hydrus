#pragma once

// WS-15.2.4b: typed popup-messages dialog (no QJsonObject).

#include <QDialog>

class QLabel;
class QTableWidget;
class HydrusMainWindow;
struct NotificationsDialogState;

class HydrusNotificationsDialog : public QDialog {
public:
    HydrusNotificationsDialog(HydrusMainWindow *main_window, const NotificationsDialogState &state, QWidget *parent = nullptr);

private:
    void rebuild(const NotificationsDialogState &state);

    HydrusMainWindow *main = nullptr;
    QLabel *summary = nullptr;
    QTableWidget *table = nullptr;
};
