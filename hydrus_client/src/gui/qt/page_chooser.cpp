#include "page_chooser.hpp"

// WS-15.1.3c2: new page chooser moved verbatim from bridge.cpp (move-only split).

#include "bridge.hpp"
#include "support/support.hpp"

#include <QDialogButtonBox>
#include <QLabel>
#include <QPushButton>
#include <QStackedWidget>
#include <QVBoxLayout>
#include <QWidget>

HydrusPageChooser::HydrusPageChooser(const QString &notebook_id, QWidget *parent)
    : QDialog(parent), notebook(notebook_id) {
    setWindowTitle(QStringLiteral("new page"));
    resize(460, 320);

    auto *layout = new QVBoxLayout(this);
    stack = new QStackedWidget(this);
    layout->addWidget(stack, 1);

    build_home();
    build_search();
    build_download();
    build_special();

    auto *buttons = new QDialogButtonBox(QDialogButtonBox::Cancel, this);
    connect(buttons, &QDialogButtonBox::rejected, this, &QDialog::reject);
    layout->addWidget(buttons);
}

QWidget *HydrusPageChooser::make_page() {
    auto *page = new QWidget(stack);
    auto *layout = new QVBoxLayout(page);
    layout->setSpacing(8);
    stack->addWidget(page);
    page_layout = layout;
    return page;
}

QPushButton *HydrusPageChooser::add_button(const QString &text, const QString &type) {
    auto *button = new QPushButton(text, stack->currentWidget());
    button->setMinimumHeight(34);
    connect(button, &QPushButton::clicked, this, [this, type]() {
        result = type;
        accept();
    });
    page_layout->addWidget(button);
    return button;
}

QPushButton *HydrusPageChooser::add_nav(const QString &text, int target) {
    auto *button = new QPushButton(text, stack->currentWidget());
    connect(button, &QPushButton::clicked, this, [this, target]() { stack->setCurrentIndex(target); });
    page_layout->addWidget(button);
    return button;
}

void HydrusPageChooser::build_home() {
    make_page();
    page_layout->addWidget(new QLabel(QStringLiteral("choose a page type"), stack->currentWidget()));
    add_nav(QStringLiteral("file search"), 1);
    add_nav(QStringLiteral("download"), 2);
    add_nav(QStringLiteral("special"), 3);
    page_layout->addStretch(1);
}

void HydrusPageChooser::build_search() {
    make_page();
    page_layout->addWidget(new QLabel(QStringLiteral("file search"), stack->currentWidget()));
    add_button(QStringLiteral("my files"), QStringLiteral("query:my files"));
    add_button(QStringLiteral("all local files"), QStringLiteral("query:all local files"));
    add_button(QStringLiteral("trash"), QStringLiteral("query:trash"));
    add_nav(QStringLiteral("back"), 0);
    page_layout->addStretch(1);
}

void HydrusPageChooser::build_download() {
    make_page();
    page_layout->addWidget(new QLabel(QStringLiteral("download"), stack->currentWidget()));
    add_button(QStringLiteral("urls"), QStringLiteral("urls"));
    add_button(QStringLiteral("watcher"), QStringLiteral("watcher"));
    add_button(QStringLiteral("gallery"), QStringLiteral("gallery"));
    add_button(QStringLiteral("simple downloader"), QStringLiteral("simple"));
    add_button(QStringLiteral("subscriptions"), QStringLiteral("subscriptions"));
    add_nav(QStringLiteral("back"), 0);
    page_layout->addStretch(1);
}

void HydrusPageChooser::build_special() {
    make_page();
    page_layout->addWidget(new QLabel(QStringLiteral("special"), stack->currentWidget()));
    add_button(QStringLiteral("page of pages"), QStringLiteral("notebook"));
    add_button(QStringLiteral("duplicates processing"), QStringLiteral("duplicates"));
    add_button(QStringLiteral("import folders"), QStringLiteral("import_folders"));
    add_nav(QStringLiteral("back"), 0);
    page_layout->addStretch(1);
}

void HydrusMainWindow::show_page_chooser(const QString &notebook_id) {
    HydrusPageChooser chooser(notebook_id, this);

    if (chooser.exec() == QDialog::Accepted && !chooser.result.isEmpty()) {
        notify(QStringLiteral("create_page"), notebook_id, chooser.result);
    }
}
