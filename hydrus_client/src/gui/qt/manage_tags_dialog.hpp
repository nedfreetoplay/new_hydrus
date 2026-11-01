#pragma once

// WS-15.1.3c1: manage tags dialog moved from bridge.cpp (move-only split;
// inline methods outlined into manage_tags_dialog.cpp, no logic change).

#include <QDialog>
#include <QStringList>

#include <vector>

struct ManageTagService;
struct ManageTagEntry;
struct ManageTagsDialogState;

class QComboBox;
class QLabel;
class QLineEdit;
class QListWidget;
class HydrusMainWindow;

class HydrusManageTagsDialog : public QDialog {
public:
    HydrusManageTagsDialog(HydrusMainWindow *main_window, const ManageTagsDialogState &state, QWidget *parent = nullptr);

private:
    const ManageTagService *current_service() const;
    void rebuild();
    void send(const QStringList &values, const QString &operation);
    void add_tags();
    void remove_tags();

    HydrusMainWindow *main = nullptr;
    std::vector<ManageTagService> services_state;
    std::vector<std::string> hashes_state;
    QComboBox *services = nullptr;
    QLabel *summary = nullptr;
    QListWidget *tags = nullptr;
    QLineEdit *input = nullptr;
};
