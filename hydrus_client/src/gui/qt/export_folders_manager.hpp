#pragma once

// WS-15.1.4: manager dialog + legacy page moved verbatim from bridge.hpp (move-only split).

#include <QDialog>
#include <QLabel>
#include <QPushButton>
#include <QString>
#include <QTableWidget>
#include <QWidget>

class HydrusMainWindow;
enum class UiAction : std::int32_t;
struct FolderPageState;
struct FolderRow;
class HydrusExportFoldersDialog : public QDialog {
public:
    HydrusExportFoldersDialog(HydrusMainWindow *main, const FolderPageState &state, QWidget *parent = nullptr);

    void set_state(const FolderPageState &state);

private:
    void with_selected(UiAction action);
    void refresh_buttons();

    HydrusMainWindow *main = nullptr;
    QTableWidget *table = nullptr;
    QPushButton *edit_button = nullptr;
    QPushButton *delete_button = nullptr;
};

class HydrusExportFoldersPage : public QWidget {
public:
    HydrusExportFoldersPage(HydrusMainWindow *main, const QString &page_id, const FolderPageState &state);

    const QString &page_id() const;
    void update_state(const FolderPageState &state);

private:
    void with_selected(UiAction action);

    HydrusMainWindow *main = nullptr;
    QString id;
    QTableWidget *table = nullptr;
    QLabel *status = nullptr;
};
