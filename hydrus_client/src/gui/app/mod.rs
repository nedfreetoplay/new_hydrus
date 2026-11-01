//! Application state and event handling for the real client GUI.

use std::cell::RefCell;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;

use hydrus_core::hydrus_constants::{ContentStatus, TimestampType, dirs};
use hydrus_core::hydrus_time;
use serde_json::Value;

use crate::controller::Controller;
use crate::db::{files_storage, files_timestamps, files_viewing_stats, mappings, master_tags};
use crate::gui::qt::api::ffi;
use crate::gui::qt::panels;

pub mod api_permissions;
pub mod auto_resolution;
pub mod backup;
pub mod commands;
pub mod download;
pub mod downloader_export;
pub mod duplicates;
pub mod export;
pub mod favourites;
pub mod file_locations;
pub mod import;
pub mod import_folders;
pub mod import_options;
pub mod local_import;
pub mod metadata;
pub mod model;
pub mod notifications;
pub mod options;
pub mod orphans;
pub mod query;
pub mod render;
pub mod search;
pub mod search_history;
pub mod seeds;
pub mod services;
pub mod session;
pub mod sessions;
pub mod static_assets;
pub mod tag_filter;
pub mod tag_relations;
pub mod worker;

pub use commands::UiCommand;
use model::{
    AppState, DownloaderKind, DownloaderPage, DuplicatesPage, Notebook, PageNode, PlaceholderPage,
    QueryPage, SubscriptionsPage, ViewerContext,
};

thread_local! {
    static STATE: RefCell<Option<AppState>> = const { RefCell::new(None) };
}

pub fn install(controller: Arc<Controller>) {
    let state = AppState::new(controller);
    STATE.with(|slot| *slot.borrow_mut() = Some(state));

    with_state_mut(|state| {
        state.refresh_all();

        match state.controller.refresh_api() {
            Ok(status) if status != "api disabled" => state.message = status,
            Ok(_) => {}
            Err(error) => state.message = format!("api failed: {error}"),
        }
    });
}

pub fn initial_commands() -> Vec<UiCommand> {
    with_state_mut(|state| {
        state.refresh_all();

        let tags = serde_json::to_string(&query::all_tags(&state.controller))
            .unwrap_or_else(|_| "[]".to_string());

        let mut commands = vec![
            UiCommand::Menus(render::menus(state)),
            UiCommand::TagSuggestions(tags),
            UiCommand::BuildPages(render::pages(state)),
            UiCommand::Status(render::status(state)),
        ];

        if state
            .controller
            .files_manager()
            .subfolders()
            .iter()
            .any(|subfolder| !subfolder.path_exists(false))
        {
            commands.extend(file_locations::open(state));
        }

        commands
    })
}

pub fn save_session() {
    let _ = with_state(|state| session::save(state));
}

/// Development helper: imports files before the first render. Used by the
/// screenshot smoke test and useful for manual checks.
pub fn import_for_testing(paths: &[String]) {
    with_state_mut(|state| {
        let paths: Vec<PathBuf> = paths.iter().map(PathBuf::from).collect();
        let summary = import::import_paths(&state.controller, &paths);
        state.message = summary.message();
        state.refresh_all();
    });
}

/// Development helper: attaches a source URL to the first file of the active page.
pub fn add_url_for_testing(url: &str) {
    with_state_mut(|state| {
        let Some(page) = state.active_query() else {
            return;
        };
        let Some(hash) = page.hashes.first().cloned() else {
            return;
        };
        let Some(hash_id) = query::hash_id_for(&state.controller, &hash) else {
            return;
        };

        let _ = crate::db::url_map::add_url(state.controller.db().db(), hash_id, url);
        state.message = format!("added url {url}");
    });
}

/// Development helper: leaves file pages in their searching state so the
/// loading panel can be checked in screenshots.
pub fn show_page_loading_for_testing() {
    with_state_mut(|state| {
        for id in state.query_ids() {
            if let Some(page) = state.find_query_mut(&id) {
                page.loading = true;
            }
        }
    });
}

/// Development helper: sets the thumbnail layout options before the first page
/// is built so the results panel can be checked in screenshots.
pub fn set_thumbnail_options_for_testing(scale_percent: i64, masonry: bool, horizontal: bool) {
    with_state_mut(|state| {
        let mut options = state.controller.options();
        options.set_integer("thumbnail_scale_percent", scale_percent.clamp(25, 400));
        options.set_boolean("thumbnail_layout_masonry", masonry);
        options.set_boolean("thumbnail_scroll_horizontal", horizontal);
        state.controller.replace_options(options);
    });
}

/// Development helper: records views on the first file of the active page.
pub fn add_viewing_stats_for_testing(views: i64) {
    with_state_mut(|state| {
        let Some(page) = state.active_query() else {
            return;
        };
        let Some(hash) = page.hashes.first().cloned() else {
            return;
        };
        let Some(hash_id) = query::hash_id_for(&state.controller, &hash) else {
            return;
        };

        let _ = files_viewing_stats::add_viewing_stats(
            state.controller.db().db(),
            hash_id,
            0,
            views,
            1500,
            Some(hydrus_time::get_now_ms()),
        );
        state.message = format!("recorded {views} views");
    });
}

/// Development helper: applies several tags to the first file on the active page.
pub fn add_tags_for_testing(tags: &[String]) {
    for tag in tags {
        add_tag_for_testing(tag);
    }
}

/// Development helper: changes tag presentation options before the first page
/// is rendered.
pub fn set_tag_display_options_for_testing(
    sort: &str,
    show_namespaces: bool,
    replace_underscores: bool,
) {
    with_state_mut(|state| {
        let mut options = state.controller.options();
        options.set_string("tag_display_sort", sort);
        options.set_boolean("show_namespaces", show_namespaces);
        options.set_boolean("replace_tag_underscores_with_spaces", replace_underscores);
        state.controller.replace_options(options);
    });
}

/// Development helper: applies a tag to the first file on the active page.
pub fn add_tag_for_testing(tag: &str) {
    with_state_mut(|state| {
        let Some(page) = state.active_query() else {
            return;
        };
        let Some(hash) = page.hashes.first().cloned() else {
            return;
        };

        let Some(hash_id) = query::hash_id_for(&state.controller, &hash) else {
            return;
        };
        let Some(service_id) = query::tag_service_id(&state.controller) else {
            return;
        };

        if let Ok(tag_id) = master_tags::get_tag_id(state.controller.db().db(), tag) {
            let _ = mappings::add_mappings(
                state.controller.db().db(),
                service_id,
                ContentStatus::Current,
                hash_id,
                &[tag_id],
            );
            state.message = format!("added tag '{tag}'");
        }
    });
}

pub fn set_query_for_testing(query: &str) {
    with_state_mut(|state| {
        if let Some(page) = state.active_query() {
            let id = page.id.clone();

            if let Some(page) = state.find_query_mut(&id) {
                page.query = query.to_string();
            }
        }
    });
}

/// Development helper: runs `action|target|payload` steps separated by `;`.
pub fn run_script_for_testing(script: &str) -> Vec<UiCommand> {
    with_state_mut(|state| {
        let mut commands = Vec::new();

        for step in script.split(';') {
            let mut parts = step.split('|');
            let action = parts.next().unwrap_or_default().trim();
            let target = parts.next().unwrap_or_default().trim();
            let payload = parts.next().unwrap_or_default().trim();

            if !action.is_empty() {
                commands.extend(state.dispatch(crate::gui::app::action_from_str(action), target, payload));
            }
        }

        commands
    })
}

pub fn focus_first_for_testing() -> Vec<UiCommand> {
    with_state_mut(|state| {
        let Some(page) = state.active_query() else {
            return vec![];
        };
        let (page_id, hash) = (page.id.clone(), page.hashes.first().cloned());

        match hash {
            Some(hash) => state.focus(&page_id, &query::hex(&hash)),
            None => vec![],
        }
    })
}

pub fn open_first_for_testing() -> Vec<UiCommand> {
    with_state_mut(|state| {
        let Some(page) = state.active_query() else {
            return vec![];
        };
        let (page_id, hash) = (page.id.clone(), page.hashes.first().cloned());

        match hash {
            Some(hash) => state.open_viewer(&page_id, &query::hex(&hash)),
            None => vec![],
        }
    })
}

pub fn handle_action(
    mut window: Pin<&mut ffi::HydrusMainWindow>,
    action: ffi::UiAction,
    target: &str,
    payload: &str,
) {
    let commands = with_state_mut(|state| state.dispatch(action, target, payload));

    record_notifications(&commands);

    for command in commands {
        panels::execute(window.as_mut(), command);
    }
}

