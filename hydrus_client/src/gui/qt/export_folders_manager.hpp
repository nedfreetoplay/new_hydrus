#pragma once

// WS-15.1.4: manager dialog + legacy page moved verbatim from bridge.hpp (move-only split).

#include <QDialog>
#include <QJsonObject>
#include <QLabel>
#include <QPushButton>
#include <QString>
#include <QTableWidget>
#include <QWidget>

class HydrusMainWindow;
class HydrusExportFoldersDialog : public QDialog {
public:
    HydrusExportFoldersDialog(HydrusMainWindow *main, const QJsonObject &state, QWidget *parent = nullptr);

    void set_state(const QJsonObject &state);

private:
    void with_selected(const QString &action);
    void refresh_buttons();

    HydrusMainWindow *main = nullptr;
    QJsonObject data;
    QTableWidget *table = nullptr;
    QPushButton *edit_button = nullptr;
    QPushButton *delete_button = nullptr;
};

class HydrusExportFoldersPage : public QWidget {
public:
    HydrusExportFoldersPage(HydrusMainWindow *main, const QString &page_id, const QJsonObject &state);

    const QString &page_id() const;
    void update_state(const QJsonObject &state);

private:
    void with_selected(const QString &action);

    HydrusMainWindow *main = nullptr;
    QString id;
    QTableWidget *table = nullptr;
    QLabel *status = nullptr;
};
