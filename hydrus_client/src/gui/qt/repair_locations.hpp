#pragma once

// WS-15.1.3c2: repair file locations dialog moved from bridge.hpp/bridge.cpp (move-only split).

#include <QDialog>
#include <QJsonObject>
#include <QStringList>

class QCheckBox;
class QLabel;
class QTableWidget;
class HydrusMainWindow;

// Remap missing media/thumbnail subfolder roots.
class HydrusRepairFileLocationsDialog : public QDialog {
public:
    HydrusRepairFileLocationsDialog(HydrusMainWindow *main, const QJsonObject &state, QWidget *parent = nullptr);

    void set_state(const QJsonObject &state);

private:
    void rebuild();
    void send(const QString &action, const QJsonObject &payload = QJsonObject());
    QStringList selected_prefixes() const;

    HydrusMainWindow *main = nullptr;
    QJsonObject data;
    QTableWidget *table = nullptr;
    QLabel *summary = nullptr;
    QLabel *message = nullptr;
    QCheckBox *create_thumbnails = nullptr;
};
