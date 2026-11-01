//! Typed cxx bridge seam (WS-15.2.1).
//!
//! The single cxx bridge of the Qt facade: shared POD structs plus every
//! `extern "C++"` declaration. One type per seam: Rust builds the struct,
//! `panels.rs` passes it by reference, C++ reads the fields directly. No
//! `QJsonObject` in the typed signatures, no `serde_json` on those seams.
//!
//! Rules for new types here (R1):
//! - only cxx-compatible fields (`String`, integers, `bool`, `f64`,
//!   `Vec<...>` of shared structs); no `Option`, no maps;
//! - missing values are an empty string / zero, never a JSON null;
//! - data of arbitrary shape (page trees, option dicts) stays out: those are
//!   converted per screen in WS-15.2.2, not added here as `&str` payload.

#![allow(dead_code)]

#[cxx::bridge]
// WS-15.2.1: single cxx module with a single `include!("bridge.hpp")`.
// bridge.hpp is a thin aggregator over the topic headers, so one include
// covers every declaration; no ffi_pages/ffi_dialogs split needed.
pub(crate) mod ffi {
    /// Status bar state: one text per label.
    #[derive(Clone, Debug, PartialEq)]
    struct StatusBar {
        message: String,
        media: String,
        bandwidth: String,
        idle: String,
        hydrus_busy: String,
        cpu_busy: String,
        db: String,
    }

    /// About dialog: versions, counts and paths. The window title stays a
    /// C++ literal; there is no `title` field on purpose.
    #[derive(Clone, Debug, PartialEq)]
    struct AboutInfo {
        software_version: u32,
        network_version: u32,
        client_api_version: u32,
        db_version: i64,
        db_dir: String,
        install_dir: String,
        services: i32,
        locations: i32,
        shortcuts: i32,
        api: String,
        sqlite: String,
        rust_crate: String,
        license: String,
        site: String,
        credits: String,
    }

    /// One row of the file viewing statistics table.
    #[derive(Clone, Debug, PartialEq)]
    struct ViewingStatRow {
        canvas: String,
        last_viewed: String,
        views: i64,
        viewtime_secs: i64,
    }

    /// File viewing statistics dialog: hex hash plus media/preview rows.
    #[derive(Clone, Debug, PartialEq)]
    struct ViewingStats {
        hash: String,
        rows: Vec<ViewingStatRow>,
    }

    /// One flat entry of the main-window menu bar (WS-15.2.2a).
    ///
    /// The menu tree is flattened in Rust with an explicit `depth`:
    /// `0` is a top-level menu (`title`), deeper rows are its items.
    /// A submenu row (`is_submenu`) owns the rows that follow it at
    /// `depth + 1`; a separator row (`is_separator`) only carries `depth`.
    #[derive(Clone, Debug, PartialEq)]
    struct MenuEntry {
        depth: i32,
        title: String,
        label: String,
        action: UiAction,
        target: String,
        payload: String,
        tooltip: String,
        enabled: bool,
        checkable: bool,
        checked: bool,
        is_separator: bool,
        is_submenu: bool,
    }

    /// Main-window menu bar: the whole menu tree as flat rows.
    #[derive(Clone, Debug, PartialEq)]
    struct MenuBar {
        entries: Vec<MenuEntry>,
    }

    /// Page kind for the page/tree seams (WS-15.2.2a). Numeric enum, no
    /// `Option`: every row carries a concrete kind.
    #[derive(Debug)]
    enum PageKind {
        Notebook,
        Query,
        Downloader,
        Subscriptions,
        Duplicates,
        ImportFolders,
        ExportFolders,
        Placeholder,
    }

