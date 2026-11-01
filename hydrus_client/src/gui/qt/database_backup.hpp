#pragma once

// WS-15.2.4b: typed database-backup dialog (no QJsonObject).

#include <QDialog>

class QLabel;
class QLineEdit;
class HydrusMainWindow;
struct DatabaseBackupDialogState;

class HydrusDatabaseBackupDialog : public QDialog {
public:
    explicit HydrusDatabaseBackupDialog(HydrusMainWindow *main, const DatabaseBackupDialogState &state, QWidget *parent = nullptr);

    void set_state(const DatabaseBackupDialogState &state);

private:
    void choose_path();

    HydrusMainWindow *main = nullptr;
    QLineEdit *path = nullptr;
    QLabel *last_backup = nullptr;
    QLabel *status = nullptr;
};