impl ffi::UiAction {
    pub fn as_str(self) -> &'static str {
        match self {
        ffi::UiAction::None => "",
        ffi::UiAction::A => "a",
        ffi::UiAction::About => "about",
        ffi::UiAction::AccountType => "account_type",
        ffi::UiAction::AccountTypeDelete => "account_type_delete",
        ffi::UiAction::AccountTypeSave => "account_type_save",
        ffi::UiAction::AccountTypesOpen => "account_types_open",
        ffi::UiAction::AddHashesToPage => "add_hashes_to_page",
        ffi::UiAction::AdminAccountAction => "admin_account_action",
        ffi::UiAction::AdminAddService => "admin_add_service",
        ffi::UiAction::AdminConnect => "admin_connect",
        ffi::UiAction::AdminEditService => "admin_edit_service",
        ffi::UiAction::AdminRemoveService => "admin_remove_service",
        ffi::UiAction::AdminServiceAction => "admin_service_action",
        ffi::UiAction::All => "all",
        ffi::UiAction::Alternate => "alternate",
        ffi::UiAction::Alternates => "alternates",
        ffi::UiAction::AlwaysBoth => "always_both",
        ffi::UiAction::Api => "api",
        ffi::UiAction::ApiPermissions => "api_permissions",
        ffi::UiAction::ApiPermissionsDelete => "api_permissions_delete",
        ffi::UiAction::ApiPermissionsSave => "api_permissions_save",
        ffi::UiAction::ApplySearchHistory => "apply_search_history",
        ffi::UiAction::ApplyTags => "apply_tags",
        ffi::UiAction::ApplyTagsSelection => "apply_tags_selection",
        ffi::UiAction::ApplyTagsSelectionApply => "apply_tags_selection_apply",
        ffi::UiAction::ArchiveActive => "archive_active",
        ffi::UiAction::ArchiveFiles => "archive_files",
        ffi::UiAction::ArchiveHashes => "archive_hashes",
        ffi::UiAction::ArchivedDays => "archived_days",
        ffi::UiAction::Auto => "auto",
        ffi::UiAction::Autocomplete => "autocomplete",
        ffi::UiAction::Avi => "avi",
        ffi::UiAction::Avif => "avif",
        ffi::UiAction::B => "b",
        ffi::UiAction::Ban => "ban",
        ffi::UiAction::Better => "better",
        ffi::UiAction::Blacklist => "blacklist",
        ffi::UiAction::Bmp => "bmp",
        ffi::UiAction::Bool => "bool",
        ffi::UiAction::BothFiles => "both_files",
        ffi::UiAction::Bytes => "bytes",
        ffi::UiAction::Clear => "clear",
        ffi::UiAction::ClearFalsePositives => "clear_false_positives",
        ffi::UiAction::ClearFinished => "clear_finished",
        ffi::UiAction::ClearInternalFalsePositives => "clear_internal_false_positives",
        ffi::UiAction::ClientApi => "client_api",
        ffi::UiAction::CloseOtherPages => "close_other_pages",
        ffi::UiAction::ClosePage => "close_page",
        ffi::UiAction::CommandPalette => "command_palette",
        ffi::UiAction::CopyTags => "copy_tags",
        ffi::UiAction::CreatePage => "create_page",
        ffi::UiAction::Css => "css",
        ffi::UiAction::Danbooru => "danbooru",
        ffi::UiAction::Data => "data",
        ffi::UiAction::DbAnalyze => "db_analyze",
        ffi::UiAction::DbBackupOpen => "db_backup_open",
        ffi::UiAction::DbBackupRun => "db_backup_run",
        ffi::UiAction::DbBackupSetPath => "db_backup_set_path",
        ffi::UiAction::DbCheck => "db_check",
        ffi::UiAction::DbDeferredDeletes => "db_deferred_deletes",
        ffi::UiAction::DbFileMaintenance => "db_file_maintenance",
        ffi::UiAction::DbFixArchiveTimes => "db_fix_archive_times",
        ffi::UiAction::DbInfo => "db_info",
        ffi::UiAction::DbRunFileMaintenance => "db_run_file_maintenance",
        ffi::UiAction::DbStats => "db_stats",
        ffi::UiAction::DbVacuum => "db_vacuum",
        ffi::UiAction::DbVacuumInfo => "db_vacuum_info",
        ffi::UiAction::Delete => "delete",
        ffi::UiAction::DeleteAllContent => "delete_all_content",
        ffi::UiAction::DeleteBoth => "delete_both",
        ffi::UiAction::DeleteFiles => "delete_files",
        ffi::UiAction::DeleteHashes => "delete_hashes",
        ffi::UiAction::Desired => "desired",
        ffi::UiAction::DifferentSearches => "different_searches",
        ffi::UiAction::Dislike => "dislike",
        ffi::UiAction::DissolveAlternateGroup => "dissolve_alternate_group",
        ffi::UiAction::DissolveDuplicateGroup => "dissolve_duplicate_group",
        ffi::UiAction::Docs => "docs",
        ffi::UiAction::Domain => "domain",
        ffi::UiAction::Done => "done",
        ffi::UiAction::DownloadUrl => "download_url",
        ffi::UiAction::Downloader => "downloader",
        ffi::UiAction::DownloaderClear => "downloader_clear",
        ffi::UiAction::DownloaderExport => "downloader_export",
        ffi::UiAction::DownloaderExportTo => "downloader_export_to",
        ffi::UiAction::DownloaderImport => "downloader_import",
        ffi::UiAction::DownloaderImportFrom => "downloader_import_from",
        ffi::UiAction::DownloaderImportOptions => "downloader_import_options",
        ffi::UiAction::DownloaderImportOptionsSave => "downloader_import_options_save",
        ffi::UiAction::DownloaderJobsSet => "downloader_jobs_set",
        ffi::UiAction::DownloaderOpenResults => "downloader_open_results",
        ffi::UiAction::DownloaderPage => "downloader_page",
        ffi::UiAction::DownloaderPause => "downloader_pause",
        ffi::UiAction::DownloaderRetry => "downloader_retry",
        ffi::UiAction::DownloaderSetFormula => "downloader_set_formula",
        ffi::UiAction::DownloaderSetSite => "downloader_set_site",
        ffi::UiAction::DownloaderStart => "downloader_start",
        ffi::UiAction::Downloads => "downloads",
        ffi::UiAction::DuplicatePage => "duplicate_page",
        ffi::UiAction::Duplicates => "duplicates",
        ffi::UiAction::DuplicatesAlternates => "duplicates_alternates",
        ffi::UiAction::DuplicatesAutoToggle => "duplicates_auto_toggle",
        ffi::UiAction::DuplicatesBatchSize => "duplicates_batch_size",
        ffi::UiAction::DuplicatesCustom => "duplicates_custom",
        ffi::UiAction::DuplicatesCustomAction => "duplicates_custom_action",
        ffi::UiAction::DuplicatesDeleteBoth => "duplicates_delete_both",
        ffi::UiAction::DuplicatesFalse => "duplicates_false",
        ffi::UiAction::DuplicatesGroupAction => "duplicates_group_action",
        ffi::UiAction::DuplicatesKeepA => "duplicates_keep_a",
        ffi::UiAction::DuplicatesKeepAKeep => "duplicates_keep_a_keep",
        ffi::UiAction::DuplicatesKeepB => "duplicates_keep_b",
        ffi::UiAction::DuplicatesKeepBKeep => "duplicates_keep_b_keep",
        ffi::UiAction::DuplicatesMergeOptions => "duplicates_merge_options",
        ffi::UiAction::DuplicatesMergeOptionsDialog => "duplicates_merge_options_dialog",
        ffi::UiAction::DuplicatesNextBatch => "duplicates_next_batch",
        ffi::UiAction::DuplicatesRefresh => "duplicates_refresh",
        ffi::UiAction::DuplicatesReview => "duplicates_review",
        ffi::UiAction::DuplicatesReviewApply => "duplicates_review_apply",
        ffi::UiAction::DuplicatesRuleAdd => "duplicates_rule_add",
        ffi::UiAction::DuplicatesRulePreview => "duplicates_rule_preview",
        ffi::UiAction::DuplicatesRuleRemove => "duplicates_rule_remove",
        ffi::UiAction::DuplicatesRuleSave => "duplicates_rule_save",
        ffi::UiAction::DuplicatesRules => "duplicates_rules",
        ffi::UiAction::DuplicatesSame => "duplicates_same",
        ffi::UiAction::DuplicatesSearchContext => "duplicates_search_context",
        ffi::UiAction::DuplicatesShowPair => "duplicates_show_pair",
        ffi::UiAction::DuplicatesSkip => "duplicates_skip",
        ffi::UiAction::Duration => "duration",
        ffi::UiAction::E621 => "e621",
        ffi::UiAction::EditTimestamps => "edit_timestamps",
        ffi::UiAction::Either => "either",
        ffi::UiAction::Eot => "eot",
        ffi::UiAction::Error => "error",
        ffi::UiAction::ExactMatch => "exact_match",
        ffi::UiAction::Excluded => "excluded",
        ffi::UiAction::Exit => "exit",
        ffi::UiAction::Expires => "expires",
        ffi::UiAction::ExportFolderEdit => "export_folder_edit",
        ffi::UiAction::ExportFolderRemove => "export_folder_remove",
        ffi::UiAction::ExportFolderReveal => "export_folder_reveal",
        ffi::UiAction::ExportFolderReview => "export_folder_review",
        ffi::UiAction::ExportFolderRun => "export_folder_run",
        ffi::UiAction::ExportFolderRunAll => "export_folder_run_all",
        ffi::UiAction::ExportFolderSave => "export_folder_save",
        ffi::UiAction::ExportFolderTestExamples => "export_folder_test_examples",
        ffi::UiAction::ExportFolderToggle => "export_folder_toggle",
        ffi::UiAction::ExportFolders => "export_folders",
        ffi::UiAction::Exporting => "exporting",
        ffi::UiAction::ExternalPrograms => "external_programs",
        ffi::UiAction::FalsePositive => "false_positive",
        ffi::UiAction::FavouriteApply => "favourite_apply",
        ffi::UiAction::FavouriteDelete => "favourite_delete",
        ffi::UiAction::FavouriteDeleteConfirmed => "favourite_delete_confirmed",
        ffi::UiAction::FavouriteSave => "favourite_save",
        ffi::UiAction::FavouriteSaveConfirmed => "favourite_save_confirmed",
        ffi::UiAction::FewerTags => "fewer_tags",
        ffi::UiAction::File => "file",
        ffi::UiAction::FileFiltering => "file_filtering",
        ffi::UiAction::FileId => "file_id",
        ffi::UiAction::FileSeedCache => "file_seed_cache",
        ffi::UiAction::FileSeedCacheAction => "file_seed_cache_action",
        ffi::UiAction::Files => "files",
        ffi::UiAction::Flac => "flac",
        ffi::UiAction::FocusHash => "focus_hash",
        ffi::UiAction::Frames => "frames",
        ffi::UiAction::Gallery => "gallery",
        ffi::UiAction::GallerySeedLog => "gallery_seed_log",
        ffi::UiAction::GallerySeedLogAction => "gallery_seed_log_action",
        ffi::UiAction::Gelbooru => "gelbooru",
        ffi::UiAction::General => "general",
        ffi::UiAction::Generic => "generic",
        ffi::UiAction::Gif => "gif",
        ffi::UiAction::Global => "global",
        ffi::UiAction::HasAudio => "has_audio",
        ffi::UiAction::Hash => "hash",
        ffi::UiAction::Heic => "heic",
        ffi::UiAction::Height => "height",
        ffi::UiAction::HigherResolution => "higher_resolution",
        ffi::UiAction::HistoryBack => "history_back",
        ffi::UiAction::HistoryForward => "history_forward",
        ffi::UiAction::Http => "http",
        ffi::UiAction::Hydrus => "hydrus",
        ffi::UiAction::Ico => "ico",
        ffi::UiAction::Import => "import",
        ffi::UiAction::ImportFiles => "import_files",
        ffi::UiAction::ImportFolderAdd => "import_folder_add",
        ffi::UiAction::ImportFolderCheck => "import_folder_check",
        ffi::UiAction::ImportFolderCheckAll => "import_folder_check_all",
        ffi::UiAction::ImportFolderEdit => "import_folder_edit",
        ffi::UiAction::ImportFolderRemove => "import_folder_remove",
        ffi::UiAction::ImportFolderSave => "import_folder_save",
        ffi::UiAction::ImportFolderToggle => "import_folder_toggle",
        ffi::UiAction::ImportFolders => "import_folders",
        ffi::UiAction::ImportOptions => "import_options",
        ffi::UiAction::ImportOptionsSave => "import_options_save",
        ffi::UiAction::ImportPaths => "import_paths",
        ffi::UiAction::ImportUrl => "import_url",
        ffi::UiAction::ImportedDays => "imported_days",
        ffi::UiAction::InboxActive => "inbox_active",
        ffi::UiAction::InboxFiles => "inbox_files",
        ffi::UiAction::InboxHashes => "inbox_hashes",
        ffi::UiAction::Int => "int",
        ffi::UiAction::Jpeg => "jpeg",
        ffi::UiAction::Jpg => "jpg",
        ffi::UiAction::Js => "js",
        ffi::UiAction::Json => "json",
        ffi::UiAction::Jxl => "jxl",
        ffi::UiAction::KeepBetter => "keep_better",
        ffi::UiAction::LargerFile => "larger_file",
        ffi::UiAction::LastViewedDays => "last_viewed_days",
        ffi::UiAction::Like => "like",
        ffi::UiAction::LocalImport => "local_import",
        ffi::UiAction::LocalImportAddPaths => "local_import_add_paths",
        ffi::UiAction::LocalImportDeleteAfterSuccess => "local_import_delete_after_success",
        ffi::UiAction::LocalImportFolder => "local_import_folder",
        ffi::UiAction::LocalImportImport => "local_import_import",
        ffi::UiAction::LocalImportOpenPaths => "local_import_open_paths",
        ffi::UiAction::LocalImportOptions => "local_import_options",
        ffi::UiAction::LocalImportRemove => "local_import_remove",
        ffi::UiAction::LocalImportSearchSubdirectories => "local_import_search_subdirectories",
        ffi::UiAction::LocalImportTaggingOpen => "local_import_tagging_open",
        ffi::UiAction::LocalImportTaggingSave => "local_import_tagging_save",
        ffi::UiAction::Locations => "locations",
        ffi::UiAction::LockOff => "lock_off",
        ffi::UiAction::LockOn => "lock_on",
        ffi::UiAction::LoginRemove => "login_remove",
        ffi::UiAction::LoginSave => "login_save",
        ffi::UiAction::M4a => "m4a",
        ffi::UiAction::M4v => "m4v",
        ffi::UiAction::Maintenance => "maintenance",
        ffi::UiAction::ManageExportFolders => "manage_export_folders",
        ffi::UiAction::ManageImportFolders => "manage_import_folders",
        ffi::UiAction::ManageNotesActive => "manage_notes_active",
        ffi::UiAction::ManageServices => "manage_services",
        ffi::UiAction::ManageSubscriptions => "manage_subscriptions",
        ffi::UiAction::ManageTags => "manage_tags",
        ffi::UiAction::ManageTagsActive => "manage_tags_active",
        ffi::UiAction::Media => "media",
        ffi::UiAction::Message => "message",
        ffi::UiAction::Mime => "mime",
        ffi::UiAction::MinFilesize => "min_filesize",
        ffi::UiAction::Mkv => "mkv",
        ffi::UiAction::MoreTags => "more_tags",
        ffi::UiAction::Mov => "mov",
        ffi::UiAction::Move => "move",
        ffi::UiAction::MovePage => "move_page",
        ffi::UiAction::Mp3 => "mp3",
        ffi::UiAction::Mp4 => "mp4",
        ffi::UiAction::Namespace => "namespace",
        ffi::UiAction::Network => "network",
        ffi::UiAction::NetworkHeaderAdd => "network_header_add",
        ffi::UiAction::NetworkHeaderRemove => "network_header_remove",
        ffi::UiAction::NetworkPause => "network_pause",
        ffi::UiAction::NetworkPoolRemove => "network_pool_remove",
        ffi::UiAction::NetworkPoolSet => "network_pool_set",
        ffi::UiAction::NetworkRuleRemove => "network_rule_remove",
        ffi::UiAction::NetworkRuleSet => "network_rule_set",
        ffi::UiAction::NetworkSave => "network_save",
        ffi::UiAction::NetworkUrlClassAdd => "network_url_class_add",
        ffi::UiAction::NetworkUrlClassRemove => "network_url_class_remove",
        ffi::UiAction::NetworkUrlClassSave => "network_url_class_save",
        ffi::UiAction::NewDuplicates => "new_duplicates",
        ffi::UiAction::NewExportFolders => "new_export_folders",
        ffi::UiAction::NewGallery => "new_gallery",
        ffi::UiAction::NewImportFolders => "new_import_folders",
        ffi::UiAction::NewNotebook => "new_notebook",
        ffi::UiAction::NewPage => "new_page",
        ffi::UiAction::NewPageMenu => "new_page_menu",
        ffi::UiAction::NewSimple => "new_simple",
        ffi::UiAction::NewSubscriptions => "new_subscriptions",
        ffi::UiAction::NewUrls => "new_urls",
        ffi::UiAction::NewWatcher => "new_watcher",
        ffi::UiAction::Next => "next",
        ffi::UiAction::NoneableInt => "noneable_int",
        ffi::UiAction::NotDuplicates => "not_duplicates",
        ffi::UiAction::Notebook => "notebook",
        ffi::UiAction::Notes => "notes",
        ffi::UiAction::NotesManage => "notes_manage",
        ffi::UiAction::NotesSave => "notes_save",
        ffi::UiAction::Notifications => "notifications",
        ffi::UiAction::NotificationsClear => "notifications_clear",
        ffi::UiAction::Notify => "notify",
        ffi::UiAction::Nullification => "nullification",
        ffi::UiAction::NullificationPeriod => "nullification_period",
        ffi::UiAction::NumFrames => "num_frames",
        ffi::UiAction::NumPixels => "num_pixels",
        ffi::UiAction::NumTags => "num_tags",
        ffi::UiAction::NumUrls => "num_urls",
        ffi::UiAction::NumWords => "num_words",
        ffi::UiAction::Numerical => "numerical",
        ffi::UiAction::Ogg => "ogg",
        ffi::UiAction::Older => "older",
        ffi::UiAction::OpenDbDir => "open_db_dir",
        ffi::UiAction::OpenInstallDir => "open_install_dir",
        ffi::UiAction::OpenViewer => "open_viewer",
        ffi::UiAction::Options => "options",
        ffi::UiAction::OptionsApply => "options_apply",
        ffi::UiAction::OptionsNetwork => "options_network",
        ffi::UiAction::Opus => "opus",
        ffi::UiAction::OrDialog => "or_dialog",
        ffi::UiAction::OrInsert => "or_insert",
        ffi::UiAction::OrphanFilesDelete => "orphan_files_delete",
        ffi::UiAction::OrphanFilesMove => "orphan_files_move",
        ffi::UiAction::OrphanFilesScan => "orphan_files_scan",
        ffi::UiAction::Otf => "otf",
        ffi::UiAction::PageManager => "page_manager",
        ffi::UiAction::PageManagerActivate => "page_manager_activate",
        ffi::UiAction::PageManagerClose => "page_manager_close",
        ffi::UiAction::PageSelected => "page_selected",
        ffi::UiAction::Parent => "parent",
        ffi::UiAction::PasteTags => "paste_tags",
        ffi::UiAction::PasteTagsPrompt => "paste_tags_prompt",
        ffi::UiAction::PauseNetwork => "pause_network",
        ffi::UiAction::Pdf => "pdf",
        ffi::UiAction::Pending => "pending",
        ffi::UiAction::Pixels => "pixels",
        ffi::UiAction::Pixiv => "pixiv",
        ffi::UiAction::Placeholder => "placeholder",
        ffi::UiAction::Png => "png",
        ffi::UiAction::Post => "post",
        ffi::UiAction::PostUrls => "post_urls",
        ffi::UiAction::Prefetch => "prefetch",
        ffi::UiAction::Presentation => "presentation",
        ffi::UiAction::Preview => "preview",
        ffi::UiAction::PurgeTags => "purge_tags",
        ffi::UiAction::Qoi => "qoi",
        ffi::UiAction::Query => "query",
        ffi::UiAction::Random => "random",
        ffi::UiAction::Rar => "rar",
        ffi::UiAction::RatingSet => "rating_set",
        ffi::UiAction::Ratio => "ratio",
        ffi::UiAction::Readonly => "readonly",
        ffi::UiAction::RedoSearch => "redo_search",
        ffi::UiAction::Refresh => "refresh",
        ffi::UiAction::RefreshPage => "refresh_page",
        ffi::UiAction::RegenInfo => "regen_info",
        ffi::UiAction::Regex => "regex",
        ffi::UiAction::Remove => "remove",
        ffi::UiAction::RemoveFromAlternateGroup => "remove_from_alternate_group",
        ffi::UiAction::RemoveFromDuplicateGroup => "remove_from_duplicate_group",
        ffi::UiAction::RemovePotentials => "remove_potentials",
        ffi::UiAction::RenamePage => "rename_page",
        ffi::UiAction::RepairFileLocations => "repair_file_locations",
        ffi::UiAction::RepairLocationsApply => "repair_locations_apply",
        ffi::UiAction::RepairLocationsProbe => "repair_locations_probe",
        ffi::UiAction::RepairLocationsSet => "repair_locations_set",
        ffi::UiAction::Required => "required",
        ffi::UiAction::ResetPotentialSearch => "reset_potential_search",
        ffi::UiAction::Retry => "retry",
        ffi::UiAction::ReviewLocalImports => "review_local_imports",
        ffi::UiAction::Rss => "rss",
        ffi::UiAction::Rule34 => "rule34",
        ffi::UiAction::RunQuery => "run_query",
        ffi::UiAction::Safebooru => "safebooru",
        ffi::UiAction::Same => "same",
        ffi::UiAction::Search => "search",
        ffi::UiAction::SelectionChanged => "selection_changed",
        ffi::UiAction::ServiceAdd => "service_add",
        ffi::UiAction::ServiceDelete => "service_delete",
        ffi::UiAction::ServiceRename => "service_rename",
        ffi::UiAction::ServiceSave => "service_save",
        ffi::UiAction::ServiceInfo => "service_info",
        ffi::UiAction::SessionAppend => "session_append",
        ffi::UiAction::SessionAppendBackup => "session_append_backup",
        ffi::UiAction::SessionBackup => "session_backup",
        ffi::UiAction::SessionDelete => "session_delete",
        ffi::UiAction::SessionDeleteConfirmed => "session_delete_confirmed",
        ffi::UiAction::SessionLoad => "session_load",
        ffi::UiAction::SessionSave => "session_save",
        ffi::UiAction::SessionSaveAs => "session_save_as",
        ffi::UiAction::SetLocation => "set_location",
        ffi::UiAction::SetSort => "set_sort",
        ffi::UiAction::SetTagService => "set_tag_service",
        ffi::UiAction::Sibling => "sibling",
        ffi::UiAction::Similarity => "similarity",
        ffi::UiAction::Simple => "simple",
        ffi::UiAction::Size => "size",
        ffi::UiAction::Source => "source",
        ffi::UiAction::Statistics => "statistics",
        ffi::UiAction::String => "string",
        ffi::UiAction::Subscription => "subscription",
        ffi::UiAction::SubscriptionAdd => "subscription_add",
        ffi::UiAction::SubscriptionCheck => "subscription_check",
        ffi::UiAction::SubscriptionDelete => "subscription_delete",
        ffi::UiAction::SubscriptionDuplicate => "subscription_duplicate",
        ffi::UiAction::SubscriptionEdit => "subscription_edit",
        ffi::UiAction::SubscriptionEditSave => "subscription_edit_save",
        ffi::UiAction::SubscriptionImportOptions => "subscription_import_options",
        ffi::UiAction::SubscriptionImportOptionsClear => "subscription_import_options_clear",
        ffi::UiAction::SubscriptionImportOptionsSave => "subscription_import_options_save",
        ffi::UiAction::SubscriptionLowercaseQueries => "subscription_lowercase_queries",
        ffi::UiAction::SubscriptionOpenResults => "subscription_open_results",
        ffi::UiAction::SubscriptionReset => "subscription_reset",
        ffi::UiAction::SubscriptionScrubDelays => "subscription_scrub_delays",
        ffi::UiAction::SubscriptionToggle => "subscription_toggle",
        ffi::UiAction::Subscriptions => "subscriptions",
        ffi::UiAction::Subtag => "subtag",
        ffi::UiAction::Success => "success",
        ffi::UiAction::Svg => "svg",
        ffi::UiAction::Synchronise => "synchronise",
        ffi::UiAction::TagAdd => "tag_add",
        ffi::UiAction::TagFilterOpen => "tag_filter_open",
        ffi::UiAction::TagFilterSave => "tag_filter_save",
        ffi::UiAction::TagFilterTest => "tag_filter_test",
        ffi::UiAction::TagFiltering => "tag_filtering",
        ffi::UiAction::TagParents => "tag_parents",
        ffi::UiAction::TagRelationsAdd => "tag_relations_add",
        ffi::UiAction::TagRelationsRemove => "tag_relations_remove",
        ffi::UiAction::TagRemove => "tag_remove",
        ffi::UiAction::TagSiblings => "tag_siblings",
        ffi::UiAction::Tags => "tags",
        ffi::UiAction::Text => "text",
        ffi::UiAction::Thread => "thread",
        ffi::UiAction::Thumbnail => "thumbnail",
        ffi::UiAction::Thumbnails => "thumbnails",
        ffi::UiAction::Tif => "tif",
        ffi::UiAction::Tiff => "tiff",
        ffi::UiAction::TimestampsSave => "timestamps_save",
        ffi::UiAction::Trash => "trash",
        ffi::UiAction::True => "true",
        ffi::UiAction::Ttf => "ttf",
        ffi::UiAction::TwoWayMerge => "two_way_merge",
        ffi::UiAction::Unban => "unban",
        ffi::UiAction::Undo => "undo",
        ffi::UiAction::UndoSearch => "undo_search",
        ffi::UiAction::Unknown => "unknown",
        ffi::UiAction::Update => "update",
        ffi::UiAction::UpdatePeriod => "update_period",
        ffi::UiAction::UrlClass => "url_class",
        ffi::UiAction::Urls => "urls",
        ffi::UiAction::ViewerClose => "viewer_close",
        ffi::UiAction::ViewerNav => "viewer_nav",
        ffi::UiAction::ViewingStats => "viewing_stats",
        ffi::UiAction::Watchable => "watchable",
        ffi::UiAction::Watcher => "watcher",
        ffi::UiAction::WatcherAddUrl => "watcher_add_url",
        ffi::UiAction::WatcherAuto => "watcher_auto",
        ffi::UiAction::WatcherCheck => "watcher_check",
        ffi::UiAction::WatcherCheckerOptions => "watcher_checker_options",
        ffi::UiAction::WatcherPage => "watcher_page",
        ffi::UiAction::WatcherRemove => "watcher_remove",
        ffi::UiAction::WatcherUrls => "watcher_urls",
        ffi::UiAction::Wav => "wav",
        ffi::UiAction::Webm => "webm",
        ffi::UiAction::Webp => "webp",
        ffi::UiAction::Whitelist => "whitelist",
        ffi::UiAction::Width => "width",
        ffi::UiAction::Woff => "woff",
        ffi::UiAction::Woff2 => "woff2",
        ffi::UiAction::Words => "words",
        ffi::UiAction::Worse => "worse",
        ffi::UiAction::Xml => "xml",
        ffi::UiAction::Zip => "zip",
        _ => "",
        }
    }
}

