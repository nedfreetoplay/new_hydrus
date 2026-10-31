#include "bridge.hpp"

// WS-15.1.3b: HydrusMainWindow shell moved verbatim from bridge.cpp (move-only split).

#include "support/support.hpp"

#include <QAction>
#include <QApplication>
#include <QCoreApplication>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QMenu>
#include <QMenuBar>
#include <QMessageBox>
#include <QMimeData>
#include <QSettings>
#include <QStatusBar>
#include <QStyle>
#include <QSystemTrayIcon>
#include <QTimer>
#include <QUrl>

HydrusMainWindow::HydrusMainWindow(QWidget *parent) : QMainWindow(parent) {
    active_notebook = QStringLiteral("root");
    setAcceptDrops(true);
}

void HydrusMainWindow::dragEnterEvent(QDragEnterEvent *event) {
    if (event->mimeData()->hasUrls()) {
        event->acceptProposedAction();
    }
}

void HydrusMainWindow::dropEvent(QDropEvent *event) {
    QStringList paths;

    for (const QUrl &url : event->mimeData()->urls()) {
        if (url.isLocalFile()) {
            paths.append(url.toLocalFile());
        }
    }

    if (paths.isEmpty()) {
        return;
    }

    QJsonArray array;

    for (const QString &path : paths) {
        array.append(path);
    }

    notify(QStringLiteral("local_import_open_paths"), QString(), QString::fromUtf8(QJsonDocument(array).toJson(QJsonDocument::Compact)));
    event->acceptProposedAction();
}

void HydrusMainWindow::closeEvent(QCloseEvent *event) {
    if (tick_timer != nullptr) {
        tick_timer->stop();
    }

    QSettings settings;
    settings.setValue(QStringLiteral("main/geometry"), saveGeometry());
    settings.setValue(QStringLiteral("main/state"), saveState());
    settings.sync();

    if (tray != nullptr) {
        tray->hide();
    }

    if (services_dialog != nullptr) {
        services_dialog->close();
    }

    if (network_dialog != nullptr) {
        network_dialog->close();
    }

    QMainWindow::closeEvent(event);
}

void HydrusMainWindow::notify(const QString &action, const QString &target, const QString &payload) {
    QByteArray action_bytes = action.toUtf8();
    QByteArray target_bytes = target.toUtf8();
    QByteArray payload_bytes = payload.toUtf8();

    app_action(
        *this,
        rust::Str(action_bytes.constData(), action_bytes.size()),
        rust::Str(target_bytes.constData(), target_bytes.size()),
        rust::Str(payload_bytes.constData(), payload_bytes.size()));
}

void HydrusMainWindow::build_shell() {
    if (root != nullptr) {
        return;
    }

    setStyleSheet(QString());

    root = new HydrusNotebook(this, QStringLiteral("root"), this);
    setCentralWidget(root);

    auto text_width = [this](int chars) {
        return fontMetrics().horizontalAdvance(QString(chars, QLatin1Char('0')));
    };

    status_media = new QLabel(this);
    status_bandwidth = new QLabel(this);
    status_bandwidth->setFixedWidth(text_width(17));
    status_idle = new QLabel(this);
    status_idle->setFixedWidth(text_width(6));
    status_hydrus_busy = new QLabel(this);
    status_hydrus_busy->setFixedWidth(text_width(11));
    status_cpu_busy = new QLabel(this);
    status_cpu_busy->setFixedWidth(text_width(13));
    status_db = new QLabel(this);
    status_db->setFixedWidth(text_width(14));

    statusBar()->addWidget(status_media, 1);
    statusBar()->addWidget(status_bandwidth);
    statusBar()->addWidget(status_idle);
    statusBar()->addWidget(status_hydrus_busy);
    statusBar()->addWidget(status_cpu_busy);
    statusBar()->addWidget(status_db);
    statusBar()->setSizeGripEnabled(true);

    if (QSystemTrayIcon::isSystemTrayAvailable()) {
        tray = new QSystemTrayIcon(style()->standardIcon(QStyle::SP_ComputerIcon), this);
        tray->setToolTip(QStringLiteral("Hydrus Client"));

        auto *tray_menu = new QMenu(this);
        auto *show_action = tray_menu->addAction(QStringLiteral("show/hide"));
        auto *tray_exit = tray_menu->addAction(QStringLiteral("exit"));

        connect(show_action, &QAction::triggered, this, [this]() {
            if (isVisible()) {
                hide();
            } else {
                show();
                raise();
                activateWindow();
            }
        });
        connect(tray_exit, &QAction::triggered, this, [this]() { notify(QStringLiteral("exit")); });
        connect(tray, &QSystemTrayIcon::activated, this, [this](QSystemTrayIcon::ActivationReason reason) {
            if (reason == QSystemTrayIcon::Trigger) {
                if (isVisible()) {
                    hide();
                } else {
                    show();
                    raise();
                    activateWindow();
                }
            }
        });

        tray->setContextMenu(tray_menu);
        tray->show();
    }

    build_shortcuts();

    resize(1280, 800);
    setWindowTitle(QStringLiteral("Hydrus Client"));

    QSettings settings;
    const QByteArray geometry = settings.value(QStringLiteral("main/geometry")).toByteArray();
    const QByteArray state = settings.value(QStringLiteral("main/state")).toByteArray();

    if (!geometry.isEmpty()) {
        restoreGeometry(geometry);
    }

    if (!state.isEmpty()) {
        restoreState(state);
    }
}

