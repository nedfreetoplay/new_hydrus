#pragma once

// WS-15.2.4b: typed orphan-files dialog (no QJsonObject).

#include <QDialog>
#include <QStringList>

class QLabel;
class QTableWidget;
class HydrusMainWindow;
struct OrphanFilesDialogState;

class HydrusOrphanFilesDialog : public QDialog {
public:
    explicit HydrusOrphanFilesDialog(HydrusMainWindow *main, const OrphanFilesDialogState &state, QWidget *parent = nullptr);

    void set_state(const OrphanFilesDialogState &state);

private:
    void delete_selected(bool all);
    void move_selected(bool all);
    QStringList selected_paths(bool all) const;

    HydrusMainWindow *main = nullptr;
    QTableWidget *table = nullptr;
    QLabel *status = nullptr;
};
