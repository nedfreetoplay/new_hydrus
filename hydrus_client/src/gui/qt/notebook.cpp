#include "notebook.hpp"

#include "bridge.hpp"
#include "support/support.hpp"

#include <QApplication>
#include <QDrag>
#include <QInputDialog>
#include <QJsonDocument>
#include <QLineEdit>
#include <QMenu>
#include <QMimeData>
#include <QMouseEvent>

namespace {
const QString kPageMimeType = QStringLiteral("application/x-hydrus-page");
}

// A tab bar that starts a drag for pages and accepts drops from any notebook.
// Ordering and cross-notebook moves are applied by the Rust page model.

HydrusTabBar::HydrusTabBar(HydrusMainWindow *main_window, HydrusNotebook *notebook_widget, QWidget *parent)
    : QTabBar(parent), main(main_window), notebook(notebook_widget) {
    setAcceptDrops(true);
    setMovable(false);
}

void HydrusTabBar::mousePressEvent(QMouseEvent *event) {
        if (event->button() == Qt::LeftButton) {
            press_position = event->pos();
        }

        QTabBar::mousePressEvent(event);
    }

void HydrusTabBar::mouseMoveEvent(QMouseEvent *event) {
        if (!(event->buttons() & Qt::LeftButton)) {
            QTabBar::mouseMoveEvent(event);
            return;
        }

        if ((event->pos() - press_position).manhattanLength() < QApplication::startDragDistance()) {
            QTabBar::mouseMoveEvent(event);
            return;
        }

        const int index = tabAt(press_position);
        const QPoint pressed = press_position;
        press_position = QPoint(-1000, -1000);

        if (index < 0) {
            return;
        }

        const QString page_id = tabData(index).toString();

        if (page_id.isEmpty() || page_id == QStringLiteral("__plus__")) {
            return;
        }

        auto *mime = new QMimeData();
        mime->setData(kPageMimeType, page_id.toUtf8());

        auto *drag = new QDrag(this);
        drag->setMimeData(mime);
        drag->exec(Qt::MoveAction);

        Q_UNUSED(pressed);
    }

void HydrusTabBar::dragEnterEvent(QDragEnterEvent *event) {
        if (event->mimeData()->hasFormat(kPageMimeType)) {
            event->acceptProposedAction();
            return;
        }

        QTabBar::dragEnterEvent(event);
    }

void HydrusTabBar::dragMoveEvent(QDragMoveEvent *event) {
        if (event->mimeData()->hasFormat(kPageMimeType)) {
            event->acceptProposedAction();
            return;
        }

        QTabBar::dragMoveEvent(event);
    }

void HydrusTabBar::dropEvent(QDropEvent *event) {
        if (!event->mimeData()->hasFormat(kPageMimeType)) {
            QTabBar::dropEvent(event);
            return;
        }

        const QString source = QString::fromUtf8(event->mimeData()->data(kPageMimeType));
        const int index = tabAt(event->position().toPoint());

        QJsonObject request;
        request.insert(QStringLiteral("target"), notebook->notebook_id());
        request.insert(QStringLiteral("position"), QStringLiteral("inside"));

        if (index >= 0 && index < count() - 1) {
            const QString target = tabData(index).toString();

            if (!target.isEmpty() && target != QStringLiteral("__plus__") && target != source) {
                request.insert(QStringLiteral("target"), target);
                request.insert(QStringLiteral("position"), QStringLiteral("before"));
            }
        }

        const QByteArray payload = QJsonDocument(request).toJson(QJsonDocument::Compact);
        main->notify(QStringLiteral("move_page"), source, QString::fromUtf8(payload));
        event->acceptProposedAction();
    }

