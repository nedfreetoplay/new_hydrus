#pragma once

#include "rust/cxx.h"

#include <QApplication>
#include <QCloseEvent>
#include <QContextMenuEvent>
#include <QCheckBox>
#include <QComboBox>
#include <QDialog>
#include <QFrame>
#include <QGraphicsPixmapItem>
#include <QGraphicsScene>
#include <QGraphicsView>
#include <QHash>
#include <QJsonArray>
#include <QJsonObject>
#include <QLabel>
#include <QLineEdit>
#include <QListWidget>
#include <QMainWindow>
#include <QPlainTextEdit>
#include <QProgressBar>
#include <QPushButton>
#include <QResizeEvent>
#include <QScrollArea>
#include <QSpinBox>
#include <QSplashScreen>
#include <QString>
#include <QStringList>
#include <QTabWidget>
#include <QTableWidget>
#include <QTimer>
#include <QToolButton>
#include <QWheelEvent>
#include <QWidget>
#include <memory>
#include "screens.hpp"

class HydrusMainWindow;
class HydrusNotebook;
class HydrusQueryPage;
class HydrusThumbnailGrid;
class HydrusViewerWindow;
class HydrusDownloaderPage;
class HydrusSubscriptionsPage;
class HydrusSubscriptionsDialog;
class HydrusImportFoldersDialog;
class HydrusExportFoldersDialog;
class HydrusImportOptionsDialog;
class HydrusDuplicatesPage;
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
class HydrusTagRelationsDialog;
class HydrusSeedCacheDialog;
class HydrusLocalImportReviewDialog;
class HydrusLocalImportTaggingDialog;
class HydrusRepairFileLocationsDialog;
class HydrusApiPermissionsDialog;
class HydrusAccountTypesDialog;
class HydrusOrphanFilesDialog;
class HydrusDatabaseBackupDialog;
class QTabBar;
class QCompleter;
class QVBoxLayout;
class QDragEnterEvent;
class QDropEvent;
class QEvent;
class HydrusTabBar;
class QMovie;
class QSystemTrayIcon;



// Import folders manager page.
class HydrusImportFoldersPage : public QWidget {
public:
    HydrusImportFoldersPage(HydrusMainWindow *main, const QString &page_id, const QJsonObject &state);

    const QString &page_id() const;
    void update_state(const QJsonObject &state);

private:
    void with_selected(const QString &action);

    HydrusMainWindow *main = nullptr;
    QString id;
    QTableWidget *table = nullptr;
    QLabel *status = nullptr;
};

// Export folders manager page.
class HydrusExportFoldersPage : public QWidget {
public:
    HydrusExportFoldersPage(HydrusMainWindow *main, const QString &page_id, const QJsonObject &state);

    const QString &page_id() const;
    void update_state(const QJsonObject &state);

private:
    void with_selected(const QString &action);

    HydrusMainWindow *main = nullptr;
    QString id;
    QTableWidget *table = nullptr;
    QLabel *status = nullptr;
};




// Edit one subscription's site, query, filter and timing.
class HydrusSubscriptionEditorDialog : public QDialog {
public:
    HydrusSubscriptionEditorDialog(HydrusMainWindow *main, const QJsonObject &state, QWidget *parent = nullptr);

    void set_state(const QJsonObject &state);

private:
    void save();

    HydrusMainWindow *main = nullptr;
    QJsonObject data;
    QString page_id;
    QLineEdit *name = nullptr;
    QLineEdit *url = nullptr;
    QLineEdit *query = nullptr;
    QComboBox *site = nullptr;
    QLineEdit *filter = nullptr;
    QSpinBox *period = nullptr;
    QCheckBox *paused = nullptr;
};

// Python's network -> subscriptions... manager (EditSubscriptionsPanel):
// one table of subscriptions and the batch action rows underneath.
class HydrusSubscriptionsDialog : public QDialog {
public:
    HydrusSubscriptionsDialog(HydrusMainWindow *main, const QJsonObject &state, QWidget *parent = nullptr);