    /// Typed UI action (WS-15.2.3b).
    #[repr(i32)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum UiAction {
        None,
        A,
        About,
        AccountType,
        AccountTypeDelete,
        AccountTypeSave,
        AccountTypesOpen,
        AddHashesToPage,
        AdminAccountAction,
        AdminAddService,
        AdminConnect,
        AdminEditService,
        AdminRemoveService,
        AdminServiceAction,
        All,
        Alternate,
        Alternates,
        AlwaysBoth,
        Api,
        ApiPermissions,
        ApiPermissionsDelete,
        ApiPermissionsSave,
        ApplySearchHistory,
        ApplyTags,
        ApplyTagsSelection,
        ApplyTagsSelectionApply,
        ArchiveActive,
        ArchiveFiles,
        ArchiveHashes,
        ArchivedDays,
        Auto,
        Autocomplete,
        Avi,
        Avif,
        B,
        Ban,
        Better,
        Blacklist,
        Bmp,
        Bool,
        BothFiles,
        Bytes,
        Clear,
        ClearFalsePositives,
        ClearFinished,
        ClearInternalFalsePositives,
        ClientApi,
        CloseOtherPages,
        ClosePage,
        CommandPalette,
        CopyTags,
        CreatePage,
        Css,
        Danbooru,
        Data,
        DbAnalyze,
        DbBackupOpen,
        DbBackupRun,
        DbBackupSetPath,
        DbCheck,
        DbDeferredDeletes,
        DbFileMaintenance,
        DbFixArchiveTimes,
        DbInfo,
        DbRunFileMaintenance,
        DbStats,
        DbVacuum,
        DbVacuumInfo,
        Delete,
        DeleteAllContent,
        DeleteBoth,
        DeleteFiles,
        DeleteHashes,
        Desired,
        DifferentSearches,
        Dislike,
        DissolveAlternateGroup,
        DissolveDuplicateGroup,
        Docs,
        Domain,
        Done,
        DownloadUrl,
        Downloader,
        DownloaderClear,
        DownloaderExport,
        DownloaderExportTo,
        DownloaderImport,
        DownloaderImportFrom,
        DownloaderImportOptions,
        DownloaderImportOptionsSave,
        DownloaderJobsSet,
        DownloaderOpenResults,
        DownloaderPage,
        DownloaderPause,
        DownloaderRetry,
        DownloaderSetFormula,
        DownloaderSetSite,
        DownloaderStart,
        Downloads,
        DuplicatePage,
        Duplicates,
        DuplicatesAlternates,
        DuplicatesAutoToggle,
        DuplicatesBatchSize,
        DuplicatesCustom,
        DuplicatesCustomAction,
        DuplicatesDeleteBoth,
        DuplicatesFalse,
        DuplicatesGroupAction,
        DuplicatesKeepA,
        DuplicatesKeepAKeep,
        DuplicatesKeepB,
        DuplicatesKeepBKeep,
        DuplicatesMergeOptions,
        DuplicatesMergeOptionsDialog,
        DuplicatesNextBatch,
        DuplicatesRefresh,
        DuplicatesReview,
        DuplicatesReviewApply,
        DuplicatesRuleAdd,
        DuplicatesRulePreview,
        DuplicatesRuleRemove,
        DuplicatesRuleSave,
        DuplicatesRules,
        DuplicatesSame,
        DuplicatesSearchContext,
        DuplicatesShowPair,
        DuplicatesSkip,
        Duration,
        E621,
        EditTimestamps,
        Either,
        Eot,
        Error,
        ExactMatch,
        Excluded,
        Exit,
        Expires,
        ExportFolderEdit,
        ExportFolderRemove,
        ExportFolderReveal,
        ExportFolderReview,
        ExportFolderRun,
        ExportFolderRunAll,
        ExportFolderSave,
        ExportFolderTestExamples,
        ExportFolderToggle,
        ExportFolders,
        Exporting,
        ExternalPrograms,
        FalsePositive,
        FavouriteApply,
        FavouriteDelete,
        FavouriteDeleteConfirmed,
        FavouriteSave,
        FavouriteSaveConfirmed,
        FewerTags,
        File,
        FileFiltering,
        FileId,
        FileSeedCache,
        FileSeedCacheAction,
        Files,
        Flac,
        FocusHash,
        Frames,
        Gallery,
        GallerySeedLog,
        GallerySeedLogAction,
        Gelbooru,
        General,
        Generic,
        Gif,
        Global,
        HasAudio,
        Hash,
        Heic,
        Height,
        HigherResolution,
        HistoryBack,
        HistoryForward,
        Http,
        Hydrus,
        Ico,
        Import,
        ImportFiles,
        ImportFolderAdd,
        ImportFolderCheck,
        ImportFolderCheckAll,
        ImportFolderEdit,
        ImportFolderRemove,
        ImportFolderSave,
        ImportFolderToggle,
        ImportFolders,
        ImportOptions,
        ImportOptionsSave,
        ImportPaths,
        ImportUrl,
        ImportedDays,
        InboxActive,
        InboxFiles,
        InboxHashes,
        Int,
        Jpeg,
        Jpg,
        Js,
        Json,
        Jxl,
        KeepBetter,
        LargerFile,
        LastViewedDays,
        Like,
        LocalImport,
        LocalImportAddPaths,
        LocalImportDeleteAfterSuccess,
        LocalImportFolder,
        LocalImportImport,
        LocalImportOpenPaths,
        LocalImportOptions,
        LocalImportRemove,
        LocalImportSearchSubdirectories,
        LocalImportTaggingOpen,
        LocalImportTaggingSave,
        Locations,
        LockOff,
        LockOn,
        LoginRemove,
        LoginSave,
        M4a,
        M4v,
        Maintenance,
        ManageExportFolders,
        ManageImportFolders,
        ManageNotesActive,
        ManageServices,
        ManageSubscriptions,
        ManageTags,
        ManageTagsActive,
        Media,
        Message,
        Mime,
        MinFilesize,
        Mkv,
        MoreTags,
        Mov,
        Move,
        MovePage,
        Mp3,
        Mp4,
        Namespace,
        Network,
        NetworkHeaderAdd,
        NetworkHeaderRemove,
        NetworkPause,
        NetworkPoolRemove,
        NetworkPoolSet,
        NetworkRuleRemove,
        NetworkRuleSet,
        NetworkSave,
        NetworkUrlClassAdd,
        NetworkUrlClassRemove,
        NetworkUrlClassSave,
        NewDuplicates,
        NewExportFolders,
        NewGallery,
        NewImportFolders,
        NewNotebook,
        NewPage,
        NewPageMenu,
        NewSimple,
        NewSubscriptions,
        NewUrls,
        NewWatcher,
        Next,
        NoneableInt,
        NotDuplicates,
        Notebook,
        Notes,
        NotesManage,
        NotesSave,
        Notifications,
        NotificationsClear,
        Notify,
        Nullification,
        NullificationPeriod,
        NumFrames,
        NumPixels,
        NumTags,
        NumUrls,
        NumWords,
        Numerical,
        Ogg,
        Older,
        OpenDbDir,
        OpenInstallDir,
        OpenViewer,
        Options,
        OptionsApply,
        OptionsNetwork,
        Opus,
        OrDialog,
        OrInsert,
        OrphanFilesDelete,
        OrphanFilesMove,
        OrphanFilesScan,
        Otf,
        PageManager,
        PageManagerActivate,
        PageManagerClose,
        PageSelected,
        Parent,
        PasteTags,
        PasteTagsPrompt,
        PauseNetwork,
        Pdf,
        Pending,
        Pixels,
        Pixiv,
        Placeholder,
        Png,
        Post,
        PostUrls,
        Prefetch,
        Presentation,
        Preview,
        PurgeTags,
        Qoi,
        Query,
        Random,
        Rar,
        RatingSet,
        Ratio,
        Readonly,
        RedoSearch,
        Refresh,
        RefreshPage,
        RegenInfo,
        Regex,
        Remove,
        RemoveFromAlternateGroup,
        RemoveFromDuplicateGroup,
        RemovePotentials,
        RenamePage,
        RepairFileLocations,
        RepairLocationsApply,
        RepairLocationsProbe,
        RepairLocationsSet,
        Required,
        ResetPotentialSearch,
        Retry,
        ReviewLocalImports,
        Rss,
        Rule34,
        RunQuery,
        Safebooru,
        Same,
        Search,
        SelectionChanged,
        ServiceAdd,
        ServiceDelete,
        ServiceRename,
        ServiceSave,
        ServiceInfo,
        SessionAppend,
        SessionAppendBackup,
        SessionBackup,
        SessionDelete,
        SessionDeleteConfirmed,
        SessionLoad,
        SessionSave,
        SessionSaveAs,
        SetLocation,
        SetSort,
        SetTagService,
        Sibling,
        Similarity,
        Simple,
        Size,
        Source,
        Statistics,
        String,
        Subscription,
        SubscriptionAdd,
        SubscriptionCheck,
        SubscriptionDelete,
        SubscriptionDuplicate,
        SubscriptionEdit,
        SubscriptionEditSave,
        SubscriptionImportOptions,
        SubscriptionImportOptionsClear,
        SubscriptionImportOptionsSave,
        SubscriptionLowercaseQueries,
        SubscriptionOpenResults,
        SubscriptionReset,
        SubscriptionScrubDelays,
        SubscriptionToggle,
        Subscriptions,
        Subtag,
        Success,
        Svg,
        Synchronise,
        TagAdd,
        TagFilterOpen,
        TagFilterSave,
        TagFilterTest,
        TagFiltering,
        TagParents,
        TagRelationsAdd,
        TagRelationsRemove,
        TagRemove,
        TagSiblings,
        Tags,
        Text,
        Thread,
        Thumbnail,
        Thumbnails,
        Tif,
        Tiff,
        TimestampsSave,
        Trash,
        True,
        Ttf,
        TwoWayMerge,
        Unban,
        Undo,
        UndoSearch,
        Unknown,
        Update,
        UpdatePeriod,
        UrlClass,
        Urls,
        ViewerClose,
        ViewerNav,
        ViewingStats,
        Watchable,
        Watcher,
        WatcherAddUrl,
        WatcherAuto,
        WatcherCheck,
        WatcherCheckerOptions,
        WatcherPage,
        WatcherRemove,
        WatcherUrls,
        Wav,
        Webm,
        Webp,
        Whitelist,
        Width,
        Woff,
        Woff2,
        Words,
        Worse,
        Xml,
        Zip,
    }

    /// One row of the manage-pages dialog tree (WS-15.2.2a).
    #[derive(Clone, Debug, PartialEq)]
    struct PageTreeRow {
        id: String,
        title: String,
        kind: PageKind,
        count: i64,
        active: bool,
        depth: i32,
    }

    /// Manage-pages dialog: the whole page tree as flat preorder rows.
    #[derive(Clone, Debug, PartialEq)]
    struct PageTree {
        rows: Vec<PageTreeRow>,
    }

    /// Incremental progress of a file page (WS-15.2.2a).
    ///
    /// C++ currently consumes only `loading` (as the JSON seam did); the
    /// import counters travel typed for the follow-up seams. Absence of an
    /// import is zero/empty, never JSON null.
    #[derive(Clone, Debug, PartialEq)]
    struct PageProgress {
        page_id: String,
        loading: bool,
        importing_done: i64,
        importing_total: i64,
        importing_message: String,
        count: i64,
    }

    /// One thumbnail of a file page (WS-15.2.2b).
    ///
    /// `rating` travels with an explicit `has_rating` flag instead of JSON
    /// null; C++ formats the badge exactly like the old JSON seam
    /// (`QString::number(rating, 'g', 3)`), so the quirk stays in one place.
    #[derive(Clone, Debug, PartialEq, Default)]
    struct MediaItem {
        hash: String,
        thumb: String,
        file: String,
        url: String,
        inbox: bool,
        duplicate: bool,
        has_rating: bool,
        rating: f64,
    }

    /// File page contents for `update_page` (WS-15.2.2b).
    ///
    /// Only fields the C++ side reads. The old JSON seam also carried
    /// `status`, `empty_text`, `metadata`, `count` and `importing`: `status`
    /// travels via `Status`, progress via `PageProgress`, and the metadata
    /// object was ignored by the query page (`Q_UNUSED`), so they are gone.
    #[derive(Clone, Debug, PartialEq, Default)]
    struct QueryPageState {
        page_id: String,
        query: String,
        location: String,
        tag_service: String,
        tag_services: Vec<String>,
        sort: String,
        sort_ascending: bool,
        focused: String,
        tags: Vec<String>,
        predicates: Vec<String>,
        media: Vec<MediaItem>,
        loading: bool,
        thumb_scale_percent: i32,
        thumb_masonry: bool,
        thumb_horizontal: bool,
    }

    /// One built-in gallery parser entry (WS-15.2.3c).
    #[derive(Clone, Debug, PartialEq, Default)]
    struct GallerySite {
        id: String,
        title: String,
        template_text: String,
    }

    /// Downloader page contents for `update_downloader` (WS-15.2.3c).
    ///
    /// Only fields the page reads. Status lines, relative "added" text and the
    /// two progress percentages are formatted in Rust (`render.rs`), so C++
    /// only assigns text and integers.
    #[derive(Clone, Debug, PartialEq, Default)]
    struct DownloaderPageState {
        page_id: String,
        kind: String,
        input: String,
        url: String,
        query: String,
        site: String,
        subject: String,
        formula_name: String,
        import_options_summary: String,
        import_options_text: String,
        status: String,
        status_top: String,
        status_bottom: String,
        imports_status: String,
        file_seed_status: String,
        search_status: String,
        search_seed_status: String,
        files_status: String,
        checking_status: String,
        search_state_text: String,
        queue_count_text: String,
        watcher_added_text: String,
        queue_added_text: String,
        files_progress: i32,
        search_progress: i32,
        running: bool,
        paused: bool,
        auto_watch: bool,
        check_period: i64,
        queue_count: i64,
        pending_jobs: Vec<String>,
        sites: Vec<GallerySite>,
    }

    /// One row of the subscriptions page table (WS-15.2.3c). Cell text is
    /// formatted in Rust; C++ only assigns it.
    #[derive(Clone, Debug, PartialEq, Default)]
    struct SubscriptionPageRow {
        name: String,
        url: String,
        period_text: String,
        last_check_text: String,
        next_text: String,
        imported_text: String,
        failed_text: String,
        status_text: String,
    }

    /// One row of the manage-subscriptions dialog table (WS-15.2.3c).
    /// `custom_import_options` is the display summary, not the option dict.
    /// `import_options_json` is the one remaining arbitrary nested dict in this
    /// seam: the dialog copies it verbatim to the clipboard and the importer
    /// accepts any valid container shape, so it cannot be typed.
    #[derive(Clone, Debug, PartialEq, Default)]
    struct SubscriptionDialogRow {
        name: String,
        url: String,
        source: String,
        pretty_status: String,
        last_new_file_time: String,
        last_checked: String,
        error: String,
        items: String,
        paused_display: String,
        custom_import_options: String,
        import_options_json: String,
    }

    /// Subscriptions page contents (WS-15.2.3c).
    #[derive(Clone, Debug, PartialEq, Default)]
    struct SubscriptionsPageState {
        page_id: String,
        sites: Vec<GallerySite>,
        rows: Vec<SubscriptionPageRow>,
    }

    /// Manage-subscriptions dialog contents (WS-15.2.3c).
    #[derive(Clone, Debug, PartialEq, Default)]
    struct SubscriptionsDialogState {
        subs_paused: bool,
        sites: Vec<GallerySite>,
        rows: Vec<SubscriptionDialogRow>,
    }

    /// Subscription editor dialog contents (WS-15.2.3c). `original` keeps the
    /// pre-edit name that the save action round-trips.
    #[derive(Clone, Debug, PartialEq, Default)]
    struct SubscriptionEditorState {
        page_id: String,
        original: String,
        name: String,
        url: String,
        query: String,
        site: String,
        filter: String,
        period: i64,
        paused: bool,
        sites: Vec<GallerySite>,
    }

    /// The currently shown duplicate pair (WS-15.2.3c). The pair entries are
    /// the same `MediaItem` rows the thumbnail grid already consumes; only
    /// `distance` is specific to duplicates.
    #[derive(Clone, Debug, PartialEq, Default)]
    struct DuplicatePairState {
        distance: i64,
        a: MediaItem,
        b: MediaItem,
    }

    /// Search context of the duplicates sidebar (WS-15.2.3c).
    #[derive(Clone, Debug, PartialEq, Default)]
    struct DuplicateSearchState {
        query_1: String,
        location_1: String,
        query_2: String,
        location_2: String,
        dupe_search_type: String,
        pixel_dupes: String,
        max_distance: i64,
        sort_type: String,
        sort_ascending: bool,
        matched: i64,
        summary: String,
    }

    /// One auto-resolution rule row of the duplicates page (WS-15.2.3c).
    #[derive(Clone, Debug, PartialEq, Default)]
    struct DuplicateRuleRow {
        name: String,
        search_summary: String,
        paused: bool,
    }

    /// Duplicates page contents (WS-15.2.3c). `has_pair` replaces the JSON
    /// null: the pair fields are only read when it is true.
    #[derive(Clone, Debug, PartialEq, Default)]
    struct DuplicatesPageState {
        page_id: String,
        title: String,
        status: String,
        status_line: String,
        index: i64,
        total: i64,
        searched: i64,
        batch_size: i64,
        search: DuplicateSearchState,
        has_pair: bool,
        pair: DuplicatePairState,
        media: Vec<MediaItem>,
        media_empty_status: String,
        media_count_status: String,
        auto_rules: Vec<DuplicateRuleRow>,
    }

    /// One status filter of a seed cache dialog (WS-15.2.3c).
    #[derive(Clone, Debug, PartialEq, Default)]
    struct SeedFilter {
        key: String,
        label: String,
    }

    /// One seed row of a seed cache dialog (WS-15.2.3c).
    #[derive(Clone, Debug, PartialEq, Default)]
    struct SeedRow {
        status: String,
        url: String,
        note: String,
        created: i64,
        modified: i64,
    }

    /// File seed cache and gallery seed log contents (WS-15.2.3c).
    #[derive(Clone, Debug, PartialEq, Default)]
    struct SeedCacheState {
        page: String,
        subtitle: String,
        failed: i64,
        status_filters: Vec<SeedFilter>,
        seeds: Vec<SeedRow>,
    }

    /// One import/export folder row of a folder page.
    #[derive(Clone, Debug, PartialEq, Default)]
    struct FolderRow {
        name: String,
        path: String,
        status: String,
        paused: bool,
        period_text: String,
        last_queued: String,
        extra: String,
        query_text: String,
        filters_text: String,
        mode_text: String,
        phrase_text: String,
        error_text: String,
    }

    /// Typed state of import/export folder pages (WS-15.2.4b).
    #[derive(Clone, Debug, PartialEq, Default)]
    struct FolderPageState {
        summary: String,
        folders: Vec<FolderRow>,
    }

    /// Autocomplete entries for a query page (WS-15.2.4b).
    #[derive(Clone, Debug, PartialEq, Default)]
    struct AutocompleteState {
        page_id: String,
        entries: Vec<String>,
    }

    /// Notes dialog contents (WS-15.2.4b).
    #[derive(Clone, Debug, PartialEq, Default)]
    struct NotesDialogState {
        hash: String,
        notes: Vec<NoteState>,
    }

    /// Tag dialog contents: focused metadata panel state (WS-15.2.4b).
    #[derive(Clone, Debug, PartialEq)]
    struct TagDialogState {
        metadata: MetadataState,
    }

    /// One filename-tagging rule of an import folder, fully typed
    /// (WS-15.2.4b). Rust formats `summary`; C++ only displays rows.
    #[derive(Clone, Debug, PartialEq, Default)]
    struct TaggingEntry {
        service_key: String,
        service_name: String,
        summary: String,
        tags_for_all: Vec<String>,
        add_filename: bool,
        filename_namespace: String,
        dir_enabled: Vec<bool>,
        dir_namespaces: Vec<String>,
        quick_namespaces: Vec<String>,
        quick_regexes: Vec<String>,
        regexes: Vec<String>,
    }

    /// Local tag service available for filename tagging (WS-15.2.4b).
    #[derive(Clone, Debug, PartialEq, Default)]
    struct TagServiceRef {
        key: String,
        name: String,
    }

    /// Import folder editor state (WS-15.2.4b). All labels Rust formats.
    #[derive(Clone, Debug, PartialEq, Default)]
    struct ImportFolderEditorState {
        original: String,
        name: String,
        path: String,
        search_subdirectories: bool,
        paused: bool,
        check_regularly: bool,
        period: i64,
        period_label: String,
        skip: i64,
        skip_label: String,
        check_now: bool,
        show_working_popup: bool,
        publish_popup: bool,
        publish_page: bool,
        action: String,
        action_path: String,
        tagging: Vec<TaggingEntry>,
        tag_services: Vec<TagServiceRef>,
    }

    /// Export folder editor state (WS-15.2.4b).
    #[derive(Clone, Debug, PartialEq, Default)]
    struct ExportFolderEditorState {
        original: String,
        name: String,
        path: String,
        export_type: String,
        delete_from_client: bool,
        export_symlinks: bool,
        run_regularly: bool,
        show_working_popup: bool,
        run_now: bool,
        period: i64,
        period_label: String,
        query: String,
        phrase: String,
        sidecar_format: String,
        overwrite_sidecars: bool,
        always_overwrite_sidecars: bool,
    }
    /// One page row of the notebook tree (WS-15.2.2a / WS-15.2.3c).
    ///
    /// The envelope is typed. Query, downloader, subscriptions, duplicates and
    /// folder pages carry their typed initial state here; `state` stays only
    /// for the placeholder message (plain string, not JSON).
    #[derive(Clone, Debug, PartialEq)]
    struct PageRow {
        id: String,
        title: String,
        kind: PageKind,
        current: String,
        depth: i32,
        query: QueryPageState,
        downloader: DownloaderPageState,
        subscriptions: SubscriptionsPageState,
        duplicates: DuplicatesPageState,
        folder: FolderPageState,
        state: String,
    }

    /// Notebook contents for `build_pages`: root current page plus the
    /// whole tree as flat preorder rows.
    #[derive(Clone, Debug, PartialEq)]
    struct Pages {
        current: String,
        rows: Vec<PageRow>,
    }

    /// One display-tag row with its storage tags for the tooltip (WS-15.2.2b).
    ///
    /// Maps do not cross the bridge, so the `display_tag_sources` object of
    /// the old JSON seam becomes a flat vector of pairs.
    #[derive(Clone, Debug, PartialEq)]
    struct TagSource {
        display: String,
        storage: Vec<String>,
    }

    /// One tag service of the metadata panel (WS-15.2.2b).
    #[derive(Clone, Debug, PartialEq)]
    struct TagServiceState {
        key: String,
        name: String,
        tags: Vec<String>,
        display_tags: Vec<String>,
        sources: Vec<TagSource>,
    }

    /// Rating service flavour (WS-15.2.2b). Numeric enum, no `Option`:
    /// every row carries all three value slots and C++ reads the one its
    /// `kind` selects. Absence is zero/empty, never JSON null.
    #[derive(Debug)]
    enum RatingKind {
        Numerical,
        Like,
        IncDec,
    }

    /// One rating service of the metadata panel (WS-15.2.2b).
    #[derive(Clone, Debug, PartialEq)]
    struct RatingState {
        key: String,
        name: String,
        kind: RatingKind,
        value_num: f64,
        value_text: String,
        value_int: i64,
    }

    /// One note of the metadata panel (WS-15.2.2b).
    #[derive(Clone, Debug, PartialEq)]
    struct NoteState {
        name: String,
        text: String,
    }

    /// Metadata panel contents: focused file plus its tag services, rating
    /// services and notes (WS-15.2.2b). Shared by the viewer seam; the query
    /// page never showed it (its old `set_selection` ignored the object).
    #[derive(Clone, Debug, PartialEq)]
    struct MetadataState {
        hash: String,
        tag_services: Vec<TagServiceState>,
        ratings: Vec<RatingState>,
        notes: Vec<NoteState>,
    }

    /// Focus/selection update of a file page (WS-15.2.2b).
    ///
    /// The old JSON seam carried a metadata object that the query page
    /// ignored (`Q_UNUSED(metadata_state)`), so the struct keeps only what
    /// C++ applies: focused hash, selection tags and the preview path.
    #[derive(Clone, Debug, PartialEq)]
    struct SelectionState {
        page_id: String,
        hash: String,
        tags: Vec<String>,
        preview_path: String,
    }

    /// Import options dialog contents (WS-15.2.2c).
    #[derive(Clone, Debug, PartialEq)]
    struct ImportOptionsDialogState {
        json: String,
    }

    /// Options dialog contents (WS-15.2.2d).
    #[derive(Clone, Debug, PartialEq)]
    struct OptionsDialogState {
        json: String,
    }

    /// Services dialog contents (WS-15.2.4b). All cell texts are formatted
    /// in Rust; C++ only assigns them. `port_text`/`settings_summary` replace
    /// the old dictionary formatting, `*` texts replace JSON nulls.
    #[derive(Clone, Debug, PartialEq, Default)]
    struct ServiceRow {
        key: String,
        name: String,
        type_short: String,
        port_text: String,
        settings_summary: String,
        builtin: bool,
        type_int: i32,
        host: String,
        port: i32,
        access_key: String,
        has_update_period: bool,
        update_period: i32,
        has_nullify_period: bool,
        nullify_period: i32,
        max_storage: i32,
        log_uploader_ips: bool,
    }

    /// One service-type entry for the service editor picker (WS-15.2.4b).
    #[derive(Clone, Debug, PartialEq, Default)]
    struct ServiceTypeRow {
        label: String,
        short_name: String,
        value: i32,
        family: String,
    }

    /// One remote server service row (WS-15.2.4b).
    #[derive(Clone, Debug, PartialEq, Default)]
    struct RemoteServiceRow {
        key: String,
        name: String,
        type_name: String,
        port: i32,
        access_key: String,
        selected: bool,
    }

    /// One account row of the services server tab (WS-15.2.4b).
    /// `created_text`/`expires_text`/`banned_text` are formatted in Rust.
    #[derive(Clone, Debug, PartialEq, Default)]
    struct ServiceAccountRow {
        key: String,
        type_title: String,
        created_text: String,
        expires_text: String,
        banned_text: String,
        message: String,
    }

    /// One saved login row (WS-15.2.4b). `password_display` is `-` or
    /// `********`; `expires_text` is `never` or the epoch number.
    /// `password` stays typed (never masked in transit) so the credentials
    /// editor can pre-fill it.
    #[derive(Clone, Debug, PartialEq, Default)]
    struct LoginRow {
        domain: String,
        username: String,
        password: String,
        password_display: String,
        access_key: String,
        expires_text: String,
    }

    /// One account type reference for the admin account action (WS-15.2.4b).
    #[derive(Clone, Debug, PartialEq, Default)]
    struct AccountTypeRef {
        key: String,
        title: String,
    }

    /// Services dialog contents (WS-15.2.4b).
    /// Arbitrary server blobs shown verbatim in the service review dialog
    /// (WS-15.2.4b). The schema comes from the server, so it stays JSON;
    /// every fixed field of the dialog is typed above (R1 exception).
    #[derive(Clone, Debug, PartialEq, Default)]
    struct AdminBlobs {
        options_json: String,
        service_info_json: String,
    }

    #[derive(Clone, Debug, PartialEq, Default)]
    struct ServicesDialogState {
        services: Vec<ServiceRow>,
        types: Vec<ServiceTypeRow>,
        admin_address: String,
        admin_access_key: String,
        admin_status: String,
        admin: AdminBlobs,
        remote_services: Vec<RemoteServiceRow>,
        accounts: Vec<ServiceAccountRow>,
        logins: Vec<LoginRow>,
        account_types: Vec<AccountTypeRef>,
    }

    /// One URL type entry for the URL class editor picker (WS-15.2.4b).
    #[derive(Clone, Debug, PartialEq, Default)]
    struct UrlTypeRow {
        value: i32,
        label: String,
    }

    /// One `name`/`value` header row (WS-15.2.4b).
    #[derive(Clone, Debug, PartialEq, Default)]
    struct NetworkHeaderRow {
        name: String,
        value: String,
    }

    /// One URL class row (WS-15.2.4b). `components_text` joins the path
    /// components with `/`; formatting happens in Rust. The editor fields
    /// (`url_type`, `preferred_scheme`, ...) travel typed as well so the
    /// edit dialog needs no JSON.
    #[derive(Clone, Debug, PartialEq, Default)]
    struct NetworkUrlClassRow {
        name: String,
        url_type_name: String,
        netloc: String,
        components_text: String,
        url_type: i32,
        preferred_scheme: String,
        match_subdomains: bool,
        keep_matched_subdomains: bool,
        parameters_text: String,
        can_produce_multiple_files: bool,
        should_be_associated_with_files: bool,
        example_url: String,
    }

    /// One download pool row (WS-15.2.4b). `limit_text` is the decimal limit.
    #[derive(Clone, Debug, PartialEq, Default)]
    struct NetworkPoolRow {
        domain: String,
        limit_text: String,
    }

    /// One bandwidth rule row (WS-15.2.4b). `period_text` is `month` or
    /// `N s`; `maximum_text` is `N MB` for data rules or the raw count.
    #[derive(Clone, Debug, PartialEq, Default)]
    struct NetworkBandwidthRow {
        context_type: String,
        context: String,
        bandwidth_type: String,
        period_text: String,
        maximum_text: String,
    }

    /// Network dialog contents (WS-15.2.4b). All option scalars and cell
    /// texts are formatted in Rust; C++ only assigns widgets.
    #[derive(Clone, Debug, PartialEq, Default)]
    struct NetworkDialogState {
        http_proxy: String,
        https_proxy: String,
        no_proxy: String,
        network_timeout: i32,
        max_network_jobs: i32,
        max_network_jobs_per_domain: i32,
        api_enabled: bool,
        api_port: i32,
        api_access_key: String,
        api_status: String,
        paused: bool,
        headers: Vec<NetworkHeaderRow>,
        url_classes: Vec<NetworkUrlClassRow>,
        url_types: Vec<UrlTypeRow>,
        download_pools: Vec<NetworkPoolRow>,
        bandwidth_rules: Vec<NetworkBandwidthRow>,
    }

    /// One permission pair of an account type (WS-15.2.4b). `has_action`
    /// replaces the JSON null (no permission set for the content type).
    #[derive(Clone, Debug, PartialEq, Default)]
    struct AccountPermissionPair {
        content_type: i32,
        action: i32,
        has_action: bool,
    }

    /// One selectable action of a permission group (WS-15.2.4b).
    #[derive(Clone, Debug, PartialEq, Default)]
    struct AccountPermissionAction {
        value: i32,
        has_value: bool,
        label: String,
    }

    /// One content-type group of the account type editor (WS-15.2.4b).
    #[derive(Clone, Debug, PartialEq, Default)]
    struct AccountPermissionGroup {
        content_type: i32,
        label: String,
        actions: Vec<AccountPermissionAction>,
    }

    /// One account type row (WS-15.2.4b). `summary` (permission labels) and
    /// `auto_creation_text` (`disabled` or `N / M s`) are formatted in Rust.
    #[derive(Clone, Debug, PartialEq, Default)]
    struct AccountTypeRow {
        key: String,
        title: String,
        summary: String,
        auto_creation_text: String,
        is_null: bool,
        permissions: Vec<AccountPermissionPair>,
        auto_create_accounts: i64,
        auto_create_time_delta: i64,
    }

    /// Account types dialog contents (WS-15.2.4b).
    #[derive(Clone, Debug, PartialEq, Default)]
    struct AccountTypesDialogState {
        service_key: String,
        rows: Vec<AccountTypeRow>,
        possible: Vec<AccountPermissionGroup>,
    }

    /// One API permission rule (WS-15.2.4b). `slice` is the stored slice
    /// (empty = all unnamespaced, `:` = all namespaced), `display` its
    /// dialog spelling (`*`, `*:*`), `rule` either `whitelist`/`blacklist`.
    #[derive(Clone, Debug, PartialEq, Default)]
    struct ApiPermissionRule {
        slice: String,
        display: String,
        rule: String,
    }

    /// One API access key row (WS-15.2.4b). `summary` (`permits everything` /
    /// labels / `no permissions`) is formatted in Rust.
    #[derive(Clone, Debug, PartialEq, Default)]
    struct ApiPermissionRow {
        name: String,
        access_key: String,
        summary: String,
        permits_everything: bool,
        basic_permissions: Vec<i32>,
        rules: Vec<ApiPermissionRule>,
    }

    /// One available basic permission of the editor checklist (WS-15.2.4b).
    #[derive(Clone, Debug, PartialEq, Default)]
    struct ApiAvailablePermission {
        value: i32,
        label: String,
    }

    /// API permissions dialog contents (WS-15.2.4b). `status` is the ready
    /// `api: ...` line; C++ only assigns it.
    #[derive(Clone, Debug, PartialEq, Default)]
    struct ApiPermissionsDialogState {
        api_status: String,
        status: String,
        rows: Vec<ApiPermissionRow>,
        available: Vec<ApiAvailablePermission>,
    }

    /// Tag filter dialog contents (WS-15.2.4b).
    ///
    /// One row of the dialog rule list: `slice` is the stored filter slice
    /// (empty means all unnamespaced tags, `:` all namespaced ones),
    /// `display` its dialog spelling (`*`, `*:*`) and `rule` either
    /// `whitelist` or `blacklist`.
    #[derive(Clone, Debug, PartialEq, Default)]
    struct TagFilterRule {
        slice: String,
        display: String,
        rule: String,
    }

    /// Tag filter dialog contents (WS-15.2.4b).
    #[derive(Clone, Debug, PartialEq, Default)]
    struct TagFilterDialogState {
        service_key: String,
        name: String,
        can_purge: bool,
        filter_string: String,
        note: String,
        rules: Vec<TagFilterRule>,
    }

    /// One local tag service of the tag relations dialog (WS-15.2.4b).
    #[derive(Clone, Debug, PartialEq, Default)]
    struct TagRelationService {
        key: String,
        name: String,
    }

    /// One sibling/parent pair of the tag relations dialog (WS-15.2.4b).
    #[derive(Clone, Debug, PartialEq, Default)]
    struct TagRelationRow {
        relation: String,
        service: String,
        first: String,
        second: String,
    }

    /// Tag relations dialog contents (WS-15.2.4b).
    #[derive(Clone, Debug, PartialEq, Default)]
    struct TagRelationsDialogState {
        tab: String,
        services: Vec<TagRelationService>,
        relations: Vec<TagRelationRow>,
    }

    /// One tag of a manage-tags service, with the count of selected files
    /// already carrying it (WS-15.2.4b).
    #[derive(Clone, Debug, PartialEq, Default)]
    struct ManageTagEntry {
        tag: String,
        count: i64,
    }

    /// One local tag service of the manage-tags dialog (WS-15.2.4b).
    #[derive(Clone, Debug, PartialEq, Default)]
    struct ManageTagService {
        name: String,
        tags: Vec<ManageTagEntry>,
    }

    /// Manage tags dialog contents (WS-15.2.4b). `summary` is formatted in
    /// Rust (`N file(s) selected`); C++ only assigns it.
    #[derive(Clone, Debug, PartialEq, Default)]
    struct ManageTagsDialogState {
        hashes: Vec<String>,
        summary: String,
        services: Vec<ManageTagService>,
    }

    /// Timestamps dialog contents: typed optional epoch seconds.
    #[derive(Clone, Debug, PartialEq)]
    struct TimestampsDialogState {
        hash: String,
        has_archived: bool,
        archived: i64,
        has_modified: bool,
        modified: i64,
    }

    /// One popup-message row (time/level preformatted in Rust).
    #[derive(Clone, Debug, PartialEq)]
    struct NotificationRow {
        time: String,
        level: String,
        text: String,
    }

    /// Notifications dialog contents.
    #[derive(Clone, Debug, PartialEq)]
    struct NotificationsDialogState {
        count: i64,
        summary: String,
        entries: Vec<NotificationRow>,
    }

    /// One export-review file row (WS-15.2.4b). `tags_text` is the joined
    /// `", "` list, `relative` the destination trimmed of the folder root,
    /// `note` the ready/source-missing/unsafe/will-be-updated verdict and
    /// `checked` whether the row starts checked. All formatted in Rust.
    #[derive(Clone, Debug, PartialEq, Default)]
    struct ExportReviewFile {
        hash: String,
        tags_text: String,
        destination: String,
        relative: String,
        note: String,
        checked: bool,
    }

    /// Export review dialog contents (WS-15.2.4b). `title` and `summary` are
    /// formatted in Rust; C++ only assigns them.
    #[derive(Clone, Debug, PartialEq, Default)]
    struct ExportReviewDialogState {
        name: String,
        destination: String,
        title: String,
        summary: String,
        files: Vec<ExportReviewFile>,
    }

    /// One repair-location row.
    #[derive(Clone, Debug, PartialEq)]
    struct RepairLocationRow {
        prefix: String,
        old_base: String,
        base: String,
        status: String,
        exists: bool,
    }

    /// Repair file locations dialog contents.
    #[derive(Clone, Debug, PartialEq)]
    struct RepairFileLocationsDialogState {
        title: String,
        summary: String,
        message: String,
        error: String,
        missing_files: i64,
        missing_thumbs: i64,
        rows: Vec<RepairLocationRow>,
    }

    /// One orphan-file row.
    #[derive(Clone, Debug, PartialEq)]
    struct OrphanFileRow {
        kind: String,
        size_display: String,
        hash: String,
        path: String,
    }

    /// Orphan files dialog contents.
    #[derive(Clone, Debug, PartialEq)]
    struct OrphanFilesDialogState {
        status: String,
        bytes_display: String,
        files: i64,
        thumbnails: i64,
        bytes: i64,
        orphans: Vec<OrphanFileRow>,
    }

    /// Database backup dialog contents.
    #[derive(Clone, Debug, PartialEq)]
    struct DatabaseBackupDialogState {
        path: String,
        db_dir: String,
        last_backup: String,
        target: String,
    }

    /// One local-import path row (WS-15.2.4b). `result_text` is the pretty
    /// mime/result label, `size_display` the formatted size; colours follow
    /// the `bad`/`sidecar` flags. All formatted in Rust.
    #[derive(Clone, Debug, PartialEq, Default)]
    struct LocalImportPathRow {
        index: i64,
        path: String,
        result_text: String,
        size_display: String,
        bad: bool,
        sidecar: bool,
    }

    /// Local import review dialog contents (WS-15.2.4b). `summary` (with the
    /// `note` already appended) and `import_label` (`import now` /
    /// `import now (N)`) are formatted in Rust; C++ only assigns them.
    #[derive(Clone, Debug, PartialEq, Default)]
    struct LocalImportReviewDialogState {
        summary: String,
        search_subdirectories: bool,
        delete_after_success: bool,
        can_import: bool,
        import_label: String,
        paths: Vec<LocalImportPathRow>,
    }

    /// Local import tagging dialog contents (WS-15.2.4b).
    ///
    /// R1 exception: the dialog edits arbitrarily nested
    /// `FilenameTaggingOptions` (regex lists, per-service dicts) through the
    /// shared options editor, whose schema is not fixed. The typed part
    /// (`tag_services`, `example_path`, `import_after_save`) travels typed;
    /// only the free-form rules stay JSON.
    #[derive(Clone, Debug, PartialEq, Default)]
    struct LocalImportTaggingDialogState {
        json: String,
    }

    /// One `value`/`label` choice of a duplicates dialog combo (WS-15.2.4b).
    /// C++ only fills combo items; Rust owns the labels.
    #[derive(Clone, Debug, PartialEq, Default)]
    struct DuplicateChoiceRow {
        value: String,
        label: String,
    }

    /// One service action of a merge options page (WS-15.2.4b).
    #[derive(Clone, Debug, PartialEq, Default)]
    struct MergeServiceActionRow {
        service: String,
        action: String,
    }

    /// One merge options page (`this is better` / `same quality`) (WS-15.2.4b).
    #[derive(Clone, Debug, PartialEq, Default)]
    struct MergeOptionsPage {
        archive_action: String,
        modified_date_action: String,
        urls_action: String,
        notes_action: String,
        tag_actions: Vec<MergeServiceActionRow>,
        rating_actions: Vec<MergeServiceActionRow>,
    }

    /// Shared merge options data of the merge dialog and the per-pair
    /// custom action dialog (WS-15.2.4b). `better_summary`/`same_summary`
    /// are formatted in Rust.
    #[derive(Clone, Debug, PartialEq, Default)]
    struct MergeOptionsData {
        better: MergeOptionsPage,
        same: MergeOptionsPage,
        tag_services: Vec<String>,
        rating_services: Vec<String>,
        actions: Vec<DuplicateChoiceRow>,
        archive_actions: Vec<DuplicateChoiceRow>,
        better_summary: String,
        same_summary: String,
    }

    /// One flattened comparator of a rule selector (WS-15.2.4b). Groups open
    /// a new depth level; C++ rebuilds the nested array from `depth`
    /// without parsing JSON. `pretty` is formatted in Rust.
    #[derive(Clone, Debug, PartialEq, Default)]
    struct RuleComparatorRow {
        kind: String,
        looking_at: String,
        property: String,
        op: String,
        num_value: f64,
        text_value: String,
        multiplier: f64,
        delta: f64,
        same: bool,
        group_mode: String,
        depth: i64,
        pretty: String,
    }

    /// One auto-resolution rule of the rules dialog (WS-15.2.4b).
    /// `operation_label`/`search_summary`/`selector_summary`/`action_summary`
    /// are formatted in Rust (R2); C++ only assigns cell texts.
    #[derive(Clone, Debug, PartialEq, Default)]
    struct AutoRuleRow {
        name: String,
        paused: bool,
        operation_mode: String,
        operation_label: String,
        max_pending_pairs: i64,
        search_query: String,
        search_mode: String,
        max_distance: i64,
        action: String,
        delete_a: bool,
        delete_b: bool,
        search_summary: String,
        selector_summary: String,
        action_summary: String,
        selectors: Vec<RuleComparatorRow>,
    }

    /// Auto-resolution rules dialog contents (WS-15.2.4b).
    #[derive(Clone, Debug, PartialEq, Default)]
    struct DuplicatesRulesDialogState {
        enabled: bool,
        pending: i64,
        pending_tooltip: String,
        rules: Vec<AutoRuleRow>,
        properties: Vec<DuplicateChoiceRow>,
        operators: Vec<DuplicateChoiceRow>,
        looking_at: Vec<DuplicateChoiceRow>,
        search_modes: Vec<DuplicateChoiceRow>,
        group_modes: Vec<DuplicateChoiceRow>,
        actions_list: Vec<DuplicateChoiceRow>,
        operation_modes: Vec<DuplicateChoiceRow>,
    }

    /// Auto-resolution merge options dialog contents (WS-15.2.4b).
    #[derive(Clone, Debug, PartialEq, Default)]
    struct DuplicatesMergeOptionsDialogState {
        options: MergeOptionsData,
    }

    /// Auto-resolution custom action dialog contents (WS-15.2.4b).
    /// `has_pair` replaces the JSON null: the pair fields are only read
    /// when it is true. `pair_summary` is formatted in Rust (R2).
    #[derive(Clone, Debug, PartialEq, Default)]
    struct DuplicateCustomActionDialogState {
        page_id: String,
        has_pair: bool,
        pair_a_hash: String,
        pair_a_info: String,
        pair_b_hash: String,
        pair_b_info: String,
        pair_summary: String,
        duplicate_types: Vec<DuplicateChoiceRow>,
        better_sides: Vec<DuplicateChoiceRow>,
        delete_modes: Vec<DuplicateChoiceRow>,
        options: MergeOptionsData,
    }

    /// One pair sample of a rule preview (WS-15.2.4b).
    #[derive(Clone, Debug, PartialEq, Default)]
    struct PreviewPairRow {
        hash_a: String,
        hash_b: String,
        thumb_a: String,
        thumb_b: String,
        distance: i64,
        decision: String,
    }

    /// Auto-resolution rule preview contents (WS-15.2.4b). `status`,
    /// `selector_text` and `search_text` are formatted in Rust (R2).
    #[derive(Clone, Debug, PartialEq, Default)]
    struct DuplicatesRulePreviewState {
        status: String,
        selector_text: String,
        search_text: String,
        total: i64,
        matched: i64,
        failed: i64,
        samples: Vec<PreviewPairRow>,
        failed_samples: Vec<PreviewPairRow>,
    }

    /// One pair row of the review dialog (WS-15.2.4b). Unused columns of a
    /// tab stay empty; `status` is formatted in Rust (R2).
    #[derive(Clone, Debug, PartialEq, Default)]
    struct ReviewPairRow {
        hash_a: String,
        hash_b: String,
        thumb_a: String,
        thumb_b: String,
        distance: i64,
        rule: String,
        decision: String,
        decision_label: String,
        banned: bool,
        delete_a: bool,
        delete_b: bool,
    }

    /// Auto-resolution review dialog contents (WS-15.2.4b).
    #[derive(Clone, Debug, PartialEq, Default)]
    struct DuplicateReviewDialogState {
        status: String,
        pending: Vec<ReviewPairRow>,
        denied: Vec<ReviewPairRow>,
        actioned: Vec<ReviewPairRow>,
    }

    /// Media viewer contents (WS-15.2.2b).
    ///
    /// Only fields the C++ side reads. The old JSON seam also carried `tags`,
    /// `muted` and `prefetch_*`: none of them was consumed by the viewer
    /// window, so they are gone.
    #[derive(Clone, Debug, PartialEq)]
    struct ViewerState {
        viewer_id: String,
        page_id: String,
        hash: String,
        title: String,
        path: String,
        file_path: String,
        url: String,
        previous_path: String,
        next_path: String,
        info: String,
        always_on_top: bool,
        frameless: bool,
        recenter: bool,
        zoom_step_percent: i32,
        metadata: MetadataState,
    }

    unsafe extern "C++" {
        include!("bridge.hpp");

        type QApplication;
        type QSplashScreen;
        type HydrusMainWindow;

        fn new_application() -> UniquePtr<QApplication>;
        fn exec_application(application: Pin<&mut QApplication>) -> i32;
        fn quit_application(application: Pin<&mut QApplication>);
        fn qapp_process_events();

        fn new_hydrus_splash() -> UniquePtr<QSplashScreen>;
        fn hydrus_splash_show(splash: Pin<&mut QSplashScreen>);
        fn hydrus_splash_message(splash: Pin<&mut QSplashScreen>, message: &str);
        fn hydrus_splash_finish(splash: Pin<&mut QSplashScreen>);
        fn hydrus_splash_finish_later(splash: Pin<&mut QSplashScreen>, delay_ms: i32);
        fn hydrus_splash_screenshot(splash: Pin<&mut QSplashScreen>, path: &str, delay_ms: i32);

        fn new_hydrus_main_window() -> UniquePtr<HydrusMainWindow>;
        fn build_hydrus_shell(window: Pin<&mut HydrusMainWindow>);
        fn show_hydrus_main_window(window: Pin<&mut HydrusMainWindow>);

        fn hydrus_set_title(window: Pin<&mut HydrusMainWindow>, title: &str);
        fn hydrus_resize_window(window: Pin<&mut HydrusMainWindow>, width: i32, height: i32);
        fn hydrus_set_menus(window: Pin<&mut HydrusMainWindow>, menus: &MenuBar);
        fn hydrus_set_tag_suggestions(window: Pin<&mut HydrusMainWindow>, json: &str);
        fn hydrus_show_autocomplete(window: Pin<&mut HydrusMainWindow>, state: &AutocompleteState);
        fn hydrus_set_status(window: Pin<&mut HydrusMainWindow>, status: &StatusBar);
        fn hydrus_build_pages(window: Pin<&mut HydrusMainWindow>, pages: &Pages);
        fn hydrus_update_page(window: Pin<&mut HydrusMainWindow>, page: &QueryPageState);
        fn hydrus_update_page_progress(window: Pin<&mut HydrusMainWindow>, progress: &PageProgress);
        fn hydrus_update_downloader(
            window: Pin<&mut HydrusMainWindow>,
            state: &DownloaderPageState,
        );
        fn hydrus_update_subscriptions(
            window: Pin<&mut HydrusMainWindow>,
            state: &SubscriptionsPageState,
        );
        fn hydrus_show_subscriptions_dialog(
            window: Pin<&mut HydrusMainWindow>,
            state: &SubscriptionsDialogState,
        );
        fn hydrus_update_subscriptions_dialog(
            window: Pin<&mut HydrusMainWindow>,
            state: &SubscriptionsDialogState,
        );
        fn hydrus_show_subscription_editor(
            window: Pin<&mut HydrusMainWindow>,
            state: &SubscriptionEditorState,
        );
        fn hydrus_update_duplicates(
            window: Pin<&mut HydrusMainWindow>,
            state: &DuplicatesPageState,
        );
        fn hydrus_update_import_folders(
            window: Pin<&mut HydrusMainWindow>,
            page_id: &str,
            state: &FolderPageState,
        );
        fn hydrus_show_import_folders_dialog(window: Pin<&mut HydrusMainWindow>, state: &FolderPageState);
        fn hydrus_update_import_folders_dialog(window: Pin<&mut HydrusMainWindow>, state: &FolderPageState);
        fn hydrus_update_export_folders(
            window: Pin<&mut HydrusMainWindow>,
            page_id: &str,
            state: &FolderPageState,
        );
        fn hydrus_show_export_folders_dialog(window: Pin<&mut HydrusMainWindow>, state: &FolderPageState);
        fn hydrus_update_export_folders_dialog(window: Pin<&mut HydrusMainWindow>, state: &FolderPageState);
        fn hydrus_show_duplicates_rules(
            window: Pin<&mut HydrusMainWindow>,
            state: &DuplicatesRulesDialogState,
        );
        fn hydrus_show_duplicate_merge_options(
            window: Pin<&mut HydrusMainWindow>,
            state: &DuplicatesMergeOptionsDialogState,
        );
        fn hydrus_show_import_options(
            window: Pin<&mut HydrusMainWindow>,
            state: &ImportOptionsDialogState,
        );
        fn hydrus_show_duplicate_custom_action(
            window: Pin<&mut HydrusMainWindow>,
            state: &DuplicateCustomActionDialogState,
        );
        fn hydrus_update_auto_resolution_preview(
            window: Pin<&mut HydrusMainWindow>,
            state: &DuplicatesRulePreviewState,
        );
        fn hydrus_show_duplicate_review(
            window: Pin<&mut HydrusMainWindow>,
            state: &DuplicateReviewDialogState,
        );
        fn hydrus_update_duplicate_review(
            window: Pin<&mut HydrusMainWindow>,
            state: &DuplicateReviewDialogState,
        );
        fn hydrus_show_tag_relations(
            window: Pin<&mut HydrusMainWindow>,
            state: &TagRelationsDialogState,
        );
        fn hydrus_show_import_folder(window: Pin<&mut HydrusMainWindow>, state: &ImportFolderEditorState);
        fn hydrus_show_local_import_review(
            window: Pin<&mut HydrusMainWindow>,
            state: &LocalImportReviewDialogState,
        );
        fn hydrus_show_local_import_tagging(
            window: Pin<&mut HydrusMainWindow>,
            state: &LocalImportTaggingDialogState,
        );
        fn hydrus_close_local_import_review(window: Pin<&mut HydrusMainWindow>);
        fn hydrus_show_repair_file_locations(
            window: Pin<&mut HydrusMainWindow>,
            state: &RepairFileLocationsDialogState,
        );
        fn hydrus_close_repair_file_locations(window: Pin<&mut HydrusMainWindow>);
        fn hydrus_show_export_folder(window: Pin<&mut HydrusMainWindow>, state: &ExportFolderEditorState);
        fn hydrus_show_export_review(
            window: Pin<&mut HydrusMainWindow>,
            state: &ExportReviewDialogState,
        );
        fn hydrus_show_notes(window: Pin<&mut HydrusMainWindow>, state: &NotesDialogState);
        fn hydrus_set_selection(window: Pin<&mut HydrusMainWindow>, selection: &SelectionState);
        fn hydrus_show_page_chooser(window: Pin<&mut HydrusMainWindow>, notebook_id: &str);
        fn hydrus_show_page_manager(window: Pin<&mut HydrusMainWindow>, tree: &PageTree);
        fn hydrus_show_command_palette(window: Pin<&mut HydrusMainWindow>);
        fn hydrus_prompt_text(
            window: Pin<&mut HydrusMainWindow>,
            action: UiAction,
            title: &str,
            label: &str,
            initial: &str,
        );
        fn hydrus_confirm_action(
            window: Pin<&mut HydrusMainWindow>,
            action: UiAction,
            target: &str,
            title: &str,
            text: &str,
        );
        fn hydrus_show_or_dialog(window: Pin<&mut HydrusMainWindow>, page_id: &str, initial: &str);
        fn hydrus_pick_files(window: Pin<&mut HydrusMainWindow>);
        fn hydrus_prompt_url(window: Pin<&mut HydrusMainWindow>);
        fn hydrus_open_viewer(window: Pin<&mut HydrusMainWindow>, state: &ViewerState);
        fn hydrus_update_viewer(window: Pin<&mut HydrusMainWindow>, state: &ViewerState);
        fn hydrus_close_viewer(window: Pin<&mut HydrusMainWindow>, viewer_id: &str);
        fn hydrus_show_tag_dialog(window: Pin<&mut HydrusMainWindow>, state: &TagDialogState);
        fn hydrus_show_notifications(
            window: Pin<&mut HydrusMainWindow>,
            state: &NotificationsDialogState,
        );
        fn hydrus_show_viewing_stats(window: Pin<&mut HydrusMainWindow>, state: &ViewingStats);
        fn hydrus_show_about(window: Pin<&mut HydrusMainWindow>, state: &AboutInfo);
        fn hydrus_export_downloader(window: Pin<&mut HydrusMainWindow>, page_id: &str);
        fn hydrus_show_file_seed_cache(window: Pin<&mut HydrusMainWindow>, state: &SeedCacheState);
        fn hydrus_show_gallery_seed_log(window: Pin<&mut HydrusMainWindow>, state: &SeedCacheState);
        fn hydrus_import_downloader(window: Pin<&mut HydrusMainWindow>, target: &str);
        fn hydrus_show_manage_tags(
            window: Pin<&mut HydrusMainWindow>,
            state: &ManageTagsDialogState,
        );
        fn hydrus_show_timestamps(
            window: Pin<&mut HydrusMainWindow>,
            state: &TimestampsDialogState,
        );
        fn hydrus_show_options(window: Pin<&mut HydrusMainWindow>, state: &OptionsDialogState);
        fn hydrus_show_services(window: Pin<&mut HydrusMainWindow>, state: &ServicesDialogState);
        fn hydrus_update_services(window: Pin<&mut HydrusMainWindow>, state: &ServicesDialogState);
        fn hydrus_show_tag_filter(window: Pin<&mut HydrusMainWindow>, state: &TagFilterDialogState);
        fn hydrus_show_api_permissions(
            window: Pin<&mut HydrusMainWindow>,
            state: &ApiPermissionsDialogState,
        );
        fn hydrus_update_api_permissions(
            window: Pin<&mut HydrusMainWindow>,
            state: &ApiPermissionsDialogState,
        );
        fn hydrus_show_account_types(
            window: Pin<&mut HydrusMainWindow>,
            state: &AccountTypesDialogState,
        );
        fn hydrus_update_account_types(
            window: Pin<&mut HydrusMainWindow>,
            state: &AccountTypesDialogState,
        );
        fn hydrus_show_orphan_files(
            window: Pin<&mut HydrusMainWindow>,
            state: &OrphanFilesDialogState,
        );
        fn hydrus_update_orphan_files(
            window: Pin<&mut HydrusMainWindow>,
            state: &OrphanFilesDialogState,
        );
        fn hydrus_show_database_backup(
            window: Pin<&mut HydrusMainWindow>,
            state: &DatabaseBackupDialogState,
        );
        fn hydrus_update_database_backup(
            window: Pin<&mut HydrusMainWindow>,
            state: &DatabaseBackupDialogState,
        );
        fn hydrus_show_network(window: Pin<&mut HydrusMainWindow>, state: &NetworkDialogState);
        fn hydrus_update_network(window: Pin<&mut HydrusMainWindow>, state: &NetworkDialogState);
        fn hydrus_start_tick(window: Pin<&mut HydrusMainWindow>);
        fn hydrus_defer_action(
            window: Pin<&mut HydrusMainWindow>,
            action: UiAction,
            target: &str,
            payload: &str,
            delay_ms: i32,
        );
        fn hydrus_show_message(window: Pin<&mut HydrusMainWindow>, title: &str, text: &str);
        fn hydrus_notify_message(window: Pin<&mut HydrusMainWindow>, message: &str);
        fn hydrus_set_clipboard(window: Pin<&mut HydrusMainWindow>, text: &str);
        fn hydrus_open_path(path: &str);
        fn hydrus_open_url(url: &str);
        fn hydrus_schedule_screenshot(window: Pin<&mut HydrusMainWindow>, path: &str);

        fn qapp_set_application_name(name: &str);
        fn qapp_set_application_display_name(name: &str);
        fn qapp_set_organization_name(name: &str);
        fn qapp_set_style(name: &str);
        fn qapp_set_stylesheet(stylesheet: &str);
        fn qapp_quit();
    }

    extern "Rust" {
        fn app_action(
            window: Pin<&mut HydrusMainWindow>,
            action: UiAction,
            target: &str,
            payload: &str,
        );
        fn app_tick(window: Pin<&mut HydrusMainWindow>);
        fn action_from_str(action: &str) -> UiAction;
        /// Typed reverse seam (WS-15.2.2a): tab drag & drop between notebooks.
        fn app_move_page(
            window: Pin<&mut HydrusMainWindow>,
            source: &str,
            target: &str,
            position: &str,
        );
        /// Typed reverse seam (WS-15.2.2a): manage-pages dialog close actions.
        fn app_close_pages(window: Pin<&mut HydrusMainWindow>, ids: Vec<String>);
        /// Typed reverse seams (WS-15.2.2b): thumbnail grid selection, focus,
        /// viewer open and page-to-page drag. Same cores as the `"..."` notify
        /// actions, without any JSON payload.
        fn app_set_selection(
            window: Pin<&mut HydrusMainWindow>,
            page_id: &str,
            hashes: Vec<String>,
        );
        fn app_focus_hash(window: Pin<&mut HydrusMainWindow>, page_id: &str, hash: &str);
        fn app_open_viewer(window: Pin<&mut HydrusMainWindow>, page_id: &str, hash: &str);
        fn app_add_hashes(window: Pin<&mut HydrusMainWindow>, page_id: &str, hashes: Vec<String>);
        /// Typed reverse seams (WS-15.2.2b): metadata panel tag/rating/notes
        /// actions. Same cores as `tag_add`/`tag_remove`, `rating_set` and
        /// `notes_manage`, without any JSON payload.
        fn app_tag_edit(
            window: Pin<&mut HydrusMainWindow>,
            hash: &str,
            service: &str,
            tag: &str,
            add: bool,
        );
        fn app_set_rating(
            window: Pin<&mut HydrusMainWindow>,
            hash: &str,
            service: &str,
            action: &str,
            value: f64,
        );
        fn app_manage_notes(window: Pin<&mut HydrusMainWindow>, hash: &str);
        /// Typed reverse seams (WS-15.2.3a): page/tab/session/history events.
        fn app_page_selected(window: Pin<&mut HydrusMainWindow>, page_id: &str);
        fn app_close_page(window: Pin<&mut HydrusMainWindow>, page_id: &str);
        fn app_close_other_pages(window: Pin<&mut HydrusMainWindow>, page_id: &str);
        fn app_rename_page(window: Pin<&mut HydrusMainWindow>, page_id: &str, name: &str);
        fn app_refresh_page(window: Pin<&mut HydrusMainWindow>, page_id: &str);
        fn app_page_manager_activate(window: Pin<&mut HydrusMainWindow>, page_id: &str);
        fn app_page_sort(
            window: Pin<&mut HydrusMainWindow>,
            page_id: &str,
            sort: &str,
            ascending: bool,
        );
        fn app_history_step(window: Pin<&mut HydrusMainWindow>, step: i32);
        /// Typed reverse seams (WS-15.2.4c): services + network.
        fn app_service_save(
            window: Pin<&mut HydrusMainWindow>,
            original: &str,
            name: &str,
            type_int: i32,
            remote: bool,
            host: &str,
            port: i32,
            access_key: &str,
            has_update: bool,
            update_period: i64,
            has_nullify: bool,
            nullify_period: i64,
            max_storage: i64,
            log_uploader_ips: bool,
        );
        fn app_service_delete(window: Pin<&mut HydrusMainWindow>, key: &str);
        fn app_login_save(
            window: Pin<&mut HydrusMainWindow>,
            domain: &str,
            username: &str,
            password: &str,
            access_key: &str,
            expires: i64,
        );
        fn app_login_remove(window: Pin<&mut HydrusMainWindow>, domain: &str);
        fn app_admin_connect(window: Pin<&mut HydrusMainWindow>, address: &str, access_key: &str);
        fn app_admin_select_service(
            window: Pin<&mut HydrusMainWindow>,
            service_key: &str,
            access_key: &str,
        );
        fn app_admin_service_action(window: Pin<&mut HydrusMainWindow>, action: &str, period: i64);
        fn app_admin_add_service(
            window: Pin<&mut HydrusMainWindow>,
            name: &str,
            type_int: i32,
            port: i32,
        );
        fn app_admin_edit_service(
            window: Pin<&mut HydrusMainWindow>,
            service_key: &str,
            name: &str,
            port: i32,
        );
        fn app_admin_account_action(
            window: Pin<&mut HydrusMainWindow>,
            action: &str,
            subject_account_key: &str,
            text: &str,
            expires: i64,
        );
        fn app_network_save(
            window: Pin<&mut HydrusMainWindow>,
            api_enabled: bool,
            network_timeout: i32,
            max_network_jobs: i32,
            max_network_jobs_per_domain: i32,
            api_port: i32,
            api_access_key: &str,
            http_proxy: &str,
            https_proxy: &str,
            no_proxy: &str,
        );
        fn app_network_header(
            window: Pin<&mut HydrusMainWindow>,
            name: &str,
            value: &str,
            remove: bool,
        );
        fn app_network_url_class_save(
            window: Pin<&mut HydrusMainWindow>,
            original: &str,
            name: &str,
            url_type: i32,
            preferred_scheme: &str,
            netloc: &str,
            path: &str,
            parameters: &str,
            example_url: &str,
            match_subdomains: bool,
            keep_matched_subdomains: bool,
            can_produce_multiple_files: bool,
            should_be_associated_with_files: bool,
            is_add: bool,
        );
        fn app_network_url_class_remove(window: Pin<&mut HydrusMainWindow>, name: &str);
        fn app_network_pool_set(window: Pin<&mut HydrusMainWindow>, domain: &str, limit: i32);
        fn app_network_pool_remove(window: Pin<&mut HydrusMainWindow>, domain: &str);
        fn app_network_rule_set(
            window: Pin<&mut HydrusMainWindow>,
            context_type: &str,
            context: &str,
            bandwidth_type: &str,
            time_delta: i64,
            max_allowed: i64,
        );
        fn app_network_rule_remove(
            window: Pin<&mut HydrusMainWindow>,
            context_type: &str,
            context: &str,
            bandwidth_type: &str,
        );
        /// Typed reverse seams (WS-15.2.4c): tags, duplicates, subscriptions.
        fn app_tag_filter_test(
            window: Pin<&mut HydrusMainWindow>,
            slices: Vec<String>,
            rules: Vec<String>,
            tag: &str,
        );
        fn app_tag_filter_save(
            window: Pin<&mut HydrusMainWindow>,
            service_key: &str,
            slices: Vec<String>,
            rules: Vec<String>,
        );
        fn app_purge_tags(
            window: Pin<&mut HydrusMainWindow>,
            service_key: &str,
            tags: Vec<String>,
            reason: &str,
        );
        fn app_tag_relation_add(
            window: Pin<&mut HydrusMainWindow>,
            relation: &str,
            service: &str,
            first: &str,
            second: &str,
        );
        fn app_tag_relation_remove(
            window: Pin<&mut HydrusMainWindow>,
            relation: &str,
            service: &str,
            first: &str,
            second: &str,
        );
        fn app_duplicates_batch_size(
            window: Pin<&mut HydrusMainWindow>,
            page_id: &str,
            value: i64,
        );
        fn app_duplicates_search_context(
            window: Pin<&mut HydrusMainWindow>,
            page_id: &str,
            query_1: &str,
            location_1: &str,
            query_2: &str,
            location_2: &str,
            dupe_search_type: &str,
            pixel_dupes: &str,
            max_distance: i64,
            sort_type: &str,
            sort_ascending: bool,
        );
        fn app_duplicates_group_action(
            window: Pin<&mut HydrusMainWindow>,
            page_id: &str,
            kind: &str,
            target: &str,
        );
        fn app_subscription_add(
            window: Pin<&mut HydrusMainWindow>,
            page_id: &str,
            name: &str,
            url: &str,
            query: &str,
            site: &str,
            filter: &str,
            period: i64,
        );
        fn app_subscription_edit_save(
            window: Pin<&mut HydrusMainWindow>,
            page_id: &str,
            original: &str,
            name: &str,
            url: &str,
            query: &str,
            site: &str,
            filter: &str,
            period: i64,
            paused: bool,
        );
        /// `container_json` holds the arbitrary nested import options dict
        /// (same R1 exception as `SubscriptionDialogRow.import_options_json`).
        fn app_subscription_import_options_save(
            window: Pin<&mut HydrusMainWindow>,
            name: &str,
            container_json: &str,
        );
        fn app_file_seed_action(
            window: Pin<&mut HydrusMainWindow>,
            page: &str,
            action: &str,
            urls: Vec<String>,
        );
        fn app_gallery_seed_action(
            window: Pin<&mut HydrusMainWindow>,
            page: &str,
            action: &str,
            urls: Vec<String>,
        );
        fn app_downloader_start(
            window: Pin<&mut HydrusMainWindow>,
            page_id: &str,
            input: &str,
            url: &str,
            query: &str,
            site: &str,
            formula: &str,
            subject: &str,
            auto_watch: bool,
            period: i64,
            no_limit: bool,
            file_limit: i64,
            present: i64,
        );
        fn app_downloader_jobs_set(
            window: Pin<&mut HydrusMainWindow>,
            page_id: &str,
            urls: Vec<String>,
            remove_missing: bool,
        );
        fn app_duplicates_rule_save(
            window: Pin<&mut HydrusMainWindow>,
            name: &str,
            paused: bool,
            operation_mode: &str,
            max_pending_pairs: i64,
            search_query: &str,
            search_mode: &str,
            max_distance: i64,
            selectors: Vec<RuleComparatorRow>,
            action: &str,
            delete_a: bool,
            delete_b: bool,
            original_name: &str,
        );
        fn app_duplicates_rule_preview(
            window: Pin<&mut HydrusMainWindow>,
            name: &str,
            paused: bool,
            operation_mode: &str,
            max_pending_pairs: i64,
            search_query: &str,
            search_mode: &str,
            max_distance: i64,
            selectors: Vec<RuleComparatorRow>,
            action: &str,
            delete_a: bool,
            delete_b: bool,
        );
        fn hydrus_validate_export_phrase(phrase: &str) -> String;
        fn hydrus_static_dir() -> String;
    }
}

