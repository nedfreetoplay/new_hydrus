#pragma once

// WS-15.1.3c1: manage tags dialog moved from bridge.cpp (move-only split;
// inline methods outlined into manage_tags_dialog.cpp, no logic change).

#include <QDialog>
#include <QJsonObject>
#include <QStringList>

class QComboBox;
class QLabel;
class QLineEdit;
class QListWidget;
class HydrusMainWindow;

class HydrusManageTagsDialog : public QDialog {
public:
    HydrusManageTagsDialog(HydrusMainWindow *main_window, const QJsonObject &state, QWidget *parent = nullptr);

private:
    QJsonObject current_service() const;
    void rebuild();
    void send(const QStringList &values, const QString &operation);
    void add_tags();
    void remove_tags();

    HydrusMainWindow *main = nullptr;
    QJsonObject data;
    QComboBox *services = nullptr;
    QLabel *summary = nullptr;
    QListWidget *tags = nullptr;
    QLineEdit *input = nullptr;
};