    void set_state(const QJsonObject &state);

private:
    void add_subscription();
    void with_selected(const QString &action);
    void import_options_action(const QString &action);
    void refresh_buttons();
    QStringList selected_names() const;

    HydrusMainWindow *main = nullptr;
    QJsonObject data;
    QTableWidget *table = nullptr;
    QLabel *warning = nullptr;
    QList<QWidget *> selection_buttons;
};

// Python's file -> manage import folders... dialog (EditImportFoldersPanel).
class HydrusImportFoldersDialog : public QDialog {
public:
    HydrusImportFoldersDialog(HydrusMainWindow *main, const QJsonObject &state, QWidget *parent = nullptr);

    void set_state(const QJsonObject &state);

private:
    void with_selected(const QString &action);
    void refresh_buttons();

    HydrusMainWindow *main = nullptr;
    QJsonObject data;
    QTableWidget *table = nullptr;
    QPushButton *edit_button = nullptr;
    QPushButton *delete_button = nullptr;
};

// Python's file -> manage export folders... dialog (EditExportFoldersPanel).
class HydrusExportFoldersDialog : public QDialog {
public:
    HydrusExportFoldersDialog(HydrusMainWindow *main, const QJsonObject &state, QWidget *parent = nullptr);

    void set_state(const QJsonObject &state);

private:
    void with_selected(const QString &action);
    void refresh_buttons();

    HydrusMainWindow *main = nullptr;
    QJsonObject data;
    QTableWidget *table = nullptr;
    QPushButton *edit_button = nullptr;
    QPushButton *delete_button = nullptr;
};



// Subscription manager page.
class HydrusSubscriptionsPage : public QWidget {
public:
    HydrusSubscriptionsPage(HydrusMainWindow *main, const QString &page_id, const QJsonObject &state);

    const QString &page_id() const;
    void update_state(const QJsonObject &state);

private:
    void add_subscription();
    void with_selected(const QString &action);

    HydrusMainWindow *main = nullptr;
    QString id;
    QTableWidget *table = nullptr;
    QLabel *status = nullptr;
    QJsonArray sites;
    QPushButton *import_options_button = nullptr;
};







class HydrusMainWindow : public QMainWindow {
public:
    explicit HydrusMainWindow(QWidget *parent = nullptr);