use std::pin::Pin;

fn app_action(
    mut window: Pin<&mut ffi::HydrusMainWindow>,
    action: ffi::UiAction,
    target: &str,
    payload: &str,
) {
    crate::gui::app::handle_action(window.as_mut(), action, target, payload);
}

fn app_tick(mut window: Pin<&mut ffi::HydrusMainWindow>) {
    crate::gui::app::handle_tick(window.as_mut());
}

fn app_move_page(
    mut window: Pin<&mut ffi::HydrusMainWindow>,
    source: &str,
    target: &str,
    position: &str,
) {
    crate::gui::app::handle_move_page(window.as_mut(), source, target, position);
}

fn app_close_pages(mut window: Pin<&mut ffi::HydrusMainWindow>, ids: Vec<String>) {
    crate::gui::app::handle_close_pages(window.as_mut(), ids);
}

/// Typed reverse seams (WS-15.2.2b): media/selection/viewer actions from the
/// thumbnail grid, the query page and the metadata panel.
fn app_set_selection(
    mut window: Pin<&mut ffi::HydrusMainWindow>,
    page_id: &str,
    hashes: Vec<String>,
) {
    crate::gui::app::handle_set_selection(window.as_mut(), page_id, &hashes);
}

fn app_focus_hash(mut window: Pin<&mut ffi::HydrusMainWindow>, page_id: &str, hash: &str) {
    crate::gui::app::handle_focus_hash(window.as_mut(), page_id, hash);
}