pub(crate) fn action_from_str(action: &str) -> ffi::UiAction {
    match action {
            "a" => ffi::UiAction::A,
            "about" => ffi::UiAction::About,
            "account_type" => ffi::UiAction::AccountType,
            "account_type_delete" => ffi::UiAction::AccountTypeDelete,
            "account_type_save" => ffi::UiAction::AccountTypeSave,
            "account_types_open" => ffi::UiAction::AccountTypesOpen,
            "add_hashes_to_page" => ffi::UiAction::AddHashesToPage,
            "admin_account_action" => ffi::UiAction::AdminAccountAction,
            "admin_add_service" => ffi::UiAction::AdminAddService,
            "admin_connect" => ffi::UiAction::AdminConnect,
            "admin_edit_service" => ffi::UiAction::AdminEditService,
            "admin_remove_service" => ffi::UiAction::AdminRemoveService,
            "admin_service_action" => ffi::UiAction::AdminServiceAction,
            "all" => ffi::UiAction::All,
            "alternate" => ffi::UiAction::Alternate,
            "alternates" => ffi::UiAction::Alternates,
            "always_both" => ffi::UiAction::AlwaysBoth,
            "api" => ffi::UiAction::Api,
            "api_permissions" => ffi::UiAction::ApiPermissions,
            "api_permissions_delete" => ffi::UiAction::ApiPermissionsDelete,
            "api_permissions_save" => ffi::UiAction::ApiPermissionsSave,
            "apply_search_history" => ffi::UiAction::ApplySearchHistory,
            "apply_tags" => ffi::UiAction::ApplyTags,
            "apply_tags_selection" => ffi::UiAction::ApplyTagsSelection,
            "apply_tags_selection_apply" => ffi::UiAction::ApplyTagsSelectionApply,
            "archive_active" => ffi::UiAction::ArchiveActive,
            "archive_files" => ffi::UiAction::ArchiveFiles,
            "archive_hashes" => ffi::UiAction::ArchiveHashes,
            "archived_days" => ffi::UiAction::ArchivedDays,
            "auto" => ffi::UiAction::Auto,
            "autocomplete" => ffi::UiAction::Autocomplete,
            "avi" => ffi::UiAction::Avi,
            "avif" => ffi::UiAction::Avif,
            "b" => ffi::UiAction::B,
            "ban" => ffi::UiAction::Ban,
            "better" => ffi::UiAction::Better,
            "blacklist" => ffi::UiAction::Blacklist,
            "bmp" => ffi::UiAction::Bmp,
            "bool" => ffi::UiAction::Bool,
            "both_files" => ffi::UiAction::BothFiles,
            "bytes" => ffi::UiAction::Bytes,
            "clear" => ffi::UiAction::Clear,
            "clear_false_positives" => ffi::UiAction::ClearFalsePositives,
            "clear_finished" => ffi::UiAction::ClearFinished,
            "clear_internal_false_positives" => ffi::UiAction::ClearInternalFalsePositives,
            "client_api" => ffi::UiAction::ClientApi,
            "close_other_pages" => ffi::UiAction::CloseOtherPages,
            "close_page" => ffi::UiAction::ClosePage,
            "command_palette" => ffi::UiAction::CommandPalette,
            "copy_tags" => ffi::UiAction::CopyTags,
            "create_page" => ffi::UiAction::CreatePage,
            "css" => ffi::UiAction::Css,
            "danbooru" => ffi::UiAction::Danbooru,
            "data" => ffi::UiAction::Data,
            "db_analyze" => ffi::UiAction::DbAnalyze,
            "db_backup_open" => ffi::UiAction::DbBackupOpen,
            "db_backup_run" => ffi::UiAction::DbBackupRun,
            "db_backup_set_path" => ffi::UiAction::DbBackupSetPath,
            "db_check" => ffi::UiAction::DbCheck,
            "db_deferred_deletes" => ffi::UiAction::DbDeferredDeletes,
            "db_file_maintenance" => ffi::UiAction::DbFileMaintenance,
            "db_fix_archive_times" => ffi::UiAction::DbFixArchiveTimes,
            "db_info" => ffi::UiAction::DbInfo,
            "db_run_file_maintenance" => ffi::UiAction::DbRunFileMaintenance,
            "db_stats" => ffi::UiAction::DbStats,
            "db_vacuum" => ffi::UiAction::DbVacuum,
            "db_vacuum_info" => ffi::UiAction::DbVacuumInfo,
            "delete" => ffi::UiAction::Delete,
            "delete_all_content" => ffi::UiAction::DeleteAllContent,
            "delete_both" => ffi::UiAction::DeleteBoth,
            "delete_files" => ffi::UiAction::DeleteFiles,
            "delete_hashes" => ffi::UiAction::DeleteHashes,
            "desired" => ffi::UiAction::Desired,
            "different_searches" => ffi::UiAction::DifferentSearches,
            "dislike" => ffi::UiAction::Dislike,
            "dissolve_alternate_group" => ffi::UiAction::DissolveAlternateGroup,
            "dissolve_duplicate_group" => ffi::UiAction::DissolveDuplicateGroup,
            "docs" => ffi::UiAction::Docs,
            "domain" => ffi::UiAction::Domain,
            "done" => ffi::UiAction::Done,
            "download_url" => ffi::UiAction::DownloadUrl,
            "downloader" => ffi::UiAction::Downloader,
            "downloader_clear" => ffi::UiAction::DownloaderClear,
            "downloader_export" => ffi::UiAction::DownloaderExport,
            "downloader_export_to" => ffi::UiAction::DownloaderExportTo,
            "downloader_import" => ffi::UiAction::DownloaderImport,
            "downloader_import_from" => ffi::UiAction::DownloaderImportFrom,
            "downloader_import_options" => ffi::UiAction::DownloaderImportOptions,
            "downloader_import_options_save" => ffi::UiAction::DownloaderImportOptionsSave,
            "downloader_jobs_set" => ffi::UiAction::DownloaderJobsSet,
            "downloader_open_results" => ffi::UiAction::DownloaderOpenResults,
            "downloader_page" => ffi::UiAction::DownloaderPage,
            "downloader_pause" => ffi::UiAction::DownloaderPause,
            "downloader_retry" => ffi::UiAction::DownloaderRetry,
            "downloader_set_formula" => ffi::UiAction::DownloaderSetFormula,
            "downloader_set_site" => ffi::UiAction::DownloaderSetSite,
            "downloader_start" => ffi::UiAction::DownloaderStart,
            "downloads" => ffi::UiAction::Downloads,
            "duplicate_page" => ffi::UiAction::DuplicatePage,
            "duplicates" => ffi::UiAction::Duplicates,
            "duplicates_alternates" => ffi::UiAction::DuplicatesAlternates,
            "duplicates_auto_toggle" => ffi::UiAction::DuplicatesAutoToggle,
            "duplicates_batch_size" => ffi::UiAction::DuplicatesBatchSize,
            "duplicates_custom" => ffi::UiAction::DuplicatesCustom,
            "duplicates_custom_action" => ffi::UiAction::DuplicatesCustomAction,
            "duplicates_delete_both" => ffi::UiAction::DuplicatesDeleteBoth,
            "duplicates_false" => ffi::UiAction::DuplicatesFalse,
            "duplicates_group_action" => ffi::UiAction::DuplicatesGroupAction,
            "duplicates_keep_a" => ffi::UiAction::DuplicatesKeepA,
            "duplicates_keep_a_keep" => ffi::UiAction::DuplicatesKeepAKeep,
            "duplicates_keep_b" => ffi::UiAction::DuplicatesKeepB,
            "duplicates_keep_b_keep" => ffi::UiAction::DuplicatesKeepBKeep,
            "duplicates_merge_options" => ffi::UiAction::DuplicatesMergeOptions,
            "duplicates_merge_options_dialog" => ffi::UiAction::DuplicatesMergeOptionsDialog,
            "duplicates_next_batch" => ffi::UiAction::DuplicatesNextBatch,
            "duplicates_refresh" => ffi::UiAction::DuplicatesRefresh,
            "duplicates_review" => ffi::UiAction::DuplicatesReview,
            "duplicates_review_apply" => ffi::UiAction::DuplicatesReviewApply,
            "duplicates_rule_add" => ffi::UiAction::DuplicatesRuleAdd,
            "duplicates_rule_preview" => ffi::UiAction::DuplicatesRulePreview,
            "duplicates_rule_remove" => ffi::UiAction::DuplicatesRuleRemove,
            "duplicates_rule_save" => ffi::UiAction::DuplicatesRuleSave,
            "duplicates_rules" => ffi::UiAction::DuplicatesRules,
            "duplicates_same" => ffi::UiAction::DuplicatesSame,
            "duplicates_search_context" => ffi::UiAction::DuplicatesSearchContext,
            "duplicates_show_pair" => ffi::UiAction::DuplicatesShowPair,
            "duplicates_skip" => ffi::UiAction::DuplicatesSkip,
            "duration" => ffi::UiAction::Duration,
            "e621" => ffi::UiAction::E621,
            "edit_timestamps" => ffi::UiAction::EditTimestamps,
            "either" => ffi::UiAction::Either,
            "eot" => ffi::UiAction::Eot,
            "error" => ffi::UiAction::Error,
            "exact_match" => ffi::UiAction::ExactMatch,
            "excluded" => ffi::UiAction::Excluded,
            "exit" => ffi::UiAction::Exit,
            "expires" => ffi::UiAction::Expires,
            "export_folder_edit" => ffi::UiAction::ExportFolderEdit,
            "export_folder_remove" => ffi::UiAction::ExportFolderRemove,
            "export_folder_reveal" => ffi::UiAction::ExportFolderReveal,
            "export_folder_review" => ffi::UiAction::ExportFolderReview,
            "export_folder_run" => ffi::UiAction::ExportFolderRun,
            "export_folder_run_all" => ffi::UiAction::ExportFolderRunAll,
            "export_folder_save" => ffi::UiAction::ExportFolderSave,
            "export_folder_test_examples" => ffi::UiAction::ExportFolderTestExamples,
            "export_folder_toggle" => ffi::UiAction::ExportFolderToggle,
            "export_folders" => ffi::UiAction::ExportFolders,
            "exporting" => ffi::UiAction::Exporting,
            "external_programs" => ffi::UiAction::ExternalPrograms,
            "false_positive" => ffi::UiAction::FalsePositive,
            "favourite_apply" => ffi::UiAction::FavouriteApply,
            "favourite_delete" => ffi::UiAction::FavouriteDelete,
            "favourite_delete_confirmed" => ffi::UiAction::FavouriteDeleteConfirmed,
            "favourite_save" => ffi::UiAction::FavouriteSave,
            "favourite_save_confirmed" => ffi::UiAction::FavouriteSaveConfirmed,
            "fewer_tags" => ffi::UiAction::FewerTags,
            "file" => ffi::UiAction::File,
            "file_filtering" => ffi::UiAction::FileFiltering,
            "file_id" => ffi::UiAction::FileId,
            "file_seed_cache" => ffi::UiAction::FileSeedCache,
            "file_seed_cache_action" => ffi::UiAction::FileSeedCacheAction,
            "files" => ffi::UiAction::Files,
            "flac" => ffi::UiAction::Flac,
            "focus_hash" => ffi::UiAction::FocusHash,
            "frames" => ffi::UiAction::Frames,
            "gallery" => ffi::UiAction::Gallery,
            "gallery_seed_log" => ffi::UiAction::GallerySeedLog,
            "gallery_seed_log_action" => ffi::UiAction::GallerySeedLogAction,
            "gelbooru" => ffi::UiAction::Gelbooru,
            "general" => ffi::UiAction::General,
            "generic" => ffi::UiAction::Generic,
            "gif" => ffi::UiAction::Gif,
            "global" => ffi::UiAction::Global,
            "has_audio" => ffi::UiAction::HasAudio,
            "hash" => ffi::UiAction::Hash,
            "heic" => ffi::UiAction::Heic,
            "height" => ffi::UiAction::Height,
            "higher_resolution" => ffi::UiAction::HigherResolution,
            "history_back" => ffi::UiAction::HistoryBack,
            "history_forward" => ffi::UiAction::HistoryForward,
            "http" => ffi::UiAction::Http,
            "hydrus" => ffi::UiAction::Hydrus,
            "ico" => ffi::UiAction::Ico,
            "import" => ffi::UiAction::Import,
            "import_files" => ffi::UiAction::ImportFiles,
            "import_folder_add" => ffi::UiAction::ImportFolderAdd,
            "import_folder_check" => ffi::UiAction::ImportFolderCheck,
            "import_folder_check_all" => ffi::UiAction::ImportFolderCheckAll,
            "import_folder_edit" => ffi::UiAction::ImportFolderEdit,
            "import_folder_remove" => ffi::UiAction::ImportFolderRemove,
            "import_folder_save" => ffi::UiAction::ImportFolderSave,
            "import_folder_toggle" => ffi::UiAction::ImportFolderToggle,
            "import_folders" => ffi::UiAction::ImportFolders,
            "import_options" => ffi::UiAction::ImportOptions,
            "import_options_save" => ffi::UiAction::ImportOptionsSave,
            "import_paths" => ffi::UiAction::ImportPaths,
            "import_url" => ffi::UiAction::ImportUrl,
            "imported_days" => ffi::UiAction::ImportedDays,
            "inbox_active" => ffi::UiAction::InboxActive,
            "inbox_files" => ffi::UiAction::InboxFiles,
            "inbox_hashes" => ffi::UiAction::InboxHashes,
            "int" => ffi::UiAction::Int,
            "jpeg" => ffi::UiAction::Jpeg,
            "jpg" => ffi::UiAction::Jpg,
            "js" => ffi::UiAction::Js,
            "json" => ffi::UiAction::Json,
            "jxl" => ffi::UiAction::Jxl,
            "keep_better" => ffi::UiAction::KeepBetter,
            "larger_file" => ffi::UiAction::LargerFile,
            "last_viewed_days" => ffi::UiAction::LastViewedDays,
            "like" => ffi::UiAction::Like,
            "local_import" => ffi::UiAction::LocalImport,
            "local_import_add_paths" => ffi::UiAction::LocalImportAddPaths,
            "local_import_delete_after_success" => ffi::UiAction::LocalImportDeleteAfterSuccess,
            "local_import_folder" => ffi::UiAction::LocalImportFolder,
            "local_import_import" => ffi::UiAction::LocalImportImport,
            "local_import_open_paths" => ffi::UiAction::LocalImportOpenPaths,
            "local_import_options" => ffi::UiAction::LocalImportOptions,
            "local_import_remove" => ffi::UiAction::LocalImportRemove,
            "local_import_search_subdirectories" => ffi::UiAction::LocalImportSearchSubdirectories,
            "local_import_tagging_open" => ffi::UiAction::LocalImportTaggingOpen,
            "local_import_tagging_save" => ffi::UiAction::LocalImportTaggingSave,
            "locations" => ffi::UiAction::Locations,
            "lock_off" => ffi::UiAction::LockOff,
            "lock_on" => ffi::UiAction::LockOn,
            "login_remove" => ffi::UiAction::LoginRemove,
            "login_save" => ffi::UiAction::LoginSave,
            "m4a" => ffi::UiAction::M4a,
            "m4v" => ffi::UiAction::M4v,
            "maintenance" => ffi::UiAction::Maintenance,
            "manage_export_folders" => ffi::UiAction::ManageExportFolders,
            "manage_import_folders" => ffi::UiAction::ManageImportFolders,
            "manage_notes_active" => ffi::UiAction::ManageNotesActive,
            "manage_services" => ffi::UiAction::ManageServices,
            "manage_subscriptions" => ffi::UiAction::ManageSubscriptions,
            "manage_tags" => ffi::UiAction::ManageTags,
            "manage_tags_active" => ffi::UiAction::ManageTagsActive,
            "media" => ffi::UiAction::Media,
            "message" => ffi::UiAction::Message,
            "mime" => ffi::UiAction::Mime,
            "min_filesize" => ffi::UiAction::MinFilesize,
            "mkv" => ffi::UiAction::Mkv,
            "more_tags" => ffi::UiAction::MoreTags,
            "mov" => ffi::UiAction::Mov,
            "move" => ffi::UiAction::Move,
            "move_page" => ffi::UiAction::MovePage,
            "mp3" => ffi::UiAction::Mp3,
            "mp4" => ffi::UiAction::Mp4,
            "namespace" => ffi::UiAction::Namespace,
            "network" => ffi::UiAction::Network,
            "network_header_add" => ffi::UiAction::NetworkHeaderAdd,
            "network_header_remove" => ffi::UiAction::NetworkHeaderRemove,
            "network_pause" => ffi::UiAction::NetworkPause,
            "network_pool_remove" => ffi::UiAction::NetworkPoolRemove,
            "network_pool_set" => ffi::UiAction::NetworkPoolSet,
            "network_rule_remove" => ffi::UiAction::NetworkRuleRemove,
            "network_rule_set" => ffi::UiAction::NetworkRuleSet,
            "network_save" => ffi::UiAction::NetworkSave,
            "network_url_class_add" => ffi::UiAction::NetworkUrlClassAdd,
            "network_url_class_remove" => ffi::UiAction::NetworkUrlClassRemove,
            "network_url_class_save" => ffi::UiAction::NetworkUrlClassSave,
            "new_duplicates" => ffi::UiAction::NewDuplicates,
            "new_export_folders" => ffi::UiAction::NewExportFolders,
            "new_gallery" => ffi::UiAction::NewGallery,
            "new_import_folders" => ffi::UiAction::NewImportFolders,
            "new_notebook" => ffi::UiAction::NewNotebook,
            "new_page" => ffi::UiAction::NewPage,
            "new_page_menu" => ffi::UiAction::NewPageMenu,
            "new_simple" => ffi::UiAction::NewSimple,
            "new_subscriptions" => ffi::UiAction::NewSubscriptions,
            "new_urls" => ffi::UiAction::NewUrls,
            "new_watcher" => ffi::UiAction::NewWatcher,
            "next" => ffi::UiAction::Next,
            "noneable_int" => ffi::UiAction::NoneableInt,
            "not_duplicates" => ffi::UiAction::NotDuplicates,
            "notebook" => ffi::UiAction::Notebook,
            "notes" => ffi::UiAction::Notes,
            "notes_manage" => ffi::UiAction::NotesManage,
            "notes_save" => ffi::UiAction::NotesSave,
            "notifications" => ffi::UiAction::Notifications,
            "notifications_clear" => ffi::UiAction::NotificationsClear,
            "notify" => ffi::UiAction::Notify,
            "nullification" => ffi::UiAction::Nullification,
            "nullification_period" => ffi::UiAction::NullificationPeriod,
            "num_frames" => ffi::UiAction::NumFrames,
            "num_pixels" => ffi::UiAction::NumPixels,
            "num_tags" => ffi::UiAction::NumTags,
            "num_urls" => ffi::UiAction::NumUrls,
            "num_words" => ffi::UiAction::NumWords,
            "numerical" => ffi::UiAction::Numerical,
            "ogg" => ffi::UiAction::Ogg,
            "older" => ffi::UiAction::Older,
            "open_db_dir" => ffi::UiAction::OpenDbDir,
            "open_install_dir" => ffi::UiAction::OpenInstallDir,
            "open_viewer" => ffi::UiAction::OpenViewer,
            "options" => ffi::UiAction::Options,
            "options_apply" => ffi::UiAction::OptionsApply,
            "options_network" => ffi::UiAction::OptionsNetwork,
            "opus" => ffi::UiAction::Opus,
            "or_dialog" => ffi::UiAction::OrDialog,
            "or_insert" => ffi::UiAction::OrInsert,
            "orphan_files_delete" => ffi::UiAction::OrphanFilesDelete,
            "orphan_files_move" => ffi::UiAction::OrphanFilesMove,
            "orphan_files_scan" => ffi::UiAction::OrphanFilesScan,
            "otf" => ffi::UiAction::Otf,
            "page_manager" => ffi::UiAction::PageManager,
            "page_manager_activate" => ffi::UiAction::PageManagerActivate,
            "page_manager_close" => ffi::UiAction::PageManagerClose,
            "page_selected" => ffi::UiAction::PageSelected,
            "parent" => ffi::UiAction::Parent,
            "paste_tags" => ffi::UiAction::PasteTags,
            "paste_tags_prompt" => ffi::UiAction::PasteTagsPrompt,
            "pause_network" => ffi::UiAction::PauseNetwork,
            "pdf" => ffi::UiAction::Pdf,
            "pending" => ffi::UiAction::Pending,
            "pixels" => ffi::UiAction::Pixels,
            "pixiv" => ffi::UiAction::Pixiv,
            "placeholder" => ffi::UiAction::Placeholder,
            "png" => ffi::UiAction::Png,
            "post" => ffi::UiAction::Post,
            "post_urls" => ffi::UiAction::PostUrls,
            "prefetch" => ffi::UiAction::Prefetch,
            "presentation" => ffi::UiAction::Presentation,
            "preview" => ffi::UiAction::Preview,
            "purge_tags" => ffi::UiAction::PurgeTags,
            "qoi" => ffi::UiAction::Qoi,
            "query" => ffi::UiAction::Query,
            "random" => ffi::UiAction::Random,
            "rar" => ffi::UiAction::Rar,
            "rating_set" => ffi::UiAction::RatingSet,
            "ratio" => ffi::UiAction::Ratio,
            "readonly" => ffi::UiAction::Readonly,
            "redo_search" => ffi::UiAction::RedoSearch,
            "refresh" => ffi::UiAction::Refresh,
            "refresh_page" => ffi::UiAction::RefreshPage,
            "regen_info" => ffi::UiAction::RegenInfo,
            "regex" => ffi::UiAction::Regex,
            "remove" => ffi::UiAction::Remove,
            "remove_from_alternate_group" => ffi::UiAction::RemoveFromAlternateGroup,
            "remove_from_duplicate_group" => ffi::UiAction::RemoveFromDuplicateGroup,
            "remove_potentials" => ffi::UiAction::RemovePotentials,
            "rename_page" => ffi::UiAction::RenamePage,
            "repair_file_locations" => ffi::UiAction::RepairFileLocations,
            "repair_locations_apply" => ffi::UiAction::RepairLocationsApply,
            "repair_locations_probe" => ffi::UiAction::RepairLocationsProbe,
            "repair_locations_set" => ffi::UiAction::RepairLocationsSet,
            "required" => ffi::UiAction::Required,
            "reset_potential_search" => ffi::UiAction::ResetPotentialSearch,
            "retry" => ffi::UiAction::Retry,
            "review_local_imports" => ffi::UiAction::ReviewLocalImports,
            "rss" => ffi::UiAction::Rss,
            "rule34" => ffi::UiAction::Rule34,
            "run_query" => ffi::UiAction::RunQuery,
            "safebooru" => ffi::UiAction::Safebooru,
            "same" => ffi::UiAction::Same,
            "search" => ffi::UiAction::Search,
            "selection_changed" => ffi::UiAction::SelectionChanged,
            "service_add" => ffi::UiAction::ServiceAdd,
            "service_delete" => ffi::UiAction::ServiceDelete,
            "service_rename" => ffi::UiAction::ServiceRename,
            "service_save" => ffi::UiAction::ServiceSave,
            "service_info" => ffi::UiAction::ServiceInfo,
            "session_append" => ffi::UiAction::SessionAppend,
            "session_append_backup" => ffi::UiAction::SessionAppendBackup,
            "session_backup" => ffi::UiAction::SessionBackup,
            "session_delete" => ffi::UiAction::SessionDelete,
            "session_delete_confirmed" => ffi::UiAction::SessionDeleteConfirmed,
            "session_load" => ffi::UiAction::SessionLoad,
            "session_save" => ffi::UiAction::SessionSave,
            "session_save_as" => ffi::UiAction::SessionSaveAs,
            "set_location" => ffi::UiAction::SetLocation,
            "set_sort" => ffi::UiAction::SetSort,
            "set_tag_service" => ffi::UiAction::SetTagService,
            "sibling" => ffi::UiAction::Sibling,
            "similarity" => ffi::UiAction::Similarity,
            "simple" => ffi::UiAction::Simple,
            "size" => ffi::UiAction::Size,
            "source" => ffi::UiAction::Source,
            "statistics" => ffi::UiAction::Statistics,
            "string" => ffi::UiAction::String,
            "subscription" => ffi::UiAction::Subscription,
            "subscription_add" => ffi::UiAction::SubscriptionAdd,
            "subscription_check" => ffi::UiAction::SubscriptionCheck,
            "subscription_delete" => ffi::UiAction::SubscriptionDelete,
            "subscription_duplicate" => ffi::UiAction::SubscriptionDuplicate,
            "subscription_edit" => ffi::UiAction::SubscriptionEdit,
            "subscription_edit_save" => ffi::UiAction::SubscriptionEditSave,
            "subscription_import_options" => ffi::UiAction::SubscriptionImportOptions,
            "subscription_import_options_clear" => ffi::UiAction::SubscriptionImportOptionsClear,
            "subscription_import_options_save" => ffi::UiAction::SubscriptionImportOptionsSave,
            "subscription_lowercase_queries" => ffi::UiAction::SubscriptionLowercaseQueries,
            "subscription_open_results" => ffi::UiAction::SubscriptionOpenResults,
            "subscription_reset" => ffi::UiAction::SubscriptionReset,
            "subscription_scrub_delays" => ffi::UiAction::SubscriptionScrubDelays,
            "subscription_toggle" => ffi::UiAction::SubscriptionToggle,
            "subscriptions" => ffi::UiAction::Subscriptions,
            "subtag" => ffi::UiAction::Subtag,
            "success" => ffi::UiAction::Success,
            "svg" => ffi::UiAction::Svg,
            "synchronise" => ffi::UiAction::Synchronise,
            "tag_add" => ffi::UiAction::TagAdd,
            "tag_filter_open" => ffi::UiAction::TagFilterOpen,
            "tag_filter_save" => ffi::UiAction::TagFilterSave,
            "tag_filter_test" => ffi::UiAction::TagFilterTest,
            "tag_filtering" => ffi::UiAction::TagFiltering,
            "tag_parents" => ffi::UiAction::TagParents,
            "tag_relations_add" => ffi::UiAction::TagRelationsAdd,
            "tag_relations_remove" => ffi::UiAction::TagRelationsRemove,
            "tag_remove" => ffi::UiAction::TagRemove,
            "tag_siblings" => ffi::UiAction::TagSiblings,
            "tags" => ffi::UiAction::Tags,
            "text" => ffi::UiAction::Text,
            "thread" => ffi::UiAction::Thread,
            "thumbnail" => ffi::UiAction::Thumbnail,
            "thumbnails" => ffi::UiAction::Thumbnails,
            "tif" => ffi::UiAction::Tif,
            "tiff" => ffi::UiAction::Tiff,
            "timestamps_save" => ffi::UiAction::TimestampsSave,
            "trash" => ffi::UiAction::Trash,
            "true" => ffi::UiAction::True,
            "ttf" => ffi::UiAction::Ttf,
            "two_way_merge" => ffi::UiAction::TwoWayMerge,
            "unban" => ffi::UiAction::Unban,
            "undo" => ffi::UiAction::Undo,
            "undo_search" => ffi::UiAction::UndoSearch,
            "unknown" => ffi::UiAction::Unknown,
            "update" => ffi::UiAction::Update,
            "update_period" => ffi::UiAction::UpdatePeriod,
            "url_class" => ffi::UiAction::UrlClass,
            "urls" => ffi::UiAction::Urls,
            "viewer_close" => ffi::UiAction::ViewerClose,
            "viewer_nav" => ffi::UiAction::ViewerNav,
            "viewing_stats" => ffi::UiAction::ViewingStats,
            "watchable" => ffi::UiAction::Watchable,
            "watcher" => ffi::UiAction::Watcher,
            "watcher_add_url" => ffi::UiAction::WatcherAddUrl,
            "watcher_auto" => ffi::UiAction::WatcherAuto,
            "watcher_check" => ffi::UiAction::WatcherCheck,
            "watcher_checker_options" => ffi::UiAction::WatcherCheckerOptions,
            "watcher_page" => ffi::UiAction::WatcherPage,
            "watcher_remove" => ffi::UiAction::WatcherRemove,
            "watcher_urls" => ffi::UiAction::WatcherUrls,
            "wav" => ffi::UiAction::Wav,
            "webm" => ffi::UiAction::Webm,
            "webp" => ffi::UiAction::Webp,
            "whitelist" => ffi::UiAction::Whitelist,
            "width" => ffi::UiAction::Width,
            "woff" => ffi::UiAction::Woff,
            "woff2" => ffi::UiAction::Woff2,
            "words" => ffi::UiAction::Words,
            "worse" => ffi::UiAction::Worse,
            "xml" => ffi::UiAction::Xml,
            "zip" => ffi::UiAction::Zip,
        _ => ffi::UiAction::None,
    }
}
/// Typed reverse seam (WS-15.2.2a): tab drag & drop between notebooks.
/// Same core as the `"move_page"` notify, without any JSON payload.
pub fn handle_move_page(
    mut window: Pin<&mut ffi::HydrusMainWindow>,
    source: &str,
    target: &str,
    position: &str,
) {
    let commands = with_state_mut(|state| state.page_move_request(source, target, position));

    record_notifications(&commands);

    for command in commands {
        panels::execute(window.as_mut(), command);
    }
}