HydrusNotebook::HydrusNotebook(HydrusMainWindow *main_window, const QString &notebook_id, QWidget *parent)
    : QTabWidget(parent), main(main_window), id(notebook_id) {
    setTabBar(new HydrusTabBar(main_window, this, this));
    setTabsClosable(false);
    setMovable(false);
    setAcceptDrops(true);

    connect(this, &QTabWidget::tabCloseRequested, this, [this](int index) { request_close(index); });

    connect(tabBar(), &QTabBar::tabBarDoubleClicked, this, [this](int index) {
        if (index < 0 || index == plus_index) {
            return;
        }

        const QString page_id = tabBar()->tabData(index).toString();

        if (page_id.isEmpty()) {
            return;
        }

        bool ok = false;
        const QString name = QInputDialog::getText(
            this,
            QStringLiteral("rename page"),
            QStringLiteral("page name"),
            QLineEdit::Normal,
            tabText(index),
            &ok);

        if (ok && !name.isEmpty()) {
            QByteArray page_bytes = page_id.toUtf8();
            QByteArray name_bytes = name.toUtf8();
            app_action(*main, rust::Str("rename_page", 11), rust::Str(page_bytes.constData(), page_bytes.size()), rust::Str(name_bytes.constData(), name_bytes.size()));
        }
    });

    connect(this, &QTabWidget::currentChanged, this, [this](int index) {
        if (main->building || index < 0) {
            return;
        }

        if (index == plus_index) {
            if (last_normal_index >= 0 && last_normal_index < count()) {
                setCurrentIndex(last_normal_index);
            }

            main->show_page_chooser(id);
            return;
        }

        last_normal_index = index;

        const QString page_id = tabBar()->tabData(index).toString();

        if (page_id.isEmpty()) {
            return;
        }

        main->active_page = page_id;
        main->active_notebook = id;

        QByteArray page_bytes = page_id.toUtf8();
        QByteArray notebook_bytes = id.toUtf8();
        app_action(*main, rust::Str("page_selected", 13), rust::Str(page_bytes.constData(), page_bytes.size()), rust::Str(notebook_bytes.constData(), notebook_bytes.size()));
    });

    tabBar()->setContextMenuPolicy(Qt::CustomContextMenu);
    connect(tabBar(), &QTabBar::customContextMenuRequested, this, [this](const QPoint &position) {
        show_tab_menu(position);
    });

    tabBar()->installEventFilter(this);
}

bool HydrusNotebook::eventFilter(QObject *watched, QEvent *event) {
    if (watched == tabBar() && event->type() == QEvent::MouseButtonDblClick) {
        auto *mouse = static_cast<QMouseEvent *>(event);

        if (mouse->button() == Qt::LeftButton && tabBar()->tabAt(mouse->pos()) < 0) {
            main->show_page_chooser(id);
            return true;
        }
    }

    if (watched == tabBar() && event->type() == QEvent::MouseButtonRelease) {
        auto *mouse = static_cast<QMouseEvent *>(event);

        if (mouse->button() == Qt::MiddleButton) {
            request_close(tabBar()->tabAt(mouse->pos()));
            return true;
        }
    }

    return QTabWidget::eventFilter(watched, event);
}

const QString &HydrusNotebook::notebook_id() const {
    return id;
}

void HydrusNotebook::add_page_widget(const QString &page_id, const QString &title, QWidget *page) {
    const int index = plus_index >= 0 ? plus_index : count();
    const int inserted = insertTab(index, page, title);

    tabBar()->setTabData(inserted, page_id);
    tabBar()->setTabToolTip(inserted, title);

    if (plus_index >= 0 && inserted <= plus_index) {
        plus_index += 1;
    }
}

void HydrusNotebook::install_plus_tab() {
    if (plus_index >= 0) {
        return;
    }

    auto *blank = new QWidget(this);
    plus_index = addTab(blank, QStringLiteral("+"));
    tabBar()->setTabData(plus_index, QStringLiteral("__plus__"));
    tabBar()->setTabToolTip(plus_index, QStringLiteral("open a new page"));
    tabBar()->setTabButton(plus_index, QTabBar::RightSide, nullptr);
    tabBar()->setTabButton(plus_index, QTabBar::LeftSide, nullptr);
}

