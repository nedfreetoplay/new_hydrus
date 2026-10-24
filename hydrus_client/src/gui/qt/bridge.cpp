#include "bridge.hpp"

#include <QAction>
#include <QCheckBox>
#include <QDialog>
#include <QDialogButtonBox>
#include <QGraphicsScene>
#include <QGraphicsView>
#include <QHeaderView>
#include <QIcon>
#include <QLabel>
#include <QLineEdit>
#include <QListWidget>
#include <QMainWindow>
#include <QMenu>
#include <QMenuBar>
#include <QPainter>
#include <QPixmap>
#include <QPushButton>
#include <QScrollArea>
#include <QSpinBox>
#include <QSplitter>
#include <QStatusBar>
#include <QStyle>
#include <QStyleFactory>
#include <QString>
#include <QTableWidget>
#include <QTreeWidget>
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

int HydrusMainWindow::add_page_widget(rust::Str title, HydrusPanel *panel) {
    if (panel == nullptr) {
        return -1;
    }

    const int index = tabs->addTab(panel, to_qstring(title));
    tabs->setCurrentIndex(index);

    return index;
}

int HydrusMainWindow::add_media_page(rust::Str title, HydrusMediaPanel *panel) {
    if (panel == nullptr) {
        return -1;
    }

    const int index = tabs->addTab(panel, to_qstring(title));
    tabs->setCurrentIndex(index);

    return index;
}

int HydrusMainWindow::add_canvas_page(rust::Str title, HydrusCanvas *canvas) {
    if (canvas == nullptr) {
        return -1;
    }

    const int index = tabs->addTab(canvas, to_qstring(title));
    tabs->setCurrentIndex(index);

    return index;
}

int HydrusMainWindow::add_scrolled_page(rust::Str title, HydrusScrolledPanel *panel) {
    if (panel == nullptr) {
        return -1;
    }

    const int index = tabs->addTab(panel, to_qstring(title));
    tabs->setCurrentIndex(index);

    return index;
}

