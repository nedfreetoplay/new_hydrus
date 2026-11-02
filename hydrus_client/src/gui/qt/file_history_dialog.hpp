#pragma once

// WS-13.9: `database -> view file history` dialog. Mirrors
// `ReviewFileHistory`: left search panel (simplified: the port has no file
// search context editor, the domain is fixed to all known files), right side
// with the status line, the `HydrusFileHistoryChart`, `show ...` checkboxes,
// `min y`/`max y` spinners plus `refit y axis`, and two `QDateEdit`s plus
// `refit x axis`. All labels, ranges and series come from Rust; this dialog
// only assigns widgets and forwards refit requests to
// `hydrus_file_history_auto_range`.

#include <QDialog>

#include <QDate>
#include <QVector>

#include "rust/cxx.h"

#include "file_history_chart.hpp"

struct FileHistoryDialogState;

class QCheckBox;
class QDateEdit;
class QLabel;
class QSpinBox;
class HydrusMainWindow;

class HydrusFileHistoryDialog : public QDialog {
public:
    HydrusFileHistoryDialog(HydrusMainWindow *main_window, const FileHistoryDialogState &state, QWidget *parent = nullptr);

    void refresh(const FileHistoryDialogState &state);

private:
    void autoYRange();
    void autoXRange();
    void updateYRange();
    void updateXRange();
    void applyCheckboxes();

    rust::Vec<FileHistoryPoint> rustPoints(int index) const;

    QVector<HydrusFileHistoryChart::DrawSeries> series_;
    bool user_custom_x_ = false;
    bool user_custom_y_ = false;

    QLabel *status_ = nullptr;
    HydrusFileHistoryChart *chart_ = nullptr;
    QWidget *chart_panel_ = nullptr;
    QCheckBox *show_all_ = nullptr;
    QCheckBox *show_inbox_ = nullptr;
    QCheckBox *show_archive_ = nullptr;
    QCheckBox *show_deleted_ = nullptr;
    QSpinBox *min_y_ = nullptr;
    QSpinBox *max_y_ = nullptr;
    QDateEdit *start_date_ = nullptr;
    QDateEdit *end_date_ = nullptr;
};

void hydrus_show_file_history(HydrusMainWindow &window, const FileHistoryDialogState &state);