fn app_open_viewer(mut window: Pin<&mut ffi::HydrusMainWindow>, page_id: &str, hash: &str) {
    crate::gui::app::handle_open_viewer(window.as_mut(), page_id, hash);
}

fn app_add_hashes(mut window: Pin<&mut ffi::HydrusMainWindow>, page_id: &str, hashes: Vec<String>) {
    crate::gui::app::handle_add_hashes(window.as_mut(), page_id, &hashes);
}

fn app_tag_edit(
    mut window: Pin<&mut ffi::HydrusMainWindow>,
    hash: &str,
    service: &str,
    tag: &str,
    add: bool,
) {
    crate::gui::app::handle_tag_edit(window.as_mut(), hash, service, tag, add);
}

fn app_set_rating(
    mut window: Pin<&mut ffi::HydrusMainWindow>,
    hash: &str,
    service: &str,
    action: &str,
    value: f64,
) {
    crate::gui::app::handle_set_rating(window.as_mut(), hash, service, action, value);
}

fn app_manage_notes(mut window: Pin<&mut ffi::HydrusMainWindow>, hash: &str) {
    crate::gui::app::handle_manage_notes(window.as_mut(), hash);
}

/// Typed reverse seams (WS-15.2.3a): page/tab/session/history events.
fn app_page_selected(mut window: Pin<&mut ffi::HydrusMainWindow>, page_id: &str) {
    crate::gui::app::handle_page_selected(window.as_mut(), page_id);
}

