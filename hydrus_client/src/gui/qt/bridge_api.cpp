#include <hydrus_client/src/gui/qt/api.cxx.h>
#include <hydrus_client/src/gui/qt/api.cxx.h>

#include "support/support.hpp"

#include <QApplication>
#include <QClipboard>
#include <QCoreApplication>
#include <QDesktopServices>
#include <QFileDialog>
#include <QInputDialog>
#include <QJsonDocument>
#include <QLineEdit>
#include <QMessageBox>
#include <QPainter>
#include <QPixmap>
#include <QStyle>
#include <QStyleFactory>
#include <QTimer>
#include <QUrl>

// WS-15.1.5: free hydrus_*/splash/qapp glue moved verbatim from bridge.cpp (move-only split).

void hydrus_show_or_dialog(HydrusMainWindow &window, rust::Str page_id, rust::Str initial) {
    show_or_predicate_dialog(window, to_qstring(page_id), to_qstring(initial));
}

void hydrus_show_page_manager(HydrusMainWindow &window, const PageTree &tree) {
    HydrusPageManager manager(&window, tree, &window);
    manager.exec();
}

void hydrus_show_command_palette(HydrusMainWindow &window) {
    show_command_palette(&window);
}

void hydrus_prompt_text(
    HydrusMainWindow &window,
    UiAction action,
    rust::Str title,
    rust::Str label,
    rust::Str initial) {
    bool ok = false;
    const QString text = QInputDialog::getText(
        &window,
        from_rust(title),
        from_rust(label),
        QLineEdit::Normal,
        from_rust(initial),
        &ok);

    if (ok) {
        window.notify(action, QString(), text);
    }
}

void hydrus_confirm_action(
    HydrusMainWindow &window,
    UiAction action,
    rust::Str target,
    rust::Str title,
    rust::Str text) {
    const QMessageBox::StandardButton answer = QMessageBox::question(
        &window,
        from_rust(title),
        from_rust(text),
        QMessageBox::Yes | QMessageBox::No,
        QMessageBox::No);

    if (answer == QMessageBox::Yes) {
        window.notify(action, from_rust(target));
    }
}

void hydrus_show_manage_tags(HydrusMainWindow &window, const ManageTagsDialogState &state) {
    HydrusManageTagsDialog dialog(&window, parse_object(rust::Str(state.json.data(), state.json.size())), &window);
    dialog.exec();
}

void hydrus_show_timestamps(HydrusMainWindow &window, const TimestampsDialogState &state) {
    HydrusTimestampsDialog dialog(&window, parse_object(rust::Str(state.json.data(), state.json.size())), &window);
    dialog.exec();
}

QApplication *new_application_impl() {
    static int argc = 1;
    static char name[] = "hydrus_client";
    static char *argv[] = {name, nullptr};
    return new QApplication(argc, argv);
}

std::unique_ptr<QApplication> new_application() {
    return std::unique_ptr<QApplication>(new_application_impl());
}

int exec_application(QApplication &application) {
    return application.exec();
}

void quit_application(QApplication &application) {
    application.quit();
}

void qapp_process_events() {
    QApplication::processEvents();
}

std::unique_ptr<QSplashScreen> new_hydrus_splash() {
    QPixmap pixmap(620, 340);
    pixmap.fill(QColor(36, 36, 36));

    QPainter painter(&pixmap);
    painter.setRenderHint(QPainter::Antialiasing);

    QFont title_font = painter.font();
    title_font.setPointSize(30);
    title_font.setBold(true);
    painter.setFont(title_font);
    painter.setPen(QColor(238, 238, 238));
    painter.drawText(QRect(36, 48, 548, 64), Qt::AlignLeft | Qt::AlignVCenter, QStringLiteral("Hydrus Network"));

    QFont subtitle_font = painter.font();
    subtitle_font.setPointSize(13);
    subtitle_font.setBold(false);
    painter.setFont(subtitle_font);
    painter.setPen(QColor(170, 170, 170));
    painter.drawText(
        QRect(38, 118, 548, 30),
        Qt::AlignLeft | Qt::AlignVCenter,
        QStringLiteral("Rust + Qt 6 client"));

    painter.setPen(QColor(120, 120, 120));
    painter.drawLine(38, 168, 582, 168);

    painter.setPen(QColor(150, 150, 150));
    painter.drawText(
        QRect(38, 186, 548, 80),
        Qt::AlignLeft | Qt::AlignTop,
        QStringLiteral("A media management application for the hydrus network.\n"
                       "Opening the client database and preparing the session..."));
    painter.end();

    auto *splash = new QSplashScreen(pixmap);
    splash->setWindowFlag(Qt::WindowStaysOnTopHint, true);
    splash->setWindowFlag(Qt::FramelessWindowHint, true);
    return std::unique_ptr<QSplashScreen>(splash);
}