void HydrusNotebook::finalize_notebook(const QString &current_page_id) {
    int target = -1;

    if (!current_page_id.isEmpty()) {
        for (int index = 0; index < count(); ++index) {
            if (tabBar()->tabData(index).toString() == current_page_id) {
                target = index;
                break;
            }
        }
    }

    if (target < 0 && count() > 1) {
        target = 0;
    }

    if (target >= 0) {
        last_normal_index = target;
        setCurrentIndex(target);
    } else {
        last_normal_index = 0;
    }
}

void HydrusNotebook::clear_pages() {
    plus_index = -1;

    while (count() > 0) {
        QWidget *page = widget(0);
        removeTab(0);

        if (page != nullptr) {
            page->deleteLater();
        }
    }
}

QString HydrusNotebook::current_page_id() const {
    const int index = currentIndex();

    if (index < 0) {
        return {};
    }

    const QString tab_data = tabBar()->tabData(index).toString();

    if (tab_data == QStringLiteral("__plus__")) {
        return {};
    }

    return tab_data;
}

void HydrusNotebook::request_close(int index) {
    if (index < 0 || index == plus_index) {
        return;
    }

    const QString page_id = tabBar()->tabData(index).toString();

    if (page_id.isEmpty()) {
        return;
    }

    QByteArray page_bytes = page_id.toUtf8();
    app_action(*main, rust::Str("close_page", 10), rust::Str(page_bytes.constData(), page_bytes.size()), rust::Str());
}

void HydrusNotebook::show_tab_menu(const QPoint &position) {
    const int index = tabBar()->tabAt(position);

    if (index < 0 || index == plus_index) {
        QMenu menu(this);
        QAction *new_page = menu.addAction(QStringLiteral("new page..."));
        QAction *chosen = menu.exec(tabBar()->mapToGlobal(position));

        if (chosen == new_page) {
            main->show_page_chooser(id);
        }

        return;
    }

    const QString page_id = tabBar()->tabData(index).toString();

    QMenu menu(this);
    QAction *close = menu.addAction(QStringLiteral("close page"));
    QAction *rename = menu.addAction(QStringLiteral("rename page..."));
    QAction *new_page = menu.addAction(QStringLiteral("new page..."));
    menu.addSeparator();
    QAction *close_others = menu.addAction(QStringLiteral("close other pages"));
    menu.addSeparator();
    QAction *refresh = menu.addAction(QStringLiteral("refresh"));

    QAction *chosen = menu.exec(tabBar()->mapToGlobal(position));

    if (chosen == nullptr) {
        return;
    }

    if (chosen == close) {
        request_close(index);
        return;
    }

    if (chosen == new_page) {
        main->show_page_chooser(id);
        return;
    }

    if (chosen == refresh) {
        QByteArray page_bytes = page_id.toUtf8();
        app_action(*main, rust::Str("refresh_page", 12), rust::Str(page_bytes.constData(), page_bytes.size()), rust::Str());
        return;
    }

    if (chosen == close_others) {
        QByteArray page_bytes = page_id.toUtf8();
        app_action(*main, rust::Str("close_other_pages", 17), rust::Str(page_bytes.constData(), page_bytes.size()), rust::Str());
        return;
    }

    if (chosen == rename) {
        bool ok = false;
        const QString name = QInputDialog::getText(
            this,
            QStringLiteral("rename page"),
            QStringLiteral("page name"),
            QLineEdit::Normal,
            tabText(index),
            &ok);

        if (ok && !name.isEmpty()) {
            QByteArray page_bytes = page_id.toUtf8();
            QByteArray name_bytes = name.toUtf8();
            app_action(*main, rust::Str("rename_page", 11), rust::Str(page_bytes.constData(), page_bytes.size()), rust::Str(name_bytes.constData(), name_bytes.size()));
        }
    }
}
