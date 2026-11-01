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
    if (duplicates_rules_dialog == nullptr) {
        duplicates_rules_dialog = new HydrusDuplicatesRulesDialog(this, typed, this);
        connect(duplicates_rules_dialog, &QObject::destroyed, this, [this]() { duplicates_rules_dialog = nullptr; });
    } else {
        duplicates_rules_dialog->set_state(typed);
        duplicates_rules_dialog->raise();
        duplicates_rules_dialog->activateWindow();
    }

    duplicates_rules_dialog->show();
    for (auto it = duplicates.begin(); it != duplicates.end(); ++it) {
        if (it.value() != nullptr) {
            it.value()->update_auto_resolution_rules(typed);
        }
    }
}

void HydrusMainWindow::show_duplicate_merge_options(const DuplicatesMergeOptionsDialogState &typed) {
    if (merge_options_dialog == nullptr) {
        merge_options_dialog = new HydrusDuplicateMergeOptionsDialog(this, typed, this);
        connect(merge_options_dialog, &QObject::destroyed, this, [this]() { merge_options_dialog = nullptr; });
    } else {
        merge_options_dialog->set_state(typed);
        merge_options_dialog->raise();
        merge_options_dialog->activateWindow();
    }

    merge_options_dialog->show();
}

void HydrusMainWindow::show_duplicate_custom_action(const DuplicateCustomActionDialogState &typed) {
    if (duplicate_custom_action_dialog != nullptr) {
        QObject::disconnect(duplicate_custom_action_dialog, &QObject::destroyed, this, nullptr);
        duplicate_custom_action_dialog->close();
        duplicate_custom_action_dialog = nullptr;
    }

    duplicate_custom_action_dialog = new HydrusDuplicateCustomActionDialog(this, typed, this);
    connect(duplicate_custom_action_dialog, &QObject::destroyed, this, [this]() { duplicate_custom_action_dialog = nullptr; });
    duplicate_custom_action_dialog->show();
}

void HydrusMainWindow::update_auto_resolution_preview(const DuplicatesRulePreviewState &typed) {
    if (duplicates_rules_dialog != nullptr) {
        duplicates_rules_dialog->set_preview(typed);
    }
}

void HydrusMainWindow::show_duplicate_review(const DuplicateReviewDialogState &typed) {
    if (duplicate_review_dialog == nullptr) {
        duplicate_review_dialog = new HydrusDuplicateReviewDialog(this, typed, this);
        connect(duplicate_review_dialog, &QObject::destroyed, this, [this]() { duplicate_review_dialog = nullptr; });
        duplicate_review_dialog->show();
    } else {
        duplicate_review_dialog->set_state(typed);
        duplicate_review_dialog->raise();
        duplicate_review_dialog->activateWindow();
    }
}

void HydrusMainWindow::update_duplicate_review(const DuplicateReviewDialogState &typed) {
    if (duplicate_review_dialog != nullptr) {
        duplicate_review_dialog->set_state(typed);
    }
}

// WS-15.2.2b: show_tag_dialog moved to typed_media.cpp (the dialog envelope
// is still JSON, but the metadata panel takes a typed MetadataState).

void HydrusMainWindow::show_services(const ServicesDialogState &typed) {
    if (services_dialog == nullptr) {
        services_dialog = new HydrusServicesDialog(this, typed, this);
        connect(services_dialog, &QObject::destroyed, this, [this]() { services_dialog = nullptr; });
        services_dialog->show();
    } else {
        services_dialog->set_state(typed);
        services_dialog->raise();
        services_dialog->activateWindow();
    }
}

void HydrusMainWindow::update_services(const ServicesDialogState &typed) {
    if (services_dialog != nullptr) {
        services_dialog->set_state(typed);
    }
}

void HydrusMainWindow::show_network(const NetworkDialogState &typed) {
    if (network_dialog == nullptr) {
        network_dialog = new HydrusNetworkDialog(this, typed, this);
        connect(network_dialog, &QObject::destroyed, this, [this]() { network_dialog = nullptr; });
        network_dialog->show();
    } else {
        network_dialog->set_state(typed);
        network_dialog->raise();
        network_dialog->activateWindow();
    }
}

void HydrusMainWindow::update_network(const NetworkDialogState &typed) {
    if (network_dialog != nullptr) {
        network_dialog->set_state(typed);
    }
}