fn app_close_page(mut window: Pin<&mut ffi::HydrusMainWindow>, page_id: &str) {
    crate::gui::app::handle_close_page(window.as_mut(), page_id);
}

fn app_close_other_pages(mut window: Pin<&mut ffi::HydrusMainWindow>, page_id: &str) {
    crate::gui::app::handle_close_other_pages(window.as_mut(), page_id);
}

fn app_rename_page(mut window: Pin<&mut ffi::HydrusMainWindow>, page_id: &str, name: &str) {
    crate::gui::app::handle_rename_page(window.as_mut(), page_id, name);
}

fn app_refresh_page(mut window: Pin<&mut ffi::HydrusMainWindow>, page_id: &str) {
    crate::gui::app::handle_refresh_page(window.as_mut(), page_id);
}

fn app_page_manager_activate(mut window: Pin<&mut ffi::HydrusMainWindow>, page_id: &str) {
    crate::gui::app::handle_page_manager_activate(window.as_mut(), page_id);
}

fn app_page_sort(
    mut window: Pin<&mut ffi::HydrusMainWindow>,
    page_id: &str,
    sort: &str,
    ascending: bool,
) {
    crate::gui::app::handle_page_sort(window.as_mut(), page_id, sort, ascending);
}

fn app_history_step(mut window: Pin<&mut ffi::HydrusMainWindow>, step: i32) {
    crate::gui::app::handle_history_step(window.as_mut(), step);
}

