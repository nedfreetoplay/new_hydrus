#pragma once

// WS-15.1.3c1: command palette moved from bridge.cpp (move-only split; inline
// methods outlined into command_palette.cpp, no logic change).

#include <QDialog>

class QLineEdit;
class QListWidget;
class HydrusMainWindow;

class HydrusCommandPalette : public QDialog {
public:
    explicit HydrusCommandPalette(HydrusMainWindow *main_window, QWidget *parent = nullptr);

private:
    void run_current();

    HydrusMainWindow *main = nullptr;
    QLineEdit *filter = nullptr;
    QListWidget *list = nullptr;
};

void show_command_palette(HydrusMainWindow *main);