    void build_shell();
    void set_status(const QJsonObject &status);
    void set_menus(const QJsonArray &menus);
    void build_pages(const QJsonObject &pages);
    void update_page(const QString &page_id, const QJsonObject &state);
    void update_page_progress(const QString &page_id, const QJsonObject &state);
    void update_downloader(const QString &page_id, const QJsonObject &state);
    void update_subscriptions(const QString &page_id, const QJsonObject &state);
    void show_subscriptions_dialog(const QJsonObject &state);
    void update_subscriptions_dialog(const QJsonObject &state);
    void show_subscription_editor(const QJsonObject &state);
    void update_duplicates(const QString &page_id, const QJsonObject &state);
    void update_import_folders(const QString &page_id, const QJsonObject &state);
    void show_import_folders_dialog(const QJsonObject &state);
    void update_import_folders_dialog(const QJsonObject &state);
    void update_export_folders(const QString &page_id, const QJsonObject &state);
    void show_export_folders_dialog(const QJsonObject &state);
    void update_export_folders_dialog(const QJsonObject &state);
    void set_selection(
        const QString &page_id,
        const QString &hash,
        const QStringList &tags,
        const QString &preview_path,
        const QJsonObject &metadata);
    void set_tag_suggestions(const QStringList &suggestions);
    const QStringList &suggestions() const;
    void show_autocomplete(const QJsonObject &state);
    void show_page_chooser(const QString &notebook_id);
    void prompt_url();
    void open_viewer(const QJsonObject &state);
    void update_viewer(const QString &viewer_id, const QJsonObject &state);
    void close_viewer(const QString &viewer_id);
    void show_tag_dialog(const QJsonObject &state);
    void show_notifications(const QJsonObject &state);
    void show_viewing_stats(const QJsonObject &state);
    void show_about(const QJsonObject &state);
    void show_notes(const QJsonObject &state);
    void show_duplicates_rules(const QJsonObject &state);
    void show_duplicate_merge_options(const QJsonObject &state);
    void show_duplicate_custom_action(const QJsonObject &state);
    void show_import_options(const QJsonObject &state);
    void show_file_seed_cache(const QJsonObject &state);
    void show_gallery_seed_log(const QJsonObject &state);
    void update_auto_resolution_preview(const QJsonObject &state);
    void show_duplicate_review(const QJsonObject &state);
    void update_duplicate_review(const QJsonObject &state);
    void show_tag_relations(const QJsonObject &state);
    void show_import_folder(const QJsonObject &state);
    void show_local_import_review(const QJsonObject &state);
    void show_local_import_tagging(const QJsonObject &state);
    void close_local_import_review();
    void show_repair_file_locations(const QJsonObject &state);
    void close_repair_file_locations();
    void show_export_folder(const QJsonObject &state);
    void show_export_review(const QJsonObject &state);
    void show_options(const QJsonObject &state);
    void show_services(const QJsonObject &state);
    void update_services(const QJsonObject &state);
    void show_tag_filter(const QJsonObject &state);
    void show_api_permissions(const QJsonObject &state);
    void update_api_permissions(const QJsonObject &state);
    void show_account_types(const QJsonObject &state);
    void update_account_types(const QJsonObject &state);
    void show_orphan_files(const QJsonObject &state);
    void update_orphan_files(const QJsonObject &state);
    void show_database_backup(const QJsonObject &state);
    void update_database_backup(const QJsonObject &state);
    void show_network(const QJsonObject &state);
    void update_network(const QJsonObject &state);
    void start_tick();
    void show_message(const QString &title, const QString &text);
    void notify_message(const QString &message);
    void schedule_screenshot(const QString &path);

    HydrusQueryPage *find_page(const QString &page_id) const;

    void notify(const QString &action, const QString &target = {}, const QString &payload = {});