fn app_service_save(
    mut window: Pin<&mut ffi::HydrusMainWindow>,
    original: &str,
    name: &str,
    type_int: i32,
    remote: bool,
    host: &str,
    port: i32,
    access_key: &str,
    has_update: bool,
    update_period: i64,
    has_nullify: bool,
    nullify_period: i64,
    max_storage: i64,
    log_uploader_ips: bool,
) {
    crate::gui::app::handle_service_save(
        window.as_mut(), original, name, type_int, remote, host, port, access_key, has_update,
        update_period, has_nullify, nullify_period, max_storage, log_uploader_ips,
    );
}

fn app_service_delete(mut window: Pin<&mut ffi::HydrusMainWindow>, key: &str) {
    crate::gui::app::handle_service_delete(window.as_mut(), key);
}

fn app_login_save(
    mut window: Pin<&mut ffi::HydrusMainWindow>,
    domain: &str,
    username: &str,
    password: &str,
    access_key: &str,
    expires: i64,
) {
    crate::gui::app::handle_login_save_typed(
        window.as_mut(), domain, username, password, access_key, expires,
    );
}

fn app_login_remove(mut window: Pin<&mut ffi::HydrusMainWindow>, domain: &str) {
    crate::gui::app::handle_login_remove(window.as_mut(), domain);
}

