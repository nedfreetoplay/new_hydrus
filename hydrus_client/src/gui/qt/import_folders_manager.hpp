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
class HydrusImportFoldersDialog : public QDialog {
public:
    HydrusImportFoldersDialog(HydrusMainWindow *main, const QJsonObject &state, QWidget *parent = nullptr);

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

// Import folders manager page.
class HydrusImportFoldersPage : public QWidget {
public:
    HydrusImportFoldersPage(HydrusMainWindow *main, const QString &page_id, const QJsonObject &state);

    const QString &page_id() const;
    void update_state(const QJsonObject &state);

private:
    void with_selected(const QString &action);

    HydrusMainWindow *main = nullptr;
    QString id;
    QTableWidget *table = nullptr;
    QLabel *status = nullptr;
};
