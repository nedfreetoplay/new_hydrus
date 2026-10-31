#pragma once

// WS-15.1.3b: export folder editor helpers (move-only split).

#include <QCheckBox>
#include <QDialog>
#include <QJsonObject>
#include <QString>
#include <QWidget>

#include <functional>

class QGridLayout;
class QLineEdit;
class QVBoxLayout;
class HydrusMainWindow;

// "sidecar exporting" section of the edit export folder dialog.
void build_export_sidecar_section(
    HydrusMainWindow *main,
    QDialog *dialog,
    QWidget *content,
    QVBoxLayout *vbox,
    const QJsonObject &folder,
    QLineEdit *query,
    std::function<QGridLayout *()> make_grid,
    QString &sidecar_format,
    QCheckBox *&overwrite_next,
    QCheckBox *&overwrite_always);

