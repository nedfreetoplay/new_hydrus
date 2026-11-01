
// WS-15.1.2b: query-page state and media commands moved verbatim from bridge.cpp (move-only split).

#include <hydrus_client/src/gui/qt/api.cxx.h>
#include "support/support.hpp"

#include <QColor>
#include <QJsonArray>
#include <QJsonObject>
#include <QJsonValue>

const QString &HydrusQueryPage::page_id() const {
    return id;
}

void HydrusQueryPage::focus_hash(const QString &hash) {
    const QByteArray page_bytes = id.toUtf8();
    const QByteArray hash_bytes = hash.toUtf8();
    app_focus_hash(*main, rust::Str(page_bytes.constData(), page_bytes.size()), rust::Str(hash_bytes.constData(), hash_bytes.size()));
}

void HydrusQueryPage::open_focused() {
    const QString hash = focused_hash();

    if (hash.isEmpty()) {
        return;
    }

    const QByteArray page_bytes = id.toUtf8();
    const QByteArray hash_bytes = hash.toUtf8();
    app_open_viewer(*main, rust::Str(page_bytes.constData(), page_bytes.size()), rust::Str(hash_bytes.constData(), hash_bytes.size()));
}

void HydrusQueryPage::archive_focused() {
    const QString hash = focused_hash();

    if (hash.isEmpty()) {
        return;
    }

    const QByteArray page_bytes = id.toUtf8();
    const QByteArray hash_bytes = hash.toUtf8();
    app_action(*main, UiAction::ArchiveFiles, rust::Str(page_bytes.constData(), page_bytes.size()), rust::Str(hash_bytes.constData(), hash_bytes.size()));
}

void HydrusQueryPage::inbox_focused() {
    const QString hash = focused_hash();

    if (hash.isEmpty()) {
        return;
    }

    const QByteArray page_bytes = id.toUtf8();
    const QByteArray hash_bytes = hash.toUtf8();
    app_action(*main, UiAction::InboxFiles, rust::Str(page_bytes.constData(), page_bytes.size()), rust::Str(hash_bytes.constData(), hash_bytes.size()));
}

void HydrusQueryPage::delete_focused() {
    const QString hash = focused_hash();

    if (hash.isEmpty()) {
        return;
    }

    const QByteArray page_bytes = id.toUtf8();
    const QByteArray hash_bytes = hash.toUtf8();
    app_action(*main, UiAction::DeleteFiles, rust::Str(page_bytes.constData(), page_bytes.size()), rust::Str(hash_bytes.constData(), hash_bytes.size()));
}

QString HydrusQueryPage::focused_hash() const {
    return grid->focused_hash();
}

void HydrusQueryPage::set_suggestions(const QStringList &suggestions) {
    Q_UNUSED(suggestions);
}