    bool building = true;
    QString active_page;
    QString active_notebook;

protected:
    void dragEnterEvent(QDragEnterEvent *event) override;
    void dropEvent(QDropEvent *event) override;
    void closeEvent(QCloseEvent *event) override;

public:

private:
    void populate_notebook(HydrusNotebook *notebook, const QJsonObject &object);
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

std::unique_ptr<QApplication> new_application();
int exec_application(QApplication &application);
void quit_application(QApplication &application);
void qapp_process_events();

std::unique_ptr<QSplashScreen> new_hydrus_splash();
void hydrus_splash_show(QSplashScreen &splash);
void hydrus_splash_message(QSplashScreen &splash, rust::Str message);
void hydrus_splash_finish(QSplashScreen &splash);
void hydrus_splash_finish_later(QSplashScreen &splash, int delay_ms);
void hydrus_splash_screenshot(QSplashScreen &splash, rust::Str path, int delay_ms);

std::unique_ptr<HydrusMainWindow> new_hydrus_main_window();
void build_hydrus_shell(HydrusMainWindow &window);
void show_hydrus_main_window(HydrusMainWindow &window);

void hydrus_set_title(HydrusMainWindow &window, rust::Str title);
void hydrus_resize_window(HydrusMainWindow &window, int width, int height);
void hydrus_set_menus(HydrusMainWindow &window, rust::Str json);
void hydrus_set_tag_suggestions(HydrusMainWindow &window, rust::Str json);
void hydrus_show_autocomplete(HydrusMainWindow &window, rust::Str json);
void hydrus_set_status(HydrusMainWindow &window, rust::Str json);
void hydrus_build_pages(HydrusMainWindow &window, rust::Str json);
void hydrus_update_page(HydrusMainWindow &window, rust::Str page_id, rust::Str json);
void hydrus_update_page_progress(
    HydrusMainWindow &window,
    rust::Str page_id,
    rust::Str json);
void hydrus_update_downloader(HydrusMainWindow &window, rust::Str page_id, rust::Str json);
void hydrus_update_subscriptions(HydrusMainWindow &window, rust::Str page_id, rust::Str json);
void hydrus_show_subscriptions_dialog(HydrusMainWindow &window, rust::Str json);
void hydrus_update_subscriptions_dialog(HydrusMainWindow &window, rust::Str json);
void hydrus_show_subscription_editor(HydrusMainWindow &window, rust::Str json);
void hydrus_update_duplicates(HydrusMainWindow &window, rust::Str page_id, rust::Str json);
void hydrus_update_import_folders(HydrusMainWindow &window, rust::Str page_id, rust::Str json);
void hydrus_show_import_folders_dialog(HydrusMainWindow &window, rust::Str json);
void hydrus_update_import_folders_dialog(HydrusMainWindow &window, rust::Str json);
void hydrus_update_export_folders(HydrusMainWindow &window, rust::Str page_id, rust::Str json);
void hydrus_show_export_folders_dialog(HydrusMainWindow &window, rust::Str json);
void hydrus_update_export_folders_dialog(HydrusMainWindow &window, rust::Str json);
void hydrus_show_duplicates_rules(HydrusMainWindow &window, rust::Str json);
void hydrus_show_duplicate_merge_options(HydrusMainWindow &window, rust::Str json);
void hydrus_show_duplicate_custom_action(HydrusMainWindow &window, rust::Str json);
void hydrus_show_import_options(HydrusMainWindow &window, rust::Str json);
void hydrus_show_file_seed_cache(HydrusMainWindow &window, rust::Str json);
void hydrus_show_gallery_seed_log(HydrusMainWindow &window, rust::Str json);
void hydrus_update_auto_resolution_preview(HydrusMainWindow &window, rust::Str json);
void hydrus_show_duplicate_review(HydrusMainWindow &window, rust::Str json);
void hydrus_update_duplicate_review(HydrusMainWindow &window, rust::Str json);
void hydrus_show_tag_relations(HydrusMainWindow &window, rust::Str json);
void hydrus_show_import_folder(HydrusMainWindow &window, rust::Str json);
void hydrus_show_local_import_review(HydrusMainWindow &window, rust::Str json);
void hydrus_show_local_import_tagging(HydrusMainWindow &window, rust::Str json);
void hydrus_close_local_import_review(HydrusMainWindow &window);
void hydrus_show_repair_file_locations(HydrusMainWindow &window, rust::Str json);
void hydrus_close_repair_file_locations(HydrusMainWindow &window);
void hydrus_show_export_folder(HydrusMainWindow &window, rust::Str json);
void hydrus_show_export_review(HydrusMainWindow &window, rust::Str json);
void hydrus_show_notes(HydrusMainWindow &window, rust::Str json);
void hydrus_set_selection(
    HydrusMainWindow &window,
    rust::Str page_id,
    rust::Str hash,
    rust::Str tags_json,
    rust::Str preview_path,
    rust::Str metadata_json);
void hydrus_show_page_chooser(HydrusMainWindow &window, rust::Str notebook_id);
void hydrus_show_page_manager(HydrusMainWindow &window, rust::Str json);
void hydrus_show_command_palette(HydrusMainWindow &window);
void hydrus_prompt_text(
    HydrusMainWindow &window,
    rust::Str action,
    rust::Str title,
    rust::Str label,
    rust::Str initial);
void hydrus_confirm_action(
    HydrusMainWindow &window,
    rust::Str action,
    rust::Str target,
    rust::Str title,
    rust::Str text);
void hydrus_show_or_dialog(
    HydrusMainWindow &window,
    rust::Str page_id,
    rust::Str initial);
void hydrus_pick_files(HydrusMainWindow &window);
void hydrus_prompt_url(HydrusMainWindow &window);
void hydrus_open_viewer(HydrusMainWindow &window, rust::Str json);
void hydrus_update_viewer(HydrusMainWindow &window, rust::Str viewer_id, rust::Str json);
void hydrus_close_viewer(HydrusMainWindow &window, rust::Str viewer_id);
void hydrus_show_tag_dialog(HydrusMainWindow &window, rust::Str json);
void hydrus_show_notifications(HydrusMainWindow &window, rust::Str json);
void hydrus_show_viewing_stats(HydrusMainWindow &window, rust::Str json);
void hydrus_show_about(HydrusMainWindow &window, rust::Str json);
void hydrus_export_downloader(HydrusMainWindow &window, rust::Str page_id);
void hydrus_import_downloader(HydrusMainWindow &window, rust::Str target);
void hydrus_show_manage_tags(HydrusMainWindow &window, rust::Str json);
void hydrus_show_timestamps(HydrusMainWindow &window, rust::Str json);
void hydrus_show_options(HydrusMainWindow &window, rust::Str json);
void hydrus_show_services(HydrusMainWindow &window, rust::Str json);
void hydrus_update_services(HydrusMainWindow &window, rust::Str json);
void hydrus_show_tag_filter(HydrusMainWindow &window, rust::Str json);
void hydrus_show_api_permissions(HydrusMainWindow &window, rust::Str json);
void hydrus_update_api_permissions(HydrusMainWindow &window, rust::Str json);
void hydrus_show_account_types(HydrusMainWindow &window, rust::Str json);
void hydrus_update_account_types(HydrusMainWindow &window, rust::Str json);
void hydrus_show_orphan_files(HydrusMainWindow &window, rust::Str json);
void hydrus_update_orphan_files(HydrusMainWindow &window, rust::Str json);
void hydrus_show_database_backup(HydrusMainWindow &window, rust::Str json);
void hydrus_update_database_backup(HydrusMainWindow &window, rust::Str json);
void hydrus_show_network(HydrusMainWindow &window, rust::Str json);
void hydrus_update_network(HydrusMainWindow &window, rust::Str json);
void hydrus_start_tick(HydrusMainWindow &window);
void hydrus_defer_action(
    HydrusMainWindow &window,
    rust::Str action,
    rust::Str target,
    rust::Str payload,
    int delay_ms);
void hydrus_show_message(HydrusMainWindow &window, rust::Str title, rust::Str text);
void hydrus_notify_message(HydrusMainWindow &window, rust::Str message);
void hydrus_set_clipboard(HydrusMainWindow &window, rust::Str text);
void hydrus_open_path(rust::Str path);
void hydrus_open_url(rust::Str url);
void hydrus_schedule_screenshot(HydrusMainWindow &window, rust::Str path);
void qapp_set_application_name(rust::Str name);
void qapp_set_application_display_name(rust::Str name);
void qapp_set_organization_name(rust::Str name);
void qapp_set_style(rust::Str name);
void qapp_set_stylesheet(rust::Str stylesheet);
void qapp_quit();

// Implemented in Rust (see bridge.rs / gui::app).
void app_action(
    HydrusMainWindow &window,
    rust::Str action,
    rust::Str target,
    rust::Str payload) noexcept;

void app_tick(HydrusMainWindow &window) noexcept;

// Implemented in Rust: the Python VetoException text for an invalid export
// filename phrase, or an empty string when the phrase parses.
rust::String hydrus_validate_export_phrase(rust::Str phrase) noexcept;

// Implemented in Rust: resolves the vendored static/ asset directory.
rust::String hydrus_static_dir() noexcept;

