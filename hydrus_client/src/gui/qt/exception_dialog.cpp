
// WS-13.8: typed exception report dialog (no QJsonObject). Mirrors the Hydrus
// popup message for uncaught errors: title line, first-line text, a
// show/hide traceback toggle and a copy-traceback-information button. The
// dialog is modeless and singleton: a repeated error refreshes the same
// window instead of cascading modals.

#include <hydrus_client/src/gui/qt/api.cxx.h>
#include "support/support.hpp"

#include <QApplication>
#include <QClipboard>
#include <QDialogButtonBox>
#include <QLabel>
#include <QPointer>
#include <QPushButton>
#include <QTextEdit>
#include <QVBoxLayout>

namespace {

QPointer<HydrusExceptionDialog> g_exception_dialog;

} // namespace

HydrusExceptionDialog::HydrusExceptionDialog(HydrusMainWindow *main_window, const ExceptionReportState &state, QWidget *parent)
    : QDialog(parent), main(main_window) {
    setWindowTitle(to_qstring(state.title));
    setAttribute(Qt::WA_DeleteOnClose);
    resize(640, 420);

    auto *layout = new QVBoxLayout(this);

    summary = new QLabel(this);
    summary->setWordWrap(true);
    summary->setTextInteractionFlags(Qt::TextSelectableByMouse);
    layout->addWidget(summary);

    trace_toggle = new QPushButton(QStringLiteral("show traceback"), this);
    connect(trace_toggle, &QPushButton::clicked, this, [this]() {
        const bool showing = trace_view->isVisible();
        trace_view->setVisible(!showing);
        trace_toggle->setText(showing ? QStringLiteral("show traceback") : QStringLiteral("hide traceback"));
    });
    layout->addWidget(trace_toggle);

    trace_view = new QTextEdit(this);
    trace_view->setReadOnly(true);
    trace_view->setLineWrapMode(QTextEdit::NoWrap);
    trace_view->setVisible(false);
    layout->addWidget(trace_view, 1);

    auto *copy_button = new QPushButton(QStringLiteral("copy traceback information"), this);
    connect(copy_button, &QPushButton::clicked, this, [this]() {
        if (QClipboard *clipboard = QApplication::clipboard()) {
            clipboard->setText(clipboard_text);
        }
    });
    layout->addWidget(copy_button);

    auto *buttons = new QDialogButtonBox(QDialogButtonBox::Close, this);
    connect(buttons, &QDialogButtonBox::rejected, this, &QDialog::accept);
    layout->addWidget(buttons);

    refresh(state);
}

void HydrusExceptionDialog::refresh(const ExceptionReportState &state) {
    setWindowTitle(to_qstring(state.title));
    summary->setText(to_qstring(state.display_text));
    trace_view->setPlainText(to_qstring(state.traceback_text));
    clipboard_text = to_qstring(state.clipboard_text);
}

void hydrus_show_exception(HydrusMainWindow &window, const ExceptionReportState &state) {
    if (g_exception_dialog) {
        g_exception_dialog->refresh(state);
        g_exception_dialog->show();
        g_exception_dialog->raise();
        g_exception_dialog->activateWindow();
        return;
    }

    auto *dialog = new HydrusExceptionDialog(&window, state, &window);
    g_exception_dialog = dialog;
    dialog->show();
    dialog->raise();
    dialog->activateWindow();
}
