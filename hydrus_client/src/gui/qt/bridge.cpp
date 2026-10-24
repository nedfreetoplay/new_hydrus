#include "bridge.hpp"

#include <QAction>
#include <QLabel>
#include <QMenuBar>
#include <QString>
#include <QTreeWidgetItem>
#include <QVBoxLayout>
#include <QWidget>

namespace {

QString to_qstring(rust::Str text) {
    return QString::fromUtf8(text.data(), static_cast<int>(text.size()));
}

} // namespace

HydrusMainWindow::HydrusMainWindow(QWidget *parent) : QMainWindow(parent) {}

void HydrusMainWindow::build_shell() {
    if (tabs != nullptr) {
        return;
    }

    tabs = new QTabWidget(this);
    tabs->setTabsClosable(true);
    tabs->setMovable(true);

    sidebar = new QTreeWidget(this);
    sidebar->setHeaderLabel("Pages");
    sidebar->setMinimumWidth(180);

    splitter = new QSplitter(Qt::Horizontal, this);
    splitter->addWidget(sidebar);
    splitter->addWidget(tabs);
    splitter->setStretchFactor(1, 1);

    setCentralWidget(splitter);
    statusBar()->showMessage("Ready");

    auto *file_menu = menuBar()->addMenu("File");
    file_menu->addSeparator();

    menuBar()->addMenu("Pages");
    menuBar()->addMenu("Help");
}

QMenu *HydrusMainWindow::menu_for(const QString &title) {
    for (QAction *action : menuBar()->actions()) {
        if (action->text() == title && action->menu() != nullptr) {
            return action->menu();
        }
    }

    return menuBar()->addMenu(title);
}

int HydrusMainWindow::add_page(rust::Str title) {
    auto *page = new QWidget(tabs);
    auto *layout = new QVBoxLayout(page);
    auto *label = new QLabel(QString("Hydrus page: %1").arg(to_qstring(title)), page);
    label->setAlignment(Qt::AlignCenter);
    layout->addWidget(label);

    const int index = tabs->addTab(page, to_qstring(title));
    tabs->setCurrentIndex(index);

    return index;
}

void HydrusMainWindow::add_sidebar_item(rust::Str title) {
    if (sidebar == nullptr) {
        return;
    }

    sidebar->addTopLevelItem(new QTreeWidgetItem(QStringList(to_qstring(title))));
}

void HydrusMainWindow::add_menu_action(rust::Str menu_title, rust::Str action_title) {
    menu_for(to_qstring(menu_title))->addAction(to_qstring(action_title));
}

void HydrusMainWindow::add_exit_action(rust::Str menu_title, rust::Str action_title) {
    QAction *action = menu_for(to_qstring(menu_title))->addAction(to_qstring(action_title));
    connect(action, &QAction::triggered, qApp, &QApplication::quit);
}

void HydrusMainWindow::set_status(rust::Str message) {
    statusBar()->showMessage(to_qstring(message));
}

void HydrusMainWindow::set_title(rust::Str title) {
    setWindowTitle(to_qstring(title));
}

void HydrusMainWindow::resize_window(int width, int height) {
    resize(width, height);
}

std::unique_ptr<QApplication> new_application() {
    static int argc = 1;
    static char name[] = "hydrus_client";
    static char *argv[] = {name, nullptr};
    return std::make_unique<QApplication>(argc, argv);
}

int exec_application(QApplication &application) {
    return application.exec();
}

void quit_application(QApplication &application) {
    application.quit();
}

std::unique_ptr<HydrusMainWindow> new_hydrus_main_window() {
    return std::make_unique<HydrusMainWindow>();
}

void show_hydrus_main_window(HydrusMainWindow &window) {
    window.show();
}

void build_hydrus_shell(HydrusMainWindow &window) {
    window.build_shell();
}

int hydrus_add_page(HydrusMainWindow &window, rust::Str title) {
    return window.add_page(title);
}

void hydrus_add_sidebar_item(HydrusMainWindow &window, rust::Str title) {
    window.add_sidebar_item(title);
}

void hydrus_add_menu_action(HydrusMainWindow &window, rust::Str menu, rust::Str action) {
    window.add_menu_action(menu, action);
}

void hydrus_add_exit_action(HydrusMainWindow &window, rust::Str menu, rust::Str action) {
    window.add_exit_action(menu, action);
}

void hydrus_set_status(HydrusMainWindow &window, rust::Str message) {
    window.set_status(message);
}

void hydrus_set_title(HydrusMainWindow &window, rust::Str title) {
    window.set_title(title);
}

void hydrus_resize_window(HydrusMainWindow &window, int width, int height) {
    window.resize_window(width, height);
}