/// Typed reverse seam (WS-15.2.2a): manage-pages dialog close actions.
pub fn handle_close_pages(mut window: Pin<&mut ffi::HydrusMainWindow>, ids: Vec<String>) {
    let commands = with_state_mut(|state| state.close_pages(&ids));

    record_notifications(&commands);

    for command in commands {
        panels::execute(window.as_mut(), command);
    }
}

/// Typed reverse seams (WS-15.2.2b): thumbnail grid selection, focus, viewer
/// open and page-to-page drag. Each reuses the same core as its `"..."`
/// notify action, without any JSON payload.
pub fn handle_set_selection(
    mut window: Pin<&mut ffi::HydrusMainWindow>,
    page_id: &str,
    hashes: &[String],
) {
    let commands = with_state_mut(|state| state.set_selection(page_id, hashes));

    record_notifications(&commands);

    for command in commands {
        panels::execute(window.as_mut(), command);
    }
}

pub fn handle_focus_hash(
    mut window: Pin<&mut ffi::HydrusMainWindow>,
    page_id: &str,
    hash: &str,
) {
    let commands = with_state_mut(|state| state.focus(page_id, hash));

    record_notifications(&commands);

    for command in commands {
        panels::execute(window.as_mut(), command);
    }
}

pub fn handle_open_viewer(
    mut window: Pin<&mut ffi::HydrusMainWindow>,
    page_id: &str,
    hash: &str,
) {
    let commands = with_state_mut(|state| state.open_viewer(page_id, hash));

    record_notifications(&commands);

    for command in commands {
        panels::execute(window.as_mut(), command);
    }
}

pub fn handle_add_hashes(
    mut window: Pin<&mut ffi::HydrusMainWindow>,
    page_id: &str,
    hashes: &[String],
) {
    let commands = with_state_mut(|state| state.add_hashes(page_id, hashes));

    record_notifications(&commands);

    for command in commands {
        panels::execute(window.as_mut(), command);
    }
}

/// Typed reverse seams (WS-15.2.2b): metadata panel tag/rating/notes actions.
pub fn handle_tag_edit(
    mut window: Pin<&mut ffi::HydrusMainWindow>,
    hash: &str,
    service: &str,
    tag: &str,
    add: bool,
) {
    let commands =
        with_state_mut(|state| crate::gui::app::metadata::set_tag_typed(state, hash, service, tag, add));

    record_notifications(&commands);

    for command in commands {
        panels::execute(window.as_mut(), command);
    }
}

pub fn handle_set_rating(
    mut window: Pin<&mut ffi::HydrusMainWindow>,
    hash: &str,
    service: &str,
    action: &str,
    value: f64,
) {
    let commands = with_state_mut(|state| {
        crate::gui::app::metadata::set_rating_typed(state, hash, service, action, value)
    });

    record_notifications(&commands);

    for command in commands {
        panels::execute(window.as_mut(), command);
    }
}

pub fn handle_manage_notes(mut window: Pin<&mut ffi::HydrusMainWindow>, hash: &str) {
    let commands =
        with_state_mut(|state| crate::gui::app::metadata::manage_notes_typed(state, hash));

    record_notifications(&commands);

    for command in commands {
        panels::execute(window.as_mut(), command);
    }
}

/// Typed reverse seams (WS-15.2.3a): page/tab/session/history events.
pub fn handle_page_selected(mut window: Pin<&mut ffi::HydrusMainWindow>, page_id: &str) {
    let commands = with_state_mut(|state| state.page_selected_command(page_id));

    record_notifications(&commands);

    for command in commands {
        panels::execute(window.as_mut(), command);
    }
}

pub fn handle_close_page(mut window: Pin<&mut ffi::HydrusMainWindow>, page_id: &str) {
    let commands = with_state_mut(|state| state.close_page(page_id));

    record_notifications(&commands);

    for command in commands {
        panels::execute(window.as_mut(), command);
    }
}

pub fn handle_close_other_pages(mut window: Pin<&mut ffi::HydrusMainWindow>, page_id: &str) {
    let commands = with_state_mut(|state| state.close_other_pages(page_id));

    record_notifications(&commands);

    for command in commands {
        panels::execute(window.as_mut(), command);
    }
}

pub fn handle_rename_page(
    mut window: Pin<&mut ffi::HydrusMainWindow>,
    page_id: &str,
    name: &str,
) {
    let commands = with_state_mut(|state| state.rename_page(page_id, name));

    record_notifications(&commands);

    for command in commands {
        panels::execute(window.as_mut(), command);
    }
}

pub fn handle_refresh_page(mut window: Pin<&mut ffi::HydrusMainWindow>, page_id: &str) {
    let commands = with_state_mut(|state| {
        let id = state.page_id_or_active(page_id);
        state.refresh_page_commands(&id)
    });

    record_notifications(&commands);

    for command in commands {
        panels::execute(window.as_mut(), command);
    }
}

pub fn handle_page_manager_activate(mut window: Pin<&mut ffi::HydrusMainWindow>, page_id: &str) {
    let commands = with_state_mut(|state| state.page_manager_activate_command(page_id));

    record_notifications(&commands);

    for command in commands {
        panels::execute(window.as_mut(), command);
    }
}

pub fn handle_page_sort(
    mut window: Pin<&mut ffi::HydrusMainWindow>,
    page_id: &str,
    sort: &str,
    ascending: bool,
) {
    let commands = with_state_mut(|state| {
        state.set_sort_command(page_id, Some(sort), Some(ascending))
    });

    record_notifications(&commands);

    for command in commands {
        panels::execute(window.as_mut(), command);
    }
}

pub fn handle_history_step(mut window: Pin<&mut ffi::HydrusMainWindow>, step: i32) {
    let commands = with_state_mut(|state| {
        if step < 0 {
            state.navigate_history(true)
        } else if step > 0 {
            state.navigate_history(false)
        } else {
            Vec::new()
        }
    });

    record_notifications(&commands);

    for command in commands {
        panels::execute(window.as_mut(), command);
    }
}

fn record_notifications(commands: &[UiCommand]) {
    with_state_mut(|state| {
        for command in commands {
            if let UiCommand::NotifyMessage(text) = command {
                state.notifications.push("info", text);
                let limit = state
                    .controller
                    .options()
                    .get_integer("popup_message_max_messages")
                    .clamp(1, 5_000) as usize;
                state.notifications.trim_to(limit);
            }
        }
    });
}

/// Qt timer tick: drains background download/import events and runs due
/// watcher/subscription checks.
pub fn handle_tick(mut window: Pin<&mut ffi::HydrusMainWindow>) {
    let commands = with_state_mut(download::tick);

    for command in commands {
        panels::execute(window.as_mut(), command);
    }

    let commands = with_state_mut(maintenance_tick);

    for command in commands {
        panels::execute(window.as_mut(), command);
    }
}

/// Runs a couple of due maintenance jobs on each tick when the corresponding
/// option is enabled. Big jobs (blurhash, perceptual hashes) therefore never
/// block the UI for long.
pub(crate) fn maintenance_tick(state: &mut AppState) -> Vec<UiCommand> {
    if state.import_queue.running {
        return Vec::new();
    }

    let options = state.controller.options();

    if !options.get_boolean("file_maintenance_during_active")
        && !options.get_boolean("file_maintenance_during_idle")
    {
        return Vec::new();
    }

    let (done, failed) = run_maintenance(state, 2);

    if done == 0 && failed == 0 {
        return Vec::new();
    }

    state.message = format!("file maintenance: {done} done, {failed} failed");
    vec![UiCommand::Status(render::status(state))]
}

pub(crate) fn run_maintenance(state: &mut AppState, limit: usize) -> (usize, usize) {
    let db = state.controller.db().db();
    let executor = crate::files::maintenance_executor::FilesMaintenanceExecutor::new(
        state.controller.files_manager(),
    );
    let manager = state.controller.maintenance();
    let mut done = 0;
    let mut failed = 0;

    for _ in 0..limit {
        let mut failure = false;
        let Some(job) = manager.run_next(|job| {
            let Ok(Some(hash_id)) = crate::db::master::find_hash_id(db, &job.hash) else {
                failure = true;
                return false;
            };

            match executor.run_job(db, hash_id, job.job_type) {
                Ok(true) => true,
                _ => {
                    failure = true;
                    false
                }
            }
        }) else {
            break;
        };

        if failure {
            failed += 1;
            // Unsupported or broken jobs are dropped instead of being retried
            // forever on every tick.
            manager.clear_jobs(&[job.hash], job.job_type);
        } else {
            done += 1;
        }
    }

    (done, failed)
}

/// Stops the API server and saves the page tree when the application exits.
pub fn shutdown() {
    save_session();

    with_state(|state| {
        state.controller.stop_api();
    });
}

fn with_state_mut<R>(call: impl FnOnce(&mut AppState) -> R) -> R {
    STATE.with(|slot| {
        let mut borrow = slot.borrow_mut();
        let state = borrow
            .as_mut()
            .expect("GUI application state is not installed");

        call(state)
    })
}

fn with_state<R>(call: impl FnOnce(&AppState) -> R) -> R {
    STATE.with(|slot| {
        let borrow = slot.borrow();
        let state = borrow
            .as_ref()
            .expect("GUI application state is not installed");

        call(state)
    })
}

impl AppState {
    fn status_command(&self) -> UiCommand {
        UiCommand::Status(render::status(self))
    }

    /// Progress-only updates for every file page; used while imports change
    /// so the loading panel can animate without a full results rebuild.
    pub(crate) fn query_progress_commands(&self) -> Vec<UiCommand> {
        self.query_ids()
            .into_iter()
            .filter_map(|id| {
                let page = self.find_query(&id)?;
                Some(UiCommand::PageProgress(render::query_progress(
                    page,
                    Some(&self.import_queue),
                )))
            })
            .collect()
    }

    pub(crate) fn refresh_all(&mut self) {
        let controller = self.controller.clone();
        let rules = self.auto_rules.clone();
        let mut review = std::mem::take(&mut self.duplicate_review);
        let mut queued = 0usize;

        refresh_notebook(&mut self.root, &controller, &rules, &mut review, &mut queued);

        self.duplicate_review = review;

        if queued > 0 {
            let _ = auto_resolution::save_state_review(self);
        }
    }

    pub(crate) fn query_ids(&self) -> Vec<String> {
        let mut ids = Vec::new();
        collect_query_ids(&self.root, &mut ids);
        ids
    }

