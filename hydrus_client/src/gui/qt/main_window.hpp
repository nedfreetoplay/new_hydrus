#pragma once

// WS-15.1.4: HydrusMainWindow moved verbatim from bridge.hpp (move-only split).
// The class stays whole; its methods live in the .cpp of their screen.

#include <QCloseEvent>
#include <QDragEnterEvent>
#include <QDropEvent>
#include <QEvent>
#include <QHash>
#include <QJsonArray>
#include <QJsonObject>
#include <QLabel>
#include <QMainWindow>
#include <QString>
#include <QStringList>
#include <QSystemTrayIcon>
#include <QTimer>
#include <QWidget>

#include <cstddef>

class QCompleter;
class QMovie;
class QTabBar;
class QVBoxLayout;
class HydrusMainWindow;
class HydrusNotebook;
enum class UiAction : std::int32_t;
class HydrusTabBar;
class HydrusQueryPage;
class HydrusThumbnailGrid;
class HydrusViewerWindow;
class HydrusDownloaderPage;
class HydrusSubscriptionsPage;
class HydrusSubscriptionEditorDialog;
class HydrusSubscriptionsDialog;
class HydrusImportFoldersDialog;
class HydrusExportFoldersDialog;
class HydrusImportOptionsDialog;
class HydrusDuplicatesPage;
struct DownloaderPageState;
struct SubscriptionsPageState;
struct SubscriptionsDialogState;
struct SubscriptionEditorState;
struct FileSeedCacheState;
struct GallerySeedLogState;
struct ImportOptionsDialogState;
struct OptionsDialogState;
struct ServicesDialogState;
struct NetworkDialogState;
struct AccountTypesDialogState;
struct ApiPermissionsDialogState;
struct TagFilterDialogState;
struct TagRelationsDialogState;
struct TimestampsDialogState;
struct NotificationsDialogState;
struct ExportReviewDialogState;
struct RepairFileLocationsDialogState;
struct OrphanFilesDialogState;
struct DatabaseBackupDialogState;
struct LocalImportReviewDialogState;
struct LocalImportTaggingDialogState;
struct DuplicatesRulesDialogState;
struct DuplicatesMergeOptionsDialogState;
struct DuplicateCustomActionDialogState;
struct DuplicatesRulePreviewState;
struct DuplicateReviewDialogState;
class HydrusServicesDialog;
class HydrusApiPermissionsDialog;
class HydrusAccountTypesDialog;
class HydrusOrphanFilesDialog;
class HydrusDatabaseBackupDialog;
class HydrusNetworkDialog;
class HydrusMetadataPanel;
class HydrusImportFoldersPage;
class HydrusExportFoldersPage;
class HydrusDuplicatesRulesDialog;
class HydrusAutoResolutionRuleDialog;
class HydrusDuplicateReviewDialog;
class HydrusDuplicateMergeOptionsDialog;
class HydrusDuplicateCustomActionDialog;
class HydrusTagRelationsDialog;
class HydrusSeedCacheDialog;
class HydrusLocalImportReviewDialog;
class HydrusLocalImportTaggingDialog;
class HydrusRepairFileLocationsDialog;

struct StatusBar;
struct AboutInfo;
struct ViewingStats;
struct MenuBar;
struct Pages;
struct PageProgress;
struct PageTree;
// WS-15.2.2b: media/selection/viewer seams (typed bodies in typed_media.cpp).
struct QueryPageState;
struct SelectionState;
struct MetadataState;
struct ViewerState;

class HydrusMainWindow : public QMainWindow {
public:
    explicit HydrusMainWindow(QWidget *parent = nullptr);