void hydrus_splash_show(QSplashScreen &splash) {
    splash.show();
    QApplication::processEvents();
}

void hydrus_splash_message(QSplashScreen &splash, rust::Str message) {
    splash.showMessage(
        to_qstring(message),
        Qt::AlignLeft | Qt::AlignBottom,
        QColor(225, 225, 225));
    QApplication::processEvents();
}

void hydrus_splash_finish_later(QSplashScreen &splash, int delay_ms) {
    QTimer::singleShot(delay_ms, &splash, [&splash]() { splash.close(); });
}

void hydrus_splash_screenshot(QSplashScreen &splash, rust::Str path, int delay_ms) {
    const QString target = to_qstring(path);

    QTimer::singleShot(delay_ms, &splash, [&splash, target]() {
        splash.grab().save(target);
    });
}

void hydrus_splash_finish(QSplashScreen &splash) {
    splash.close();
}

std::unique_ptr<HydrusMainWindow> new_hydrus_main_window() {
    return std::make_unique<HydrusMainWindow>();
}

void build_hydrus_shell(HydrusMainWindow &window) {
    window.build_shell();
}

void show_hydrus_main_window(HydrusMainWindow &window) {
    window.show();
}

void hydrus_set_title(HydrusMainWindow &window, rust::Str title) {
    window.setWindowTitle(to_qstring(title));
}

void hydrus_resize_window(HydrusMainWindow &window, int width, int height) {
    window.resize(width, height);
}

void hydrus_set_menus(HydrusMainWindow &window, const MenuBar &menus) {
    window.set_menus(menus);
}

void hydrus_set_tag_suggestions(HydrusMainWindow &window, rust::Str json) {
    window.set_tag_suggestions(json_to_strings(parse_array(json)));
}

void hydrus_show_autocomplete(HydrusMainWindow &window, rust::Str json) {
    window.show_autocomplete(parse_object(json));
}

void hydrus_set_status(HydrusMainWindow &window, const StatusBar &status) {
    window.set_status(status);
}

void hydrus_build_pages(HydrusMainWindow &window, const Pages &pages) {
    window.build_pages(pages);
}

void hydrus_update_page(HydrusMainWindow &window, const QueryPageState &page) {
    window.update_page(page);
}

void hydrus_update_page_progress(HydrusMainWindow &window, const PageProgress &progress) {
    window.update_page_progress(progress);
}

void hydrus_update_downloader(HydrusMainWindow &window, const DownloaderPageState &state) {
    window.update_downloader(state);
}

void hydrus_update_subscriptions(HydrusMainWindow &window, const SubscriptionsPageState &state) {
    window.update_subscriptions(state);
}

void hydrus_show_subscriptions_dialog(HydrusMainWindow &window, const SubscriptionsDialogState &state) {
    window.show_subscriptions_dialog(state);
}

void hydrus_update_subscriptions_dialog(HydrusMainWindow &window, const SubscriptionsDialogState &state) {
    window.update_subscriptions_dialog(state);
}

void hydrus_show_subscription_editor(HydrusMainWindow &window, const SubscriptionEditorState &state) {
    window.show_subscription_editor(state);
}

void hydrus_update_duplicates(HydrusMainWindow &window, rust::Str page_id, rust::Str json) {
    window.update_duplicates(to_qstring(page_id), parse_object(json));
}

void hydrus_update_import_folders(HydrusMainWindow &window, rust::Str page_id, rust::Str json) {
    window.update_import_folders(to_qstring(page_id), parse_object(json));
}

void hydrus_show_import_folders_dialog(HydrusMainWindow &window, rust::Str json) {
    window.show_import_folders_dialog(parse_object(json));
}

void hydrus_update_import_folders_dialog(HydrusMainWindow &window, rust::Str json) {
    window.update_import_folders_dialog(parse_object(json));
}

void hydrus_update_export_folders(HydrusMainWindow &window, rust::Str page_id, rust::Str json) {
    window.update_export_folders(to_qstring(page_id), parse_object(json));
}

void hydrus_show_export_folders_dialog(HydrusMainWindow &window, rust::Str json) {
    window.show_export_folders_dialog(parse_object(json));
}

void hydrus_update_export_folders_dialog(HydrusMainWindow &window, rust::Str json) {
    window.update_export_folders_dialog(parse_object(json));
}

void hydrus_show_duplicates_rules(HydrusMainWindow &window, const DuplicatesRulesDialogState &state) {
    window.show_duplicates_rules(state);
}

void hydrus_show_duplicate_merge_options(HydrusMainWindow &window, const DuplicatesMergeOptionsDialogState &state) {
    window.show_duplicate_merge_options(state);
}