    fn dispatch(&mut self, action: ffi::UiAction, target: &str, payload: &str) -> Vec<UiCommand> {
        match action {
            ffi::UiAction::PageSelected => self.page_selected_command(target),
            ffi::UiAction::SelectionChanged => {
                let hashes: Vec<String> = serde_json::from_str(payload).unwrap_or_default();
                self.set_selection(target, &hashes)
            }
            ffi::UiAction::NewPageMenu => {
                let notebook = if target.is_empty() {
                    self.active_notebook_id()
                } else {
                    target.to_string()
                };

                vec![UiCommand::ShowPageChooser(notebook)]
            }
            ffi::UiAction::NewPage => vec![UiCommand::ShowPageChooser(self.active_notebook_id())],
            ffi::UiAction::NewNotebook => {
                let notebook = self.active_notebook_id();
                self.create_page(&notebook, "notebook")
            }
            ffi::UiAction::NewDuplicates => {
                let notebook = self.active_notebook_id();
                self.create_page(&notebook, "duplicates")
            }
            ffi::UiAction::NewUrls => {
                let notebook = self.active_notebook_id();
                self.create_page(&notebook, "urls")
            }
            ffi::UiAction::NewGallery => {
                let notebook = self.active_notebook_id();
                self.create_page(&notebook, "gallery")
            }
            ffi::UiAction::NewWatcher => {
                let notebook = self.active_notebook_id();
                self.create_page(&notebook, "watcher")
            }
            ffi::UiAction::NewSimple => {
                let notebook = self.active_notebook_id();
                self.create_page(&notebook, "simple")
            }
            ffi::UiAction::Notify => vec![UiCommand::NotifyMessage(payload.to_string())],
            // Python has no subscriptions page: network -> subscriptions... opens
            // the manage subscriptions dialog.
            ffi::UiAction::ManageSubscriptions => download::manage_subscriptions(self),
            ffi::UiAction::NewSubscriptions => {
                let notebook = self.active_notebook_id();
                self.create_page(&notebook, "subscriptions")
            }
            // Python has no import folders page: file -> import folders opens
            // the edit import folders dialog.
            ffi::UiAction::ManageImportFolders => import_folders::manage(self),
            ffi::UiAction::NewImportFolders | ffi::UiAction::ImportFolders => {
                let notebook = self.active_notebook_id();
                import_folders::create_page(self, &notebook)
            }
            ffi::UiAction::ImportFolderAdd => import_folders::add(self, payload),
            ffi::UiAction::ImportFolderEdit => import_folders::edit(self, payload),
            ffi::UiAction::ImportFolderSave => import_folders::save_action(self, payload),
            ffi::UiAction::ImportFolderRemove => import_folders::remove(self, payload),
            ffi::UiAction::ImportFolderToggle => import_folders::toggle(self, payload),
            ffi::UiAction::ImportFolderCheck => import_folders::check_now(self, payload),
            ffi::UiAction::ImportFolderCheckAll => {
                let names: Vec<String> = self
                    .import_folders
                    .iter()
                    .map(|folder| folder.name.clone())
                    .collect();
                let mut commands = Vec::new();

                for name in names {
                    commands.extend(import_folders::check_now(self, &name));
                }

                if commands.is_empty() {
                    commands.push(UiCommand::NotifyMessage(
                        "there are no import folders".to_string(),
                    ));
                }

                commands
            }
            ffi::UiAction::NewExportFolders | ffi::UiAction::ExportFolders => {
                let notebook = self.active_notebook_id();
                export::create_page(self, &notebook)
            }
            // Python has no export folders page: file -> export folders opens
            // the edit export folders dialog.
            ffi::UiAction::ManageExportFolders => export::manage(self),
            ffi::UiAction::ExportFolderEdit => export::edit(self, payload),
            ffi::UiAction::ExportFolderSave => export::save_action(self, payload),
            ffi::UiAction::ExportFolderRemove => export::remove(self, payload),
            ffi::UiAction::ExportFolderToggle => export::toggle(self, payload),
            ffi::UiAction::ExportFolderRun => export::run_now(self, payload),
            ffi::UiAction::ExportFolderReview => export::review(self, payload),
            ffi::UiAction::ExportFolderRunAll => export::run_all(self),
            ffi::UiAction::ExportFolderReveal => export::reveal(self, payload),
            ffi::UiAction::ExportFolderTestExamples => export::test_examples(self, payload),
            ffi::UiAction::CreatePage => self.create_page(target, payload),
            ffi::UiAction::ClosePage => self.close_page(target),
            ffi::UiAction::CloseOtherPages => self.close_other_pages(target),
            ffi::UiAction::MovePage => {
                let request: Value = match serde_json::from_str(payload) {
                    Ok(value) => value,
                    Err(_) => {
                        return vec![UiCommand::NotifyMessage(
                            "move request is invalid".to_string(),
                        )];
                    }
                };

                let drop_target = request
                    .get("target")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                let position = request
                    .get("position")
                    .and_then(Value::as_str)
                    .unwrap_or("after");

                self.page_move_request(target, drop_target, position)
            }
            ffi::UiAction::DuplicatePage => self.duplicate_page_command(target),
            ffi::UiAction::HistoryBack => self.navigate_history(true),
            ffi::UiAction::HistoryForward => self.navigate_history(false),
            ffi::UiAction::PageManager => vec![UiCommand::PageManagerDialog(render::page_manager(self))],
            ffi::UiAction::CommandPalette => vec![UiCommand::CommandPalette],
            ffi::UiAction::PageManagerActivate => self.page_manager_activate_command(target),
            ffi::UiAction::PageManagerClose => {
                let ids: Vec<String> = serde_json::from_str(payload).unwrap_or_default();
                self.close_pages(&ids)
            }
            ffi::UiAction::SessionSaveAs => vec![UiCommand::PromptText {
                action: ffi::UiAction::SessionSave,
                title: "save session".to_string(),
                label: "session name".to_string(),
                initial: self.session_name.clone().unwrap_or_default(),
            }],
            ffi::UiAction::SessionSave => match sessions::save_named(self, payload) {
                Ok(name) => {
                    self.message = format!("saved session '{name}'");
                    vec![
                        UiCommand::Menus(render::menus(self)),
                        self.status_command(),
                    ]
                }
                Err(error) => vec![UiCommand::Message(
                    "save session".to_string(),
                    format!("could not save the session: {error}"),
                )],
            },
            ffi::UiAction::SessionBackup => match sessions::backup(self) {
                Ok(name) => {
                    self.message = format!("backed up session as '{name}'");
                    vec![
                        UiCommand::Menus(render::menus(self)),
                        self.status_command(),
                    ]
                }
                Err(error) => vec![UiCommand::Message(
                    "backup session".to_string(),
                    format!("could not back up the session: {error}"),
                )],
            },
            ffi::UiAction::SessionLoad => self.load_session_named(target, false, false),
            ffi::UiAction::SessionAppend => self.load_session_named(target, true, false),
            ffi::UiAction::SessionAppendBackup => self.load_session_named(target, true, true),
            ffi::UiAction::SessionDelete => vec![UiCommand::ConfirmAction {
                action: ffi::UiAction::SessionDeleteConfirmed,
                target: target.to_string(),
                title: "delete session".to_string(),
                text: format!("Delete the session '{target}'? This cannot be undone."),
            }],
            ffi::UiAction::SessionDeleteConfirmed => match sessions::delete_named(self, target) {
                Ok(()) => {
                    self.message = format!("deleted session '{target}'");
                    vec![
                        UiCommand::Menus(render::menus(self)),
                        self.status_command(),
                    ]
                }
                Err(error) => vec![UiCommand::Message(
                    "delete session".to_string(),
                    format!("could not delete the session: {error}"),
                )],
            },
            ffi::UiAction::RenamePage => self.rename_page(target, payload),
            ffi::UiAction::RefreshPage => {
                let id = self.page_id_or_active(target);
                self.refresh_page_commands(&id)
            }
            ffi::UiAction::Search => {
                let controller = self.controller.clone();

                if let Some(page) = self.find_query_mut(target) {
                    if page.query != payload {
                        page.query_undo.push(page.query.clone());

                        if page.query_undo.len() > 50 {
                            page.query_undo.remove(0);
                        }

                        page.query_redo.clear();
                    }

                    page.query = payload.to_string();
                    page.fixed_hashes = None;
                    page.loading = true;
                }

                search_history::record(&mut self.search_history, payload);
                let _ = search_history::persist(self);

                let mut commands = vec![UiCommand::Menus(render::menus(self))];

                if let Some(page) = self.find_query(target) {
                    commands.push(UiCommand::UpdatePage(render::query_page_state(
                        &controller,
                        page,
                    )));
                    commands.push(UiCommand::Defer {
                        action: ffi::UiAction::RunQuery,
                        target: target.to_string(),
                        payload: String::new(),
                        delay_ms: 1,
                    });
                }

                commands.push(self.status_command());
                commands
            }
            ffi::UiAction::RunQuery => {
                let controller = self.controller.clone();

                let state = {
                    let Some(page) = self.find_query_mut(target) else {
                        return vec![UiCommand::NotifyMessage(
                            "page is not a file page".to_string(),
                        )];
                    };

                    page.loading = false;
                    query::refresh_page(&controller, page);
                    render::query_page_state(&controller, page)
                };

                vec![
                    UiCommand::UpdatePage(state),
                    self.status_command(),
                ]
            }
            ffi::UiAction::OrDialog => {
                let initial = self
                    .find_query(target)
                    .map(|page| page.query.clone())
                    .unwrap_or_default();

                vec![UiCommand::PromptOr {
                    page_id: target.to_string(),
                    initial,
                }]
            }
            ffi::UiAction::OrInsert => {
                let controller = self.controller.clone();

                if let Some(page) = self.find_query_mut(target) {
                    let mut query = page.query.trim().to_string();

                    if !query.is_empty() {
                        query.push(' ');
                    }

                    query.push_str(payload);

                    if page.query != query {
                        page.query_undo.push(page.query.clone());
                        page.query_redo.clear();
                    }

                    page.query = query;
                    page.fixed_hashes = None;
                    page.loading = true;
                } else {
                    return vec![UiCommand::NotifyMessage("no page selected".to_string())];
                }

                let mut commands = vec![UiCommand::Menus(render::menus(self))];

                if let Some(page) = self.find_query(target) {
                    commands.push(UiCommand::UpdatePage(render::query_page_state(
                        &controller,
                        page,
                    )));
                    commands.push(UiCommand::Defer {
                        action: ffi::UiAction::RunQuery,
                        target: target.to_string(),
                        payload: String::new(),
                        delay_ms: 1,
                    });
                }

                commands.push(self.status_command());
                commands
            }
            ffi::UiAction::Notifications => vec![UiCommand::NotificationsDialog(render::notifications_state(
                self,
            ))],
            ffi::UiAction::NotificationsClear => {
                self.notifications.clear();
                vec![
                    UiCommand::Menus(render::menus(self)),
                    self.status_command(),
                ]
            }
            ffi::UiAction::DownloaderExport => {
                let Some(page_id) = self.active.clone().filter(|id| {
                    matches!(
                        model::find_node(&self.root, id),
                        Some(PageNode::Downloader(_))
                    )
                }) else {
                    return vec![UiCommand::NotifyMessage(
                        "select a downloader page first".to_string(),
                    )];
                };

                vec![UiCommand::ExportDownloader(page_id)]
            }
            ffi::UiAction::DownloaderImport => {
                let target = self.active.clone().unwrap_or_else(|| "root".to_string());
                vec![UiCommand::ImportDownloader(target)]
            }
            ffi::UiAction::DownloaderExportTo => {
                let path = PathBuf::from(payload);
                let Some(node) = model::find_node(&self.root, target) else {
                    return vec![UiCommand::NotifyMessage("page is missing".to_string())];
                };
                let PageNode::Downloader(page) = node else {
                    return vec![UiCommand::NotifyMessage(
                        "page is not a downloader".to_string(),
                    )];
                };

                let json = downloader_export::to_json(page);
                let text = match serde_json::to_string_pretty(&json) {
                    Ok(text) => text,
                    Err(error) => {
                        return vec![UiCommand::Message(
                            "export downloader".to_string(),
                            error.to_string(),
                        )];
                    }
                };

                match std::fs::write(&path, text) {
                    Ok(()) => {
                        self.message = format!("exported downloader to {}", path.display());
                        vec![
                            UiCommand::Menus(render::menus(self)),
                            self.status_command(),
                        ]
                    }
                    Err(error) => vec![UiCommand::Message(
                        "export downloader".to_string(),
                        format!("could not write the definition: {error}"),
                    )],
                }
            }
            ffi::UiAction::DownloaderImportFrom => {
                let text = match std::fs::read_to_string(payload) {
                    Ok(text) => text,
                    Err(error) => {
                        return vec![UiCommand::Message(
                            "import downloader".to_string(),
                            format!("could not read the definition: {error}"),
                        )];
                    }
                };
                let value: Value = match serde_json::from_str(&text) {
                    Ok(value) => value,
                    Err(error) => {
                        return vec![UiCommand::Message(
                            "import downloader".to_string(),
                            format!("the definition is not valid JSON: {error}"),
                        )];
                    }
                };

                let new_id = self.new_id("p");
                let page = match downloader_export::from_json(&value, &new_id) {
                    Ok(page) => page,
                    Err(error) => {
                        return vec![UiCommand::Message("import downloader".to_string(), error)];
                    }
                };

                let notebook_id = if self.find_notebook(target).is_some() {
                    target.to_string()
                } else {
                    model::parent_notebook_id(&self.root, target)
                        .unwrap_or_else(|| "root".to_string())
                };

                let Some(notebook) = self.find_notebook_mut(&notebook_id) else {
                    return vec![UiCommand::NotifyMessage(
                        "target notebook is missing".to_string(),
                    )];
                };

                notebook.add(PageNode::Downloader(page));
                notebook.current = Some(new_id.clone());
                self.select_page(&new_id);
                self.record_history(&new_id);
                self.refresh_all();
                self.message = "imported downloader definition".to_string();

                vec![
                    UiCommand::Menus(render::menus(self)),
                    UiCommand::BuildPages(render::pages(self)),
                    self.status_command(),
                ]
            }
            ffi::UiAction::ViewingStats => {
                let hash_bytes = if payload.is_empty() {
                    let Some(page) = self.active_query() else {
                        return vec![UiCommand::NotifyMessage("no page selected".to_string())];
                    };
                    let Some(hash) = page.focused.clone() else {
                        return vec![UiCommand::NotifyMessage("no file selected".to_string())];
                    };
                    hash
                } else {
                    let Some(hash) = query::unhex(payload) else {
                        return vec![UiCommand::NotifyMessage("invalid file hash".to_string())];
                    };
                    hash
                };

                vec![UiCommand::ViewingStatsDialog(render::viewing_stats(
                    &self.controller,
                    &hash_bytes,
                ))]
            }
            ffi::UiAction::UndoSearch => {
                let controller = self.controller.clone();
                let page_id = self.page_id_or_active(target);

                if let Some(page) = self.find_query_mut(&page_id) {
                    let Some(previous) = page.query_undo.pop() else {
                        return vec![UiCommand::NotifyMessage("no search to undo".to_string())];
                    };

                    page.query_redo.push(page.query.clone());
                    page.query = previous;
                    page.fixed_hashes = None;
                } else {
                    return vec![UiCommand::NotifyMessage("no page selected".to_string())];
                }

                let mut commands = vec![UiCommand::Menus(render::menus(self))];
                commands.extend(self.replace_query_and_refresh(&controller, &page_id));
                commands
            }
            ffi::UiAction::RedoSearch => {
                let controller = self.controller.clone();
                let page_id = self.page_id_or_active(target);

                if let Some(page) = self.find_query_mut(&page_id) {
                    let Some(next) = page.query_redo.pop() else {
                        return vec![UiCommand::NotifyMessage("no search to redo".to_string())];
                    };

                    page.query_undo.push(page.query.clone());
                    page.query = next;
                    page.fixed_hashes = None;
                } else {
                    return vec![UiCommand::NotifyMessage("no page selected".to_string())];
                }

                let mut commands = vec![UiCommand::Menus(render::menus(self))];
                commands.extend(self.replace_query_and_refresh(&controller, &page_id));
                commands
            }
            ffi::UiAction::ApplySearchHistory => {
                let controller = self.controller.clone();
                let page_id = self.page_id_or_active("");

                if let Some(page) = self.find_query_mut(&page_id) {
                    if page.query != target {
                        page.query_undo.push(page.query.clone());
                        page.query_redo.clear();
                    }

                    page.query = target.to_string();
                    page.fixed_hashes = None;
                } else {
                    return vec![UiCommand::NotifyMessage("no page selected".to_string())];
                }

                let mut commands = vec![UiCommand::Menus(render::menus(self))];
                commands.extend(self.replace_query_and_refresh(&controller, &page_id));
                commands
            }
            ffi::UiAction::SetLocation => {
                if let Some(page) = self.find_query_mut(target) {
                    if page.location != payload {
                        page.location = payload.to_string();
                        page.fixed_hashes = None;
                    }
                } else {
                    return vec![UiCommand::NotifyMessage(
                        "page is not a file page".to_string(),
                    )];
                }

                let controller = self.controller.clone();
                self.replace_query_and_refresh(&controller, target)
            }
            ffi::UiAction::SetTagService => {
                if let Some(page) = self.find_query_mut(target) {
                    if page.tag_service != payload {
                        page.tag_service = payload.to_string();
                        page.fixed_hashes = None;
                    }
                } else {
                    return vec![UiCommand::NotifyMessage(
                        "page is not a file page".to_string(),
                    )];
                }

                let controller = self.controller.clone();
                self.replace_query_and_refresh(&controller, target)
            }
            ffi::UiAction::Autocomplete => {
                let Some(page) = self.find_query(target) else {
                    return vec![];
                };

                let Some(tag_service) =
                    query::tag_service_id_for(&self.controller, &page.tag_service)
                else {
                    return vec![UiCommand::NotifyMessage("no local tag service".to_string())];
                };

                let tokens: Vec<&str> = payload.split_whitespace().collect();
                let mut entries = Vec::new();

                for count in (1..=tokens.len().min(3)).rev() {
                    let candidate = tokens[tokens.len() - count..].join(" ");

                    if candidate.to_ascii_lowercase().starts_with("system:") && count > 1 {
                        continue;
                    }

                    let candidate_entries = search::autocomplete_entries(
                        self.controller.db().db(),
                        tag_service,
                        &candidate,
                        25,
                    );

                    if !candidate_entries.is_empty() {
                        entries = candidate_entries;
                        break;
                    }
                }

                let entries: Vec<String> = entries
                    .into_iter()
                    .map(|entry| entry.text)
                    .collect();

                vec![UiCommand::Autocomplete(
                    crate::gui::qt::api::ffi::AutocompleteState {
                        page_id: page.id.clone(),
                        entries,
                    },
                )]
            }
            ffi::UiAction::SetSort => {
                let Ok(request) = serde_json::from_str::<Value>(payload) else {
                    return vec![UiCommand::NotifyMessage("invalid sort request".to_string())];
                };

                let sort = request
                    .get("sort")
                    .and_then(Value::as_str)
                    .map(str::to_string);
                let ascending = request.get("ascending").and_then(Value::as_bool);

                self.set_sort_command(target, sort.as_deref(), ascending)
            }
            ffi::UiAction::FavouriteSave => {
                let initial = self
                    .active_query()
                    .map(|page| {
                        if page.query.trim().is_empty() {
                            page.title.clone()
                        } else {
                            page.query.clone()
                        }
                    })
                    .unwrap_or_default();

                vec![UiCommand::PromptText {
                    action: ffi::UiAction::FavouriteSaveConfirmed,
                    title: "save favourite search".to_string(),
                    label: "name".to_string(),
                    initial,
                }]
            }
            ffi::UiAction::FavouriteSaveConfirmed => {
                let Some(page) = self.active_query() else {
                    return vec![UiCommand::NotifyMessage("no page selected".to_string())];
                };

                let favourite = favourites::FavouriteSearch {
                    name: payload.to_string(),
                    query: page.query.clone(),
                    location: page.location.clone(),
                    tag_service: page.tag_service.clone(),
                };
                let name = favourite.name.clone();
                favourites::add_or_replace(&mut self.favourites, favourite);
                let _ = favourites::persist(self);
                self.message = format!("saved favourite search '{name}'");

                vec![
                    UiCommand::Menus(render::menus(self)),
                    self.status_command(),
                ]
            }
            ffi::UiAction::FavouriteApply => {
                let Some(favourite) = self
                    .favourites
                    .iter()
                    .find(|favourite| favourite.name == target)
                    .cloned()
                else {
                    return vec![UiCommand::NotifyMessage(
                        "favourite search is missing".to_string(),
                    )];
                };

                let Some(page) = self.active_query() else {
                    return vec![UiCommand::NotifyMessage("no page selected".to_string())];
                };

                let page_id = page.id.clone();
                let controller = self.controller.clone();

                if let Some(page) = self.find_query_mut(&page_id) {
                    page.query = favourite.query;
                    page.location = favourite.location;
                    page.tag_service = favourite.tag_service;
                    page.fixed_hashes = None;
                }

                self.replace_query_and_refresh(&controller, &page_id)
            }
            ffi::UiAction::FavouriteDelete => vec![UiCommand::ConfirmAction {
                action: ffi::UiAction::FavouriteDeleteConfirmed,
                target: target.to_string(),
                title: "delete favourite search".to_string(),
                text: format!("Delete the favourite search '{target}'?"),
            }],
            ffi::UiAction::FavouriteDeleteConfirmed => {
                if favourites::remove(&mut self.favourites, target) {
                    let _ = favourites::persist(self);
                    self.message = format!("deleted favourite search '{target}'");
                }

                vec![
                    UiCommand::Menus(render::menus(self)),
                    self.status_command(),
                ]
            }
            ffi::UiAction::AddHashesToPage => {
                let hashes: Vec<String> = serde_json::from_str(payload).unwrap_or_default();
                self.add_hashes(target, &hashes)
            }
            ffi::UiAction::FocusHash => self.focus(target, payload),
            ffi::UiAction::OpenViewer => self.open_viewer(target, payload),
            ffi::UiAction::ViewerNav => self.viewer_nav(target, payload),
            ffi::UiAction::ViewerClose => {
                if let Some(context) = self.viewers.remove(target) {
                    record_viewing(
                        &self.controller,
                        &context.hash,
                        hydrus_time::get_now_ms().saturating_sub(context.opened_at_ms),
                        false,
                    );
                }

                vec![]
            }
            ffi::UiAction::ManageTags => self.manage_tags(target, payload, false),
            ffi::UiAction::ManageTagsActive => self.manage_tags("", "", true),
            ffi::UiAction::EditTimestamps => self.edit_timestamps(),
            ffi::UiAction::TimestampsSave => self.save_timestamps(payload),
            ffi::UiAction::TagAdd => metadata::tag_action(self, true, payload),
            ffi::UiAction::TagRemove => metadata::tag_action(self, false, payload),
            ffi::UiAction::CopyTags => metadata::copy_tags(self),
            ffi::UiAction::PasteTags => metadata::apply_tags_selection(self, payload),
            ffi::UiAction::PasteTagsPrompt | ffi::UiAction::ApplyTagsSelection => vec![UiCommand::PromptText {
                action: ffi::UiAction::ApplyTagsSelectionApply,
                title: "apply tags to selection".to_string(),
                label: "tags, one per line; prefix - to remove".to_string(),
                initial: String::new(),
            }],
            ffi::UiAction::ApplyTagsSelectionApply => metadata::apply_tags_selection(self, payload),
            ffi::UiAction::RatingSet => metadata::rating_action(self, payload),
            ffi::UiAction::NotesManage | ffi::UiAction::ManageNotesActive => {
                if action == ffi::UiAction::ManageNotesActive {
                    metadata::manage_notes_active(self)
                } else {
                    metadata::notes_action(self, payload, false)
                }
            }
            ffi::UiAction::NotesSave => metadata::notes_action(self, payload, true),
            ffi::UiAction::ApplyTags => self.apply_tags(payload),
            ffi::UiAction::ArchiveFiles => self.set_inbox(target, payload, false),
            ffi::UiAction::InboxFiles => self.set_inbox(target, payload, true),
            ffi::UiAction::ArchiveHashes => self.set_inbox_many(target, payload, false),
            ffi::UiAction::InboxHashes => self.set_inbox_many(target, payload, true),
            ffi::UiAction::DeleteHashes => self.delete_files_many(target, payload),
            ffi::UiAction::ArchiveActive => self.set_inbox_active(false),
            ffi::UiAction::InboxActive => self.set_inbox_active(true),
            ffi::UiAction::DeleteFiles => self.delete_files(target, payload),
            ffi::UiAction::ImportFiles | ffi::UiAction::ReviewLocalImports => local_import::open(self),
            ffi::UiAction::LocalImportOpenPaths => local_import::open_paths(self, payload),
            ffi::UiAction::LocalImportAddPaths => local_import::add_paths(self, payload),
            ffi::UiAction::LocalImportRemove => local_import::remove_paths(self, payload),
            ffi::UiAction::LocalImportSearchSubdirectories => {
                local_import::set_search_subdirectories(self, payload)
            }
            ffi::UiAction::LocalImportDeleteAfterSuccess => {
                local_import::set_delete_after_success(self, payload)
            }
            ffi::UiAction::LocalImportImport => local_import::import_now(self),
            ffi::UiAction::LocalImportOptions => import_options::open_caller(self, "local_import"),
            ffi::UiAction::LocalImportTaggingOpen => local_import::tagging_open(self),
            ffi::UiAction::LocalImportTaggingSave => local_import::tagging_save(self, payload),
            ffi::UiAction::RepairFileLocations => file_locations::open(self),
            ffi::UiAction::RepairLocationsSet => file_locations::set_locations(self, payload),
            ffi::UiAction::RepairLocationsProbe => file_locations::probe(self, payload),
            ffi::UiAction::RepairLocationsApply => file_locations::apply(self, payload),
            ffi::UiAction::ImportOptions => import_options::open(self),
            ffi::UiAction::ImportOptionsSave => import_options::save_action(self, payload),
            ffi::UiAction::ImportUrl => vec![UiCommand::PromptUrl],
            ffi::UiAction::DownloadUrl => self.download_one_url(payload),
            ffi::UiAction::ImportPaths => self.import_paths(payload),
            ffi::UiAction::DownloaderStart => download::start(self, target, payload),
            ffi::UiAction::DownloaderPause => download::pause(self, target),
            ffi::UiAction::DownloaderJobsSet => download::jobs_set(self, target, payload),
            ffi::UiAction::DownloaderSetSite => download::set_site(self, target, payload),
            ffi::UiAction::DownloaderSetFormula => download::set_formula(self, target, payload),
            ffi::UiAction::WatcherAddUrl => download::set_watcher_url(self, target, payload),
            ffi::UiAction::WatcherRemove => download::remove_watcher(self, target),
            ffi::UiAction::WatcherCheckerOptions => vec![UiCommand::NotifyMessage(
                "checker options are not implemented yet".to_string(),
            )],
            ffi::UiAction::DownloaderClear => download::clear(self, target),
            ffi::UiAction::DownloaderRetry => download::retry(self, target, payload),
            ffi::UiAction::DownloaderOpenResults => download::open_results(self, target),
            ffi::UiAction::DownloaderImportOptions => download::open_import_options(self, target),
            ffi::UiAction::DownloaderImportOptionsSave => download::save_import_options(self, payload),
            ffi::UiAction::SubscriptionImportOptions => {
                download::open_subscription_import_options(self, payload.trim())
            }
            ffi::UiAction::SubscriptionImportOptionsSave => {
                download::save_subscription_import_options(self, payload)
            }
            ffi::UiAction::FileSeedCache => seeds::open_file_seed_cache(self, target),
            ffi::UiAction::FileSeedCacheAction => seeds::file_seed_action(self, payload),
            ffi::UiAction::GallerySeedLog => seeds::open_gallery_seed_log(self, target),
            ffi::UiAction::GallerySeedLogAction => seeds::gallery_seed_action(self, payload),
            ffi::UiAction::WatcherCheck => download::start(self, target, payload),
            ffi::UiAction::WatcherAuto => {
                if let Some(page) = self.find_downloader_mut(target) {
                    page.auto_watch = !page.auto_watch;
                    page.status = if page.auto_watch {
                        "watching".to_string()
                    } else {
                        "automatic checks off".to_string()
                    };
                }
                self.downloader_update(target)
            }
            ffi::UiAction::SubscriptionCheck => download::start_subscription(self, target, payload),
            ffi::UiAction::SubscriptionToggle => download::toggle_subscription(self, target, payload),
            ffi::UiAction::SubscriptionDelete => download::delete_subscription(self, target, payload),
            ffi::UiAction::SubscriptionEdit => download::edit_subscription_dialog(self, target, payload),
            ffi::UiAction::SubscriptionEditSave => download::save_subscription_dialog(self, target, payload),
            ffi::UiAction::SubscriptionAdd => download::add_subscription(self, target, payload),
            ffi::UiAction::SubscriptionDuplicate => {
                download::duplicate_subscription(self, target, payload)
            }
            ffi::UiAction::SubscriptionLowercaseQueries => {
                download::lowercase_subscription_queries(self, target, payload)
            }
            ffi::UiAction::SubscriptionScrubDelays => {
                download::scrub_subscription_delays(self, target, payload)
            }
            ffi::UiAction::SubscriptionReset => download::reset_subscription(self, target, payload),
            ffi::UiAction::SubscriptionImportOptionsClear => {
                download::clear_subscription_import_options(self, target, payload)
            }
            ffi::UiAction::SubscriptionOpenResults => {
                download::open_subscription_results(self, target, payload)
            }
            ffi::UiAction::DuplicatesRules => vec![UiCommand::DuplicatesRulesDialog(
                auto_resolution::rules_dialog_state(self),
            )],
            ffi::UiAction::DuplicatesMergeOptionsDialog => vec![UiCommand::DuplicatesMergeOptionsDialog(
                duplicates::merge_options_dialog_state(self),
            )],
            ffi::UiAction::DuplicatesRuleAdd | ffi::UiAction::DuplicatesRuleSave => {
                auto_resolution::save_rule(self, payload)
            }
            ffi::UiAction::DuplicatesRuleRemove => auto_resolution::remove_rule(self, payload),
            ffi::UiAction::DuplicatesRulePreview => auto_resolution::preview_rule(self, payload),
            ffi::UiAction::DuplicatesReview => auto_resolution::review_open(self),
            ffi::UiAction::DuplicatesShowPair => auto_resolution::show_pair_action(self, payload),
            ffi::UiAction::DuplicatesMergeOptions => duplicates::save_merge_options_action(self, payload),
            ffi::UiAction::DuplicatesReviewApply => auto_resolution::review_apply(self, payload),
            ffi::UiAction::DuplicatesAutoToggle => {
                let mut options = self.controller.options();
                let enabled = !options.get_boolean("duplicates_auto_resolution_enabled");
                options.set_boolean("duplicates_auto_resolution_enabled", enabled);
                self.controller.replace_options(options);
                let _ = self.controller.save_options();
                self.message = if enabled {
                    "duplicate auto-resolution enabled".to_string()
                } else {
                    "duplicate auto-resolution disabled".to_string()
                };
                self.refresh_all();
                let mut commands = self.refresh_commands();
                commands.push(UiCommand::DuplicatesRulesDialog(
                    auto_resolution::rules_dialog_state(self),
                ));
                commands.push(UiCommand::Status(render::status(self)));
                commands
            }
            ffi::UiAction::DuplicatesAlternates | ffi::UiAction::DuplicatesBatchSize | ffi::UiAction::DuplicatesCustom | ffi::UiAction::DuplicatesCustomAction | ffi::UiAction::DuplicatesDeleteBoth | ffi::UiAction::DuplicatesFalse | ffi::UiAction::DuplicatesGroupAction | ffi::UiAction::DuplicatesKeepA | ffi::UiAction::DuplicatesKeepAKeep | ffi::UiAction::DuplicatesKeepB | ffi::UiAction::DuplicatesKeepBKeep | ffi::UiAction::DuplicatesNextBatch | ffi::UiAction::DuplicatesRefresh | ffi::UiAction::DuplicatesSame | ffi::UiAction::DuplicatesSearchContext | ffi::UiAction::DuplicatesSkip => {
                duplicates::action(self, target, action, payload)
            }
            ffi::UiAction::Options => vec![UiCommand::OptionsDialog(crate::gui::qt::api::ffi::OptionsDialogState { json: options::options_json(self) })],
            ffi::UiAction::OptionsApply => options::apply_options(self, payload),
            ffi::UiAction::ManageServices => vec![UiCommand::ServicesDialog(services::services_state(self))],
            ffi::UiAction::ApiPermissions => api_permissions::open(self),
            ffi::UiAction::ApiPermissionsSave => api_permissions::save(self, payload),
            ffi::UiAction::ApiPermissionsDelete => api_permissions::delete(self, payload),
            ffi::UiAction::ServiceSave => services::save_service(self, payload),
            ffi::UiAction::ServiceDelete => services::delete_service(self, payload),
            ffi::UiAction::LoginSave => services::login_save(self, payload),
            ffi::UiAction::LoginRemove => services::login_remove(self, payload),
            ffi::UiAction::AdminConnect => services::admin_connect(self, payload),
            ffi::UiAction::AdminServiceAction => services::admin_service_action(self, payload),
            ffi::UiAction::AdminAddService => services::admin_add_service(self, payload),
            ffi::UiAction::AdminEditService => services::admin_edit_service(self, payload),
            ffi::UiAction::AdminRemoveService => services::admin_remove_service(self, payload),
            ffi::UiAction::AdminAccountAction => services::admin_account_action(self, payload),
            ffi::UiAction::AccountTypesOpen => services::open_account_types(self, payload),
            ffi::UiAction::AccountTypeSave => services::save_account_type(self, payload),
            ffi::UiAction::AccountTypeDelete => services::delete_account_type(self, payload),
            ffi::UiAction::ServiceAdd | ffi::UiAction::ServiceRename => services::save_service(self, payload),
            ffi::UiAction::OptionsNetwork => vec![UiCommand::NetworkDialog(options::network_state(self))],
            ffi::UiAction::NetworkHeaderAdd
            | ffi::UiAction::NetworkHeaderRemove
            | ffi::UiAction::NetworkPause
            | ffi::UiAction::NetworkPoolRemove
            | ffi::UiAction::NetworkPoolSet
            | ffi::UiAction::NetworkRuleRemove
            | ffi::UiAction::NetworkRuleSet
            | ffi::UiAction::NetworkSave
            | ffi::UiAction::NetworkUrlClassAdd
            | ffi::UiAction::NetworkUrlClassRemove
            | ffi::UiAction::NetworkUrlClassSave => {
                options::network_action(self, action, payload)
            }
            ffi::UiAction::Exit => vec![UiCommand::Quit],
            ffi::UiAction::OpenDbDir => vec![UiCommand::OpenPath(
                self.controller.db_dir().display().to_string(),
            )],
            ffi::UiAction::OpenInstallDir => vec![UiCommand::OpenPath(dirs().base_dir.display().to_string())],
            ffi::UiAction::Docs => vec![UiCommand::OpenUrl(
                "https://hydrusnetwork.github.io/hydrus/".to_string(),
            )],
            ffi::UiAction::About => vec![UiCommand::AboutDialog(render::about(self))],
            ffi::UiAction::DbInfo => {
                let version = self.controller.db().get_version().unwrap_or(0);
                let services = self.controller.services().get_num_services();
                let dir = self.controller.db_dir().display();
                let files = self.query_ids().len();

                vec![UiCommand::Message(
                    "database".to_string(),
                    format!(
                        "database directory: {dir}\nversion: {version}\nservices: {services}\nopen pages: {files}"
                    ),
                )]
            }
            ffi::UiAction::ServiceInfo => {
                let name = target;
                let service = self
                    .controller
                    .services()
                    .get_service_key_from_name(name)
                    .and_then(|key| self.controller.services().get_service(key).cloned());

                match service {
                    Some(service) => vec![UiCommand::Message(
                        service.name.clone(),
                        format!(
                            "name: {}\ntype: {:?}\nservice id: {}\nkey: {}",
                            service.name,
                            service.service_type,
                            service.service_id,
                            query::hex(&service.service_key)
                        ),
                    )],
                    None => vec![UiCommand::NotifyMessage(format!(
                        "service '{name}' is missing"
                    ))],
                }
            }
            ffi::UiAction::PauseNetwork => options::network_action(self, ffi::UiAction::NetworkPause, ""),
            ffi::UiAction::TagSiblings => tag_relations::show(self, tag_relations::SIBLINGS),
            ffi::UiAction::TagParents => tag_relations::show(self, tag_relations::PARENTS),
            ffi::UiAction::TagRelationsAdd => tag_relations::action(self, payload, true),
            ffi::UiAction::TagRelationsRemove => tag_relations::action(self, payload, false),
            ffi::UiAction::OrphanFilesScan => orphans::open(self),
            ffi::UiAction::OrphanFilesDelete => orphans::delete(self, payload),
            ffi::UiAction::OrphanFilesMove => orphans::move_to(self, payload),
            ffi::UiAction::DbBackupOpen => backup::open(self),
            ffi::UiAction::DbBackupSetPath => backup::set_path(self, payload),
            ffi::UiAction::DbBackupRun => backup::run(self),
            ffi::UiAction::DbCheck => {
                let result =
                    self.controller
                        .db()
                        .db()
                        .query_one("PRAGMA quick_check", &[], |row| row.get::<_, String>(0));

                match result {
                    Ok(status) => vec![UiCommand::Message(
                        "database check".to_string(),
                        format!(
                            "sqlite quick_check: {}",
                            status.unwrap_or_else(|| "no result".to_string())
                        ),
                    )],
                    Err(error) => vec![UiCommand::Message(
                        "database check".to_string(),
                        format!("database check failed: {error}"),
                    )],
                }
            }
            ffi::UiAction::DbVacuum => {
                let result = self.controller.db().db().execute_batch("VACUUM;");

                match result {
                    Ok(()) => vec![UiCommand::Message(
                        "database vacuum".to_string(),
                        "the database was vacuumed successfully".to_string(),
                    )],
                    Err(error) => vec![UiCommand::Message(
                        "database vacuum".to_string(),
                        format!("vacuum failed: {error}"),
                    )],
                }
            }
            ffi::UiAction::DbAnalyze => {
                let result = self
                    .controller
                    .db()
                    .db()
                    .execute_batch("PRAGMA optimize; ANALYZE;");

                match result {
                    Ok(()) => vec![UiCommand::Message(
                        "analyze database".to_string(),
                        "statistics were rebuilt and the query planner was optimized".to_string(),
                    )],
                    Err(error) => vec![UiCommand::Message(
                        "analyze database".to_string(),
                        format!("analyze failed: {error}"),
                    )],
                }
            }
            ffi::UiAction::DbVacuumInfo => {
                let db = self.controller.db().db();
                let page_count = pragma_i64(db, "page_count");
                let freelist = pragma_i64(db, "freelist_count");
                let page_size = pragma_i64(db, "page_size");
                let free_bytes = freelist.saturating_mul(page_size);
                let total_bytes = page_count.saturating_mul(page_size);

                vec![UiCommand::Message(
                    "vacuum data".to_string(),
                    format!(
                        "pages: {page_count}\nfree pages: {freelist}\npage size: {page_size} bytes\ntotal: {}\nreusable: {}",
                        human_bytes(total_bytes as u64),
                        human_bytes(free_bytes as u64)
                    ),
                )]
            }
            ffi::UiAction::DbDeferredDeletes => {
                let db = self.controller.db().db();
                let trash = query::trash_service_id(&self.controller)
                    .and_then(|service_id| {
                        files_storage::get_hash_ids(db, service_id, ContentStatus::Current).ok()
                    })
                    .map(|ids| ids.len())
                    .unwrap_or(0);

                vec![UiCommand::Message(
                    "deferred delete tables".to_string(),
                    format!("files waiting in trash: {trash}"),
                )]
            }
            ffi::UiAction::DbFileMaintenance => vec![UiCommand::Message(
                "file maintenance jobs".to_string(),
                render::maintenance_json(self),
            )],
            ffi::UiAction::DbRunFileMaintenance => {
                let (done, failed) = run_maintenance(self, 500);
                self.message = format!("file maintenance finished: {done} done, {failed} failed");
                vec![
                    UiCommand::NotifyMessage(self.message.clone()),
                    UiCommand::Message(
                        "file maintenance jobs".to_string(),
                        format!(
                            "{}\n\n{done} job(s) completed, {failed} job(s) failed.",
                            render::maintenance_json(self)
                        ),
                    ),
                    self.status_command(),
                ]
            }
            ffi::UiAction::DbFixArchiveTimes => {
                let manager = self.controller.services();
                let Some(service) = manager.get_service(b"my files") else {
                    return vec![UiCommand::NotifyMessage(
                        "the 'my files' service is missing".to_string(),
                    )];
                };

                match files_timestamps::fix_missing_archive_times(
                    self.controller.db().db(),
                    service.service_id,
                    hydrus_time::get_now(),
                ) {
                    Ok(fixed) => {
                        self.message = format!("fixed archive times for {fixed} files");
                        vec![
                            UiCommand::NotifyMessage(self.message.clone()),
                            self.status_command(),
                        ]
                    }
                    Err(error) => vec![UiCommand::NotifyMessage(format!(
                        "could not fix archive times: {error}"
                    ))],
                }
            }
            ffi::UiAction::DbStats => {
                let db = self.controller.db().db();
                let version = self.controller.db().get_version().unwrap_or(0);
                let services = self.controller.services().get_num_services();
                let files = scalar_i64(db, "SELECT COUNT(*) FROM files_info");
                let inbox = files_storage::get_num_inbox(db).unwrap_or(0);
                let trash = query::trash_service_id(&self.controller)
                    .and_then(|service_id| {
                        files_storage::get_hash_ids(db, service_id, ContentStatus::Current).ok()
                    })
                    .map(|ids| ids.len() as i64)
                    .unwrap_or(0);
                let tags = query::tag_service_id(&self.controller)
                    .and_then(|service_id| {
                        mappings::get_num_mappings(db, service_id, ContentStatus::Current).ok()
                    })
                    .unwrap_or(0);

                vec![UiCommand::Message(
                    "how boned am I?".to_string(),
                    format!(
                        "database version: {version}\nservices: {services}\nfiles with metadata: {files}\ninbox: {inbox}\ntrash: {trash}\nlocal tag mappings: {tags}"
                    ),
                )]
            }
            ffi::UiAction::Pending => vec![UiCommand::NotifyMessage(
                "there is no pending upload in this local-only build".to_string(),
            )],
            ffi::UiAction::Undo => match self.undo_closed_page() {
                Some(id) => {
                    self.message = format!("restored page '{id}'");
                    vec![
                        UiCommand::Menus(render::menus(self)),
                        UiCommand::BuildPages(render::pages(self)),
                        self.status_command(),
                    ]
                }
                None => vec![UiCommand::NotifyMessage("nothing to undo".to_string())],
            },
            unknown => vec![UiCommand::NotifyMessage(format!(
                "unknown action: {unknown:?}"
            ))],
        }
    }

