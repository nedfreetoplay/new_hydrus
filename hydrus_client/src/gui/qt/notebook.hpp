#pragma once

// WS-15.1.2a: HydrusTabBar + HydrusNotebook moved verbatim from bridge.cpp (move-only split).

#include <QPoint>
#include <QString>
#include <QTabBar>
#include <QTabWidget>

class QDragEnterEvent;
class QDragMoveEvent;
class QDropEvent;
class QEvent;
class QMouseEvent;
class QWidget;
class HydrusMainWindow;
class HydrusNotebook;

class HydrusTabBar : public QTabBar {
public:
    HydrusTabBar(HydrusMainWindow *main_window, HydrusNotebook *notebook_widget, QWidget *parent = nullptr);

protected:
    void mousePressEvent(QMouseEvent *event) override;

    void mouseMoveEvent(QMouseEvent *event) override;

    void dragEnterEvent(QDragEnterEvent *event) override;

    void dragMoveEvent(QDragMoveEvent *event) override;

    void dropEvent(QDropEvent *event) override;

private:
    HydrusMainWindow *main = nullptr;
    HydrusNotebook *notebook = nullptr;
    QPoint press_position{-1000, -1000};
};

// A notebook is a page of pages: tabs, a trailing "+" tab, and the usual
// close/rename/context-menu behaviour.
class HydrusNotebook : public QTabWidget {
public:
    HydrusNotebook(HydrusMainWindow *main, const QString &notebook_id, QWidget *parent = nullptr);

    const QString &notebook_id() const;

    void add_page_widget(const QString &page_id, const QString &title, QWidget *page);
    void finalize_notebook(const QString &current_page_id);
    void clear_pages();

    QString current_page_id() const;

protected:
    bool eventFilter(QObject *watched, QEvent *event) override;

private:
    void install_plus_tab();
    void show_tab_menu(const QPoint &position);
    void request_close(int index);

    HydrusMainWindow *main = nullptr;
    QString id;
    int plus_index = -1;
    int last_normal_index = 0;
};
