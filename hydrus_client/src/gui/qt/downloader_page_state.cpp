#include "downloader_page.hpp"

// WS-15.1.2c: moved verbatim from bridge.cpp (move-only split).

#include "bridge.hpp"
#include "support/support.hpp"

#include <QAction>
#include <QApplication>
#include <QCheckBox>
#include <QClipboard>
#include <QComboBox>
#include <QDateTime>
#include <QFont>
#include <QFrame>
#include <QHBoxLayout>
#include <QHeaderView>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QJsonValue>
#include <QLabel>
#include <QLineEdit>
#include <QListWidget>
#include <QMenu>
#include <QMessageBox>
#include <QPalette>
#include <QPlainTextEdit>
#include <QProgressBar>
#include <QPushButton>
#include <QRegularExpression>
#include <QScrollArea>
#include <QSignalBlocker>
#include <QSizePolicy>
#include <QSpinBox>
#include <QSplitter>
#include <QStyle>
#include <QTableWidget>
#include <QTabWidget>
#include <QTimer>
#include <QToolButton>
#include <QVBoxLayout>

void HydrusDownloaderPage::start() {
    const bool gallery = kind == QStringLiteral("gallery");

    if (gallery && query != nullptr && query->text().trimmed().isEmpty()
        && (url == nullptr || url->text().isEmpty())) {
        query->setFocus();
        return;
    }

    QJsonObject payload;

    if (input != nullptr) {
        payload.insert(QStringLiteral("input"), input->toPlainText());
    }

    if (url_input != nullptr) {
        payload.insert(QStringLiteral("input"), url_input->text());
    }

    if (simple_url != nullptr) {
        payload.insert(QStringLiteral("input"), simple_url->text());
    }

    if (simple_formula != nullptr) {
        payload.insert(QStringLiteral("formula"), simple_formula->currentText());
    }

    if (url != nullptr) {
        payload.insert(QStringLiteral("url"), url->text());
    }

    if (query != nullptr) {
        payload.insert(QStringLiteral("query"), query->text());
    }

    if (site != nullptr) {
        payload.insert(QStringLiteral("site"), site->currentData().toString());
    }

    if (subject != nullptr) {
        payload.insert(QStringLiteral("subject"), subject->text());
    }

    if (period != nullptr) {
        payload.insert(QStringLiteral("period"), period->value());
    }

    payload.insert(
        QStringLiteral("auto"),
        auto_button != nullptr && auto_button->text().endsWith(QStringLiteral("on")));

    const bool unlimited = no_limit != nullptr && no_limit->isChecked();
    const int limit = file_limit != nullptr ? file_limit->value() : 2000;
    payload.insert(QStringLiteral("no_limit"), unlimited);
    payload.insert(QStringLiteral("file_limit"), unlimited ? 0 : limit);

    if (gallery && url != nullptr && url->text().isEmpty() && site != nullptr
        && site->currentIndex() > 0) {
        payload.insert(
            QStringLiteral("url"),
            site->itemData(site->currentIndex(), Qt::UserRole + 1).toString());
    }

    const QString action =
        kind == QStringLiteral("watcher") ? QStringLiteral("watcher_check")
                                          : QStringLiteral("downloader_start");
    main->notify(action, id, json_payload(payload));
}

void HydrusDownloaderPage::fill_sites(const QJsonArray &value) {
    sites = value;

    if (site == nullptr) {
        return;
    }

    bool same = site->count() == value.size() + 1;

    for (int index = 0; same && index < value.size(); ++index) {
        same = site->itemText(index + 1)
               == value.at(index).toObject().value(QStringLiteral("title")).toString();
    }

    if (same) {
        return;
    }

    const QString current = site->currentData().toString();
    const QSignalBlocker blocker(site);
    site->clear();
    site->addItem(QStringLiteral("no downloader set"), QString());

    for (const QJsonValue &entry : value) {
        const QJsonObject parser = entry.toObject();
        site->addItem(
            parser.value(QStringLiteral("title")).toString(),
            parser.value(QStringLiteral("id")).toString());
        site->setItemData(
            site->count() - 1,
            parser.value(QStringLiteral("template")).toString(),
            Qt::UserRole + 1);
    }

    const int index = site->findData(current);
    site->setCurrentIndex(index >= 0 ? index : 0);
}