void HydrusMainWindow::set_status(const QJsonObject &status) {
    const QString media = status.value(QStringLiteral("media")).toString();
    const QString message = status.value(QStringLiteral("message")).toString();

    status_media->setText(media.isEmpty() ? message : media);
    status_media->setToolTip(message.isEmpty() ? media : message);
    status_bandwidth->setText(status.value(QStringLiteral("bandwidth")).toString());
    status_idle->setText(status.value(QStringLiteral("idle")).toString());
    status_hydrus_busy->setText(status.value(QStringLiteral("hydrus_busy")).toString());
    status_cpu_busy->setText(status.value(QStringLiteral("cpu_busy")).toString());
    status_db->setText(status.value(QStringLiteral("db")).toString());
}

void HydrusMainWindow::set_menus(const QJsonArray &menus) {
    menuBar()->clear();

    for (const QJsonValue &value : menus) {
        const QJsonObject menu_object = value.toObject();
        auto *menu = menuBar()->addMenu(menu_object.value(QStringLiteral("title")).toString());
        menu->setToolTipsVisible(true);

        std::function<void(QMenu *, const QJsonArray &)> add_items;

        add_items = [this, &add_items](QMenu *menu_target, const QJsonArray &items) {
            for (const QJsonValue &item_value : items) {
                const QJsonObject item = item_value.toObject();

                if (item.value(QStringLiteral("separator")).toBool()) {
                    menu_target->addSeparator();
                    continue;
                }

                const QJsonArray children = item.value(QStringLiteral("items")).toArray();

                if (!children.isEmpty()) {
                    auto *submenu = menu_target->addMenu(item.value(QStringLiteral("label")).toString());
                    submenu->setToolTipsVisible(true);
                    add_items(submenu, children);
                    continue;
                }

                const QString label = item.value(QStringLiteral("label")).toString();
                const QString action_id = item.value(QStringLiteral("action")).toString();
                const QString item_target = item.value(QStringLiteral("target")).toString();
                const QString payload = item.value(QStringLiteral("payload")).toString();

                if (label.isEmpty()) {
                    continue;
                }

                auto *action = menu_target->addAction(label);
                action->setEnabled(item.value(QStringLiteral("enabled")).toBool(true));

                if (item.value(QStringLiteral("checkable")).toBool(false)) {
                    action->setCheckable(true);
                    action->setChecked(item.value(QStringLiteral("checked")).toBool(false));
                }

                const QString tooltip = item.value(QStringLiteral("description")).toString();

                if (!tooltip.isEmpty()) {
                    action->setToolTip(tooltip);
                }

                connect(action, &QAction::triggered, this, [this, action_id, item_target, payload]() {
                    if (!action_id.isEmpty()) {
                        notify(action_id, item_target, payload);
                    }
                });
            }
        };

        add_items(menu, menu_object.value(QStringLiteral("items")).toArray());
    }
}