    void build_shell();
    void set_status(const StatusBar &status);
    void set_menus(const MenuBar &menus);
    void build_pages(const Pages &pages);
    void update_page(const QueryPageState &page);
    void update_page_progress(const PageProgress &progress);
    void update_downloader(const DownloaderPageState &state);
    void update_subscriptions(const SubscriptionsPageState &state);
    void show_subscriptions_dialog(const SubscriptionsDialogState &state);
    void update_subscriptions_dialog(const SubscriptionsDialogState &state);
    void show_subscription_editor(const SubscriptionEditorState &state);
    void update_duplicates(const QString &page_id, const QJsonObject &state);
    void update_import_folders(const QString &page_id, const QJsonObject &state);
    void show_import_folders_dialog(const QJsonObject &state);
    void update_import_folders_dialog(const QJsonObject &state);
    void update_export_folders(const QString &page_id, const QJsonObject &state);
    void show_export_folders_dialog(const QJsonObject &state);
    void update_export_folders_dialog(const QJsonObject &state);
    void set_selection(const SelectionState &selection);
    void set_tag_suggestions(const QStringList &suggestions);
    const QStringList &suggestions() const;
    void show_autocomplete(const QJsonObject &state);
    void show_page_chooser(const QString &notebook_id);
    void prompt_url();
    void open_viewer(const ViewerState &state);
    void update_viewer(const ViewerState &state);
    void close_viewer(const QString &viewer_id);
    void show_tag_dialog(const QJsonObject &state);
    void show_notifications(const NotificationsDialogState &state);
    void show_viewing_stats(const ViewingStats &state);
    void show_about(const AboutInfo &state);
    void show_notes(const QJsonObject &state);
    void show_duplicates_rules(const DuplicatesRulesDialogState &state);
    void show_duplicate_merge_options(const DuplicatesMergeOptionsDialogState &state);
    void show_duplicate_custom_action(const DuplicateCustomActionDialogState &state);
    void show_import_options(const ImportOptionsDialogState &state);
    void show_file_seed_cache(const FileSeedCacheState &state);
    void show_gallery_seed_log(const GallerySeedLogState &state);
    void update_auto_resolution_preview(const DuplicatesRulePreviewState &state);
    void show_duplicate_review(const DuplicateReviewDialogState &state);
    void update_duplicate_review(const DuplicateReviewDialogState &state);
    void show_tag_relations(const TagRelationsDialogState &state);
    void show_import_folder(const QJsonObject &state);
    void show_local_import_review(const LocalImportReviewDialogState &state);
    void show_local_import_tagging(const LocalImportTaggingDialogState &state);
    void close_local_import_review();
    void show_repair_file_locations(const RepairFileLocationsDialogState &state);
    void close_repair_file_locations();
    void show_export_folder(const QJsonObject &state);
    void show_export_review(const ExportReviewDialogState &state);
    void show_options(const OptionsDialogState &state);
    void show_services(const ServicesDialogState &state);
    void update_services(const ServicesDialogState &state);
    void show_tag_filter(const TagFilterDialogState &state);
    void show_api_permissions(const ApiPermissionsDialogState &state);
    void update_api_permissions(const ApiPermissionsDialogState &state);
    void show_account_types(const AccountTypesDialogState &state);
    void update_account_types(const AccountTypesDialogState &state);
    void show_orphan_files(const OrphanFilesDialogState &state);
    void update_orphan_files(const OrphanFilesDialogState &state);
    void show_database_backup(const DatabaseBackupDialogState &state);
    void update_database_backup(const DatabaseBackupDialogState &state);
    void show_network(const NetworkDialogState &state);
    void update_network(const NetworkDialogState &state);
    void start_tick();
    void show_message(const QString &title, const QString &text);
    void notify_message(const QString &message);
    void schedule_screenshot(const QString &path);

    HydrusQueryPage *find_page(const QString &page_id) const;

    void notify(UiAction action, const QString &target = {}, const QString &payload = {});

    bool building = true;
    QString active_page;
    QString active_notebook;

protected:
    void dragEnterEvent(QDragEnterEvent *event) override;
    void dropEvent(QDropEvent *event) override;
    void closeEvent(QCloseEvent *event) override;

public:

private:
    void populate_notebook(
        HydrusNotebook *notebook,
        const Pages &pages,
        size_t &cursor,
        int depth,
        const QString &current);
    void clear_notebook(HydrusNotebook *notebook);
    void build_shortcuts();

    HydrusNotebook *root = nullptr;
    QLabel *status_media = nullptr;
    QLabel *status_bandwidth = nullptr;
    QLabel *status_idle = nullptr;
    QLabel *status_hydrus_busy = nullptr;
    QLabel *status_cpu_busy = nullptr;
    QLabel *status_db = nullptr;

    QHash<QString, HydrusQueryPage *> pages;
    QHash<QString, HydrusDownloaderPage *> downloaders;
    QHash<QString, HydrusSubscriptionsPage *> subscriptions;
    QHash<QString, HydrusDuplicatesPage *> duplicates;
    QHash<QString, HydrusImportFoldersPage *> import_folders;
    QHash<QString, HydrusExportFoldersPage *> export_folders;
    QHash<QString, HydrusNotebook *> notebooks;
    QHash<QString, HydrusViewerWindow *> viewers;
    HydrusServicesDialog *services_dialog = nullptr;
    HydrusApiPermissionsDialog *api_permissions_dialog = nullptr;
    HydrusAccountTypesDialog *account_types_dialog = nullptr;
    HydrusOrphanFilesDialog *orphan_files_dialog = nullptr;
    HydrusDatabaseBackupDialog *database_backup_dialog = nullptr;
    HydrusNetworkDialog *network_dialog = nullptr;
    HydrusDuplicatesRulesDialog *duplicates_rules_dialog = nullptr;
    HydrusDuplicateMergeOptionsDialog *merge_options_dialog = nullptr;
    HydrusDuplicateCustomActionDialog *duplicate_custom_action_dialog = nullptr;
    HydrusImportOptionsDialog *import_options_dialog = nullptr;
    HydrusSeedCacheDialog *file_seed_cache_dialog = nullptr;
    HydrusSeedCacheDialog *gallery_seed_log_dialog = nullptr;
    HydrusLocalImportReviewDialog *local_import_review_dialog = nullptr;
    HydrusLocalImportTaggingDialog *local_import_tagging_dialog = nullptr;
    HydrusRepairFileLocationsDialog *repair_file_locations_dialog = nullptr;
    HydrusSubscriptionEditorDialog *subscription_editor_dialog = nullptr;
    HydrusSubscriptionsDialog *subscriptions_dialog = nullptr;
    HydrusImportFoldersDialog *import_folders_dialog = nullptr;
    HydrusExportFoldersDialog *export_folders_dialog = nullptr;
    HydrusDuplicateReviewDialog *duplicate_review_dialog = nullptr;
    HydrusTagRelationsDialog *tag_relations_dialog = nullptr;
    QTimer *tick_timer = nullptr;
    QSystemTrayIcon *tray = nullptr;
    QStringList tag_suggestions;
};
