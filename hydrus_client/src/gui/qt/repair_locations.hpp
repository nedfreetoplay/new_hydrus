#pragma once

// WS-15.2.4b: typed repair-locations dialog (no QJsonObject).

#include <QDialog>
#include <QStringList>

class QCheckBox;
class QLabel;
class QTableWidget;
class HydrusMainWindow;
enum class UiAction : std::int32_t;
struct RepairFileLocationsDialogState;

class HydrusRepairFileLocationsDialog : public QDialog {
public:
    HydrusRepairFileLocationsDialog(HydrusMainWindow *main, const RepairFileLocationsDialogState &state, QWidget *parent = nullptr);

    void set_state(const RepairFileLocationsDialogState &state);

private:
    void send(UiAction action, const QString &payload = QString());
    QStringList selected_prefixes() const;

    HydrusMainWindow *main = nullptr;
    QTableWidget *table = nullptr;
    QLabel *summary = nullptr;
    QLabel *message = nullptr;
    QCheckBox *create_thumbnails = nullptr;
};