fn app_admin_connect(mut window: Pin<&mut ffi::HydrusMainWindow>, address: &str, access_key: &str) {
    crate::gui::app::handle_admin_connect(window.as_mut(), address, access_key);
}

fn app_admin_select_service(
    mut window: Pin<&mut ffi::HydrusMainWindow>,
    service_key: &str,
    access_key: &str,
) {
    crate::gui::app::handle_admin_select_service(window.as_mut(), service_key, access_key);
}

fn app_admin_service_action(
    mut window: Pin<&mut ffi::HydrusMainWindow>,
    action: &str,
    period: i64,
) {
    crate::gui::app::handle_admin_service_action(window.as_mut(), action, period);
}

fn app_admin_add_service(
    mut window: Pin<&mut ffi::HydrusMainWindow>,
    name: &str,
    type_int: i32,
    port: i32,
) {
    crate::gui::app::handle_admin_add_service(window.as_mut(), name, type_int, port);
}

fn app_admin_edit_service(
    mut window: Pin<&mut ffi::HydrusMainWindow>,
    service_key: &str,
    name: &str,
    port: i32,
) {
    crate::gui::app::handle_admin_edit_service(window.as_mut(), service_key, name, port);
}

fn app_admin_account_action(
    mut window: Pin<&mut ffi::HydrusMainWindow>,
    action: &str,
    subject_account_key: &str,
    text: &str,
    expires: i64,
) {
    crate::gui::app::handle_admin_account_action(
        window.as_mut(), action, subject_account_key, text, expires,
    );
}

