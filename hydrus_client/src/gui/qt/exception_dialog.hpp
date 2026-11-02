#pragma once

// WS-13.8: typed exception report dialog (no QJsonObject). One instance is
// reused across repeated errors: a new report refreshes the texts instead of
// opening a second modal, mirroring Hydrus' popup message manager.

#include <QDialog>

class QLabel;
class QPushButton;
class QTextEdit;
class HydrusMainWindow;
struct ExceptionReportState;

class HydrusExceptionDialog : public QDialog {
public:
    HydrusExceptionDialog(HydrusMainWindow *main_window, const ExceptionReportState &state, QWidget *parent = nullptr);

    void refresh(const ExceptionReportState &state);

private:
    HydrusMainWindow *main = nullptr;
    QLabel *summary = nullptr;
    QTextEdit *trace_view = nullptr;
    QPushButton *trace_toggle = nullptr;
    QString clipboard_text;
};
