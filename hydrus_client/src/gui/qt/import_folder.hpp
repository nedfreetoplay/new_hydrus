#pragma once

// WS-15.1.3b: import folder editor helpers (move-only split).

#include <QDialog>
#include <QJsonObject>
#include <QString>

class QVBoxLayout;
class QWidget;
class HydrusMainWindow;

// Shared filename tagging helpers (import_folder_tagging.cpp).
QString filename_tagging_summary(const QJsonObject &options);
QJsonObject default_filename_tagging_options();
bool edit_filename_tagging_options(
    QWidget *parent,
    const QString &service_name,
    const QJsonObject &initial,
    QJsonObject *result);

// "metadata import" section of the edit import folder dialog: the filename
// tagging list with add/edit/remove wired to the dialog above.
void build_import_folder_tagging_section(
    HydrusMainWindow *main,
    QDialog *dialog,
    QWidget *content,
    QVBoxLayout *vbox,
    const QJsonObject &state,
    const QString &folder_path,
    QJsonObject &filename_tagging);

