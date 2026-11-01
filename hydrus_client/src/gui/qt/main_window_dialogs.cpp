#include <hydrus_client/src/gui/qt/api.cxx.h>

// WS-15.1.3b: HydrusMainWindow small dialog openers moved verbatim from bridge.cpp (move-only split).

#include "support/support.hpp"

#include <QInputDialog>
#include <QJsonObject>
#include <QDialog>
#include <QDialogButtonBox>
#include <QVBoxLayout>

void HydrusMainWindow::prompt_url() {
    bool ok = false;
    const QString url = QInputDialog::getText(
        this,
        QStringLiteral("import url"),
        QStringLiteral("file url"),
        QLineEdit::Normal,
        QString(),
        &ok);

    if (ok && !url.isEmpty()) {
        notify(UiAction::DownloadUrl, QString(), url);
    }
}

// WS-15.2.2b: open_viewer/update_viewer bodies moved to typed_media.cpp
// (typed ViewerState; the QJsonObject seam is gone).

void HydrusMainWindow::close_viewer(const QString &viewer_id) {
    if (HydrusViewerWindow *viewer = viewers.value(viewer_id, nullptr)) {
        viewers.remove(viewer_id);
        viewer->close();
    }
}


// WS-15.1.5: duplicates/services/network dialog openers moved verbatim from bridge.cpp.

void HydrusMainWindow::show_duplicates_rules(const DuplicatesRulesDialogState &typed) {
    const QJsonObject state = QJsonDocument::fromJson(to_qstring(typed.json).toUtf8()).object();
    if (duplicates_rules_dialog == nullptr) {
        duplicates_rules_dialog = new HydrusDuplicatesRulesDialog(this, state, this);
        connect(duplicates_rules_dialog, &QObject::destroyed, this, [this]() { duplicates_rules_dialog = nullptr; });
    } else {
        duplicates_rules_dialog->set_state(state);
        duplicates_rules_dialog->raise();
        duplicates_rules_dialog->activateWindow();
    }

    duplicates_rules_dialog->show();
    for (auto it = duplicates.begin(); it != duplicates.end(); ++it) {
        if (it.value() != nullptr) {
            it.value()->update_auto_resolution_rules(state);
        }
    }
}

void HydrusMainWindow::show_duplicate_merge_options(const DuplicatesMergeOptionsDialogState &typed) {
    const QJsonObject state = QJsonDocument::fromJson(to_qstring(typed.json).toUtf8()).object();
    if (merge_options_dialog == nullptr) {
        merge_options_dialog = new HydrusDuplicateMergeOptionsDialog(this, state, this);
        connect(merge_options_dialog, &QObject::destroyed, this, [this]() { merge_options_dialog = nullptr; });
    } else {
        merge_options_dialog->set_state(state);
        merge_options_dialog->raise();
        merge_options_dialog->activateWindow();
    }

    merge_options_dialog->show();
}

void HydrusMainWindow::show_duplicate_custom_action(const DuplicateCustomActionDialogState &typed) {
    const QJsonObject state = QJsonDocument::fromJson(to_qstring(typed.json).toUtf8()).object();
    if (duplicate_custom_action_dialog != nullptr) {
        QObject::disconnect(duplicate_custom_action_dialog, &QObject::destroyed, this, nullptr);
        duplicate_custom_action_dialog->close();
        duplicate_custom_action_dialog = nullptr;
    }

    duplicate_custom_action_dialog = new HydrusDuplicateCustomActionDialog(this, state, this);
    connect(duplicate_custom_action_dialog, &QObject::destroyed, this, [this]() { duplicate_custom_action_dialog = nullptr; });
    duplicate_custom_action_dialog->show();
}

void HydrusMainWindow::update_auto_resolution_preview(const DuplicatesRulePreviewState &typed) {
    const QJsonObject state = QJsonDocument::fromJson(to_qstring(typed.json).toUtf8()).object();
    if (duplicates_rules_dialog != nullptr) {
        duplicates_rules_dialog->set_preview(state);
    }
}

void HydrusMainWindow::show_duplicate_review(const DuplicateReviewDialogState &typed) {
    const QJsonObject state = QJsonDocument::fromJson(to_qstring(typed.json).toUtf8()).object();
    if (duplicate_review_dialog == nullptr) {
        duplicate_review_dialog = new HydrusDuplicateReviewDialog(this, state, this);
        connect(duplicate_review_dialog, &QObject::destroyed, this, [this]() { duplicate_review_dialog = nullptr; });
        duplicate_review_dialog->show();
    } else {
        duplicate_review_dialog->set_state(state);
        duplicate_review_dialog->raise();
        duplicate_review_dialog->activateWindow();
    }
}

void HydrusMainWindow::update_duplicate_review(const DuplicateReviewDialogState &typed) {
    const QJsonObject state = QJsonDocument::fromJson(to_qstring(typed.json).toUtf8()).object();
    if (duplicate_review_dialog != nullptr) {
        duplicate_review_dialog->set_state(state);
    }
}

// WS-15.2.2b: show_tag_dialog moved to typed_media.cpp (the dialog envelope
// is still JSON, but the metadata panel takes a typed MetadataState).

void HydrusMainWindow::show_services(const ServicesDialogState &typed) {
    const QJsonObject state = QJsonDocument::fromJson(to_qstring(typed.json).toUtf8()).object();
    if (services_dialog == nullptr) {
        services_dialog = new HydrusServicesDialog(this, state, this);
        connect(services_dialog, &QObject::destroyed, this, [this]() { services_dialog = nullptr; });
        services_dialog->show();
    } else {
        services_dialog->set_state(state);
        services_dialog->raise();
        services_dialog->activateWindow();
    }
}

void HydrusMainWindow::update_services(const ServicesDialogState &typed) {
    const QJsonObject state = QJsonDocument::fromJson(to_qstring(typed.json).toUtf8()).object();
    if (services_dialog != nullptr) {
        services_dialog->set_state(state);
    }
}

void HydrusMainWindow::show_network(const NetworkDialogState &typed) {
    const QJsonObject state = QJsonDocument::fromJson(to_qstring(typed.json).toUtf8()).object();
    if (network_dialog == nullptr) {
        network_dialog = new HydrusNetworkDialog(this, state, this);
        connect(network_dialog, &QObject::destroyed, this, [this]() { network_dialog = nullptr; });
        network_dialog->show();
    } else {
        network_dialog->set_state(state);
        network_dialog->raise();
        network_dialog->activateWindow();
    }
}

void HydrusMainWindow::update_network(const NetworkDialogState &typed) {
    const QJsonObject state = QJsonDocument::fromJson(to_qstring(typed.json).toUtf8()).object();
    if (network_dialog != nullptr) {
        network_dialog->set_state(state);
    }
}