    fn page_id_or_active(&self, target: &str) -> String {
        if target.is_empty() {
            self.active.clone().unwrap_or_default()
        } else {
            target.to_string()
        }
    }

    pub(crate) fn refresh_page_commands(&mut self, page_id: &str) -> Vec<UiCommand> {
        let controller = self.controller.clone();

        if let Some(page) = self.find_query_mut(page_id) {
            query::refresh_page(&controller, page);
            let state = render::query_page_state(&controller, page);

            return vec![
                UiCommand::UpdatePage(state),
                self.status_command(),
            ];
        }

        if self.find_downloader(page_id).is_some() {
            return self.downloader_update(page_id);
        }

        if let Some(page) = self.find_duplicates(page_id) {
            return vec![
                UiCommand::UpdateDuplicates(render::duplicates_page_state(
                    &self.controller,
                    page,
                    &self.auto_rules,
                )),
                self.status_command(),
            ];
        }

        if self.find_subscriptions(page_id).is_some() {
            return vec![
                UiCommand::UpdateSubscriptions(render::subscriptions_page_state(self, page_id)),
                self.status_command(),
            ];
        }

        if self.find_import_folders(page_id).is_some() {
            return vec![
                UiCommand::UpdateImportFolders(
                    page_id.to_string(),
                    import_folders::folder_page_state(self, page_id),
                ),
                self.status_command(),
            ];
        }

        vec![UiCommand::NotifyMessage(
            "this page cannot be refreshed".to_string(),
        )]
    }

