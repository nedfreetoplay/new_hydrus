#pragma once

// WS-15.1.3c2: local import filename tagging dialog moved from bridge.hpp/bridge.cpp (move-only split).

#include <QDialog>
#include <QHash>
#include <QJsonObject>
#include <QString>

class QLabel;
class QListWidget;
class HydrusMainWindow;

// Per-local-tag-service filename tagging rules for the local import review.
// Shares the rule editor with the import folder dialog.
class HydrusLocalImportTaggingDialog : public QDialog {
public:
    HydrusLocalImportTaggingDialog(HydrusMainWindow *main, const QJsonObject &state, QWidget *parent = nullptr);

    void set_state(const QJsonObject &state);

private:
    void rebuild();
    void edit_rules(const QString &key, const QString &name, const QJsonObject &current);
    void save();

    HydrusMainWindow *main = nullptr;
    QJsonObject data;
    QJsonObject filename_tagging;
    QHash<QString, QString> service_name_to_key;
    QListWidget *list = nullptr;
    QLabel *help = nullptr;
    QString example_path;
    bool import_after_save = false;
};
