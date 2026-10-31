#pragma once

// WS-15.1.2a: HydrusAutocompletePopup moved verbatim from bridge.cpp (move-only split).
// The popup is a small self-contained inline class; its methods stay inline here.

#include <QFrame>
#include <QJsonArray>
#include <QJsonObject>
#include <QJsonValue>
#include <QListWidget>
#include <QSignalBlocker>
#include <QString>
#include <QStringList>
#include <QTabBar>
#include <QVBoxLayout>
#include <QWidget>

#include <functional>

// Hydrus' AC dropdown groups matches into tabs and shows how many files each
// predicate selects. The popup keeps focus in the line edit so typing can
// continue while Arrow keys move through the list.
class HydrusAutocompletePopup : public QFrame {
public:
    HydrusAutocompletePopup(QWidget *anchor, std::function<void(const QString &)> insert)
        : QFrame(anchor->window()), anchor(anchor), insert(std::move(insert)) {
        setFrameShape(QFrame::StyledPanel);
        setFrameShadow(QFrame::Raised);

        auto *layout = new QVBoxLayout(this);
        layout->setContentsMargins(0, 0, 0, 0);
        layout->setSpacing(0);

        tabs = new QTabBar(this);
        tabs->setExpanding(false);
        tabs->setDrawBase(false);
        layout->addWidget(tabs);

        list = new QListWidget(this);
        list->setFocusPolicy(Qt::NoFocus);
        list->setSelectionMode(QAbstractItemView::SingleSelection);
        list->setUniformItemSizes(true);
        layout->addWidget(list, 1);

        connect(tabs, &QTabBar::currentChanged, this, [this](int) { rebuild_list(); });
        connect(list, &QListWidget::itemClicked, this, [this](QListWidgetItem *item) {
            if (item != nullptr) {
                activate(item->data(Qt::UserRole).toString());
            }
        });

        hide();
    }

    void set_state(const QJsonObject &state) {
        entries = state.value(QStringLiteral("entries")).toArray();
        raw = state.value(QStringLiteral("raw")).toString();
        rebuild_tabs();

        if (entries.isEmpty()) {
            hide();
            return;
        }

        const int width = qMax(320, anchor->width());
        const int rows = qBound(1, list->count(), 12);
        resize(width, tabs->sizeHint().height() + rows * 24 + 12);

        const QPoint position = anchor->mapTo(parentWidget(), QPoint(0, anchor->height()));
        move(position);
        raise();
        show();
        list->setCurrentRow(0);
    }

    void move_selection(int delta) {
        if (!isVisible() || list->count() == 0) {
            return;
        }

        int row = list->currentRow() + delta;

        if (row < 0) {
            row = list->count() - 1;
        } else if (row >= list->count()) {
            row = 0;
        }

        list->setCurrentRow(row);
    }

    QString current_text() const {
        QListWidgetItem *item = list->currentItem();

        return item == nullptr ? QString() : item->data(Qt::UserRole).toString();
    }

    QString raw_text() const {
        return raw;
    }

    bool is_open() const {
        return isVisible() && list->count() > 0;
    }

private:
    QString category_at(int index) const {
        return index < 0 ? QString() : tabs->tabData(index).toString();
    }

    void rebuild_tabs() {
        const QString previous = category_at(tabs->currentIndex());
        QSignalBlocker blocker(tabs);

        while (tabs->count() > 0) {
            tabs->removeTab(0);
        }

        const QStringList order{
            QStringLiteral("tags"),
            QStringLiteral("namespaces"),
            QStringLiteral("wildcards"),
            QStringLiteral("system"),
        };

        for (const QString &category : order) {
            int count = 0;
            QString label = category;

            for (const QJsonValue &value : entries) {
                if (value.toObject().value(QStringLiteral("category")).toString() == category) {
                    count += 1;
                }
            }

            if (count == 0) {
                continue;
            }

            if (category == QStringLiteral("tags")) {
                label = QStringLiteral("tags");
            } else if (category == QStringLiteral("namespaces")) {
                label = QStringLiteral("namespaces");
            } else if (category == QStringLiteral("wildcards")) {
                label = QStringLiteral("wildcards");
            } else {
                label = QStringLiteral("system");
            }

            const int index = tabs->addTab(QStringLiteral("%1 (%2)").arg(label).arg(count));
            tabs->setTabData(index, category);
        }

        int restore = 0;

        for (int index = 0; index < tabs->count(); ++index) {
            if (category_at(index) == previous) {
                restore = index;
                break;
            }
        }

        if (tabs->count() > 0) {
            tabs->setCurrentIndex(restore);
        }

        rebuild_list();
    }

    void rebuild_list() {
        list->clear();
        const QString category = category_at(tabs->currentIndex());

        for (const QJsonValue &value : entries) {
            const QJsonObject entry = value.toObject();

            if (entry.value(QStringLiteral("category")).toString() != category) {
                continue;
            }

            const QString text = entry.value(QStringLiteral("text")).toString();
            QString label = text;
            const QJsonValue count = entry.value(QStringLiteral("count"));

            if (!count.isNull() && count.isDouble()) {
                label = QStringLiteral("%1  (%2)").arg(text).arg(static_cast<qint64>(count.toDouble()));
            }

            auto *item = new QListWidgetItem(label, list);
            item->setData(Qt::UserRole, text);
            item->setToolTip(text);
        }

        if (list->count() > 0) {
            list->setCurrentRow(0);
        }
    }

    void activate(const QString &text) {
        hide();

        if (!text.isEmpty()) {
            insert(text);
        }
    }

    QWidget *anchor = nullptr;
    QTabBar *tabs = nullptr;
    QListWidget *list = nullptr;
    QJsonArray entries;
    QString raw;
    std::function<void(const QString &)> insert;
};