void hydrus_show_duplicate_custom_action(HydrusMainWindow &window, const DuplicateCustomActionDialogState &state) {
    window.show_duplicate_custom_action(state);
}

void hydrus_show_import_options(HydrusMainWindow &window, const ImportOptionsDialogState &state) {
    window.show_import_options(state);
}

void hydrus_show_file_seed_cache(HydrusMainWindow &window, const FileSeedCacheState &state) {
    window.show_file_seed_cache(state);
}

void hydrus_show_gallery_seed_log(HydrusMainWindow &window, const GallerySeedLogState &state) {
    window.show_gallery_seed_log(state);
}

void hydrus_update_auto_resolution_preview(HydrusMainWindow &window, const DuplicatesRulePreviewState &state) {
    window.update_auto_resolution_preview(state);
}

void hydrus_show_duplicate_review(HydrusMainWindow &window, const DuplicateReviewDialogState &state) {
    window.show_duplicate_review(state);
}

void hydrus_update_duplicate_review(HydrusMainWindow &window, const DuplicateReviewDialogState &state) {
    window.update_duplicate_review(state);
}

void hydrus_show_tag_relations(HydrusMainWindow &window, const TagRelationsDialogState &state) {
    window.show_tag_relations(state);
}

void hydrus_show_import_folder(HydrusMainWindow &window, rust::Str json) {
    window.show_import_folder(parse_object(json));
}

void hydrus_show_local_import_review(HydrusMainWindow &window, const LocalImportReviewDialogState &state) {
    window.show_local_import_review(state);
}

void hydrus_show_local_import_tagging(HydrusMainWindow &window, const LocalImportTaggingDialogState &state) {
    window.show_local_import_tagging(state);
}

void hydrus_close_local_import_review(HydrusMainWindow &window) {
    window.close_local_import_review();
}

void hydrus_show_repair_file_locations(HydrusMainWindow &window, const RepairFileLocationsDialogState &state) {
    window.show_repair_file_locations(state);
}

void hydrus_close_repair_file_locations(HydrusMainWindow &window) {
    window.close_repair_file_locations();
}

void hydrus_show_export_folder(HydrusMainWindow &window, rust::Str json) {
    window.show_export_folder(parse_object(json));
}

void hydrus_show_export_review(HydrusMainWindow &window, const ExportReviewDialogState &state) {
    window.show_export_review(state);
}

void hydrus_show_notes(HydrusMainWindow &window, rust::Str json) {
    window.show_notes(parse_object(json));
}

void hydrus_set_selection(HydrusMainWindow &window, const SelectionState &selection) {
    window.set_selection(selection);
}

void hydrus_show_page_chooser(HydrusMainWindow &window, rust::Str notebook_id) {
    window.show_page_chooser(to_qstring(notebook_id));
}

void hydrus_pick_files(HydrusMainWindow &window) {
    const QStringList paths = QFileDialog::getOpenFileNames(
        &window,
        QStringLiteral("import files"),
        QString(),
        QStringLiteral("all files (*.*)"));

    if (paths.isEmpty()) {
        return;
    }

    QJsonArray array;

    for (const QString &path : paths) {
        array.append(path);
    }

    window.notify(UiAction::ImportPaths, QString(), QString::fromUtf8(QJsonDocument(array).toJson(QJsonDocument::Compact)));
}

void hydrus_prompt_url(HydrusMainWindow &window) {
    window.prompt_url();
}

void hydrus_open_viewer(HydrusMainWindow &window, const ViewerState &state) {
    window.open_viewer(state);
}

void hydrus_update_viewer(HydrusMainWindow &window, const ViewerState &state) {
    window.update_viewer(state);
}

void hydrus_close_viewer(HydrusMainWindow &window, rust::Str viewer_id) {
    window.close_viewer(to_qstring(viewer_id));
}

void hydrus_show_tag_dialog(HydrusMainWindow &window, rust::Str json) {
    window.show_tag_dialog(parse_object(json));
}

void hydrus_show_notifications(HydrusMainWindow &window, const NotificationsDialogState &state) {
    window.show_notifications(state);
}

void hydrus_show_viewing_stats(HydrusMainWindow &window, const ViewingStats &state) {
    window.show_viewing_stats(state);
}

void hydrus_show_about(HydrusMainWindow &window, const AboutInfo &state) {
    window.show_about(state);
}

void hydrus_export_downloader(HydrusMainWindow &window, rust::Str page_id) {
    const QString path = QFileDialog::getSaveFileName(
        &window,
        QStringLiteral("export downloader definition"),
        QStringLiteral("downloader.json"),
        QStringLiteral("JSON (*.json)"));

    if (!path.isEmpty()) {
        window.notify(UiAction::DownloaderExportTo, to_qstring(page_id), path);
    }
}

