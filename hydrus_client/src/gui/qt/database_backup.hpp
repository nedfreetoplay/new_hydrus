#pragma once

// WS-15.1.3c2: database backup dialog moved from bridge.hpp/bridge.cpp (move-only split).

#include <QDialog>
#include <QJsonObject>

class QLabel;
class QLineEdit;
class HydrusMainWindow;

// Non-modal database backup dialog.
class HydrusDatabaseBackupDialog : public QDialog {
public:
    explicit HydrusDatabaseBackupDialog(HydrusMainWindow *main, const QJsonObject &state, QWidget *parent = nullptr);

    void set_state(const QJsonObject &state);

private:
    void choose_path();

    HydrusMainWindow *main = nullptr;
    QLineEdit *path = nullptr;
    QLabel *last_backup = nullptr;
    QLabel *status = nullptr;
};
