#pragma once

// WS-15.1.3c2: local import review dialog moved from bridge.hpp/bridge.cpp (move-only split).

#include <QDialog>
#include <QStringList>

#include <vector>

struct LocalImportPathRow;
struct LocalImportReviewDialogState;

class QCheckBox;
class QLabel;
class QPushButton;
class QTableWidget;
class HydrusMainWindow;
enum class UiAction : std::int32_t;

// Local import review panel: parse paths, show good/bad/sidecar rows, then
// import the good ones (optionally deleting the originals).
class HydrusLocalImportReviewDialog : public QDialog {
public:
    HydrusLocalImportReviewDialog(HydrusMainWindow *main, const LocalImportReviewDialogState &state, QWidget *parent = nullptr);

    void set_state(const LocalImportReviewDialogState &state);

private:
    void rebuild();
    QStringList selected_paths() const;

    HydrusMainWindow *main = nullptr;
    std::vector<LocalImportPathRow> paths_state;
    QTableWidget *table = nullptr;
    QLabel *summary = nullptr;
    QLabel *warning = nullptr;
    QCheckBox *search = nullptr;
    QCheckBox *delete_after_success = nullptr;
    QPushButton *import_button = nullptr;
    QPushButton *tag_button = nullptr;
};
