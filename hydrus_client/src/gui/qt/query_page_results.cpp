#include "query_page_sidebar.hpp"

// WS-15.1.2b: query-page results, autocomplete and progress handling moved verbatim from bridge.cpp (move-only split).

#include "bridge.hpp"
#include "support/support.hpp"

#include <QColor>
#include <QEvent>
#include <QJsonArray>
#include <QJsonObject>
#include <QJsonValue>
#include <QKeyEvent>
#include <QListWidget>
#include <QTimer>

void HydrusQueryPage::show_autocomplete(const QJsonObject &state) {
    if (main->building || state.value(QStringLiteral("page_id")).toString() != id) {
        return;
    }

    const QJsonArray entries = state.value(QStringLiteral("entries")).toArray();

    results_list->clear();

    if (entries.isEmpty()) {
        fill_system_predicates(results_list);
        return;
    }

    for (const QJsonValue &value : entries) {
        const QString text = value.toObject().value(QStringLiteral("text")).toString();

        if (text.isEmpty()) {
            continue;
        }

        auto *item = new QListWidgetItem(text, results_list);
        item->setForeground(
            text.startsWith(QStringLiteral("system:")) ? QColor(150, 90, 10) : QColor(32, 32, 32));
    }

    if (results_list->count() > 0) {
        results_list->setCurrentRow(0);
    }
}

void HydrusQueryPage::insert_autocomplete(const QString &text) {
    QString current = search->text();
    const QString raw = autocomplete == nullptr ? QString() : autocomplete->raw_text();

    if (!raw.isEmpty() && current.endsWith(raw)) {
        current.chop(raw.size());
    } else {
        current.clear();
    }

    if (!current.isEmpty() && !current.endsWith(QLatin1Char(' '))) {
        current += QLatin1Char(' ');
    }

    QString inserted = text;

    if (!inserted.endsWith(QLatin1Char(' ')) && !inserted.endsWith(QLatin1Char(':'))) {
        inserted += QLatin1Char(' ');
    }

    search->setText(current + inserted);
    text_before_completion = search->text();
    search->setFocus();
}

void HydrusQueryPage::request_autocomplete() {
    if (main->building) {
        return;
    }

    main->notify(QStringLiteral("autocomplete"), id, search->text());
}

bool HydrusQueryPage::eventFilter(QObject *watched, QEvent *event) {
    if (watched == search && event->type() == QEvent::FocusOut && autocomplete != nullptr) {
        QTimer::singleShot(150, this, [this]() {
            if (!search->hasFocus()) {
                autocomplete->hide();
            }
        });
    }

    if (watched == search && autocomplete != nullptr && autocomplete->is_open()

        && event->type() == QEvent::KeyPress) {
        auto *key = static_cast<QKeyEvent *>(event);

        if (key->key() == Qt::Key_Down) {
            autocomplete->move_selection(1);
            return true;
        }

        if (key->key() == Qt::Key_Up) {
            autocomplete->move_selection(-1);
            return true;
        }

        if (key->key() == Qt::Key_Escape) {
            autocomplete->hide();
            return true;
        }

        if (key->key() == Qt::Key_Return || key->key() == Qt::Key_Enter) {
            const QString text = autocomplete->current_text();

            if (!text.isEmpty()) {
                insert_autocomplete(text);
                autocomplete->hide();
                return true;
            }
        }
    }

    return QWidget::eventFilter(watched, event);
}

void HydrusQueryPage::update_progress(const QJsonObject &state) {
    const bool loading = state.value(QStringLiteral("loading")).toBool();

    if (loading) {
        loading_panel->setGeometry(results->rect());
        loading_panel->raise();
        loading_panel->show();
    } else {
        loading_panel->hide();
    }
}

void HydrusQueryPage::resizeEvent(QResizeEvent *event) {
    QWidget::resizeEvent(event);

    if (preview_frame != nullptr) {
        preview_frame->setVisible(height() >= 1000);
    }

    if (loading_panel != nullptr && loading_panel->isVisible() && results != nullptr) {
        loading_panel->setGeometry(results->rect());
    }
}
