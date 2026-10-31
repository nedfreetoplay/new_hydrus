#pragma once

// WS-15.1.3c2: new page chooser moved from bridge.cpp (move-only split;
// inline methods outlined into page_chooser.cpp, no logic change).

#include <QDialog>
#include <QString>

class QPushButton;
class QStackedWidget;
class QVBoxLayout;
class QWidget;

class HydrusPageChooser : public QDialog {
public:
    explicit HydrusPageChooser(const QString &notebook_id, QWidget *parent = nullptr);

    QString result;

private:
    QWidget *make_page();
    QPushButton *add_button(const QString &text, const QString &type);
    QPushButton *add_nav(const QString &text, int target);
    void build_home();
    void build_search();
    void build_download();
    void build_special();

    QString notebook;
    QStackedWidget *stack = nullptr;
    QVBoxLayout *page_layout = nullptr;
};