    pub(crate) fn downloader_update(&self, page_id: &str) -> Vec<UiCommand> {
        match self.find_downloader(page_id) {
            Some(page) => vec![
                UiCommand::UpdateDownloader(render::downloader_page_state(
                    &self.controller,
                    page,
                )),
                self.status_command(),
            ],
            None => vec![],
        }
    }

    fn replace_query_and_refresh(
        &mut self,
        controller: &Controller,
        page_id: &str,
    ) -> Vec<UiCommand> {
        let state = {
            let Some(page) = self.find_query_mut(page_id) else {
                return vec![UiCommand::NotifyMessage(
                    "page is not a file page".to_string(),
                )];
            };

            query::refresh_page(controller, page);
            render::query_page_state(controller, page)
        };

        vec![
            UiCommand::UpdatePage(state),
            self.status_command(),
        ]
    }

    pub(crate) fn refresh_commands(&mut self) -> Vec<UiCommand> {
        let controller = self.controller.clone();
        let ids = self.query_ids();
        let mut commands = Vec::new();

        for id in ids {
            let state = {
                let Some(page) = self.find_query_mut(&id) else {
                    continue;
                };
                query::refresh_page(&controller, page);
                render::query_page_state(&controller, page)
            };

            commands.push(UiCommand::UpdatePage(state));
        }

        commands.push(self.status_command());
        commands
    }

    fn create_page(&mut self, notebook_id: &str, kind: &str) -> Vec<UiCommand> {
        let controller = self.controller.clone();
        let mut query_to_refresh = None;
        let page_id;

        if kind == "notebook" {
            let new_notebook_id = self.new_id("n");
            let query_id = self.new_id("p");
            let mut notebook = Notebook::new(new_notebook_id, "page of pages");
            notebook.add(PageNode::Query(QueryPage::new(
                query_id.clone(),
                "files",
                "my files",
            )));
            page_id = query_id.clone();
            query_to_refresh = Some(query_id);

            let Some(parent) = self.find_notebook_mut(notebook_id) else {
                return vec![UiCommand::NotifyMessage("notebook is missing".to_string())];
            };
            parent.add(PageNode::Notebook(notebook));
        } else if let Some(location) = kind.strip_prefix("query:") {
            let id = self.new_id("p");
            let title = match location {
                "trash" => "trash",
                "all local files" => "all local files",
                _ => "files",
            };
            let node = PageNode::Query(QueryPage::new(id.clone(), title, location));
            page_id = id.clone();
            query_to_refresh = Some(id.clone());

            let Some(parent) = self.find_notebook_mut(notebook_id) else {
                return vec![UiCommand::NotifyMessage("notebook is missing".to_string())];
            };
            parent.add(node);
        } else if kind == "subscriptions" {
            let id = self.new_id("p");
            page_id = id.clone();

            let Some(parent) = self.find_notebook_mut(notebook_id) else {
                return vec![UiCommand::NotifyMessage("notebook is missing".to_string())];
            };
            parent.add(PageNode::Subscriptions(SubscriptionsPage::new(id)));
        } else if kind == "import_folders" {
            return import_folders::create_page(self, notebook_id);
        } else if kind == "export_folders" {
            return export::create_page(self, notebook_id);
        } else if kind == "duplicates" {
            let id = self.new_id("p");
            page_id = id.clone();
            let mut page = DuplicatesPage::new(id);
            let (_, queued) = duplicates::refresh_for(
                &controller,
                &mut page,
                &self.auto_rules,
                &mut self.duplicate_review,
            );

            if queued > 0 {
                let _ = auto_resolution::save_state_review(self);
            }

            let Some(parent) = self.find_notebook_mut(notebook_id) else {
                return vec![UiCommand::NotifyMessage("notebook is missing".to_string())];
            };
            parent.add(PageNode::Duplicates(page));
        } else if let Some(kind) = DownloaderKind::parse(kind) {
            let id = self.new_id("p");
            page_id = id.clone();
            let mut page = DownloaderPage::new(id, kind);

            if kind == DownloaderKind::Watcher {
                let period = controller.options().get_integer("watcher_check_period");

                page.check_period = if period >= 60 {
                    period.clamp(60, 30 * 86_400)
                } else {
                    3600
                };
            }

            let Some(parent) = self.find_notebook_mut(notebook_id) else {
                return vec![UiCommand::NotifyMessage("notebook is missing".to_string())];
            };
            parent.add(PageNode::Downloader(page));
        } else {
            let id = self.new_id("p");
            let (title, message) = placeholder_for(kind);
            page_id = id.clone();

            let Some(parent) = self.find_notebook_mut(notebook_id) else {
                return vec![UiCommand::NotifyMessage("notebook is missing".to_string())];
            };
            parent.add(PageNode::Placeholder(PlaceholderPage {
                id,
                title,
                message,
                status: String::new(),
            }));
        }

        if let Some(id) = query_to_refresh {
            if let Some(page) = self.find_query_mut(&id) {
                query::refresh_page(&controller, page);
            }
        }

        self.active = Some(page_id.clone());

        self.record_history(&page_id);

        vec![
            UiCommand::BuildPages(render::pages(self)),
            self.status_command(),
        ]
    }

    fn close_page(&mut self, page_id: &str) -> Vec<UiCommand> {
        let mut ids = Vec::new();

        if let Some(node) = model::find_node(&self.root, page_id) {
            collect_page_ids(node, &mut ids);
        }

        let parent_id = model::parent_notebook_id(&self.root, page_id);
        let index = parent_id.as_deref().and_then(|parent_id| {
            self.find_notebook(parent_id).and_then(|parent| {
                parent
                    .children
                    .iter()
                    .position(|child| child.id() == page_id)
            })
        });
        let closed_node = model::find_node(&self.root, page_id).cloned();

        if !self.remove_page(page_id) {
            return vec![UiCommand::NotifyMessage("page is missing".to_string())];
        }

        if let (Some(parent_id), Some(index), Some(node)) = (parent_id, index, closed_node) {
            self.push_closed_page(parent_id, index, node);
        }

        let mut known = Vec::new();
        for child in &self.root.children {
            collect_page_ids(child, &mut known);
        }
        self.history
            .retain(|id| known.iter().any(|known_id| known_id == id));

        let mut commands = Vec::new();

        for id in &ids {
            for viewer in self.close_viewers_for_page(id) {
                commands.push(UiCommand::CloseViewer(viewer));
            }
        }

        if self
            .active
            .as_deref()
            .is_some_and(|active| ids.iter().any(|id| id.as_str() == active))
            || self.active.is_none()
        {
            self.active =
                model::current_leaf(&self.root).or_else(|| model::first_page_id(&self.root));
        }

        commands.push(UiCommand::BuildPages(render::pages(self)));
        commands.push(self.status_command());

        commands
    }

    pub(crate) fn page_selected_command(&mut self, page_id: &str) -> Vec<UiCommand> {
        self.active = Some(page_id.to_string());
        self.record_history(page_id);

        let mut commands = vec![self.status_command()];

        if self.select_page(page_id) {
            commands.insert(0, UiCommand::Menus(render::menus(self)));
        }

        commands
    }

    pub(crate) fn page_manager_activate_command(&mut self, page_id: &str) -> Vec<UiCommand> {
        if self.select_page(page_id) {
            vec![
                UiCommand::BuildPages(render::pages(self)),
                UiCommand::Menus(render::menus(self)),
                self.status_command(),
            ]
        } else {
            vec![UiCommand::NotifyMessage("page is missing".to_string())]
        }
    }

    pub(crate) fn set_sort_command(
        &mut self,
        page_id: &str,
        sort: Option<&str>,
        ascending: Option<bool>,
    ) -> Vec<UiCommand> {
        if let Some(page) = self.find_query_mut(page_id) {
            if let Some(sort) = sort {
                page.sort = sort.to_string();
            }

            if let Some(ascending) = ascending {
                page.sort_ascending = ascending;
            }
        } else {
            return vec![UiCommand::NotifyMessage("page is not a file page".to_string())];
        }

        let controller = self.controller.clone();
        self.replace_query_and_refresh(&controller, page_id)
    }

    fn close_other_pages(&mut self, page_id: &str) -> Vec<UiCommand> {
        let Some(parent_id) = model::parent_notebook_id(&self.root, page_id) else {
            return vec![UiCommand::NotifyMessage("page is missing".to_string())];
        };

        let mut removed = Vec::new();
        let mut closed = Vec::new();

        {
            let Some(parent) = self.find_notebook_mut(&parent_id) else {
                return vec![UiCommand::NotifyMessage("notebook is missing".to_string())];
            };

            let mut kept = Vec::new();

            for (index, child) in parent.children.drain(..).enumerate() {
                if child.id() == page_id {
                    kept.push(child);
                } else {
                    collect_page_ids(&child, &mut removed);
                    closed.push((parent_id.clone(), index, child));
                }
            }

            parent.children = kept;
            parent.current = Some(page_id.to_string());
        }

        for (parent_id, index, node) in closed {
            self.push_closed_page(parent_id, index, node);
        }

        let mut known = Vec::new();
        for child in &self.root.children {
            collect_page_ids(child, &mut known);
        }
        self.history
            .retain(|id| known.iter().any(|known_id| known_id == id));

        let mut commands = Vec::new();

        for id in removed {
            for viewer in self.close_viewers_for_page(&id) {
                commands.push(UiCommand::CloseViewer(viewer));
            }
        }

        self.active = Some(page_id.to_string());
        commands.push(UiCommand::BuildPages(render::pages(self)));
        commands.push(self.status_command());

        commands
    }

    fn page_move_request(&mut self, page_id: &str, target: &str, position: &str) -> Vec<UiCommand> {
        match self.move_page(page_id, target, position) {
            Ok(()) => {
                self.record_history(page_id);
                vec![
                    UiCommand::BuildPages(render::pages(self)),
                    UiCommand::Menus(render::menus(self)),
                    self.status_command(),
                ]
            }
            Err(error) => vec![UiCommand::NotifyMessage(error)],
        }
    }

    fn close_pages(&mut self, ids: &[String]) -> Vec<UiCommand> {
        let mut commands = Vec::new();

        for id in ids {
            commands.extend(self.close_page(id));
        }

        commands
    }

    fn duplicate_page_command(&mut self, page_id: &str) -> Vec<UiCommand> {
        match self.duplicate_page(page_id) {
            Ok(id) => {
                self.record_history(&id);
                vec![
                    UiCommand::BuildPages(render::pages(self)),
                    UiCommand::Menus(render::menus(self)),
                    self.status_command(),
                ]
            }
            Err(error) => vec![UiCommand::NotifyMessage(error)],
        }
    }

    fn navigate_history(&mut self, back: bool) -> Vec<UiCommand> {
        let target = if back {
            self.history_back()
        } else {
            self.history_forward()
        };

        match target {
            Some(id) => {
                self.select_page(&id);
                vec![
                    UiCommand::BuildPages(render::pages(self)),
                    UiCommand::Menus(render::menus(self)),
                    self.status_command(),
                ]
            }
            None => vec![self.status_command()],
        }
    }

    fn load_session_named(&mut self, name: &str, append: bool, backup: bool) -> Vec<UiCommand> {
        let loaded = if backup {
            sessions::load_backup(self, name)
        } else {
            sessions::load_named(self, name)
        };

        let loaded = match loaded {
            Ok(loaded) => loaded,
            Err(error) => {
                return vec![UiCommand::Message("load session".to_string(), error)];
            }
        };

        if append {
            let added = self.append_session(loaded);
            self.refresh_all();
            self.message = format!("appended {added} page groups from '{name}'");
            return vec![
                UiCommand::Menus(render::menus(self)),
                UiCommand::BuildPages(render::pages(self)),
                self.status_command(),
            ];
        }

        let viewer_ids: Vec<String> = self.viewers.keys().cloned().collect();
        let restored = loaded
            .session_name
            .clone()
            .unwrap_or_else(|| name.to_string());
        self.replace_session(loaded);
        self.session_name = Some(restored);
        self.refresh_all();

        let mut commands: Vec<UiCommand> =
            viewer_ids.into_iter().map(UiCommand::CloseViewer).collect();

        self.message = format!("loaded session '{name}'");
        commands.push(UiCommand::Menus(render::menus(self)));
        commands.push(UiCommand::BuildPages(render::pages(self)));
        commands.push(self.status_command());

        commands
    }

    fn rename_page(&mut self, page_id: &str, name: &str) -> Vec<UiCommand> {
        if name.is_empty() {
            return vec![];
        }

        let Some(node) = model::find_node_mut(&mut self.root, page_id) else {
            return vec![UiCommand::NotifyMessage("page is missing".to_string())];
        };

        match node {
            PageNode::Query(page) => page.title = name.to_string(),
            PageNode::Notebook(notebook) => notebook.title = name.to_string(),
            PageNode::Downloader(page) => page.title = name.to_string(),
            PageNode::Subscriptions(page) => page.title = name.to_string(),
            PageNode::Duplicates(page) => page.title = name.to_string(),
            PageNode::ImportFolders(page) => page.title = name.to_string(),
            PageNode::ExportFolders(page) => page.title = name.to_string(),
            PageNode::Placeholder(page) => page.title = name.to_string(),
        }

        vec![
            UiCommand::BuildPages(render::pages(self)),
            self.status_command(),
        ]
    }

    /// Typed core behind `"selection_changed"` and `app_set_selection`
    /// (WS-15.2.2b): replaces the current selection with the given hex hashes.
    fn set_selection(&mut self, page_id: &str, hashes: &[String]) -> Vec<UiCommand> {
        if let Some(page) = self.find_query_mut(page_id) {
            page.selected = hashes
                .iter()
                .filter_map(|hash| query::unhex(hash))
                .collect();
        }

        vec![self.status_command()]
    }

    /// Typed core behind `"add_hashes_to_page"` and `app_add_hashes`
    /// (WS-15.2.2b): appends dropped hashes to the page results.
    fn add_hashes(&mut self, page_id: &str, hashes: &[String]) -> Vec<UiCommand> {
        let mut parsed: Vec<Vec<u8>> = hashes
            .iter()
            .filter_map(|hash| query::unhex(hash))
            .collect();
        parsed.retain(|hash| !hash.is_empty());

        if parsed.is_empty() {
            return vec![UiCommand::NotifyMessage(
                "no files were dragged".to_string(),
            )];
        }

        let controller = self.controller.clone();
        let Some(page) = self.find_query_mut(page_id) else {
            return vec![UiCommand::NotifyMessage("page is missing".to_string())];
        };

        query::append_hashes(page, parsed);

        let state = render::query_page_state(&controller, page);

        vec![
            UiCommand::UpdatePage(state),
            self.status_command(),
        ]
    }