void HydrusMainWindow::clear_notebook(HydrusNotebook *notebook) {
    notebook->clear_pages();
}

void HydrusMainWindow::set_tag_suggestions(const QStringList &suggestions) {
    tag_suggestions = suggestions;

    for (HydrusQueryPage *page : pages) {
        page->set_suggestions(tag_suggestions);
    }
}

const QStringList &HydrusMainWindow::suggestions() const {
    return tag_suggestions;
}

void HydrusMainWindow::show_autocomplete(const QJsonObject &state) {
    const QString page_id = state.value(QStringLiteral("page_id")).toString();
    HydrusQueryPage *page = pages.value(page_id, nullptr);

    if (page != nullptr) {
        page->show_autocomplete(state);
    }
}

void HydrusMainWindow::build_pages(const QJsonObject &object) {
    if (root == nullptr) {
        return;
    }

    building = true;

    pages.clear();
    downloaders.clear();
    subscriptions.clear();
    duplicates.clear();
    import_folders.clear();
    export_folders.clear();
    notebooks.clear();
    notebooks.insert(QStringLiteral("root"), root);

    clear_notebook(root);

    const QJsonObject root_object = object.value(QStringLiteral("root")).toObject();
    populate_notebook(root, root_object);

    QString current = root->current_page_id();
    QString parent = QStringLiteral("root");

    while (HydrusNotebook *nested = notebooks.value(current, nullptr)) {
        parent = current;
        current = nested->current_page_id();
    }

    active_page = current;
    active_notebook = parent;

    building = false;
}

void HydrusMainWindow::populate_notebook(HydrusNotebook *notebook, const QJsonObject &object) {
    const QJsonArray children = object.value(QStringLiteral("children")).toArray();

    for (const QJsonValue &value : children) {
        const QJsonObject child = value.toObject();
        const QString kind = child.value(QStringLiteral("kind")).toString();
        const QString page_id = child.value(QStringLiteral("id")).toString();
        const QString title = child.value(QStringLiteral("title")).toString();

        if (kind == QStringLiteral("notebook")) {
            auto *nested = new HydrusNotebook(this, page_id, notebook);
            notebooks.insert(page_id, nested);
            notebook->add_page_widget(page_id, title.isEmpty() ? QStringLiteral("page of pages") : title, nested);
            populate_notebook(nested, child);
        } else if (kind == QStringLiteral("query")) {
            auto *page = new HydrusQueryPage(this, page_id, child.value(QStringLiteral("state")).toObject());
            pages.insert(page_id, page);
            notebook->add_page_widget(page_id, title.isEmpty() ? QStringLiteral("files") : title, page);
        } else if (kind == QStringLiteral("downloader")) {
            auto *page = new HydrusDownloaderPage(this, page_id, child.value(QStringLiteral("state")).toObject());
            downloaders.insert(page_id, page);
            notebook->add_page_widget(page_id, title.isEmpty() ? QStringLiteral("download") : title, page);
        } else if (kind == QStringLiteral("subscriptions")) {
            auto *page = new HydrusSubscriptionsPage(this, page_id, child.value(QStringLiteral("state")).toObject());
            subscriptions.insert(page_id, page);
            notebook->add_page_widget(page_id, title.isEmpty() ? QStringLiteral("subscriptions") : title, page);
        } else if (kind == QStringLiteral("duplicates")) {
            auto *page = new HydrusDuplicatesPage(this, page_id, child.value(QStringLiteral("state")).toObject());
            duplicates.insert(page_id, page);
            notebook->add_page_widget(page_id, title.isEmpty() ? QStringLiteral("duplicates") : title, page);
        } else if (kind == QStringLiteral("import_folders")) {
            auto *page = new HydrusImportFoldersPage(this, page_id, child.value(QStringLiteral("state")).toObject());
            import_folders.insert(page_id, page);
            notebook->add_page_widget(page_id, title.isEmpty() ? QStringLiteral("import folders") : title, page);
        } else if (kind == QStringLiteral("export_folders")) {
            auto *page = new HydrusExportFoldersPage(this, page_id, child.value(QStringLiteral("state")).toObject());
            export_folders.insert(page_id, page);
            notebook->add_page_widget(page_id, title.isEmpty() ? QStringLiteral("export folders") : title, page);
        } else {
            const QString message = child.value(QStringLiteral("state")).toObject().value(QStringLiteral("message")).toString();
            auto *placeholder = new QWidget(notebook);
            auto *layout = new QVBoxLayout(placeholder);
            auto *label = new QLabel(message.isEmpty() ? QStringLiteral("Unknown page type.") : message, placeholder);
            label->setAlignment(Qt::AlignCenter);
            label->setWordWrap(true);
            layout->addWidget(label);
            notebook->add_page_widget(page_id, title.isEmpty() ? QStringLiteral("page") : title, placeholder);
        }
    }

    notebook->finalize_notebook(object.value(QStringLiteral("current")).toString());
}

