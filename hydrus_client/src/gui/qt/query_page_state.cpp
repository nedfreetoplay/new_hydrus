#include "query_page_sidebar.hpp"

// WS-15.1.2b: query-page state and media commands moved verbatim from bridge.cpp (move-only split).

#include "bridge.hpp"
#include "support/support.hpp"

#include <QColor>
#include <QJsonArray>
#include <QJsonObject>
#include <QJsonValue>

const QString &HydrusQueryPage::page_id() const {
    return id;
}

void HydrusQueryPage::update_state(const QJsonObject &state) {
    const QString query = state.value(QStringLiteral("query")).toString();

    if (search->text() != query) {
        search->setText(query);
    }

    const QString location_text = state.value(QStringLiteral("location")).toString();
    location_value = location_text.isEmpty() ? QStringLiteral("my files") : location_text;
    location->setText(location_value);

    tag_services.clear();

    for (const QJsonValue &value : state.value(QStringLiteral("tag_services")).toArray()) {
        tag_services.append(value.toString());
    }

    tag_service_value = state.value(QStringLiteral("tag_service")).toString();
    tag_service->setText(
        tag_service_value.isEmpty() ? QStringLiteral("all known tags") : tag_service_value);

    const QString sort_key = state.value(QStringLiteral("sort")).toString();

    if (!sort_key.isEmpty()) {
        const bool ascending = state.value(QStringLiteral("sort_ascending")).toBool(true);
        const HydrusSortOption *option = find_sort_option(sort_key);
        sort_value = sort_key;
        sort_ascending_value = ascending;
        sort_type->setText(QString::fromUtf8(option->label));
        sort_order->setText(
            QString::fromUtf8(ascending ? option->ascending : option->descending));
    }

    predicates->clear();

    for (const QJsonValue &value : state.value(QStringLiteral("predicates")).toArray()) {
        const QString text = value.toString();
        auto *item = new QListWidgetItem(text, predicates);
        item->setForeground(
            text.startsWith(QStringLiteral("system:")) ? QColor(150, 90, 10) : QColor(32, 32, 32));
    }

    selection_tags->clear();

    for (const QJsonValue &value : state.value(QStringLiteral("tags")).toArray()) {
        selection_tags->addItem(value.toString());
    }

    selection_tags->sortItems(
        tag_sort_order->text() == QStringLiteral("a-z") ? Qt::AscendingOrder
                                                        : Qt::DescendingOrder);

    const QString focused = state.value(QStringLiteral("focused")).toString();
    const QJsonObject thumbnails = state.value(QStringLiteral("thumbnails")).toObject();

    grid->set_layout(
        thumbnails.value(QStringLiteral("masonry")).toBool(false),
        thumbnails.value(QStringLiteral("horizontal")).toBool(false),
        thumbnails.value(QStringLiteral("scale_percent")).toInt(100));
    grid->set_media(state.value(QStringLiteral("media")).toArray());
    update_progress(state);

    if (!focused.isEmpty()) {
        grid->set_focused(focused);
    }
}

void HydrusQueryPage::set_selection(
    const QString &hash,
    const QStringList &tags,
    const QString &preview_path,
    const QJsonObject &metadata_state) {
    Q_UNUSED(metadata_state);
    selection_tags->clear();
    selection_tags->addItems(tags);
    selection_tags->sortItems(
        tag_sort_order->text() == QStringLiteral("a-z") ? Qt::AscendingOrder
                                                        : Qt::DescendingOrder);

    grid->set_focused(hash);

    if (!preview_path.isEmpty() && preview->load_image(preview_path)) {
        return;
    }

    preview->clear_image();
}

void HydrusQueryPage::set_suggestions(const QStringList &suggestions) {
    Q_UNUSED(suggestions);
}

void HydrusQueryPage::focus_hash(const QString &hash) {
    const QByteArray page_bytes = id.toUtf8();
    const QByteArray hash_bytes = hash.toUtf8();
    app_action(*main, rust::Str("focus_hash", 10), rust::Str(page_bytes.constData(), page_bytes.size()), rust::Str(hash_bytes.constData(), hash_bytes.size()));
}

void HydrusQueryPage::open_focused() {
    const QString hash = focused_hash();

    if (hash.isEmpty()) {
        return;
    }

    const QByteArray page_bytes = id.toUtf8();
    const QByteArray hash_bytes = hash.toUtf8();
    app_action(*main, rust::Str("open_viewer", 11), rust::Str(page_bytes.constData(), page_bytes.size()), rust::Str(hash_bytes.constData(), hash_bytes.size()));
}

void HydrusQueryPage::archive_focused() {
    const QString hash = focused_hash();

    if (hash.isEmpty()) {
        return;
    }

    const QByteArray page_bytes = id.toUtf8();
    const QByteArray hash_bytes = hash.toUtf8();
    app_action(*main, rust::Str("archive_files", 13), rust::Str(page_bytes.constData(), page_bytes.size()), rust::Str(hash_bytes.constData(), hash_bytes.size()));
}

void HydrusQueryPage::inbox_focused() {
    const QString hash = focused_hash();

    if (hash.isEmpty()) {
        return;
    }

    const QByteArray page_bytes = id.toUtf8();
    const QByteArray hash_bytes = hash.toUtf8();
    app_action(*main, rust::Str("inbox_files", 11), rust::Str(page_bytes.constData(), page_bytes.size()), rust::Str(hash_bytes.constData(), hash_bytes.size()));
}

void HydrusQueryPage::delete_focused() {
    const QString hash = focused_hash();

    if (hash.isEmpty()) {
        return;
    }

    const QByteArray page_bytes = id.toUtf8();
    const QByteArray hash_bytes = hash.toUtf8();
    app_action(*main, rust::Str("delete_files", 12), rust::Str(page_bytes.constData(), page_bytes.size()), rust::Str(hash_bytes.constData(), hash_bytes.size()));
}

QString HydrusQueryPage::focused_hash() const {
    return grid->focused_hash();
}
