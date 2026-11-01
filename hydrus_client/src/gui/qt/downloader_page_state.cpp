
// WS-15.1.2c: moved verbatim from bridge.cpp (move-only split).

#include <hydrus_client/src/gui/qt/api.cxx.h>
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

    const UiAction action =
        kind == QStringLiteral("watcher") ? UiAction::WatcherCheck
                                          : UiAction::DownloaderStart;
    main->notify(action, id, json_payload(payload));
}

void HydrusDownloaderPage::fill_sites(const rust::Vec<GallerySite> &value) {
    if (site == nullptr) {
        return;
    }

    bool same = site->count() == static_cast<int>(value.size()) + 1;

    for (size_t index = 0; same && index < value.size(); ++index) {
        same = site->itemText(static_cast<int>(index) + 1) == to_qstring(value[index].title);
    }

    if (same) {
        return;
    }

    const QString current = site->currentData().toString();
    const QSignalBlocker blocker(site);
    site->clear();
    site->addItem(QStringLiteral("no downloader set"), QString());

    for (const GallerySite &parser : value) {
        site->addItem(to_qstring(parser.title), to_qstring(parser.id));
        site->setItemData(
            site->count() - 1,
            to_qstring(parser.template_text),
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

void HydrusDownloaderPage::update_state(const DownloaderPageState &state) {
    kind = to_qstring(state.kind);

    const bool gallery = kind == QStringLiteral("gallery");
    const bool watcher = kind == QStringLiteral("watcher");
    const bool urls = kind == QStringLiteral("urls");
    const bool simple = kind == QStringLiteral("simple");

    const bool running = state.running;
    const bool paused = state.paused;
    const bool auto_watch = state.auto_watch;
    const QString status = to_qstring(state.status);
    const QString page_url = to_qstring(state.url);
    const QString page_query = to_qstring(state.query);
    const bool has_query = !page_query.isEmpty() || !page_url.isEmpty();

    fill_sites(state.sites);

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
        const QString value = to_qstring(state.input);

        if (input->toPlainText() != value) {
            input->setPlainText(value);
        }
    }

    if (query != nullptr && !query->hasFocus()) {
        update_line_edit(query, page_query);
    }

    if (url != nullptr) {
        if (gallery) {
            if (url->text().isEmpty()) {
                if (!page_url.isEmpty()) {
                    url->setText(page_url);
                } else if (site != nullptr && site->currentIndex() > 0) {
                    url->setText(
                        site->itemData(site->currentIndex(), Qt::UserRole + 1).toString());
                }
            }
        } else if (!url->hasFocus()) {
            update_line_edit(url, page_url);
        }
    }

    if (subject != nullptr && !subject->hasFocus()) {
        update_line_edit(subject, to_qstring(state.subject));
    }

    if (period != nullptr && !period->hasFocus()) {
        period->setValue(static_cast<int>(state.check_period));
    }

    if (site != nullptr) {
        const int site_index = site->findData(to_qstring(state.site));

        if (!gallery) {
            if (site_index >= 0 && !site->hasFocus()) {
                const QSignalBlocker blocker(site);
                site->setCurrentIndex(site_index);
            }
        } else if (!site_loaded) {
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
        highlight_query->setText(watcher ? page_url : page_query);
    }

    if (highlight_subject != nullptr) {
        const QString subject_text = to_qstring(state.subject);
        highlight_subject->setVisible(!page_url.isEmpty());
        highlight_subject->setText(
            subject_text.isEmpty() ? QStringLiteral("no subject") : subject_text);
    }

    if (status_top != nullptr) {
        status_top->setText(to_qstring(state.status_top));
        status_bottom->setText(to_qstring(state.status_bottom));
    }

    if (watcher_table != nullptr) {
        watcher_table->setRowCount(page_url.isEmpty() ? 0 : 1);

        if (!page_url.isEmpty()) {
            QString subject_text = to_qstring(state.subject);

            if (subject_text.isEmpty()) {
                subject_text = QStringLiteral("* unknown");
            }

            const QStringList cells = {subject_text,
                                       to_qstring(state.files_status),
                                       to_qstring(state.checking_status),
                                       status,
                                       to_qstring(state.queue_count_text),
                                       to_qstring(state.watcher_added_text)};

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
        queue->setRowCount(has_query ? 1 : 0);

        if (has_query) {
            QString source = QStringLiteral("unknown downloader");

            if (site != nullptr && site->currentIndex() > 0) {
                source = site->currentText();
            }

            const QString query_text = page_query.isEmpty() ? page_url : page_query;
            const QStringList cells = {query_text,
                                       source,
                                       to_qstring(state.files_status),
                                       to_qstring(state.search_state_text),
                                       status,
                                       to_qstring(state.queue_count_text),
                                       to_qstring(state.queue_added_text)};

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
        imports_status->setText(to_qstring(state.imports_status));
    }

    if (file_seed_status != nullptr) {
        file_seed_status->setText(to_qstring(state.file_seed_status));
    }

    if (simple_jobs != nullptr) {
        const QString formula_name = simple_formula != nullptr
                                         ? simple_formula->currentText()
                                         : QStringLiteral("formula");

        simple_jobs->clear();

        for (const rust::String &value : state.pending_jobs) {
            const QString job_url = to_qstring(value);

            if (!job_url.isEmpty()) {
                auto *item = new QListWidgetItem(
                    QStringLiteral("%1: %2").arg(formula_name).arg(job_url), simple_jobs);
                item->setData(Qt::UserRole, job_url);
            }
        }
    }

    if (simple_formula != nullptr && !simple_formula->hasFocus()) {
        const QString formula_name = to_qstring(state.formula_name);
        const int formula_index = simple_formula->findText(formula_name);

        if (formula_index >= 0) {
            const QSignalBlocker blocker(simple_formula);
            simple_formula->setCurrentIndex(formula_index);
        }
    }

    if (file_progress != nullptr) {
        file_progress->setValue(state.files_progress);
    }

    if (files_pause != nullptr) {
        files_pause->setIcon(
            hydrus_small_icon(paused ? QStringLiteral("play") : QStringLiteral("pause")));
    }

    if (search_status != nullptr) {
        search_status->setText(to_qstring(state.search_status));
    }

    if (search_seed_status != nullptr) {
        search_seed_status->setText(to_qstring(state.search_seed_status));
    }

    if (search_progress != nullptr) {
        search_progress->setValue(state.search_progress);
    }

    if (search_pause != nullptr) {
        search_pause->setIcon(
            hydrus_small_icon(paused ? QStringLiteral("play") : QStringLiteral("pause")));
    }

    if (import_options_button != nullptr) {
        import_options_button->setText(to_qstring(state.import_options_text));
    }

    if (highlight_import_options != nullptr) {
        highlight_import_options->setText(to_qstring(state.import_options_text));
    }
}