HydrusQueryPage *HydrusMainWindow::find_page(const QString &page_id) const {
    return pages.value(page_id, nullptr);
}

void HydrusMainWindow::update_page(const QString &page_id, const QJsonObject &state) {
    if (HydrusQueryPage *page = find_page(page_id)) {
        page->update_state(state);
    }
}

void HydrusMainWindow::update_page_progress(const QString &page_id, const QJsonObject &state) {
    if (HydrusQueryPage *page = find_page(page_id)) {
        page->update_progress(state);
    }
}

void HydrusMainWindow::update_downloader(const QString &page_id, const QJsonObject &state) {
    if (HydrusDownloaderPage *page = downloaders.value(page_id, nullptr)) {
        page->update_state(state);
    }
}

void HydrusMainWindow::update_subscriptions(const QString &page_id, const QJsonObject &state) {
    if (HydrusSubscriptionsPage *page = subscriptions.value(page_id, nullptr)) {
        page->update_state(state);
    }
}

void HydrusMainWindow::update_duplicates(const QString &page_id, const QJsonObject &state) {
    if (HydrusDuplicatesPage *page = duplicates.value(page_id, nullptr)) {
        page->update_state(state);
    }
}

void HydrusMainWindow::update_import_folders(const QString &page_id, const QJsonObject &state) {
    if (HydrusImportFoldersPage *page = import_folders.value(page_id, nullptr)) {
        page->update_state(state);
    }
}

void HydrusMainWindow::update_export_folders(const QString &page_id, const QJsonObject &state) {
    if (HydrusExportFoldersPage *page = export_folders.value(page_id, nullptr)) {
        page->update_state(state);
    }
}

void HydrusMainWindow::set_selection(
    const QString &page_id,
    const QString &hash,
    const QStringList &tags,
    const QString &preview_path,
    const QJsonObject &metadata) {
    if (HydrusQueryPage *page = find_page(page_id)) {
        page->set_selection(hash, tags, preview_path, metadata);
    }
}

void HydrusMainWindow::start_tick() {
    if (tick_timer != nullptr) {
        return;
    }

    tick_timer = new QTimer(this);
    tick_timer->setInterval(120);
    connect(tick_timer, &QTimer::timeout, this, [this]() { app_tick(*this); });
    tick_timer->start();
}

void HydrusMainWindow::show_message(const QString &title, const QString &text) {
    QMessageBox::information(this, title, text);
}

void HydrusMainWindow::notify_message(const QString &message) {
    if (status_media != nullptr) {
        status_media->setText(message);
    }

    if (tray != nullptr && tray->isVisible()) {
        tray->showMessage(
            QStringLiteral("Hydrus Client"),
            message,
            QSystemTrayIcon::Information,
            3000);
    }
}

void HydrusMainWindow::schedule_screenshot(const QString &path) {
    int delay = qEnvironmentVariableIntValue("HYDRUS_QT_SCREENSHOT_DELAY_MS");

    if (delay <= 0) {
        delay = 1400;
    }

    QTimer::singleShot(delay, this, [this, path]() {
        QWidget *target = QApplication::activeWindow();

        if (target == nullptr) {
            target = this;
        }

        for (HydrusViewerWindow *viewer : viewers) {
            if (viewer != nullptr && viewer->isVisible()) {
                target = viewer;
                break;
            }
        }

        const bool ok = target->grab().save(path);
        QCoreApplication::quit();

        if (!ok) {
            qWarning("could not save screenshot to %s", path.toUtf8().constData());
        }
    });
}
