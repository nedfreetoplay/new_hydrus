#pragma once

// WS-15.1.4: manager dialog + legacy page moved verbatim from bridge.hpp (move-only split).

#include <QDialog>
#include <QString>
#include <QLabel>
#include <QPushButton>
#include <QTableWidget>
#include <QWidget>

class HydrusMainWindow;
enum class UiAction : std::int32_t;
struct FolderPageState;
struct FolderRow;
class HydrusImportFoldersDialog : public QDialog {
public:
    HydrusImportFoldersDialog(HydrusMainWindow *main, const FolderPageState &state, QWidget *parent = nullptr);

    void set_state(const FolderPageState &state);

private:
    void with_selected(UiAction action);
    void refresh_buttons();

    HydrusMainWindow *main = nullptr;
    QTableWidget *table = nullptr;
    QPushButton *edit_button = nullptr;
    QPushButton *delete_button = nullptr;
};

// Import folders manager page.
class HydrusImportFoldersPage : public QWidget {
public:
    HydrusImportFoldersPage(HydrusMainWindow *main, const QString &page_id, const FolderPageState &state);

    const QString &page_id() const;
    void update_state(const FolderPageState &state);

private:
    void with_selected(UiAction action);

    HydrusMainWindow *main = nullptr;
    QString id;
    QTableWidget *table = nullptr;
    QLabel *status = nullptr;
};
