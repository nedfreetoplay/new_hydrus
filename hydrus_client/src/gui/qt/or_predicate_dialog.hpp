#pragma once

// WS-15.1.3c1: OR predicate editor moved from bridge.cpp (move-only split;
// inline methods outlined into or_predicate_dialog.cpp, no logic change).

#include <QDialog>
#include <QString>

class QPlainTextEdit;
class HydrusMainWindow;

class HydrusOrDialog : public QDialog {
public:
    HydrusOrDialog(const QString &initial, QWidget *parent = nullptr);

    QString expression() const;

private:
    QPlainTextEdit *editor = nullptr;
};

void show_or_predicate_dialog(HydrusMainWindow &main, const QString &page_id, const QString &initial);