void HydrusDownloaderPage::apply_file_limit(int value, bool unlimited) {
    const int clamped = qMax(1, value);
    file_limit_value = unlimited ? 0 : clamped;
    no_limit_value = unlimited;

    if (file_limit != nullptr) {
        const QSignalBlocker blocker(file_limit);
        file_limit->setValue(clamped);
        file_limit->setEnabled(!unlimited);
    }

    if (no_limit != nullptr) {
        const QSignalBlocker blocker(no_limit);
        no_limit->setChecked(unlimited);
    }

    if (highlight_file_limit != nullptr) {
        const QSignalBlocker blocker(highlight_file_limit);
        highlight_file_limit->setValue(clamped);
        highlight_file_limit->setEnabled(!unlimited);
    }

    if (highlight_no_limit != nullptr) {
        const QSignalBlocker blocker(highlight_no_limit);
        highlight_no_limit->setChecked(unlimited);
    }
}

void HydrusDownloaderPage::update_state(const QJsonObject &state) {
    kind = state.value(QStringLiteral("kind")).toString();

    const bool gallery = kind == QStringLiteral("gallery");
    const bool watcher = kind == QStringLiteral("watcher");
    const bool urls = kind == QStringLiteral("urls");
    const bool simple = kind == QStringLiteral("simple");

    const bool running = state.value(QStringLiteral("running")).toBool();
    const bool paused = state.value(QStringLiteral("paused")).toBool();
    const bool auto_watch = state.value(QStringLiteral("auto_watch")).toBool();
    const int imported = state.value(QStringLiteral("imported")).toInt();
    const int failed = state.value(QStringLiteral("failed")).toInt();
    const int queue_count = state.value(QStringLiteral("queue_count")).toInt();
    const int queue_done = state.value(QStringLiteral("queue_done")).toInt();
    const int seed_total = state.value(QStringLiteral("gallery_seed_count")).toInt();
    const int seed_done = state.value(QStringLiteral("gallery_seed_done")).toInt();
    const int seed_success = state.value(QStringLiteral("gallery_seed_success")).toInt();
    const int seed_failed = state.value(QStringLiteral("gallery_seed_failed")).toInt();
    const QString status = state.value(QStringLiteral("status")).toString();

    fill_sites(state.value(QStringLiteral("sites")).toArray());

    if (input_row != nullptr) {
        input_row->setVisible(urls || simple);
    }

    if (query_row != nullptr) {
        query_row->setVisible(gallery);
    }

    if (url_row != nullptr) {
        url_row->setVisible(watcher);
    }

    if (site_row != nullptr) {
        site_row->setVisible(watcher || simple);
    }

    if (subject_row != nullptr) {
        subject_row->setVisible(watcher);
    }

    if (period_row != nullptr) {
        period_row->setVisible(watcher);
    }

    if (file_limit_row != nullptr) {
        file_limit_row->setVisible(gallery);
    }

    if (gug_button != nullptr) {
        gug_button->setVisible(gallery);
    }

    if (highlight_box != nullptr) {
        highlight_box->setVisible(gallery || watcher);
    }

    if (start_button != nullptr) {
        start_button->setVisible(!gallery);
        start_button->setEnabled(!running);
        start_button->setText(watcher ? QStringLiteral("check now") : QStringLiteral("start"));
    }

    if (auto_button != nullptr) {
        auto_button->setVisible(watcher);
        auto_button->setText(
            auto_watch ? QStringLiteral("automatic checks: on")
                       : QStringLiteral("automatic checks: off"));
    }

    if (input != nullptr && !input->hasFocus()) {
        const QString value = state.value(QStringLiteral("input")).toString();

        if (input->toPlainText() != value) {
            input->setPlainText(value);
        }
    }

    if (query != nullptr && !query->hasFocus()) {
        update_line_edit(query, state.value(QStringLiteral("query")).toString());
    }

    if (url != nullptr) {
        if (gallery) {
            if (url->text().isEmpty()) {
                const QString stored = state.value(QStringLiteral("url")).toString();

                if (!stored.isEmpty()) {
                    url->setText(stored);
                } else if (site != nullptr && site->currentIndex() > 0) {
                    url->setText(
                        site->itemData(site->currentIndex(), Qt::UserRole + 1).toString());
                }
            }
        } else if (!url->hasFocus()) {
            update_line_edit(url, state.value(QStringLiteral("url")).toString());
        }
    }

    if (subject != nullptr && !subject->hasFocus()) {
        update_line_edit(subject, state.value(QStringLiteral("subject")).toString());
    }

    if (period != nullptr && !period->hasFocus()) {
        period->setValue(state.value(QStringLiteral("check_period")).toInt(3600));
    }

    if (site != nullptr) {
        if (!gallery) {
            const int site_index =
                site->findData(state.value(QStringLiteral("site")).toString());

            if (site_index >= 0 && !site->hasFocus()) {
                const QSignalBlocker blocker(site);
                site->setCurrentIndex(site_index);
            }
        } else if (!site_loaded) {
            const int site_index =
                site->findData(state.value(QStringLiteral("site")).toString());

            if (site_index >= 0) {
                const QSignalBlocker blocker(site);
                site->setCurrentIndex(site_index);
            }

            site_loaded = true;
        }
    }

    if (gug_button != nullptr && site != nullptr) {
        gug_button->setText(
            site->currentIndex() > 0 ? site->currentText()
                                     : QStringLiteral("no downloader set"));
    }

    if (highlight_query != nullptr) {
        highlight_query->setText(
            watcher ? state.value(QStringLiteral("url")).toString()
                    : state.value(QStringLiteral("query")).toString());
    }

    if (highlight_subject != nullptr) {
        const QString subject_text = state.value(QStringLiteral("subject")).toString();
        highlight_subject->setVisible(!state.value(QStringLiteral("url")).toString().isEmpty());
        highlight_subject->setText(
            subject_text.isEmpty() ? QStringLiteral("no subject") : subject_text);
    }

    if (status_top != nullptr) {
        if (watcher) {
            const QString page_url = state.value(QStringLiteral("url")).toString();

            if (page_url.isEmpty()) {
                status_top->setText(QStringLiteral("waiting for new watchers"));
                status_bottom->setText(QString());
            } else {
                status_top->setText(
                    QStringLiteral("1 watchers - %1/%2").arg(queue_done).arg(queue_count));

                QString bottom;

                if (imported > 0 || failed > 0) {
                    bottom = QStringLiteral("%1 successful").arg(imported);

                    if (failed > 0) {
                        bottom += QStringLiteral(", %1 failed").arg(failed);
                    }
                }

                status_bottom->setText(bottom);
            }
        } else {
            const bool has_query = !state.value(QStringLiteral("query")).toString().isEmpty()
                                   || !state.value(QStringLiteral("url")).toString().isEmpty();
            status_top->setText(
                has_query ? QStringLiteral("1 query - %1/%2").arg(queue_done).arg(queue_count)
                          : QStringLiteral("waiting for new queries"));
            QString bottom;

            if (has_query) {
                bottom = QStringLiteral("%1 successful").arg(imported);

                if (failed > 0) {
                    bottom += QStringLiteral(", %1 failed").arg(failed);
                }
            }

            status_bottom->setText(bottom);
        }
    }

    if (watcher_table != nullptr) {
        const QString page_url = state.value(QStringLiteral("url")).toString();
        watcher_table->setRowCount(page_url.isEmpty() ? 0 : 1);

        if (!page_url.isEmpty()) {
            QString subject_text = state.value(QStringLiteral("subject")).toString();

            if (subject_text.isEmpty()) {
                subject_text = QStringLiteral("* unknown");
            }

            const QString files_status =
                queue_count > 0
                    ? QStringLiteral("%1/%2").arg(queue_done).arg(queue_count)
                    : QString();
            const QString checking_status =
                paused ? QStringLiteral("paused")
                       : (running ? QStringLiteral("checking") : QString());
            QString added;

            const qint64 last_check =
                static_cast<qint64>(state.value(QStringLiteral("last_check")).toDouble());

            if (last_check > 0) {
                const qint64 delta = qMax<qint64>(
                    0, QDateTime::currentSecsSinceEpoch() - last_check);

                if (delta < 60) {
                    added = QStringLiteral("just now");
                } else if (delta < 3600) {
                    added = QStringLiteral("%1 minutes ago").arg(delta / 60);
                } else if (delta < 86400) {
                    added = QStringLiteral("%1 hours ago").arg(delta / 3600);
                } else {
                    added = QStringLiteral("%1 days ago").arg(delta / 86400);
                }
            } else {
                added = QStringLiteral("now");
            }

            const QStringList cells = {subject_text,
                                       files_status,
                                       checking_status,
                                       status,
                                       QString::number(queue_count),
                                       added};

            for (int column = 0; column < cells.size(); ++column) {
                auto *item = watcher_table->item(0, column);

                if (item == nullptr) {
                    item = new QTableWidgetItem();
                    watcher_table->setItem(0, column, item);
                }

                item->setText(cells[column]);
                item->setToolTip(cells[column]);
            }
        }
    }

    if (queue != nullptr) {
        const bool has_query = !state.value(QStringLiteral("query")).toString().isEmpty()
                               || !state.value(QStringLiteral("url")).toString().isEmpty();
        queue->setRowCount(has_query ? 1 : 0);

        if (has_query) {
            QString source = QStringLiteral("unknown downloader");

            if (site != nullptr && site->currentIndex() > 0) {
                source = site->currentText();
            }

            QString added;

            const qint64 last_check =
                static_cast<qint64>(state.value(QStringLiteral("last_check")).toDouble());

            if (last_check > 0) {
                const qint64 delta = qMax<qint64>(
                    0, QDateTime::currentSecsSinceEpoch() - last_check);

                if (delta < 60) {
                    added = QStringLiteral("just now");
                } else if (delta < 3600) {
                    added = QStringLiteral("%1 minutes ago").arg(delta / 60);
                } else if (delta < 86400) {
                    added = QStringLiteral("%1 hours ago").arg(delta / 3600);
                } else {
                    added = QStringLiteral("%1 days ago").arg(delta / 86400);
                }
            }

            const QString files_status =
                queue_count > 0
                    ? QStringLiteral("%1/%2").arg(queue_done).arg(queue_count)
                    : QString();
            const QString search_state_text = running
                                                  ? (paused ? QStringLiteral("paused")
                                                            : QStringLiteral("working"))
                                                  : QStringLiteral("ready");
            const QString query_text = state.value(QStringLiteral("query")).toString().isEmpty()
                                           ? state.value(QStringLiteral("url")).toString()
                                           : state.value(QStringLiteral("query")).toString();
            const QStringList cells = {query_text,
                                       source,
                                       files_status,
                                       search_state_text,
                                       status,
                                       QString::number(queue_count),
                                       added};

            for (int column = 0; column < cells.size(); ++column) {
                auto *item = queue->item(0, column);

                if (item == nullptr) {
                    item = new QTableWidgetItem();
                    queue->setItem(0, column, item);
                }

                item->setText(cells[column]);
                item->setToolTip(cells[column]);
            }
        }
    }

    if (imports_status != nullptr) {
        imports_status->setText(
            (imported == 0 && failed == 0 && queue_count == 0)
                ? QString()
                : (failed == 0
                       ? QStringLiteral("%1 successful").arg(imported)
                       : QStringLiteral("%1 successful, %2 failed")
                             .arg(imported)
                             .arg(failed)));
    }

    if (file_seed_status != nullptr) {
        file_seed_status->setText(
            queue_count > 0 ? QStringLiteral("%1/%2").arg(queue_done).arg(queue_count)
                            : QString());
    }

    if (simple_jobs != nullptr) {
        const QJsonArray pending_jobs = state.value(QStringLiteral("pending_jobs")).toArray();
        const QString formula_name = simple_formula != nullptr
                                         ? simple_formula->currentText()
                                         : QStringLiteral("formula");

        simple_jobs->clear();

        for (const QJsonValue &value : pending_jobs) {
            const QString job_url = value.toString();

            if (!job_url.isEmpty()) {
                auto *item = new QListWidgetItem(
                    QStringLiteral("%1: %2").arg(formula_name).arg(job_url), simple_jobs);
                item->setData(Qt::UserRole, job_url);
            }
        }
    }

    if (simple_formula != nullptr && !simple_formula->hasFocus()) {
        const QString formula_name = state.value(QStringLiteral("formula_name")).toString();
        const int formula_index = simple_formula->findText(formula_name);

        if (formula_index >= 0) {
            const QSignalBlocker blocker(simple_formula);
            simple_formula->setCurrentIndex(formula_index);
        }
    }

    if (file_progress != nullptr) {
        file_progress->setValue(
            queue_count > 0 ? queue_done * 100 / queue_count : 0);
    }

    if (files_pause != nullptr) {
        files_pause->setIcon(
            hydrus_small_icon(paused ? QStringLiteral("play") : QStringLiteral("pause")));
    }

    if (search_status != nullptr) {
        if (seed_total == 0) {
            search_status->setText(QString());
        } else {
            const int pending = qMax(0, seed_total - seed_success - seed_failed);
            search_status->setText(
                QStringLiteral("%1 successful, %2 pending")
                    .arg(seed_success)
                    .arg(pending));
        }
    }

    if (search_seed_status != nullptr) {
        if (seed_total == 0) {
            search_seed_status->setText(QString());
        } else {
            const int pending = qMax(0, seed_total - seed_success - seed_failed);
            search_seed_status->setText(
                QStringLiteral("%1 successful, %2 pending")
                    .arg(seed_success)
                    .arg(pending));
        }
    }

    if (search_progress != nullptr) {
        search_progress->setValue(
            seed_total > 0 ? seed_done * 100 / seed_total : 0);
    }

    if (search_pause != nullptr) {
        search_pause->setIcon(
            hydrus_small_icon(paused ? QStringLiteral("play") : QStringLiteral("pause")));
    }

    if (import_options_button != nullptr) {
        import_options_button->setText(
            QStringLiteral("import options (%1)")
                .arg(state.value(QStringLiteral("import_options_summary")).toString()));
    }

    if (highlight_import_options != nullptr) {
        highlight_import_options->setText(
            QStringLiteral("import options (%1)")
                .arg(state.value(QStringLiteral("import_options_summary")).toString()));
    }
}