    fn focus(&mut self, page_id: &str, hash_hex: &str) -> Vec<UiCommand> {
        let controller = self.controller.clone();

        let Some(hash) = query::unhex(hash_hex) else {
            return vec![];
        };

        {
            let Some(page) = self.find_query_mut(page_id) else {
                return vec![];
            };

            page.focused = Some(hash.clone());
        };

        vec![
            UiCommand::SetSelection(render::selection_state(&controller, page_id, &hash)),
            self.status_command(),
        ]
    }

    fn open_viewer(&mut self, page_id: &str, hash_hex: &str) -> Vec<UiCommand> {
        let controller = self.controller.clone();

        let Some(hash) = query::unhex(hash_hex) else {
            return vec![];
        };

        let viewer_id = self.new_id("v");

        let Some(page) = self.find_query(page_id) else {
            return vec![UiCommand::NotifyMessage("page is missing".to_string())];
        };

        let json = render::viewer_state(&controller, &viewer_id, page, &hash);

        record_viewing(&controller, &hash, 0, true);

        self.viewers.insert(
            viewer_id.clone(),
            ViewerContext {
                page_id: page_id.to_string(),
                hash: hash.clone(),
                opened_at_ms: hydrus_time::get_now_ms(),
            },
        );

        if let Some(page) = self.find_query_mut(page_id) {
            page.focused = Some(hash);
        }

        vec![UiCommand::OpenViewer(json)]
    }

    fn viewer_nav(&mut self, viewer_id: &str, direction: &str) -> Vec<UiCommand> {
        let controller = self.controller.clone();

        let Some(context) = self.viewers.get(viewer_id).cloned() else {
            return vec![];
        };

        let next = {
            let Some(page) = self.find_query(&context.page_id) else {
                return vec![];
            };

            query::neighbour(page, &context.hash, direction)
        };

        let Some(next) = next else {
            return vec![];
        };

        record_viewing(
            &controller,
            &context.hash,
            hydrus_time::get_now_ms().saturating_sub(context.opened_at_ms),
            false,
        );
        record_viewing(&controller, &next, 0, true);

        self.viewers.insert(
            viewer_id.to_string(),
            ViewerContext {
                page_id: context.page_id.clone(),
                hash: next.clone(),
                opened_at_ms: hydrus_time::get_now_ms(),
            },
        );

        let json = {
            let Some(page) = self.find_query(&context.page_id) else {
                return vec![];
            };

            render::viewer_state(&controller, viewer_id, page, &next)
        };

        let mut commands = vec![UiCommand::UpdateViewer(json)];

        if self.active.as_deref() == Some(context.page_id.as_str()) {
            commands.extend(self.focus(&context.page_id, &query::hex(&next)));
        }

        commands
    }

    fn manage_tags(&mut self, _page_id: &str, hash_hex: &str, active: bool) -> Vec<UiCommand> {
        let controller = self.controller.clone();

        if active {
            let Some(page) = self.active_query() else {
                return vec![UiCommand::NotifyMessage("no page selected".to_string())];
            };

            let hashes = if page.selected.len() > 1 {
                page.selected.clone()
            } else if let Some(hash) = page.focused.clone() {
                vec![hash]
            } else {
                return vec![UiCommand::NotifyMessage("no file selected".to_string())];
            };

            return vec![UiCommand::ManageTagsDialog(
                metadata::manage_tags_state(&controller, &hashes),
             )];
        }

        let Some(hash) = query::unhex(hash_hex) else {
            return vec![];
        };

        vec![UiCommand::TagDialog(
            crate::gui::qt::api::ffi::TagDialogState {
                metadata: render::metadata_state(&controller, &hash),
            },
        )]
    }

    fn apply_tags(&mut self, payload: &str) -> Vec<UiCommand> {
        let Ok(value) = serde_json::from_str::<Value>(payload) else {
            return vec![UiCommand::NotifyMessage("invalid tag payload".to_string())];
        };

        let hashes: Vec<Vec<u8>> = if let Some(list) = value.get("hashes").and_then(Value::as_array)
        {
            list.iter()
                .filter_map(Value::as_str)
                .filter_map(query::unhex)
                .collect()
        } else {
            value
                .get("hash")
                .and_then(Value::as_str)
                .and_then(query::unhex)
                .into_iter()
                .collect()
        };

        if hashes.is_empty() {
            return vec![UiCommand::NotifyMessage("invalid file hash".to_string())];
        }

        let Some(service_id) = value
            .get("service")
            .and_then(Value::as_str)
            .and_then(|service| metadata::tag_service_id(&self.controller, service))
            .or_else(|| query::tag_service_id(&self.controller))
        else {
            return vec![UiCommand::NotifyMessage("no local tag service".to_string())];
        };

        let add: Vec<String> = value
            .get("add")
            .and_then(Value::as_array)
            .map(|values| {
                values
                    .iter()
                    .filter_map(Value::as_str)
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default();
        let remove: Vec<String> = value
            .get("remove")
            .and_then(Value::as_array)
            .map(|values| {
                values
                    .iter()
                    .filter_map(Value::as_str)
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default();

        let db = self.controller.db().db();
        let mut changed = 0;
        let mut found = 0;

        for hash in &hashes {
            let Some(hash_id) = query::hash_id_for(&self.controller, hash) else {
                continue;
            };

            found += 1;

            for tag in &add {
                if let Ok(tag_id) = master_tags::get_tag_id(db, tag) {
                    let _ = mappings::add_mappings(
                        db,
                        service_id,
                        ContentStatus::Current,
                        hash_id,
                        &[tag_id],
                    );
                    changed += 1;
                }
            }

            for tag in &remove {
                if let Ok(tag_id) = master_tags::get_tag_id(db, tag) {
                    let _ = mappings::remove_mappings(
                        db,
                        service_id,
                        ContentStatus::Current,
                        hash_id,
                        &[tag_id],
                    );
                    changed += 1;
                }
            }
        }

        if found == 0 {
            return vec![UiCommand::NotifyMessage(
                "file is not in the database".to_string(),
            )];
        }

        self.message = format!("tags updated ({changed} changes on {found} files)");

        let mut commands = if hashes.len() == 1 {
            metadata::refresh_hash(self, &hashes[0])
        } else {
            self.refresh_commands()
        };
        commands.push(self.status_command());

        commands
    }

    fn edit_timestamps(&mut self) -> Vec<UiCommand> {
        let Some(page) = self.active_query() else {
            return vec![UiCommand::NotifyMessage("no page selected".to_string())];
        };
        let Some(hash) = page.focused.clone() else {
            return vec![UiCommand::NotifyMessage("no file selected".to_string())];
        };
        let Some(hash_id) = query::hash_id_for(&self.controller, &hash) else {
            return vec![UiCommand::NotifyMessage(
                "file is not in the database".to_string(),
            )];
        };

        let db = self.controller.db().db();
        let archived = files_timestamps::get_file_timestamp(db, hash_id, TimestampType::Archived)
            .ok()
            .flatten();
        let modified =
            files_timestamps::get_file_timestamp(db, hash_id, TimestampType::ModifiedFile)
                .ok()
                .flatten();

        vec![UiCommand::TimestampsDialog(crate::gui::qt::api::ffi::TimestampsDialogState {
            hash: query::hex(&hash),
            has_archived: archived.is_some(),
            archived: archived.unwrap_or_default(),
            has_modified: modified.is_some(),
            modified: modified.unwrap_or_default(),
        })]
    }

    fn save_timestamps(&mut self, payload: &str) -> Vec<UiCommand> {
        let Ok(value) = serde_json::from_str::<Value>(payload) else {
            return vec![UiCommand::NotifyMessage(
                "invalid timestamp payload".to_string(),
            )];
        };

        let Some(hash) = value
            .get("hash")
            .and_then(Value::as_str)
            .and_then(query::unhex)
        else {
            return vec![UiCommand::NotifyMessage("invalid file hash".to_string())];
        };
        let Some(hash_id) = query::hash_id_for(&self.controller, &hash) else {
            return vec![UiCommand::NotifyMessage(
                "file is not in the database".to_string(),
            )];
        };

        let timestamp = |key: &str| {
            value.get(key).and_then(|entry| {
                if entry.is_null() {
                    Some(None)
                } else {
                    entry.as_i64().map(Some)
                }
            })
        };

        let db = self.controller.db().db();

        if let Some(archived) = timestamp("archived") {
            let _ = files_timestamps::set_file_timestamp(
                db,
                hash_id,
                TimestampType::Archived,
                archived,
            );
        }

        if let Some(modified) = timestamp("modified") {
            let _ = files_timestamps::set_file_timestamp(
                db,
                hash_id,
                TimestampType::ModifiedFile,
                modified,
            );
        }

        self.message = "timestamps updated".to_string();
        let mut commands = metadata::refresh_hash(self, &hash);
        commands.push(self.status_command());
        commands
    }

    fn set_inbox(&mut self, page_id: &str, hash_hex: &str, inbox: bool) -> Vec<UiCommand> {
        let Some(hash) = query::unhex(hash_hex) else {
            return vec![];
        };

        let Some(hash_id) = query::hash_id_for(&self.controller, &hash) else {
            return vec![];
        };

        let _ = files_storage::set_inbox(self.controller.db().db(), &[hash_id], inbox);
        let _ = files_timestamps::set_file_timestamp(
            self.controller.db().db(),
            hash_id,
            TimestampType::Archived,
            if inbox {
                None
            } else {
                Some(hydrus_time::get_now())
            },
        );

        self.message = if inbox {
            "file moved to inbox".to_string()
        } else {
            "file archived".to_string()
        };

        let _ = page_id;
        self.refresh_commands()
    }

    fn set_inbox_many(&mut self, page_id: &str, payload: &str, inbox: bool) -> Vec<UiCommand> {
        let hashes: Vec<String> = serde_json::from_str(payload).unwrap_or_default();
        let mut hash_ids = Vec::new();

        for hash_hex in hashes {
            if let Some(hash) = query::unhex(&hash_hex) {
                if let Some(hash_id) = query::hash_id_for(&self.controller, &hash) {
                    hash_ids.push(hash_id);
                }
            }
        }

        if hash_ids.is_empty() {
            return vec![UiCommand::NotifyMessage("no files selected".to_string())];
        }

        let _ = files_storage::set_inbox(self.controller.db().db(), &hash_ids, inbox);

        let now = hydrus_time::get_now();
        for hash_id in &hash_ids {
            let _ = files_timestamps::set_file_timestamp(
                self.controller.db().db(),
                *hash_id,
                TimestampType::Archived,
                if inbox { None } else { Some(now) },
            );
        }

        let count = hash_ids.len();

        self.message = if inbox {
            format!("{count} files moved to inbox")
        } else {
            format!("{count} files archived")
        };

        let _ = page_id;
        self.refresh_commands()
    }

    fn delete_files_many(&mut self, page_id: &str, payload: &str) -> Vec<UiCommand> {
        let hashes: Vec<String> = serde_json::from_str(payload).unwrap_or_default();
        let mut hash_ids = Vec::new();

        for hash_hex in hashes {
            if let Some(hash) = query::unhex(&hash_hex) {
                if let Some(hash_id) = query::hash_id_for(&self.controller, &hash) {
                    hash_ids.push(hash_id);
                }
            }
        }

        if hash_ids.is_empty() {
            return vec![UiCommand::NotifyMessage("no files selected".to_string())];
        }

        let db = self.controller.db().db();
        let my_files = query::local_file_service_id(&self.controller, "my files");
        let now = hydrus_time::get_now();

        if let Some(trash) = query::trash_service_id(&self.controller) {
            let _ =
                files_storage::add_files(db, trash, ContentStatus::Current, &hash_ids, Some(now));
        }

        if let Some(service) = my_files {
            let _ = files_storage::remove_files(db, service, ContentStatus::Current, &hash_ids);
        }

        for hash_id in &hash_ids {
            let _ = files_storage::set_inbox(db, &[*hash_id], false);
        }

        self.message = format!("{} files moved to trash", hash_ids.len());
        let _ = page_id;
        self.refresh_commands()
    }

    fn set_inbox_active(&mut self, inbox: bool) -> Vec<UiCommand> {
        let Some(page) = self.active_query() else {
            return vec![UiCommand::NotifyMessage("no page selected".to_string())];
        };

        let page_id = page.id.clone();

        if page.selected.len() > 1 {
            let hashes: Vec<String> = page.selected.iter().map(|hash| query::hex(hash)).collect();
            let payload = serde_json::to_string(&hashes).unwrap_or_else(|_| "[]".to_string());

            return self.set_inbox_many(&page_id, &payload, inbox);
        }

        let Some(hash) = page.focused.clone() else {
            return vec![UiCommand::NotifyMessage("no file selected".to_string())];
        };

        self.set_inbox(&page_id, &query::hex(&hash), inbox)
    }

    fn delete_files(&mut self, page_id: &str, hash_hex: &str) -> Vec<UiCommand> {
        let Some(hash) = query::unhex(hash_hex) else {
            return vec![];
        };

        let Some(hash_id) = query::hash_id_for(&self.controller, &hash) else {
            return vec![];
        };

        let db = self.controller.db().db();
        let my_files = query::local_file_service_id(&self.controller, "my files");

        if let Some(trash) = query::trash_service_id(&self.controller) {
            let _ = files_storage::add_files(
                db,
                trash,
                ContentStatus::Current,
                &[hash_id],
                Some(hydrus_time::get_now()),
            );
        }

        if let Some(service) = my_files {
            let _ = files_storage::remove_files(db, service, ContentStatus::Current, &[hash_id]);
        }

        let _ = files_storage::set_inbox(db, &[hash_id], false);

        self.message = "file moved to trash".to_string();
        let _ = page_id;
        self.refresh_commands()
    }

    fn import_paths(&mut self, payload: &str) -> Vec<UiCommand> {
        let paths: Vec<PathBuf> = serde_json::from_str::<Vec<String>>(payload)
            .unwrap_or_default()
            .into_iter()
            .map(PathBuf::from)
            .collect();

        if paths.is_empty() {
            return vec![UiCommand::NotifyMessage("no files selected".to_string())];
        }

        let job = worker::AsyncJob::ImportPaths {
            paths: paths.clone(),
            folder: None,
            filename_tagging: Default::default(),
            delete_after_success: false,
        };

        if let Err(error) = self.runtime.submit(job) {
            return vec![UiCommand::NotifyMessage(error)];
        }

        self.import_queue = model::ImportQueueState {
            running: true,
            done: 0,
            total: paths.len(),
            imported: 0,
            failed: 0,
            message: format!("queued {} files", paths.len()),
        };
        self.message = self.import_queue.message.clone();

        let mut commands = Vec::new();

        if self.query_ids().is_empty() {
            let id = self.new_id("p");
            self.root.add(PageNode::Query(QueryPage::new(
                id.clone(),
                "files",
                "my files",
            )));
            self.active = Some(id);
            self.refresh_all();
            commands.push(UiCommand::BuildPages(render::pages(self)));
        }

        commands.push(self.status_command());
        commands
    }

    fn download_one_url(&mut self, url: &str) -> Vec<UiCommand> {
        let url = url.trim();

        if !(url.starts_with("http://") || url.starts_with("https://")) {
            return vec![UiCommand::NotifyMessage(
                "the url must start with http:// or https://".to_string(),
            )];
        }

        let notebook = self.active_notebook_id();
        let id = self.new_id("p");
        let mut page = DownloaderPage::new(id.clone(), DownloaderKind::Urls);
        page.title = "url import".to_string();
        page.input = url.to_string();

        let Some(parent) = self.find_notebook_mut(&notebook) else {
            return vec![UiCommand::NotifyMessage("notebook is missing".to_string())];
        };
        parent.add(PageNode::Downloader(page));
        self.active = Some(id.clone());

        let payload = serde_json::json!({"input": url}).to_string();
        let mut commands = vec![UiCommand::BuildPages(render::pages(self))];
        commands.extend(download::start(self, &id, &payload));
        commands
    }
}

fn refresh_notebook(
    notebook: &mut Notebook,
    controller: &Controller,
    rules: &[auto_resolution::AutoResolutionRule],
    review: &mut auto_resolution::DuplicateReviewState,
    queued: &mut usize,
) {
    for child in &mut notebook.children {
        match child {
            PageNode::Query(page) => query::refresh_page(controller, page),
            PageNode::Notebook(inner) => refresh_notebook(inner, controller, rules, review, queued),
            PageNode::Duplicates(page) => {
                *queued += duplicates::refresh_for(controller, page, rules, review).1;
            }
            PageNode::Downloader(_)
            | PageNode::Subscriptions(_)
            | PageNode::ImportFolders(_)
            | PageNode::ExportFolders(_)
            | PageNode::Placeholder(_) => {}
        }
    }
}

fn collect_query_ids(notebook: &Notebook, ids: &mut Vec<String>) {
    for child in &notebook.children {
        match child {
            PageNode::Query(page) => ids.push(page.id.clone()),
            PageNode::Notebook(inner) => collect_query_ids(inner, ids),
            PageNode::Downloader(_)
            | PageNode::Subscriptions(_)
            | PageNode::Duplicates(_)
            | PageNode::ImportFolders(_)
            | PageNode::ExportFolders(_)
            | PageNode::Placeholder(_) => {}
        }
    }
}

fn collect_page_ids(node: &PageNode, ids: &mut Vec<String>) {
    ids.push(node.id().to_string());

    if let PageNode::Notebook(notebook) = node {
        for child in &notebook.children {
            collect_page_ids(child, ids);
        }
    }
}

fn collect_import_folder_ids(notebook: &Notebook, ids: &mut Vec<String>) {
    for child in &notebook.children {
        match child {
            PageNode::ImportFolders(page) => ids.push(page.id.clone()),
            PageNode::Notebook(inner) => collect_import_folder_ids(inner, ids),
            _ => {}
        }
    }
}

fn collect_export_folder_ids(notebook: &Notebook, ids: &mut Vec<String>) {
    for child in &notebook.children {
        match child {
            PageNode::ExportFolders(page) => ids.push(page.id.clone()),
            PageNode::Notebook(inner) => collect_export_folder_ids(inner, ids),
            _ => {}
        }
    }
}

fn record_viewing(controller: &Controller, hash: &[u8], viewtime_ms: i64, add_view: bool) {
    if !controller
        .options()
        .get_boolean("file_viewing_statistics_active")
    {
        return;
    }

    let Some(hash_id) = query::hash_id_for(controller, hash) else {
        return;
    };

    let views = i64::from(add_view);
    let now = hydrus_time::get_now_ms();

    let _ = files_viewing_stats::add_viewing_stats(
        controller.db().db(),
        hash_id,
        0,
        views,
        viewtime_ms.max(0),
        Some(now),
    );
}

fn pragma_i64(db: &hydrus_core::hydrus_db_base::DBBase, name: &str) -> i64 {
    db.query_one(&format!("PRAGMA {name}"), &[], |row| row.get(0))
        .ok()
        .flatten()
        .unwrap_or(0)
}

fn scalar_i64(db: &hydrus_core::hydrus_db_base::DBBase, query: &str) -> i64 {
    db.query_one(query, &[], |row| row.get(0))
        .ok()
        .flatten()
        .unwrap_or(0)
}

fn human_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut value = bytes as f64;
    let mut unit = 0;

    while value >= 1024.0 && unit + 1 < UNITS.len() {
        value /= 1024.0;
        unit += 1;
    }

    format!("{value:.1} {}", UNITS[unit])
}

fn placeholder_for(kind: &str) -> (String, String) {
    (
        kind.to_string(),
        format!("Unknown page type '{kind}'.\n\nThe page was kept so it can be renamed or closed."),
    )
}