void hydrus_import_downloader(HydrusMainWindow &window, rust::Str target) {
    const QString path = QFileDialog::getOpenFileName(
        &window,
        QStringLiteral("import downloader definition"),
        QString(),
        QStringLiteral("JSON (*.json)"));

    if (!path.isEmpty()) {
        window.notify(UiAction::DownloaderImportFrom, to_qstring(target), path);
    }
}

void hydrus_show_options(HydrusMainWindow &window, const OptionsDialogState &state) {
    window.show_options(state);
}

void hydrus_show_services(HydrusMainWindow &window, const ServicesDialogState &state) {
    window.show_services(state);
}

void hydrus_update_services(HydrusMainWindow &window, const ServicesDialogState &state) {
    window.update_services(state);
}

void hydrus_show_tag_filter(HydrusMainWindow &window, const TagFilterDialogState &state) {
    window.show_tag_filter(state);
}

void hydrus_show_api_permissions(HydrusMainWindow &window, const ApiPermissionsDialogState &state) {
    window.show_api_permissions(state);
}

void hydrus_update_api_permissions(HydrusMainWindow &window, const ApiPermissionsDialogState &state) {
    window.update_api_permissions(state);
}

void hydrus_show_account_types(HydrusMainWindow &window, const AccountTypesDialogState &state) {
    window.show_account_types(state);
}

void hydrus_update_account_types(HydrusMainWindow &window, const AccountTypesDialogState &state) {
    window.update_account_types(state);
}

void hydrus_show_orphan_files(HydrusMainWindow &window, const OrphanFilesDialogState &state) {
    window.show_orphan_files(state);
}

void hydrus_update_orphan_files(HydrusMainWindow &window, const OrphanFilesDialogState &state) {
    window.update_orphan_files(state);
}

void hydrus_show_database_backup(HydrusMainWindow &window, const DatabaseBackupDialogState &state) {
    window.show_database_backup(state);
}

void hydrus_update_database_backup(HydrusMainWindow &window, const DatabaseBackupDialogState &state) {
    window.update_database_backup(state);
}

void hydrus_show_network(HydrusMainWindow &window, const NetworkDialogState &state) {
    window.show_network(state);
}

void hydrus_update_network(HydrusMainWindow &window, const NetworkDialogState &state) {
    window.update_network(state);
}

void hydrus_start_tick(HydrusMainWindow &window) {
    window.start_tick();
}

void hydrus_defer_action(
    HydrusMainWindow &window,
    UiAction action,
    rust::Str target,
    rust::Str payload,
    int delay_ms) {
    const QString target_text = to_qstring(target);
    const QString payload_text = to_qstring(payload);

    QTimer::singleShot(delay_ms, &window, [&window, action, target_text, payload_text]() {
        QByteArray target_bytes = target_text.toUtf8();
        QByteArray payload_bytes = payload_text.toUtf8();

        app_action(
            window,
            action,
            rust::Str(target_bytes.constData(), target_bytes.size()),
            rust::Str(payload_bytes.constData(), payload_bytes.size()));
    });
}

void hydrus_show_message(HydrusMainWindow &window, rust::Str title, rust::Str text) {
    window.show_message(to_qstring(title), to_qstring(text));
}

void hydrus_notify_message(HydrusMainWindow &window, rust::Str message) {
    window.notify_message(to_qstring(message));
}

void hydrus_set_clipboard(HydrusMainWindow &window, rust::Str text) {
    Q_UNUSED(window);
    QApplication::clipboard()->setText(to_qstring(text));
}

void hydrus_open_path(rust::Str path) {
    QDesktopServices::openUrl(QUrl::fromLocalFile(to_qstring(path)));
}

void hydrus_open_url(rust::Str url) {
    QDesktopServices::openUrl(QUrl(to_qstring(url)));
}

void hydrus_schedule_screenshot(HydrusMainWindow &window, rust::Str path) {
    window.schedule_screenshot(to_qstring(path));
}

void qapp_set_application_name(rust::Str name) {
    QApplication::setApplicationName(to_qstring(name));
}

void qapp_set_application_display_name(rust::Str name) {
    QApplication::setApplicationDisplayName(to_qstring(name));
}

void qapp_set_organization_name(rust::Str name) {
    QApplication::setOrganizationName(to_qstring(name));
}

void qapp_set_style(rust::Str name) {
    if (QStyle *style = QStyleFactory::create(to_qstring(name))) {
        QApplication::setStyle(style);
    }
}

void qapp_set_stylesheet(rust::Str stylesheet) {
    qApp->setStyleSheet(to_qstring(stylesheet));
}

void qapp_quit() {
    QCoreApplication::quit();
}
