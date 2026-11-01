#include <hydrus_client/src/gui/qt/api.cxx.h>
#include <hydrus_client/src/gui/qt/api.cxx.h>

// WS-15.1.3b: HydrusMainWindow shell moved verbatim from bridge.cpp (move-only split).

#include "support/support.hpp"

#include <QAction>
#include <QApplication>
#include <QCoreApplication>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QMenu>
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

    notify(UiAction::LocalImportOpenPaths, QString(), QString::fromUtf8(QJsonDocument(array).toJson(QJsonDocument::Compact)));
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

void HydrusMainWindow::notify(UiAction action, const QString &target, const QString &payload) {
    QByteArray target_bytes = target.toUtf8();
    QByteArray payload_bytes = payload.toUtf8();

    app_action(
        *this,
        action,
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
        connect(tray_exit, &QAction::triggered, this, [this]() { notify(UiAction::Exit); });
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


HydrusQueryPage *HydrusMainWindow::find_page(const QString &page_id) const {
    return pages.value(page_id, nullptr);
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