int HydrusMainWindow::add_list_page(rust::Str title, HydrusListPanel *panel) {
    if (panel == nullptr) {
        return -1;
    }

    const int index = tabs->addTab(panel, to_qstring(title));
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

HydrusPanel::HydrusPanel(QWidget *parent) : QWidget(parent) {
    layout = new QVBoxLayout(this);
    layout->setContentsMargins(12, 12, 12, 12);
}

void HydrusPanel::add_label(rust::Str text) {
    auto *label = new QLabel(to_qstring(text), this);
    label->setWordWrap(true);
    layout->addWidget(label);
}

void HydrusPanel::add_button(rust::Str text) {
    layout->addWidget(new QPushButton(to_qstring(text), this));
}

void HydrusPanel::add_checkbox(rust::Str text, bool checked) {
    auto *checkbox = new QCheckBox(to_qstring(text), this);
    checkbox->setChecked(checked);
    layout->addWidget(checkbox);
}

void HydrusPanel::add_line_edit(rust::Str placeholder) {
    auto *edit = new QLineEdit(this);
    edit->setPlaceholderText(to_qstring(placeholder));
    layout->addWidget(edit);
}

void HydrusPanel::add_stretch() {
    layout->addStretch(1);
}

std::unique_ptr<HydrusPanel> new_hydrus_panel() {
    return std::make_unique<HydrusPanel>();
}

void hydrus_panel_add_label(HydrusPanel &panel, rust::Str text) {
    panel.add_label(text);
}

void hydrus_panel_add_button(HydrusPanel &panel, rust::Str text) {
    panel.add_button(text);
}

void hydrus_panel_add_checkbox(HydrusPanel &panel, rust::Str text, bool checked) {
    panel.add_checkbox(text, checked);
}

void hydrus_panel_add_line_edit(HydrusPanel &panel, rust::Str placeholder) {
    panel.add_line_edit(placeholder);
}

void hydrus_panel_add_stretch(HydrusPanel &panel) {
    panel.add_stretch();
}

int hydrus_add_page_widget(HydrusMainWindow &window, rust::Str title, HydrusPanel &panel) {
    return window.add_page_widget(title, &panel);
}

HydrusMediaPanel::HydrusMediaPanel(QWidget *parent) : QWidget(parent) {
    auto *layout = new QVBoxLayout(this);
    list = new QListWidget(this);
    list->setViewMode(QListWidget::IconMode);
    list->setResizeMode(QListWidget::Adjust);
    list->setSpacing(4);
    layout->addWidget(list);
}

void HydrusMediaPanel::add_item(rust::Str label) {
    list->addItem(to_qstring(label));
}

void HydrusMediaPanel::add_item_with_icon(rust::Str path, rust::Str label) {
    QPixmap pixmap(to_qstring(path));

    if (pixmap.isNull()) {
        add_item(label);
        return;
    }

    auto *item = new QListWidgetItem(QIcon(pixmap.scaled(96, 96, Qt::KeepAspectRatio, Qt::SmoothTransformation)), to_qstring(label));
    list->addItem(item);
}

void HydrusMediaPanel::clear_items() {
    list->clear();
}

int HydrusMediaPanel::item_count() const {
    return list->count();
}

std::unique_ptr<HydrusMediaPanel> new_hydrus_media_panel() {
    return std::make_unique<HydrusMediaPanel>();
}

void hydrus_media_panel_add_item(HydrusMediaPanel &panel, rust::Str label) {
    panel.add_item(label);
}

void hydrus_media_panel_add_item_with_icon(HydrusMediaPanel &panel, rust::Str path, rust::Str label) {
    panel.add_item_with_icon(path, label);
}

void hydrus_media_panel_clear(HydrusMediaPanel &panel) {
    panel.clear_items();
}

int hydrus_media_panel_count(const HydrusMediaPanel &panel) {
    return panel.item_count();
}

int hydrus_add_media_page(HydrusMainWindow &window, rust::Str title, HydrusMediaPanel &panel) {
    return window.add_media_page(title, &panel);
}

HydrusCanvas::HydrusCanvas(QWidget *parent) : QWidget(parent) {
    auto *layout = new QVBoxLayout(this);
    scene = new QGraphicsScene(this);
    view = new QGraphicsView(scene, this);
    view->setRenderHints(QPainter::Antialiasing | QPainter::SmoothPixmapTransform);
    layout->addWidget(view);
}

bool HydrusCanvas::load_image(rust::Str path) {
    QPixmap pixmap(to_qstring(path));

    if (pixmap.isNull()) {
        return false;
    }

    scene->clear();
    scene->addPixmap(pixmap);
    scene->setSceneRect(pixmap.rect());
    fit_in_view();

    return true;
}

void HydrusCanvas::clear() {
    scene->clear();
}

void HydrusCanvas::fit_in_view() {
    if (!scene->items().isEmpty()) {
        view->fitInView(scene->sceneRect(), Qt::KeepAspectRatio);
    }
}

std::unique_ptr<HydrusCanvas> new_hydrus_canvas() {
    return std::make_unique<HydrusCanvas>();
}

bool hydrus_canvas_load_image(HydrusCanvas &canvas, rust::Str path) {
    return canvas.load_image(path);
}

void hydrus_canvas_clear(HydrusCanvas &canvas) {
    canvas.clear();
}

void hydrus_canvas_fit(HydrusCanvas &canvas) {
    canvas.fit_in_view();
}

int hydrus_add_canvas_page(HydrusMainWindow &window, rust::Str title, HydrusCanvas &canvas) {
    return window.add_canvas_page(title, &canvas);
}

HydrusScrolledPanel::HydrusScrolledPanel(QWidget *parent) : QWidget(parent) {
    auto *layout = new QVBoxLayout(this);
    area = new QScrollArea(this);
    area->setWidgetResizable(true);

    content = new HydrusPanel(area);
    area->setWidget(content);

    layout->addWidget(area);
}

void HydrusScrolledPanel::add_label(rust::Str text) {
    content->add_label(text);
}

void HydrusScrolledPanel::add_button(rust::Str text) {
    content->add_button(text);
}

void HydrusScrolledPanel::add_checkbox(rust::Str text, bool checked) {
    content->add_checkbox(text, checked);
}

void HydrusScrolledPanel::add_line_edit(rust::Str placeholder) {
    content->add_line_edit(placeholder);
}

void HydrusScrolledPanel::add_stretch() {
    content->add_stretch();
}

std::unique_ptr<HydrusScrolledPanel> new_hydrus_scrolled_panel() {
    return std::make_unique<HydrusScrolledPanel>();
}

void hydrus_scrolled_add_label(HydrusScrolledPanel &panel, rust::Str text) {
    panel.add_label(text);
}

void hydrus_scrolled_add_button(HydrusScrolledPanel &panel, rust::Str text) {
    panel.add_button(text);
}

void hydrus_scrolled_add_checkbox(HydrusScrolledPanel &panel, rust::Str text, bool checked) {
    panel.add_checkbox(text, checked);
}

void hydrus_scrolled_add_line_edit(HydrusScrolledPanel &panel, rust::Str placeholder) {
    panel.add_line_edit(placeholder);
}

void hydrus_scrolled_add_stretch(HydrusScrolledPanel &panel) {
    panel.add_stretch();
}

int hydrus_add_scrolled_page(HydrusMainWindow &window, rust::Str title, HydrusScrolledPanel &panel) {
    return window.add_scrolled_page(title, &panel);
}

HydrusDialog::HydrusDialog(HydrusPanel *panel, rust::Str title, int width, int height)
    : QDialog(nullptr) {
    setWindowTitle(to_qstring(title));
    resize(width, height);

    auto *layout = new QVBoxLayout(this);
    layout->addWidget(panel);

    auto *buttons = new QDialogButtonBox(QDialogButtonBox::Close, this);
    connect(buttons, &QDialogButtonBox::rejected, this, &QDialog::reject);
    layout->addWidget(buttons);
}

int HydrusDialog::exec_dialog() {
    return exec();
}

std::unique_ptr<HydrusDialog> new_hydrus_dialog(
    HydrusPanel &content,
    rust::Str title,
    int width,
    int height) {
    return std::make_unique<HydrusDialog>(&content, title, width, height);
}

int hydrus_dialog_exec(HydrusDialog &dialog) {
    return dialog.exec_dialog();
}

HydrusListPanel::HydrusListPanel(QWidget *parent) : QWidget(parent) {
    auto *layout = new QVBoxLayout(this);
    table = new QTableWidget(this);
    table->horizontalHeader()->setStretchLastSection(true);
    table->setSelectionBehavior(QAbstractItemView::SelectRows);
    layout->addWidget(table);
}

void HydrusListPanel::set_columns(rust::Str csv) {
    const QStringList columns = to_qstring(csv).split(',', Qt::SkipEmptyParts);

    table->setColumnCount(columns.size());
    table->setHorizontalHeaderLabels(columns);
}

void HydrusListPanel::add_row(rust::Str csv) {
    const QStringList values = to_qstring(csv).split(',');
    const int row = table->rowCount();

    table->insertRow(row);

    for (int column = 0; column < table->columnCount(); ++column) {
        const QString value = column < values.size() ? values[column].trimmed() : QString();
        table->setItem(row, column, new QTableWidgetItem(value));
    }
}

void HydrusListPanel::clear_rows() {
    table->setRowCount(0);
}

int HydrusListPanel::row_count() const {
    return table->rowCount();
}

std::unique_ptr<HydrusListPanel> new_hydrus_list_panel() {
    return std::make_unique<HydrusListPanel>();
}

void hydrus_list_set_columns(HydrusListPanel &panel, rust::Str csv) {
    panel.set_columns(csv);
}

void hydrus_list_add_row(HydrusListPanel &panel, rust::Str csv) {
    panel.add_row(csv);
}

void hydrus_list_clear(HydrusListPanel &panel) {
    panel.clear_rows();
}

int hydrus_list_row_count(const HydrusListPanel &panel) {
    return panel.row_count();
}

int hydrus_add_list_page(HydrusMainWindow &window, rust::Str title, HydrusListPanel &panel) {
    return window.add_list_page(title, &panel);
}

void qapp_set_application_name(rust::Str name) {
    QApplication::setApplicationName(to_qstring(name));
}

void qapp_set_application_display_name(rust::Str name) {
    QApplication::setApplicationDisplayName(to_qstring(name));
}

void qapp_set_organization_name(rust::Str name) {
    QApplication::setOrganizationName(to_qstring(name));
}

void qapp_set_style(rust::Str name) {
    if (QStyle *style = QStyleFactory::create(to_qstring(name))) {
        QApplication::setStyle(style);
    }
}

void qapp_set_stylesheet(rust::Str stylesheet) {
    qApp->setStyleSheet(to_qstring(stylesheet));
}

void qapp_quit() {
    QCoreApplication::quit();
}