fn app_network_save(
    mut window: Pin<&mut ffi::HydrusMainWindow>,
    api_enabled: bool,
    network_timeout: i32,
    max_network_jobs: i32,
    max_network_jobs_per_domain: i32,
    api_port: i32,
    api_access_key: &str,
    http_proxy: &str,
    https_proxy: &str,
    no_proxy: &str,
) {
    crate::gui::app::handle_network_save(
        window.as_mut(), api_enabled, network_timeout, max_network_jobs,
        max_network_jobs_per_domain, api_port, api_access_key, http_proxy, https_proxy,
        no_proxy,
    );
}

fn app_network_header(
    mut window: Pin<&mut ffi::HydrusMainWindow>,
    name: &str,
    value: &str,
    remove: bool,
) {
    crate::gui::app::handle_network_header(window.as_mut(), name, value, remove);
}

#[allow(clippy::too_many_arguments)]
fn app_network_url_class_save(
    mut window: Pin<&mut ffi::HydrusMainWindow>,
    original: &str,
    name: &str,
    url_type: i32,
    preferred_scheme: &str,
    netloc: &str,
    path: &str,
    parameters: &str,
    example_url: &str,
    match_subdomains: bool,
    keep_matched_subdomains: bool,
    can_produce_multiple_files: bool,
    should_be_associated_with_files: bool,
    is_add: bool,
) {
    crate::gui::app::handle_network_url_class_save(
        window.as_mut(), original, name, url_type, preferred_scheme, netloc, path,
        parameters, example_url, match_subdomains, keep_matched_subdomains,
        can_produce_multiple_files, should_be_associated_with_files, is_add,
    );
}

fn app_network_url_class_remove(mut window: Pin<&mut ffi::HydrusMainWindow>, name: &str) {
    crate::gui::app::handle_network_url_class_remove(window.as_mut(), name);
}

fn app_network_pool_set(mut window: Pin<&mut ffi::HydrusMainWindow>, domain: &str, limit: i32) {
    crate::gui::app::handle_network_pool_set(window.as_mut(), domain, limit);
}

fn app_network_pool_remove(mut window: Pin<&mut ffi::HydrusMainWindow>, domain: &str) {
    crate::gui::app::handle_network_pool_remove(window.as_mut(), domain);
}

fn app_network_rule_set(
    mut window: Pin<&mut ffi::HydrusMainWindow>,
    context_type: &str,
    context: &str,
    bandwidth_type: &str,
    time_delta: i64,
    max_allowed: i64,
) {
    crate::gui::app::handle_network_rule_set(
        window.as_mut(), context_type, context, bandwidth_type, time_delta, max_allowed,
    );
}

fn app_network_rule_remove(
    mut window: Pin<&mut ffi::HydrusMainWindow>,
    context_type: &str,
    context: &str,
    bandwidth_type: &str,
) {
    crate::gui::app::handle_network_rule_remove(
        window.as_mut(), context_type, context, bandwidth_type,
    );
}

/// Typed reverse seams (WS-15.2.4c): tags, duplicates, subscriptions.
fn app_tag_filter_test(
    mut window: Pin<&mut ffi::HydrusMainWindow>,
    slices: Vec<String>,
    rules: Vec<String>,
    tag: &str,
) {
    crate::gui::app::handle_tag_filter_test(window.as_mut(), slices, rules, tag);
}

fn app_tag_filter_save(
    mut window: Pin<&mut ffi::HydrusMainWindow>,
    service_key: &str,
    slices: Vec<String>,
    rules: Vec<String>,
) {
    crate::gui::app::handle_tag_filter_save(window.as_mut(), service_key, slices, rules);
}

fn app_purge_tags(
    mut window: Pin<&mut ffi::HydrusMainWindow>,
    service_key: &str,
    tags: Vec<String>,
    reason: &str,
) {
    crate::gui::app::handle_purge_tags(window.as_mut(), service_key, tags, reason);
}

fn app_tag_relation_add(
    mut window: Pin<&mut ffi::HydrusMainWindow>,
    relation: &str,
    service: &str,
    first: &str,
    second: &str,
) {
    crate::gui::app::handle_tag_relation_add(window.as_mut(), relation, service, first, second);
}

fn app_tag_relation_remove(
    mut window: Pin<&mut ffi::HydrusMainWindow>,
    relation: &str,
    service: &str,
    first: &str,
    second: &str,
) {
    crate::gui::app::handle_tag_relation_remove(
        window.as_mut(), relation, service, first, second,
    );
}

fn app_duplicates_batch_size(
    mut window: Pin<&mut ffi::HydrusMainWindow>,
    page_id: &str,
    value: i64,
) {
    crate::gui::app::handle_duplicates_batch_size(window.as_mut(), page_id, value);
}

#[allow(clippy::too_many_arguments)]
fn app_duplicates_search_context(
    mut window: Pin<&mut ffi::HydrusMainWindow>,
    page_id: &str,
    query_1: &str,
    location_1: &str,
    query_2: &str,
    location_2: &str,
    dupe_search_type: &str,
    pixel_dupes: &str,
    max_distance: i64,
    sort_type: &str,
    sort_ascending: bool,
) {
    crate::gui::app::handle_duplicates_search_context(
        window.as_mut(), page_id, query_1, location_1, query_2, location_2,
        dupe_search_type, pixel_dupes, max_distance, sort_type, sort_ascending,
    );
}

fn app_duplicates_group_action(
    mut window: Pin<&mut ffi::HydrusMainWindow>,
    page_id: &str,
    kind: &str,
    target: &str,
) {
    crate::gui::app::handle_duplicates_group_action(window.as_mut(), page_id, kind, target);
}

#[allow(clippy::too_many_arguments)]
fn app_subscription_add(
    mut window: Pin<&mut ffi::HydrusMainWindow>,
    page_id: &str,
    name: &str,
    url: &str,
    query: &str,
    site: &str,
    filter: &str,
    period: i64,
) {
    crate::gui::app::handle_subscription_add(
        window.as_mut(), page_id, name, url, query, site, filter, period,
    );
}

#[allow(clippy::too_many_arguments)]
fn app_subscription_edit_save(
    mut window: Pin<&mut ffi::HydrusMainWindow>,
    page_id: &str,
    original: &str,
    name: &str,
    url: &str,
    query: &str,
    site: &str,
    filter: &str,
    period: i64,
    paused: bool,
) {
    crate::gui::app::handle_subscription_edit_save(
        window.as_mut(), page_id, original, name, url, query, site, filter, period, paused,
    );
}

fn app_subscription_import_options_save(
    mut window: Pin<&mut ffi::HydrusMainWindow>,
    name: &str,
    container_json: &str,
) {
    crate::gui::app::handle_subscription_import_options_save(
        window.as_mut(), name, container_json,
    );
}

fn app_file_seed_action(
    mut window: Pin<&mut ffi::HydrusMainWindow>,
    page: &str,
    action: &str,
    urls: Vec<String>,
) {
    crate::gui::app::handle_file_seed_action(window.as_mut(), page, action, urls);
}

fn app_gallery_seed_action(
    mut window: Pin<&mut ffi::HydrusMainWindow>,
    page: &str,
    action: &str,
    urls: Vec<String>,
) {
    crate::gui::app::handle_gallery_seed_action(window.as_mut(), page, action, urls);
}

#[allow(clippy::too_many_arguments)]
fn app_downloader_start(
    mut window: Pin<&mut ffi::HydrusMainWindow>,
    page_id: &str,
    input: &str,
    url: &str,
    query: &str,
    site: &str,
    formula: &str,
    subject: &str,
    auto_watch: bool,
    period: i64,
    no_limit: bool,
    file_limit: i64,
    present: i64,
) {
    crate::gui::app::handle_downloader_start(
        window.as_mut(), page_id, input, url, query, site, formula, subject, auto_watch,
        period, no_limit, file_limit, present,
    );
}

fn app_downloader_jobs_set(
    mut window: Pin<&mut ffi::HydrusMainWindow>,
    page_id: &str,
    urls: Vec<String>,
    remove_missing: bool,
) {
    crate::gui::app::handle_downloader_jobs_set(window.as_mut(), page_id, urls, remove_missing);
}

#[allow(clippy::too_many_arguments)]
fn app_duplicates_rule_save(
    mut window: Pin<&mut ffi::HydrusMainWindow>,
    name: &str,
    paused: bool,
    operation_mode: &str,
    max_pending_pairs: i64,
    search_query: &str,
    search_mode: &str,
    max_distance: i64,
    selectors: Vec<ffi::RuleComparatorRow>,
    action: &str,
    delete_a: bool,
    delete_b: bool,
    original_name: &str,
) {
    crate::gui::app::handle_duplicates_rule_save(
        window.as_mut(), name, paused, operation_mode, max_pending_pairs, search_query,
        search_mode, max_distance, selectors, action, delete_a, delete_b, original_name,
    );
}

#[allow(clippy::too_many_arguments)]
fn app_duplicates_rule_preview(
    mut window: Pin<&mut ffi::HydrusMainWindow>,
    name: &str,
    paused: bool,
    operation_mode: &str,
    max_pending_pairs: i64,
    search_query: &str,
    search_mode: &str,
    max_distance: i64,
    selectors: Vec<ffi::RuleComparatorRow>,
    action: &str,
    delete_a: bool,
    delete_b: bool,
) {
    crate::gui::app::handle_duplicates_rule_preview(
        window.as_mut(), name, paused, operation_mode, max_pending_pairs, search_query,
        search_mode, max_distance, selectors, action, delete_a, delete_b,
    );
}

/// Returns the Python `VetoException` text for an invalid export filename
/// phrase, or an empty string when the phrase parses.
fn hydrus_validate_export_phrase(phrase: &str) -> String {
    crate::gui::app::export::validate_phrase(phrase)
        .err()
        .unwrap_or_default()
}

fn hydrus_static_dir() -> String {
    crate::gui::app::static_assets::resolve_static_dir()
        .map(|path| path.to_string_lossy().to_string())
        .unwrap_or_default()
}

fn action_from_str(action: &str) -> ffi::UiAction {
    crate::gui::app::action_from_str(action)
}
