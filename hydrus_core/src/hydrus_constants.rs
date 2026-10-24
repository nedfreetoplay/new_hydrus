//! Rust port of `hydrus/core/HydrusConstants.py`.
//!
//! The Python module is mostly a pile of integer enums plus string lookups.
//! Here the categorical values are real enums (`#[repr(u16)]`, numeric values
//! kept identical to Python so on-disk/network data stays compatible), and the
//! Python dict lookups become `as_str()` / `from_str()` methods.
//!
//! Grouped values (`IMAGES`, `AUDIO`, ...) are exposed as static slices, and
//! membership checks get `LazyLock<HashSet<_>>` sets.

use std::collections::HashSet;
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, OnceLock};

// ============================================================================
// 1. FOLDERS
// ============================================================================

/// Python computes these at import time from `__file__`. Rust has no such
/// luxury, so the app calls [`init_dirs`] once during boot. Until then the
/// defaults are derived from the current executable's directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dirs {
    /// The directory that contains `hydrus_core`'s data (the repo root in a
    /// source run, the app bundle in a frozen run).
    pub base_dir: PathBuf,
    /// Directory containing `static`, `bin`, `help`, `LICENSE`.
    pub content_base_dir: PathBuf,
    pub bin_dir: PathBuf,
    pub help_dir: PathBuf,
    pub license_path: PathBuf,
    pub default_db_dir: PathBuf,
    /// `~/Hydrus` (`~/Library/Hydrus` on macOS), if we could resolve `~`.
    pub userpath_db_dir: Option<PathBuf>,
    /// Whether `dirs::home_dir()` actually resolved.
    pub got_a_normal_looking_venv: bool,
}

static DIRS: OnceLock<Dirs> = OnceLock::new();

impl Dirs {
    fn from_base_dir(base_dir: PathBuf) -> Self {
        let content_base_dir = base_dir.clone();
        let bin_dir = content_base_dir.join("bin");
        let help_dir = content_base_dir.join("help");

        let userpath_db_dir = dirs::home_dir().map(|home| {
            if cfg!(target_os = "macos") {
                home.join("Library").join("Hydrus")
            } else {
                home.join("Hydrus")
            }
        });

        Self {
            license_path: content_base_dir.join("LICENSE"),
            default_db_dir: base_dir.join("db"),
            got_a_normal_looking_venv: base_dir.join("venv").exists(),
            bin_dir,
            help_dir,
            userpath_db_dir,
            content_base_dir,
            base_dir,
        }
    }
}

/// Initialise the global folder layout. Safe to call more than once; the first
/// call wins. Returns the directories.
pub fn init_dirs(base_dir: impl Into<PathBuf>) -> &'static Dirs {
    DIRS.get_or_init(|| Dirs::from_base_dir(base_dir.into()))
}

/// The current global folder layout, initialising it from the executable
/// location if the caller never did so explicitly.
pub fn dirs() -> &'static Dirs {
    DIRS.get_or_init(|| {
        let base_dir = std::env::current_exe()
            .ok()
            .and_then(|exe| exe.parent().map(Path::to_path_buf))
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

        Dirs::from_base_dir(base_dir)
    })
}

/// Read-only shorthands, mirroring the Python module globals.
pub fn base_dir() -> &'static Path {
    &dirs().base_dir
}

pub fn content_base_dir() -> &'static Path {
    &dirs().content_base_dir
}

pub fn default_db_dir() -> &'static Path {
    &dirs().default_db_dir
}

pub fn userpath_db_dir() -> Option<&'static Path> {
    dirs().userpath_db_dir.as_deref()
}

/// Python's `RUNNING_FROM_FROZEN_BUILD` (`sys.frozen`). We treat release
/// builds as frozen, but the real answer is set by the boot code.
static RUNNING_FROM_FROZEN_BUILD: OnceLock<bool> = OnceLock::new();

pub fn set_running_from_frozen_build(value: bool) {
    let _ = RUNNING_FROM_FROZEN_BUILD.set(value);
}

pub fn running_from_frozen_build() -> bool {
    *RUNNING_FROM_FROZEN_BUILD.get_or_init(|| !cfg!(debug_assertions))
}

/// Python checks for a `running_from_app` marker file next to the base dir.
pub fn running_from_macos_app() -> bool {
    cfg!(target_os = "macos") && dirs().base_dir.join("running_from_app").exists()
}

pub fn running_from_source() -> bool {
    !(running_from_frozen_build() || running_from_macos_app())
}

pub fn nice_running_as_string() -> &'static str {
    if running_from_source() {
        "from source"
    } else if running_from_frozen_build() {
        "from frozen build"
    } else if running_from_macos_app() {
        "from App"
    } else {
        "from unknown?!"
    }
}

// ============================================================================
// 2. PLATFORM
// ============================================================================

pub const PLATFORM_WINDOWS: bool = cfg!(target_os = "windows");
pub const PLATFORM_MACOS: bool = cfg!(target_os = "macos");
pub const PLATFORM_LINUX: bool = cfg!(target_os = "linux");
pub const PLATFORM_HAIKU: bool = cfg!(target_os = "haiku");

pub const NICE_PLATFORM_STRING: &str = if PLATFORM_WINDOWS {
    "Windows"
} else if PLATFORM_MACOS {
    "macOS"
} else if PLATFORM_LINUX {
    "Linux"
} else if PLATFORM_HAIKU {
    "Haiku"
} else {
    "Unknown?!"
};

pub const ARCHITECTURE: &str = std::env::consts::ARCH;

/// Python's `platform.machine()` normalised for display.
pub const NICE_ARCHITECTURE_STRING: &str = match ARCHITECTURE.as_bytes() {
    b"x86_64" => "x86_64",
    b"aarch64" => "aarch64",
    b"x86" => "x86",
    b"arm" => "arm",
    _ => ARCHITECTURE,
};

// ============================================================================
// 3. CORE CONSTANTS
// ============================================================================

pub const NETWORK_VERSION: u32 = 20;
pub const SOFTWARE_VERSION: u32 = 689;
pub const CLIENT_API_VERSION: u32 = 95;
pub const SERVER_THUMBNAIL_DIMENSIONS: (u32, u32) = (200, 200);
pub const HYDRUS_KEY_LENGTH: usize = 32;
pub const READ_BLOCK_SIZE: usize = 256 * 1024;

pub const LIFETIMES: &[(&str, Option<u64>)] = &[
    ("one month", Some(30 * 86400)),
    ("three months", Some(3 * 30 * 86400)),
    ("six months", Some(6 * 30 * 86400)),
    ("one year", Some(365 * 86400)),
    ("two years", Some(2 * 365 * 86400)),
    ("five years", Some(5 * 365 * 86400)),
    ("does not expire", None),
];

pub const USERPATH_SVG_ICON: &str = "star_shapes";

pub const REMOTE_HELP: &str = "https://hydrusnetwork.github.io/hydrus";

pub const DOCUMENTATION_INDEX: &str = "index.html";
pub const DOCUMENTATION_CHANGELOG: LazyLock<String> =
    LazyLock::new(|| format!("changelog.html#version_{}", SOFTWARE_VERSION));
pub const DOCUMENTATION_DOWNLOADER_GUGS: &str = "downloader_gugs.html";
pub const DOCUMENTATION_DOWNLOADER_LOGIN: &str = "downloader_login.html";
pub const DOCUMENTATION_ADDING_NEW_DOWNLOADERS: &str = "adding_new_downloaders.html";
pub const DOCUMENTATION_DOWNLOADER_URL_CLASSES: &str = "downloader_url_classes.html";
pub const DOCUMENTATION_GETTING_STARTED_SUBSCRIPTIONS: &str = "getting_started_subscriptions.html";
pub const DOCUMENTATION_DATABASE_MIGRATION: &str = "database_migration.html";
pub const DOCUMENTATION_DATABASE_MIGRATION_GRANULARITY: &str =
    "database_migration.html#granularity";
pub const DOCUMENTATION_DUPLICATES: &str = "duplicates.html";
pub const DOCUMENTATION_DUPLICATES_AUTO_RESOLUTION: &str =
    "advanced_duplicates_auto_resolution.html";
pub const DOCUMENTATION_DOWNLOADER_SHARING: &str = "downloader_sharing.html";
pub const DOCUMENTATION_DOWNLOADER_PARSERS_PAGE_PARSERS_PAGE_PARSERS: &str =
    "downloader_parsers_page_parsers.html#page_parsers";
pub const DOCUMENTATION_DOWNLOADER_PARSERS_CONTENT_PARSERS_CONTENT_PARSERS: &str =
    "downloader_parsers_content_parsers.html#content_parsers";
pub const DOCUMENTATION_DOWNLOADER_PARSERS_FORMULAE_ZIPPER_FORMULA: &str =
    "downloader_parsers_formulae.html#zipper_formula";
pub const DOCUMENTATION_DOWNLOADER_PARSERS_FORMULAE_CONTEXT_VARIABLE_FORMULA: &str =
    "downloader_parsers_formulae.html#context_variable_formula";
pub const DOCUMENTATION_DOWNLOADER_PARSERS_FORMULAE_HTML_FORMULA: &str =
    "downloader_parsers_formulae.html#html_formula";
pub const DOCUMENTATION_DOWNLOADER_PARSERS_FORMULAE_JSON_FORMULA: &str =
    "downloader_parsers_formulae.html#json_formula";
pub const DOCUMENTATION_DOWNLOADER_PARSERS_FORMULAE_NESTED_FORMULA: &str =
    "downloader_parsers_formulae.html#nested_formula";
pub const DOCUMENTATION_DOWNLOADER_PARSERS_FORMULAE_STATIC: &str =
    "downloader_parsers_formulae.html#static_formula";
pub const DOCUMENTATION_RATINGS: &str = "getting_started_ratings.html";
pub const DOCUMENTATION_SIDECARS: &str = "advanced_sidecars.html";
pub const DOCUMENTATION_ABOUT_DOCS: &str = "about_docs.html";
pub const DOCUMENTATION_IMPORT_OPTIONS: &str = "getting_started_import_options.html";
pub const DOCUMENTATION_EXTERNAL_PROGRAMS: &str = "external_programs.html";

pub const DEFAULT_SERVER_ADMIN_PORT: u16 = 45870;
pub const DEFAULT_SERVICE_PORT: u16 = 45871;
pub const SERVER_ADMIN_KEY: &[u8] = b"server admin";

pub const TIMEZONE_UTC: u8 = 0;
pub const TIMEZONE_LOCAL: u8 = 1;
pub const TIMEZONE_OFFSET: u8 = 2;

// ============================================================================
// 4. ENUM HELPERS
// ============================================================================

/// Generate `as_u16`/`try_from_u16`/`as_str`/`from_str` boilerplate for the
/// simple integer enums below.
macro_rules! int_enum {
    (
        $(#[$meta:meta])*
        $name:ident : $repr:ty {
            $($variant:ident = $value:expr => $text:literal),+ $(,)?
        }
    ) => {
        $(#[$meta])*
        #[repr($repr)]
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub enum $name {
            $($variant = $value),+
        }

        impl $name {
            pub const ALL: &'static [$name] = &[$($name::$variant),+];

            pub const fn as_int(self) -> $repr {
                self as $repr
            }

            pub fn try_from_int(value: $repr) -> Option<Self> {
                match value {
                    $($value => Some($name::$variant),)+
                    _ => None,
                }
            }

            /// The Python `*_string_lookup` entry.
            pub const fn as_str(self) -> &'static str {
                match self {
                    $($name::$variant => $text),+
                }
            }

            pub fn from_str(text: &str) -> Option<Self> {
                match text {
                    $($text => Some($name::$variant),)+
                    _ => None,
                }
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(self.as_str())
            }
        }
    };
}

/// Generate just the numeric conversion for enums whose strings need custom
/// logic (or whose Python lookup omits variants).
macro_rules! int_enum_numbers {
    (
        $(#[$meta:meta])*
        $name:ident : $repr:ty {
            $($variant:ident = $value:expr),+ $(,)?
        }
    ) => {
        $(#[$meta])*
        #[repr($repr)]
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub enum $name {
            $($variant = $value),+
        }

        impl $name {
            pub const ALL: &'static [$name] = &[$($name::$variant),+];

            pub const fn as_int(self) -> $repr {
                self as $repr
            }

            pub fn try_from_int(value: $repr) -> Option<Self> {
                match value {
                    $($value => Some($name::$variant),)+
                    _ => None,
                }
            }
        }
    };
}

// ============================================================================
// 5. ENUMS
// ============================================================================

int_enum!(BandwidthType : u8 {
    Data = 0 => "data",
    Requests = 1 => "requests",
});

int_enum!(ContentMergeAction : u8 {
    Copy = 0 => "copy from worse to better",
    Move = 1 => "move from worse to better",
    TwoWayMerge = 2 => "copy in both directions",
    None = 3 => "make no change",
    AddMove = 4 => "add move",
});

impl ContentMergeAction {
    /// `content_number_merge_string_lookup`
    pub const fn number_merge_str(self) -> &'static str {
        match self {
            ContentMergeAction::Copy => "add worse to better",
            ContentMergeAction::Move => "take from worse and add to better",
            ContentMergeAction::TwoWayMerge => "add in both directions",
            ContentMergeAction::None => "make no change",
            ContentMergeAction::AddMove => "add move",
        }
    }

    /// `content_modified_date_merge_string_lookup`. The Python lookup has no
    /// `MOVE`/`ADD_MOVE` entries.
    pub const fn modified_date_merge_str(self) -> Option<&'static str> {
        match self {
            ContentMergeAction::Copy => Some("earlier worse overwrites later better"),
            ContentMergeAction::TwoWayMerge => Some("both get earliest"),
            ContentMergeAction::None => Some("make no change"),
            ContentMergeAction::Move | ContentMergeAction::AddMove => None,
        }
    }
}

int_enum!(ContentStatus : u8 {
    Current = 0 => "current",
    Pending = 1 => "pending",
    Deleted = 2 => "deleted",
    Petitioned = 3 => "petitioned",
});

/// `content_status_to_prefix`
pub const fn content_status_prefix(status: ContentStatus) -> &'static str {
    match status {
        ContentStatus::Current => "",
        ContentStatus::Pending => "(+) ",
        ContentStatus::Petitioned => "(-) ",
        ContentStatus::Deleted => "(X) ",
    }
}

int_enum!(ContentType : u16 {
    Mappings = 0 => "tag mappings",
    TagSiblings = 1 => "tag siblings",
    TagParents = 2 => "tag parents",
    Files = 3 => "files",
    Ratings = 4 => "ratings",
    Mapping = 5 => "tag mapping",
    Directories = 6 => "directories",
    Urls = 7 => "urls",
    Veto = 8 => "veto",
    Accounts = 9 => "accounts",
    Options = 10 => "options",
    Services = 11 => "services",
    Unknown = 12 => "unknown",
    AccountTypes = 13 => "account types",
    Variable = 14 => "variable",
    Hash = 15 => "hash",
    Timestamp = 16 => "timestamp",
    Title = 17 => "title",
    Notes = 18 => "notes",
    FileViewingStats = 19 => "file viewing stats",
    Tag = 20 => "tag",
    Definitions = 21 => "definitions",
    HttpHeaders = 22 => "http headers",
});

int_enum_numbers!(ContentUpdate : u16 {
    Add = 0,
    Delete = 1,
    Pend = 2,
    RescindPend = 3,
    Petition = 4,
    RescindPetition = 5,
    EditLog = 6,
    Archive = 7,
    Inbox = 8,
    Rating = 9,
    // 10 is unused in Python
    DenyPend = 11,
    DenyPetition = 12,
    Advanced = 13,
    Undelete = 14,
    Set = 15,
    Flip = 16,
    ClearDeleteRecord = 17,
    Increment = 18,
    Decrement = 19,
    Move = 20,
    DeleteFromSourceAfterMigrate = 21,
    MoveMerge = 22,
});

impl ContentUpdate {
    /// `content_update_string_lookup`. `EditLog` has no display string in
    /// Python, so it gets its variant name here.
    pub const fn as_str(self) -> &'static str {
        match self {
            ContentUpdate::Add => "add",
            ContentUpdate::Delete => "delete",
            ContentUpdate::Pend => "pending",
            ContentUpdate::RescindPend => "rescind pending",
            ContentUpdate::Petition => "petition",
            ContentUpdate::RescindPetition => "rescind petition",
            ContentUpdate::EditLog => "edit log",
            ContentUpdate::Archive => "archive",
            ContentUpdate::Inbox => "inbox",
            ContentUpdate::Rating => "rating",
            ContentUpdate::DenyPend => "deny pend",
            ContentUpdate::DenyPetition => "deny petition",
            ContentUpdate::Advanced => "advanced",
            ContentUpdate::Undelete => "undelete",
            ContentUpdate::Set => "set",
            ContentUpdate::Flip => "flip on/off",
            ContentUpdate::ClearDeleteRecord => "clear deletion record",
            ContentUpdate::Increment => "increment",
            ContentUpdate::Decrement => "decrement",
            ContentUpdate::Move => "move (if not already in destination)",
            ContentUpdate::MoveMerge => "move (even if already in destination)",
            ContentUpdate::DeleteFromSourceAfterMigrate => "delete from source after migrate",
        }
    }
}

impl fmt::Display for ContentUpdate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

int_enum!(DefinitionsType : u8 {
    Hashes = 0 => "hashes",
    Tags = 1 => "tags",
});

int_enum!(DuplicateType : u8 {
    Potential = 0 => "potential duplicates",
    FalsePositive = 1 => "not related/false positive",
    SameQuality = 2 => "same quality",
    Alternate = 3 => "alternates",
    Better = 4 => "this is a better duplicate",
    SmallerBetter = 5 => "smaller hash_id is a better duplicate",
    LargerBetter = 6 => "larger hash_id is a better duplicate",
    Worse = 7 => "this is a worse duplicate",
    Member = 8 => "duplicates",
    King = 9 => "the best quality duplicate",
    ConfirmedAlternate = 10 => "confirmed alternates",
});

impl DuplicateType {
    /// `duplicate_type_auto_resolution_action_description_lookup`
    pub const fn auto_resolution_action_description(self) -> Option<&'static str> {
        match self {
            DuplicateType::FalsePositive => Some("set as not related/false positive"),
            DuplicateType::SameQuality => Some("set as same quality"),
            DuplicateType::Alternate => Some("set as alternates"),
            DuplicateType::Better => Some("set as duplicates--A better"),
            DuplicateType::Worse => Some("set as duplicates--B better"),
            _ => None,
        }
    }
}

int_enum!(Encoding : u8 {
    Raw = 0 => "raw bytes",
    Hex = 1 => "hexadecimal",
    Base64 = 2 => "base64",
});

int_enum!(ImportFolderType : u8 {
    Delete = 0 => "delete",
    Synchronise = 1 => "synchronise",
});

int_enum!(ExportFolderType : u8 {
    Regular = 0 => "regular",
    Synchronise = 1 => "synchronise",
});

int_enum!(Filter : u8 {
    Whitelist = 0 => "whitelist",
    Blacklist = 1 => "blacklist",
});

// Python's `HYDRUS_CLIENT`/`HYDRUS_SERVER`/`HYDRUS_TEST`.
int_enum!(HydrusMode : u8 {
    Client = 0 => "client",
    Server = 1 => "server",
    Test = 2 => "test",
});

int_enum!(LogicalOperator : u8 {
    All = 0 => "all",
    Any = 1 => "any",
    Only = 2 => "only",
});

int_enum!(MaintenanceMode : u8 {
    Idle = 0 => "idle",
    Shutdown = 1 => "shutdown",
    Forced = 2 => "forced",
    Active = 3 => "active",
});

int_enum!(Permission : u8 {
    GetData = 0 => "get data",
    PostData = 1 => "post data",
    PostPetitions = 2 => "post petitions",
    ResolvePetitions = 3 => "resolve petitions",
    ManageUsers = 4 => "manage users",
    GeneralAdmin = 5 => "general administration",
    EditServices = 6 => "edit services",
    Unknown = 7 => "unknown",
});

pub const CREATABLE_PERMISSIONS: &[Permission] = &[
    Permission::GetData,
    Permission::PostData,
    Permission::PostPetitions,
    Permission::ResolvePetitions,
    Permission::ManageUsers,
    Permission::GeneralAdmin,
];

pub const ADMIN_PERMISSIONS: &[Permission] = &[
    Permission::ResolvePetitions,
    Permission::ManageUsers,
    Permission::GeneralAdmin,
    Permission::EditServices,
];

int_enum!(PermissionAction : u8 {
    Petition = 0 => "petition",
    Create = 1 => "create",
    Moderate = 2 => "moderate",
});

/// `( content type, optional action ) -> description`
pub const PERMISSION_PAIR_LOOKUP: &[(ContentType, Option<PermissionAction>, &str)] = &[
    (ContentType::Accounts, None, "cannot change accounts"),
    (
        ContentType::Accounts,
        Some(PermissionAction::Create),
        "can create accounts",
    ),
    (
        ContentType::Accounts,
        Some(PermissionAction::Moderate),
        "can manage accounts completely",
    ),
    (
        ContentType::AccountTypes,
        None,
        "cannot change account types",
    ),
    (
        ContentType::AccountTypes,
        Some(PermissionAction::Moderate),
        "can manage account types completely",
    ),
    (ContentType::Options, None, "cannot change service options"),
    (
        ContentType::Options,
        Some(PermissionAction::Moderate),
        "can manage service options completely",
    ),
    (ContentType::Services, None, "cannot change services"),
    (
        ContentType::Services,
        Some(PermissionAction::Moderate),
        "can manage services completely",
    ),
    (ContentType::Files, None, "can only download files"),
    (
        ContentType::Files,
        Some(PermissionAction::Petition),
        "can petition to remove existing files",
    ),
    (
        ContentType::Files,
        Some(PermissionAction::Create),
        "can upload new files and petition existing ones",
    ),
    (
        ContentType::Files,
        Some(PermissionAction::Moderate),
        "can upload and delete files and process petitions",
    ),
    (ContentType::Mappings, None, "can only download mappings"),
    (
        ContentType::Mappings,
        Some(PermissionAction::Petition),
        "can petition to remove existing mappings",
    ),
    (
        ContentType::Mappings,
        Some(PermissionAction::Create),
        "can upload new mappings and petition existing ones",
    ),
    (
        ContentType::Mappings,
        Some(PermissionAction::Moderate),
        "can upload and delete mappings and process petitions",
    ),
    (
        ContentType::TagParents,
        None,
        "can only download tag parents",
    ),
    (
        ContentType::TagParents,
        Some(PermissionAction::Petition),
        "can petition to add or remove tag parents",
    ),
    (
        ContentType::TagParents,
        Some(PermissionAction::Moderate),
        "can upload and delete tag parents and process petitions",
    ),
    (
        ContentType::TagSiblings,
        None,
        "can only download tag siblings",
    ),
    (
        ContentType::TagSiblings,
        Some(PermissionAction::Petition),
        "can petition to add or remove tag siblings",
    ),
    (
        ContentType::TagSiblings,
        Some(PermissionAction::Moderate),
        "can upload and delete tag siblings and process petitions",
    ),
];

pub fn permission_pair_description(
    content_type: ContentType,
    action: Option<PermissionAction>,
) -> Option<&'static str> {
    PERMISSION_PAIR_LOOKUP
        .iter()
        .find(|(ct, act, _)| *ct == content_type && *act == action)
        .map(|(_, _, text)| *text)
}

int_enum!(ServiceType : u16 {
    TagRepository = 0 => "hydrus tag repository",
    FileRepository = 1 => "hydrus file repository",
    LocalFileDomain = 2 => "local file domain",
    MessageDepot = 3 => "hydrus message depot",
    LocalTag = 5 => "local tag domain",
    LocalRatingNumerical = 6 => "local numerical rating service",
    LocalRatingLike = 7 => "local like/dislike rating service",
    RatingNumericalRepository = 8 => "hydrus numerical rating repository",
    RatingLikeRepository = 9 => "hydrus like/dislike rating repository",
    CombinedTag = 10 => "virtual combined tag domain",
    CombinedFile = 11 => "virtual combined file domain",
    LocalBooru = 12 => "client local booru",
    Ipfs = 13 => "ipfs daemon",
    LocalFileTrashDomain = 14 => "local trash file domain",
    HydrusLocalFileStorage = 15 => "virtual combined local file domain",
    TestService = 16 => "test service",
    LocalNotes = 17 => "local file notes service",
    ClientApiService = 18 => "client api",
    CombinedDeletedFile = 19 => "virtual deleted file service",
    LocalFileUpdateDomain = 20 => "local update file domain",
    CombinedLocalFileDomains = 21 => "virtual combined local media domain",
    LocalRatingIncDec = 22 => "local inc/dec rating service",
    ServerAdmin = 99 => "hydrus server administration service",
    NullService = 100 => "null service",
});

impl ServiceType {
    /// `service_string_lookup_short`
    pub const fn short_str(self) -> &'static str {
        match self {
            ServiceType::TagRepository => "tag repository",
            ServiceType::FileRepository => "file repository",
            ServiceType::LocalFileDomain => "local file domain",
            ServiceType::LocalFileTrashDomain => "trash",
            ServiceType::LocalFileUpdateDomain => "local update file domain",
            ServiceType::HydrusLocalFileStorage => "hydrus local file storage",
            ServiceType::CombinedLocalFileDomains => "combined local file domains",
            ServiceType::MessageDepot => "hydrus message depot",
            ServiceType::LocalTag => "local tag domain",
            ServiceType::LocalRatingIncDec => "inc/dec ratings",
            ServiceType::LocalRatingNumerical => "numerical ratings",
            ServiceType::LocalRatingLike => "like/dislike ratings",
            ServiceType::RatingNumericalRepository => "hydrus numerical rating repository",
            ServiceType::RatingLikeRepository => "hydrus like/dislike rating repository",
            ServiceType::CombinedTag => "all known tags",
            ServiceType::CombinedFile => "all known files",
            ServiceType::LocalBooru => "local booru",
            ServiceType::ClientApiService => "client api",
            ServiceType::Ipfs => "ipfs",
            ServiceType::TestService => "test service",
            ServiceType::LocalNotes => "notes",
            ServiceType::ServerAdmin => "hydrus server administration",
            ServiceType::CombinedDeletedFile => "virtual deleted file service",
            ServiceType::NullService => "null service",
        }
    }

    /// `service_string_lookup`
    pub const fn long_str(self) -> &'static str {
        match self {
            ServiceType::TagRepository => "hydrus tag repository",
            ServiceType::FileRepository => "hydrus file repository",
            ServiceType::LocalFileDomain => "local file domain",
            ServiceType::LocalFileTrashDomain => "local trash file domain",
            ServiceType::LocalFileUpdateDomain => "local update file domain",
            ServiceType::HydrusLocalFileStorage => "virtual combined local file domain",
            ServiceType::CombinedLocalFileDomains => "virtual combined local media domain",
            ServiceType::MessageDepot => "hydrus message depot",
            ServiceType::LocalTag => "local tag domain",
            ServiceType::LocalRatingIncDec => "local inc/dec rating service",
            ServiceType::LocalRatingNumerical => "local numerical rating service",
            ServiceType::LocalRatingLike => "local like/dislike rating service",
            ServiceType::RatingNumericalRepository => "hydrus numerical rating repository",
            ServiceType::RatingLikeRepository => "hydrus like/dislike rating repository",
            ServiceType::CombinedTag => "virtual combined tag domain",
            ServiceType::CombinedFile => "virtual combined file domain",
            ServiceType::LocalBooru => "client local booru",
            ServiceType::ClientApiService => "client api",
            ServiceType::Ipfs => "ipfs daemon",
            ServiceType::TestService => "test service",
            ServiceType::LocalNotes => "local file notes service",
            ServiceType::ServerAdmin => "hydrus server administration service",
            ServiceType::CombinedDeletedFile => "virtual deleted file service",
            ServiceType::NullService => "null service",
        }
    }

    /// `service_description_lookup`
    pub const fn description(self) -> &'static str {
        match self {
            ServiceType::TagRepository => {
                "A repository of tag mapping data (tag-file pairs) stored on a remote server. Clients can sync with it, which means to download all the file hashes, tags, and mappings it knows of and synchronise that to local metadata storage. Clients can also upload new mapping data, which will in turn be shared to all users.\n\nThe \"all known tags\" service is a union of all local tag domains and remote tag repositories."
            }
            ServiceType::FileRepository => {
                "A repository of files stored on a remote server. Clients can sync with it, which means to regularly download all the file hashes and thumbnails. You can search it like any other file domain, viewing the thumbnails, and download files. Clients can also upload new files, which will in turn be shared to all users."
            }
            ServiceType::LocalFileDomain => {
                "This stores media files you have imported. You can have multiple local file domains, and files may be in multiple local file domains at once. Each domain is completely isolated from one another when you perform a file or tag search (if your search page is pointed at A, you will not get any file search results or tag autocomplete suggestions from B)."
            }
            ServiceType::LocalFileTrashDomain => {
                "This is where your files go when they have been removed from all your local file domains. It is a waiting area to allow for undeletes of mistakes. When files are deleted from this place, they are physically deleted."
            }
            ServiceType::LocalFileUpdateDomain => {
                "This holds repository update files if you decide to sync with a file or tag repository. It is full of zipped up JSON and is not covered by \"combined local file domains\" like the normal local file domains. You can generally ignore it."
            }
            ServiceType::HydrusLocalFileStorage => {
                "This represents all files currently tracked and stored in your database. If a file is in here, it is either in \"repository updates\", \"combined local file domains\", or \"trash\", and it should be loadable from disk.\n\nWhen a file is deleted from the trash, it also leaves this domain and is scheduled to be physically deleted--which usually happens within a few seconds."
            }
            ServiceType::CombinedLocalFileDomains => {
                "This represents all files currently in any of your local file domains. You start with only \"my files\", but if you add more, this is a convenient umbrella that unions all of them with efficient search tech. When a file is deleted from all of its local file domains, it leaves this service and enters the trash."
            }
            ServiceType::MessageDepot => "You should not see this text",
            ServiceType::LocalTag => {
                "This stores tag mappings (tag-file pairs) that you have added/imported. You can have multiple local tag domains, and the exact same mappings map appear in multiple domains.\n\nTags are not removed when files are deleted, and if you later re-import the file, you will see it still has its tags.\n\nThe \"all known tags\" service is a union of all local tag services and tag repositories."
            }
            ServiceType::LocalRatingIncDec => {
                "This stores inc/dec ratings.\n\nRatings are not removed when files are deleted, and if you later re-import the file, you will see it still has its ratings."
            }
            ServiceType::LocalRatingNumerical => {
                "This stores numerical ratings.\n\nRatings are not removed when files are deleted, and if you later re-import the file, you will see it still has its ratings."
            }
            ServiceType::LocalRatingLike => {
                "This stores like/dislike ratings.\n\nRatings are not removed when files are deleted, and if you later re-import the file, you will see it still has its ratings."
            }
            ServiceType::RatingNumericalRepository => "You should not see this text",
            ServiceType::RatingLikeRepository => "You should not see this text",
            ServiceType::CombinedTag => {
                "This is a union of all your local tag domains and remote tag repositories."
            }
            ServiceType::CombinedFile => {
                "This is a special view on a tag service. It will deliver results without any cross-reference with a known file service, and thus if the file is tagged on a service, you will see it whether you have it, had it, or have never had it."
            }
            ServiceType::LocalBooru => "You should not see this text",
            ServiceType::ClientApiService => "The Client API.",
            ServiceType::Ipfs => {
                "A remote listing of files stored/pinned on an IPFS daemon. You can search it like any other file domain."
            }
            ServiceType::TestService => "You should not see this text",
            ServiceType::LocalNotes => {
                "This stores your file notes.\n\nNotes are not removed when files are deleted, and if you later re-import the file, you will see it still has its notes."
            }
            ServiceType::ServerAdmin => {
                "This is an interface to a server that allows editing of admin accounts and which remote services are running."
            }
            ServiceType::CombinedDeletedFile => {
                "This represents all the files that have ever been deleted from any location. It is for advanced technical jobs, and you can generally ignore it."
            }
            ServiceType::NullService => "You should not see this text",
        }
    }
}

int_enum!(ServiceUpdate : u8 {
    DeletePending = 0 => "delete pending",
    Reset = 1 => "reset",
});

int_enum!(TimestampType : u8 {
    ModifiedDomain = 0 => "domain modified time",
    ModifiedFile = 1 => "file modified time",
    ModifiedAggregate = 2 => "aggregate modified time",
    Imported = 3 => "imported time",
    Deleted = 4 => "deleted time",
    Archived = 5 => "archived time",
    LastViewed = 6 => "last viewed time",
    PreviouslyImported = 7 => "previous imported time (for undelete)",
});

int_enum!(HttpMethod : u8 {
    Get = 0 => "GET",
    Post = 1 => "POST",
    Options = 2 => "OPTIONS",
});

int_enum!(Approval : u8 {
    Approve = 0 => "approve",
    Deny = 1 => "deny",
});

int_enum!(UrlType : u8 {
    Post = 0 => "post url",
    Api = 1 => "api/redirect url",
    File = 2 => "file url",
    Gallery = 3 => "gallery url",
    Watchable = 4 => "watchable url",
    Unknown = 5 => "unknown url",
    Next = 6 => "next page url",
    Desired = 7 => "downloadable/pursuable url",
    Source = 8 => "source url",
    SubGallery = 9 => "sub-gallery url (is queued even if creator found no post/file urls)",
});

// `DELETE_FILES_PETITION` / `DELETE_TAG_PETITION`.
int_enum_numbers!(PetitionType : u8 {
    DeleteFiles = 0,
    DeleteTags = 1,
});

int_enum_numbers!(BanType : u8 {
    Ban = 0,
    Superban = 1,
});

pub const SCORE_PETITION: u8 = 0;

// ============================================================================
// 6. RESOLUTIONS & RATIOS
// ============================================================================

pub const NICE_RESOLUTIONS: &[((u32, u32), &str)] = &[
    ((640, 480), "480p"),
    ((1280, 720), "720p"),
    ((1920, 1080), "1080p"),
    ((3840, 2160), "2160p"),
    ((720, 1280), "vertical 720p"),
    ((1080, 1920), "vertical 1080p"),
    ((2160, 3840), "vertical 2160p"),
];

pub const NICE_RATIOS: &[(f64, &str)] = &[
    (1.0, "1:1"),
    (4.0 / 3.0, "4:3"),
    (5.0 / 4.0, "5:4"),
    (16.0 / 9.0, "16:9"),
    (21.0 / 9.0, "21:9"),
    (47.0 / 20.0, "2.35:1"),
    (9.0 / 16.0, "9:16"),
    (2.0 / 3.0, "2:3"),
    (4.0 / 5.0, "4:5"),
];

pub fn nice_resolution(resolution: (u32, u32)) -> Option<&'static str> {
    NICE_RESOLUTIONS
        .iter()
        .find(|(res, _)| *res == resolution)
        .map(|(_, text)| *text)
}

// ============================================================================
// 7. MIME TYPES
// ============================================================================

int_enum!(Mime : u16 {
    Collection = 0 => "collection",
    Jpeg = 1 => "jpeg",
    Png = 2 => "png",
    AnimationGif = 3 => "animated gif",
    Bmp = 4 => "bitmap",
    Flash = 5 => "flash",
    Yaml = 6 => "yaml",
    Icon = 7 => "icon",
    Html = 8 => "html",
    Flv = 9 => "flv",
    Pdf = 10 => "pdf",
    Zip = 11 => "zip",
    EncryptedZip = 12 => "application/hydrus-encrypted-zip",
    Mp3 = 13 => "mp3",
    Mp4Video = 14 => "mp4",
    Ogg = 15 => "ogg",
    Flac = 16 => "flac",
    Wma = 17 => "wma",
    Wmv = 18 => "wmv",
    UndeterminedWm = 19 => "wma or wmv",
    Mkv = 20 => "matroska",
    Webm = 21 => "webm",
    Json = 22 => "json",
    Apng = 23 => "apng",
    UndeterminedPng = 24 => "png or apng",
    Mpeg = 25 => "mpeg",
    Mov = 26 => "quicktime",
    Avi = 27 => "avi",
    UpdateDefinitions = 28 => "application/hydrus-update-definitions",
    UpdateContent = 29 => "application/hydrus-update-content",
    PlainText = 30 => "plaintext",
    Rar = 31 => "rar",
    SevenZ = 32 => "7z",
    Webp = 33 => "webp",
    Tiff = 34 => "tiff",
    Psd = 35 => "psd",
    M4a = 36 => "m4a",
    RealVideo = 37 => "realvideo",
    RealAudio = 38 => "realaudio",
    TrueAudio = 39 => "tta",
    GeneralAudio = 40 => "audio",
    GeneralImage = 41 => "image",
    GeneralVideo = 42 => "video",
    GeneralApplication = 43 => "application",
    GeneralAnimation = 44 => "animation",
    Clip = 45 => "clip",
    Wave = 46 => "wave",
    Ogv = 47 => "ogv",
    AudioMkv = 48 => "matroska audio",
    AudioMp4 = 49 => "mp4 audio",
    UndeterminedMp4 = 50 => "mp4 with or without audio",
    Cbor = 51 => "cbor",
    WindowsExe = 52 => "windows exe",
    WavPack = 53 => "wavpack",
    Sai2 = 54 => "sai2",
    Krita = 55 => "krita",
    Svg = 56 => "svg",
    Xcf = 57 => "xcf",
    Gzip = 58 => "gzip",
    GeneralAppArchive = 59 => "archive",
    GeneralImageProject = 60 => "image project file",
    Heif = 61 => "heif",
    HeifSequence = 62 => "heif sequence",
    Heic = 63 => "heic",
    HeicSequence = 64 => "heic sequence",
    Avif = 65 => "avif",
    AvifSequence = 66 => "avif sequence",
    UndeterminedGif = 67 => "undetermined gif",
    StaticGif = 68 => "static gif",
    Procreate = 69 => "procreate",
    Qoi = 70 => "qoi",
    Epub = 71 => "epub",
    Djvu = 72 => "djvu",
    Cbz = 73 => "cbz",
    Ugoira = 74 => "ugoira",
    Rtf = 75 => "rtf",
    Docx = 76 => "docx",
    Xlsx = 77 => "xlsx",
    Pptx = 78 => "pptx",
    UndeterminedOle = 79 => "ole file",
    Doc = 80 => "doc",
    Xls = 81 => "xls",
    Ppt = 82 => "ppt",
    AnimationWebp = 83 => "animated webp",
    UndeterminedWebp = 84 => "webp with or without animation",
    Jxl = 85 => "jxl",
    PaintDotNet = 86 => "paint.net",
    UndeterminedJxl = 87 => "jxl with or without animation",
    AnimationJxl = 88 => "animated jxl",
    OpenRaster = 89 => "ora",
    GeneralFile = 90 => "all files",
    // 91..=99 unused
    OctetStream = 100 => "application/octet-stream",
    Unknown = 101 => "unknown filetype",
});

impl Mime {
    /// `mime_mimetype_string_lookup`
    pub const fn mimetype_str(self) -> &'static str {
        match self {
            Mime::Collection => "collection",
            Mime::Jpeg => "image/jpeg",
            Mime::Png => "image/png",
            Mime::Apng => "image/apng",
            Mime::StaticGif | Mime::AnimationGif => "image/gif",
            Mime::Bmp => "image/bmp",
            Mime::Webp | Mime::AnimationWebp => "image/webp",
            Mime::Tiff => "image/tiff",
            Mime::Qoi => "image/qoi",
            Mime::Icon => "image/x-icon",
            Mime::Svg => "image/svg+xml",
            Mime::Heif => "image/heif",
            Mime::HeifSequence => "image/heif-sequence",
            Mime::Heic => "image/heic",
            Mime::HeicSequence => "image/heic-sequence",
            Mime::Avif => "image/avif",
            Mime::AvifSequence => "image/avif-sequence",
            Mime::Jxl | Mime::AnimationJxl => "image/jxl",
            Mime::Ugoira => "application/zip",
            Mime::Flash => "application/x-shockwave-flash",
            Mime::OctetStream => "application/octet-stream",
            Mime::Cbz => "application/vnd.comicbook+zip",
            Mime::Yaml => "application/x-yaml",
            Mime::Json => "application/json",
            Mime::Cbor => "application/cbor",
            Mime::Pdf => "application/pdf",
            Mime::Docx => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
            Mime::Xlsx => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
            Mime::Pptx => {
                "application/vnd.openxmlformats-officedocument.presentationml.presentation"
            }
            Mime::Doc => "application/msword",
            Mime::Xls => "application/vnd.ms-excel",
            Mime::Ppt => "application/vnd.ms-powerpoint",
            Mime::Epub => "application/epub+zip",
            Mime::Djvu => "image/vnd.djvu",
            Mime::Rtf => "application/rtf",
            Mime::Psd => "image/vnd.adobe.photoshop",
            Mime::Clip => "application/clip",
            Mime::Sai2 => "application/sai2",
            Mime::Krita => "application/x-krita",
            Mime::OpenRaster => "image/openraster",
            Mime::PaintDotNet => "application/x-paintnet",
            Mime::Xcf => "image/x-xcf",
            Mime::Procreate => "application/x-procreate",
            Mime::Zip => "application/zip",
            Mime::Rar => "application/vnd.rar",
            Mime::SevenZ => "application/x-7z-compressed",
            Mime::Gzip => "application/gzip",
            Mime::WindowsExe => "application/octet-stream",
            Mime::EncryptedZip => "application/hydrus-encrypted-zip",
            Mime::UpdateContent => "application/hydrus-update-content",
            Mime::UpdateDefinitions => "application/hydrus-update-definitions",
            Mime::M4a => "audio/mp4",
            Mime::Mp3 => "audio/mp3",
            Mime::Ogg => "audio/ogg",
            Mime::Flac => "audio/flac",
            Mime::AudioMkv => "audio/x-matroska",
            Mime::AudioMp4 => "audio/mp4",
            Mime::Wave => "audio/x-wav",
            Mime::RealAudio => "audio/vnd.rn-realaudio",
            Mime::TrueAudio => "audio/x-tta",
            Mime::Wma => "audio/x-ms-wma",
            Mime::WavPack => "audio/wavpack",
            Mime::Html => "text/html",
            Mime::PlainText => "text/plain",
            Mime::Avi => "video/x-msvideo",
            Mime::Flv => "video/x-flv",
            Mime::Mov => "video/quicktime",
            Mime::Mp4Video => "video/mp4",
            Mime::Mpeg => "video/mpeg",
            Mime::Wmv => "video/x-ms-wmv",
            Mime::Mkv => "video/x-matroska",
            Mime::Ogv => "video/ogg",
            Mime::RealVideo => "video/vnd.rn-realvideo",
            Mime::Webm => "video/webm",
            // Python overrides these after building the dict
            Mime::UndeterminedWm => "audio/x-ms-wma or video/x-ms-wmv",
            Mime::UndeterminedMp4 => "audio/mp4 or video/mp4",
            Mime::UndeterminedPng => "image/png or image/apng",
            Mime::UndeterminedWebp => "image/webp, static or animated",
            Mime::UndeterminedJxl => "image/jxl, static or animated",
            Mime::UndeterminedOle => "application/x-ole-storage",
            Mime::Unknown => "unknown filetype",
            Mime::GeneralApplication => "application",
            Mime::GeneralAppArchive => "archive",
            Mime::GeneralImageProject => "image project file",
            Mime::GeneralAudio => "audio",
            Mime::GeneralImage => "image",
            Mime::GeneralVideo => "video",
            Mime::GeneralAnimation => "animation",
            Mime::GeneralFile => "file",
            Mime::UndeterminedGif => "image/gif",
        }
    }

    /// `mime_ext_lookup`
    pub const fn extension(self) -> &'static str {
        match self {
            Mime::Collection => ".collection",
            Mime::Jpeg => ".jpg",
            Mime::Png | Mime::Apng => ".png",
            Mime::StaticGif | Mime::AnimationGif | Mime::UndeterminedGif => ".gif",
            Mime::Bmp => ".bmp",
            Mime::Webp | Mime::AnimationWebp | Mime::UndeterminedWebp => ".webp",
            Mime::Tiff => ".tiff",
            Mime::Qoi => ".qoi",
            Mime::Icon => ".ico",
            Mime::Svg => ".svg",
            Mime::Heif => ".heif",
            Mime::HeifSequence => ".heifs",
            Mime::Heic => ".heic",
            Mime::HeicSequence => ".heics",
            Mime::Avif => ".avif",
            Mime::AvifSequence => ".avifs",
            Mime::Jxl | Mime::AnimationJxl | Mime::UndeterminedJxl => ".jxl",
            Mime::Ugoira => ".zip",
            Mime::Cbz => ".cbz",
            Mime::Flash => ".swf",
            Mime::OctetStream => ".bin",
            Mime::Yaml => ".yaml",
            Mime::Json => ".json",
            Mime::Cbor => ".cbor",
            Mime::Pdf => ".pdf",
            Mime::Docx => ".docx",
            Mime::Xlsx => ".xlsx",
            Mime::Pptx => ".pptx",
            Mime::Doc => ".doc",
            Mime::Xls => ".xls",
            Mime::Ppt => ".ppt",
            Mime::Epub => ".epub",
            Mime::Djvu => ".djvu",
            Mime::Rtf => ".rtf",
            Mime::Psd => ".psd",
            Mime::Clip => ".clip",
            Mime::Sai2 => ".sai2",
            Mime::Krita => ".kra",
            Mime::OpenRaster => ".ora",
            Mime::PaintDotNet => ".pdn",
            Mime::Xcf => ".xcf",
            Mime::Procreate => ".procreate",
            Mime::Zip => ".zip",
            Mime::Rar => ".rar",
            Mime::SevenZ => ".7z",
            Mime::Gzip => ".gz",
            Mime::WindowsExe => ".exe",
            Mime::EncryptedZip => ".zip.encrypted",
            Mime::UpdateContent | Mime::UpdateDefinitions => "",
            Mime::M4a => ".m4a",
            Mime::Mp3 => ".mp3",
            Mime::Mkv | Mime::AudioMkv => ".mkv",
            Mime::AudioMp4 => ".mp4",
            Mime::Ogg => ".ogg",
            Mime::RealAudio => ".ra",
            Mime::Flac => ".flac",
            Mime::Wave => ".wav",
            Mime::TrueAudio => ".tta",
            Mime::Wma => ".wma",
            Mime::WavPack => ".wv",
            Mime::Html => ".html",
            Mime::PlainText => ".txt",
            Mime::Avi => ".avi",
            Mime::Flv => ".flv",
            Mime::Mov => ".mov",
            Mime::Mp4Video => ".mp4",
            Mime::Mpeg => ".mpeg",
            Mime::Wmv => ".wmv",
            Mime::Ogv => ".ogv",
            Mime::RealVideo => ".rm",
            Mime::Webm => ".webm",
            Mime::Unknown
            | Mime::UndeterminedWm
            | Mime::UndeterminedPng
            | Mime::UndeterminedMp4
            | Mime::UndeterminedOle
            | Mime::GeneralFile
            | Mime::GeneralApplication
            | Mime::GeneralAppArchive
            | Mime::GeneralImageProject
            | Mime::GeneralAudio
            | Mime::GeneralImage
            | Mime::GeneralVideo
            | Mime::GeneralAnimation => "",
        }
    }

    /// PYTHON: `mime_string_lookup` reverse (`string_enum_lookup`). Note this
    /// is *not* the same as MIME type lookup.
    pub fn from_nice_string(text: &str) -> Option<Self> {
        Mime::ALL.iter().copied().find(|mime| mime.as_str() == text)
    }

    /// Python: `mime_enum_lookup`
    pub fn from_mimetype(text: &str) -> Option<MimeLookup> {
        let lookup = match text {
            "collection" => MimeLookup::One(Mime::Collection),
            "image/jpe" | "image/jpeg" | "image/jpg" => MimeLookup::One(Mime::Jpeg),
            "image/x-png" | "image/png" => MimeLookup::One(Mime::Png),
            "image/apng" => MimeLookup::One(Mime::Apng),
            "image/gif" => MimeLookup::One(Mime::AnimationGif),
            "image/bmp" => MimeLookup::One(Mime::Bmp),
            "image/webp" => MimeLookup::One(Mime::Webp),
            "image/tiff" => MimeLookup::One(Mime::Tiff),
            "image/qoi" => MimeLookup::One(Mime::Qoi),
            "image/x-icon" | "image/vnd.microsoft.icon" => MimeLookup::One(Mime::Icon),
            "image/svg+xml" => MimeLookup::One(Mime::Svg),
            "image/heif" => MimeLookup::One(Mime::Heif),
            "image/heif-sequence" => MimeLookup::One(Mime::HeifSequence),
            "image/heic" => MimeLookup::One(Mime::Heic),
            "image/heic-sequence" => MimeLookup::One(Mime::HeicSequence),
            "image/avif" => MimeLookup::One(Mime::Avif),
            "image/avif-sequence" => MimeLookup::One(Mime::AvifSequence),
            "image/jxl" => MimeLookup::One(Mime::Jxl),
            "image" => MimeLookup::Many(IMAGES),
            "application/x-shockwave-flash" => MimeLookup::One(Mime::Flash),
            "application/x-photoshop"
            | "image/vnd.adobe.photoshop"
            | "application/vnd.adobe.photoshop" => MimeLookup::One(Mime::Psd),
            "application/clip" => MimeLookup::One(Mime::Clip),
            "application/sai2" => MimeLookup::One(Mime::Sai2),
            "application/x-krita" => MimeLookup::One(Mime::Krita),
            "image/openraster" => MimeLookup::One(Mime::OpenRaster),
            "image/vnd.paint.net" | "application/x-paintnet" => MimeLookup::One(Mime::PaintDotNet),
            "application/x-procreate" => MimeLookup::One(Mime::Procreate),
            "image/x-xcf" => MimeLookup::One(Mime::Xcf),
            "application/octet-stream" => MimeLookup::One(Mime::OctetStream),
            "application/x-yaml" => MimeLookup::One(Mime::Yaml),
            "PDF document" | "application/pdf" => MimeLookup::One(Mime::Pdf),
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document" => {
                MimeLookup::One(Mime::Docx)
            }
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet" => {
                MimeLookup::One(Mime::Xlsx)
            }
            "application/vnd.openxmlformats-officedocument.presentationml.presentation" => {
                MimeLookup::One(Mime::Pptx)
            }
            "application/msword" | "application/vnd.ms-word" => MimeLookup::One(Mime::Doc),
            "application/vnd.ms-excel" | "application/msexcel" => MimeLookup::One(Mime::Xls),
            "application/vnd.ms-powerpoint"
            | "application/powerpoint"
            | "application/mspowerpoint" => MimeLookup::One(Mime::Ppt),
            "application/epub+zip" => MimeLookup::One(Mime::Epub),
            "image/vnd.djvu" | "image/vnd.djvu+multipage" | "image/x-djvu" => {
                MimeLookup::One(Mime::Djvu)
            }
            "text/rtf" | "application/rtf" => MimeLookup::One(Mime::Rtf),
            "application/vnd.comicbook+zip" => MimeLookup::One(Mime::Cbz),
            "application/zip" => MimeLookup::One(Mime::Zip),
            "application/vnd.rar" => MimeLookup::One(Mime::Rar),
            "application/x-7z-compressed" => MimeLookup::One(Mime::SevenZ),
            "application/gzip" => MimeLookup::One(Mime::Gzip),
            "application/json" => MimeLookup::One(Mime::Json),
            "application/cbor" => MimeLookup::One(Mime::Cbor),
            "application/hydrus-encrypted-zip" => MimeLookup::One(Mime::EncryptedZip),
            "application/hydrus-update-content" => MimeLookup::One(Mime::UpdateContent),
            "application/hydrus-update-definitions" => MimeLookup::One(Mime::UpdateDefinitions),
            "application" => MimeLookup::Many(APPLICATIONS),
            "audio/mp4" => MimeLookup::One(Mime::M4a),
            "audio/mp3" => MimeLookup::One(Mime::Mp3),
            "audio/ogg" => MimeLookup::One(Mime::Ogg),
            "audio/vnd.rn-realaudio" => MimeLookup::One(Mime::RealAudio),
            "audio/x-tta" => MimeLookup::One(Mime::TrueAudio),
            "audio/flac" => MimeLookup::One(Mime::Flac),
            "audio/x-wav" | "audio/wav" | "audio/wave" => MimeLookup::One(Mime::Wave),
            "audio/x-ms-wma" => MimeLookup::One(Mime::Wma),
            "audio/wavpack" => MimeLookup::One(Mime::WavPack),
            "text/html" => MimeLookup::One(Mime::Html),
            "text/plain" => MimeLookup::One(Mime::PlainText),
            "video/x-msvideo" => MimeLookup::One(Mime::Avi),
            "video/x-flv" => MimeLookup::One(Mime::Flv),
            "video/quicktime" => MimeLookup::One(Mime::Mov),
            "video/mp4" => MimeLookup::One(Mime::Mp4Video),
            "video/mpeg" => MimeLookup::One(Mime::Mpeg),
            "video/x-ms-wmv" => MimeLookup::One(Mime::Wmv),
            "video/x-matroska" => MimeLookup::One(Mime::Mkv),
            "video/ogg" => MimeLookup::One(Mime::Ogv),
            "video/vnd.rn-realvideo" | "application/vnd.rn-realmedia" => {
                MimeLookup::One(Mime::RealVideo)
            }
            "video/webm" => MimeLookup::One(Mime::Webm),
            "video" => MimeLookup::Many(VIDEO),
            "application/x-ole-storage" => MimeLookup::One(Mime::UndeterminedOle),
            "unknown filetype" => MimeLookup::One(Mime::Unknown),
            _ => return None,
        };

        Some(lookup)
    }
}

/// The Python lookup sometimes yields a single mime and sometimes a group
/// (`'image'` -> `IMAGES`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MimeLookup {
    One(Mime),
    Many(&'static [Mime]),
}

impl MimeLookup {
    pub fn contains(self, mime: Mime) -> bool {
        match self {
            MimeLookup::One(one) => one == mime,
            MimeLookup::Many(many) => many.contains(&mime),
        }
    }

    pub fn first(self) -> Option<Mime> {
        match self {
            MimeLookup::One(one) => Some(one),
            MimeLookup::Many(many) => many.first().copied(),
        }
    }

    pub fn as_slice(self) -> Option<&'static [Mime]> {
        match self {
            MimeLookup::One(_) => None,
            MimeLookup::Many(many) => Some(many),
        }
    }
}

// Ordered lists, as in Python -- some UI code relies on the order.
pub const IMAGES: &[Mime] = &[
    Mime::Jpeg,
    Mime::Png,
    Mime::StaticGif,
    Mime::Webp,
    Mime::Jxl,
    Mime::Avif,
    Mime::Bmp,
    Mime::Heic,
    Mime::Heif,
    Mime::Icon,
    Mime::Qoi,
    Mime::Tiff,
];

pub const ANIMATIONS: &[Mime] = &[
    Mime::AnimationGif,
    Mime::Apng,
    Mime::AnimationWebp,
    Mime::AnimationJxl,
    Mime::AvifSequence,
    Mime::HeicSequence,
    Mime::HeifSequence,
    Mime::Ugoira,
];

pub const VIEWABLE_ANIMATIONS: &[Mime] = ANIMATIONS;

pub const HEIF_TYPE_SEQUENCES: &[Mime] = &[Mime::HeicSequence, Mime::HeifSequence];

pub const AUDIO: &[Mime] = &[
    Mime::Mp3,
    Mime::Ogg,
    Mime::Flac,
    Mime::M4a,
    Mime::AudioMkv,
    Mime::AudioMp4,
    Mime::RealAudio,
    Mime::TrueAudio,
    Mime::Wave,
    Mime::WavPack,
    Mime::Wma,
];

pub const VIDEO: &[Mime] = &[
    Mime::Mp4Video,
    Mime::Webm,
    Mime::Mkv,
    Mime::Avi,
    Mime::Flv,
    Mime::Mov,
    Mime::Mpeg,
    Mime::Ogv,
    Mime::RealVideo,
    Mime::Wmv,
];

pub const APPLICATIONS: &[Mime] = &[
    Mime::Flash,
    Mime::Pdf,
    Mime::Epub,
    Mime::Djvu,
    Mime::Docx,
    Mime::Xlsx,
    Mime::Pptx,
    Mime::Doc,
    Mime::Xls,
    Mime::Ppt,
    Mime::Rtf,
];

pub const IMAGE_PROJECT_FILES: &[Mime] = &[
    Mime::Clip,
    Mime::Krita,
    Mime::OpenRaster,
    Mime::PaintDotNet,
    Mime::Procreate,
    Mime::Psd,
    Mime::Sai2,
    Mime::Svg,
    Mime::Xcf,
];

pub const ARCHIVES: &[Mime] = &[Mime::Cbz, Mime::SevenZ, Mime::Gzip, Mime::Rar, Mime::Zip];

pub const GENERAL_CLASSES_OF_FILETYPE: &[Mime] = &[
    Mime::GeneralApplication,
    Mime::GeneralAudio,
    Mime::GeneralImage,
    Mime::GeneralVideo,
    Mime::GeneralAnimation,
    Mime::GeneralAppArchive,
    Mime::GeneralImageProject,
];

pub const VIEWABLE_IMAGE_PROJECT_FILES: &[Mime] = &[Mime::Psd, Mime::Krita, Mime::OpenRaster];

/// zip files that have a `mimetype` file inside
pub const OPEN_DOCUMENT_ZIPS: &[Mime] = &[Mime::Krita, Mime::Epub, Mime::OpenRaster];

/// zip files that have a `[Content_Types].xml` file inside
pub const MICROSOFT_OPEN_XML_DOCUMENT_ZIPS: &[Mime] = &[Mime::Docx, Mime::Xlsx, Mime::Pptx];

pub const POTENTIAL_OFFICE_OLE_MIMES: &[Mime] = &[Mime::Doc, Mime::Ppt, Mime::Xls];

pub const SEARCHABLE_MIMES: &[Mime] = &[
    Mime::Jpeg,
    Mime::Png,
    Mime::Apng,
    Mime::StaticGif,
    Mime::AnimationGif,
    Mime::Webp,
    Mime::AnimationWebp,
    Mime::Tiff,
    Mime::Qoi,
    Mime::Icon,
    Mime::Svg,
    Mime::Heif,
    Mime::HeifSequence,
    Mime::Heic,
    Mime::HeicSequence,
    Mime::Avif,
    Mime::AvifSequence,
    Mime::Bmp,
    Mime::Jxl,
    Mime::AnimationJxl,
    Mime::Ugoira,
    Mime::Flash,
    Mime::Avi,
    Mime::Flv,
    Mime::Mov,
    Mime::Mp4Video,
    Mime::Mkv,
    Mime::RealVideo,
    Mime::Webm,
    Mime::Ogv,
    Mime::Mpeg,
    Mime::Cbz,
    Mime::Clip,
    Mime::Psd,
    Mime::Sai2,
    Mime::Krita,
    Mime::OpenRaster,
    Mime::Xcf,
    Mime::Procreate,
    Mime::Pdf,
    Mime::Docx,
    Mime::Xlsx,
    Mime::Pptx,
    Mime::Doc,
    Mime::Xls,
    Mime::Ppt,
    Mime::Epub,
    Mime::Djvu,
    Mime::PaintDotNet,
    Mime::Rtf,
    Mime::Zip,
    Mime::Rar,
    Mime::SevenZ,
    Mime::Gzip,
    Mime::M4a,
    Mime::Mp3,
    Mime::RealAudio,
    Mime::Ogg,
    Mime::Flac,
    Mime::Wave,
    Mime::TrueAudio,
    Mime::Wma,
    Mime::Wmv,
    Mime::AudioMkv,
    Mime::AudioMp4,
    Mime::WavPack,
];

pub static SEARCHABLE_MIMES_SET: LazyLock<HashSet<Mime>> =
    LazyLock::new(|| SEARCHABLE_MIMES.iter().copied().collect());

pub fn allowed_mimes() -> HashSet<Mime> {
    let mut allowed = SEARCHABLE_MIMES_SET.clone();
    allowed.insert(Mime::UpdateContent);
    allowed.insert(Mime::UpdateDefinitions);
    allowed
}

pub const DECOMPRESSION_BOMB_IMAGES: &[Mime] = &[Mime::Jpeg, Mime::Png];

pub const PIL_HEIF_MIMES: &[Mime] = &[
    Mime::Heif,
    Mime::HeifSequence,
    Mime::Heic,
    Mime::HeicSequence,
];

pub const PIL_AVIF_MIMES: &[Mime] = &[Mime::Avif];

pub const MIMES_THAT_WE_CAN_CHECK_FOR_TRANSPARENCY: &[Mime] = &[
    Mime::Jxl,
    Mime::Png,
    Mime::StaticGif,
    Mime::Webp,
    Mime::Bmp,
    Mime::Icon,
    Mime::Tiff,
    Mime::Qoi,
    Mime::Avif,
    Mime::Heif,
    Mime::Heic,
    Mime::AnimationGif,
    Mime::Apng,
    Mime::AnimationWebp,
    Mime::AnimationJxl,
    Mime::AvifSequence,
    Mime::HeifSequence,
    Mime::HeicSequence,
];

pub const FILES_THAT_CAN_HAVE_IPTC: &[Mime] = &[Mime::Jpeg, Mime::Tiff];

/// `mimes_to_general_mimetypes` -- which general class a specific mime belongs
/// to (only the listed groups map).
pub fn general_mimetype_for(mime: Mime) -> Option<Mime> {
    let groups: &[(&[Mime], Mime)] = &[
        (APPLICATIONS, Mime::GeneralApplication),
        (ARCHIVES, Mime::GeneralAppArchive),
        (IMAGE_PROJECT_FILES, Mime::GeneralImageProject),
        (AUDIO, Mime::GeneralAudio),
        (IMAGES, Mime::GeneralImage),
        (VIDEO, Mime::GeneralVideo),
        (ANIMATIONS, Mime::GeneralAnimation),
    ];

    groups
        .iter()
        .find(|(mimes, _)| mimes.contains(&mime))
        .map(|(_, general)| *general)
}

pub fn mimes_with_thumbnails() -> HashSet<Mime> {
    let mut mimes: HashSet<Mime> = IMAGES.iter().copied().collect();
    mimes.extend(ANIMATIONS);
    mimes.extend(VIDEO);
    mimes.extend(AUDIO);
    mimes.extend([
        Mime::Svg,
        Mime::Pdf,
        Mime::Flash,
        Mime::Clip,
        Mime::Procreate,
        Mime::Cbz,
        Mime::Pptx,
        Mime::PaintDotNet,
        Mime::Epub,
    ]);
    mimes.extend(VIEWABLE_IMAGE_PROJECT_FILES);
    mimes
}

pub fn mimes_that_always_have_good_resolution() -> HashSet<Mime> {
    let mut mimes: HashSet<Mime> = IMAGES.iter().copied().collect();
    mimes.extend(ANIMATIONS);
    mimes.extend(VIDEO);
    mimes
}

pub fn mimes_that_definitely_have_audio() -> Vec<Mime> {
    let mut mimes = vec![Mime::Flash];
    mimes.extend(AUDIO);
    mimes
}

pub fn mimes_that_may_have_audio() -> Vec<Mime> {
    let mut mimes = mimes_that_definitely_have_audio();
    mimes.extend(VIDEO);
    mimes
}

pub fn mimes_that_may_theoretically_have_transparency() -> HashSet<Mime> {
    let mut mimes: HashSet<Mime> = MIMES_THAT_WE_CAN_CHECK_FOR_TRANSPARENCY
        .iter()
        .copied()
        .collect();
    mimes.extend([
        Mime::Psd,
        Mime::Sai2,
        Mime::Krita,
        Mime::OpenRaster,
        Mime::Xcf,
        Mime::Procreate,
        Mime::Clip,
        Mime::Svg,
    ]);
    mimes
}

pub fn files_that_can_have_icc_profile() -> HashSet<Mime> {
    let mut mimes: HashSet<Mime> = [
        Mime::Bmp,
        Mime::Jpeg,
        Mime::Jxl,
        Mime::Tiff,
        Mime::Png,
        Mime::StaticGif,
        Mime::Psd,
        Mime::Avif,
        Mime::AvifSequence,
    ]
    .into_iter()
    .collect();
    mimes.extend(PIL_HEIF_MIMES);
    mimes
}

pub fn files_that_can_have_exif() -> HashSet<Mime> {
    let mut mimes: HashSet<Mime> = [
        Mime::Jpeg,
        Mime::Jxl,
        Mime::Tiff,
        Mime::Png,
        Mime::Webp,
        Mime::AnimationJxl,
        Mime::Apng,
        Mime::AnimationWebp,
        Mime::Avif,
        Mime::AvifSequence,
    ]
    .into_iter()
    .collect();
    mimes.extend(PIL_HEIF_MIMES);
    mimes
}

pub fn files_that_can_have_xmp() -> HashSet<Mime> {
    let mut mimes: HashSet<Mime> = [
        Mime::Jpeg,
        Mime::Tiff,
        Mime::Png,
        Mime::Webp,
        Mime::AnimationWebp,
        Mime::Apng,
        Mime::Avif,
        Mime::AvifSequence,
    ]
    .into_iter()
    .collect();
    mimes.extend(PIL_HEIF_MIMES);
    mimes
}

pub fn files_that_can_have_human_readable_embedded_metadata() -> HashSet<Mime> {
    let mut mimes: HashSet<Mime> = [
        Mime::Jpeg,
        Mime::Jxl,
        Mime::Png,
        Mime::Bmp,
        Mime::Webp,
        Mime::Tiff,
        Mime::Icon,
        Mime::StaticGif,
        Mime::Avif,
        Mime::AnimationJxl,
        Mime::AnimationGif,
        Mime::Apng,
        Mime::AnimationWebp,
    ]
    .into_iter()
    .collect();
    mimes.extend(PIL_HEIF_MIMES);
    mimes
}

pub fn files_that_can_have_software_source() -> HashSet<Mime> {
    files_that_can_have_human_readable_embedded_metadata()
}

pub fn files_that_can_have_pixel_hash() -> HashSet<Mime> {
    let mut mimes: HashSet<Mime> = IMAGES.iter().copied().collect();
    mimes.extend(VIEWABLE_IMAGE_PROJECT_FILES);
    mimes
}

pub fn files_that_have_perceptual_hash() -> HashSet<Mime> {
    files_that_can_have_pixel_hash()
}

pub const HYDRUS_UPDATE_FILES: &[Mime] = &[Mime::UpdateDefinitions, Mime::UpdateContent];

pub static IMAGE_FILE_EXTS: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    let mut exts: HashSet<&'static str> = IMAGES.iter().map(|m| m.extension()).collect();
    exts.insert(".jpe");
    exts.insert(".jpeg");
    exts
});

pub static VIDEO_FILE_EXTS: LazyLock<HashSet<&'static str>> =
    LazyLock::new(|| VIDEO.iter().map(|m| m.extension()).collect());

pub fn allowed_mime_extensions() -> Vec<&'static str> {
    let mut extensions: Vec<&'static str> = allowed_mimes()
        .iter()
        .map(|mime| mime.extension())
        .collect();
    extensions.sort_unstable();
    extensions.dedup();
    extensions
}

// ============================================================================
// 8. SERVICE GROUPS
// ============================================================================

pub const SPECIFIC_LOCAL_FILE_SERVICES: &[ServiceType] = &[
    ServiceType::LocalFileDomain,
    ServiceType::LocalFileUpdateDomain,
    ServiceType::LocalFileTrashDomain,
];

pub const LOCAL_FILE_SERVICES: &[ServiceType] = &[
    ServiceType::LocalFileDomain,
    ServiceType::LocalFileUpdateDomain,
    ServiceType::LocalFileTrashDomain,
    ServiceType::HydrusLocalFileStorage,
    ServiceType::CombinedLocalFileDomains,
];

pub const LOCAL_FILE_SERVICES_IN_NICE_ORDER: &[ServiceType] = &[
    ServiceType::LocalFileDomain,
    ServiceType::CombinedLocalFileDomains,
    ServiceType::LocalFileTrashDomain,
    ServiceType::LocalFileUpdateDomain,
    ServiceType::HydrusLocalFileStorage,
];

pub const LOCAL_TAG_SERVICES: &[ServiceType] = &[ServiceType::LocalTag];

pub const LOCAL_SERVICES: &[ServiceType] = &[
    ServiceType::LocalFileDomain,
    ServiceType::LocalFileUpdateDomain,
    ServiceType::LocalFileTrashDomain,
    ServiceType::HydrusLocalFileStorage,
    ServiceType::CombinedLocalFileDomains,
    ServiceType::LocalTag,
    ServiceType::LocalRatingLike,
    ServiceType::LocalRatingNumerical,
    ServiceType::LocalRatingIncDec,
    ServiceType::LocalNotes,
    ServiceType::ClientApiService,
];

pub const STAR_RATINGS_SERVICES: &[ServiceType] = &[
    ServiceType::LocalRatingLike,
    ServiceType::LocalRatingNumerical,
    ServiceType::RatingLikeRepository,
    ServiceType::RatingNumericalRepository,
];

pub const LOCAL_RATINGS_SERVICES: &[ServiceType] = &[
    ServiceType::LocalRatingLike,
    ServiceType::LocalRatingNumerical,
    ServiceType::LocalRatingIncDec,
];

pub const RATINGS_SERVICES: &[ServiceType] = &[
    ServiceType::LocalRatingLike,
    ServiceType::LocalRatingNumerical,
    ServiceType::LocalRatingIncDec,
    ServiceType::RatingLikeRepository,
    ServiceType::RatingNumericalRepository,
];

pub const REPOSITORIES: &[ServiceType] = &[
    ServiceType::TagRepository,
    ServiceType::FileRepository,
    ServiceType::RatingLikeRepository,
    ServiceType::RatingNumericalRepository,
];

pub const RESTRICTED_SERVICES: &[ServiceType] = &[
    ServiceType::TagRepository,
    ServiceType::FileRepository,
    ServiceType::RatingLikeRepository,
    ServiceType::RatingNumericalRepository,
    ServiceType::ServerAdmin,
    ServiceType::MessageDepot,
];

pub const REMOTE_SERVICES: &[ServiceType] = &[
    ServiceType::TagRepository,
    ServiceType::FileRepository,
    ServiceType::RatingLikeRepository,
    ServiceType::RatingNumericalRepository,
    ServiceType::ServerAdmin,
    ServiceType::MessageDepot,
    ServiceType::Ipfs,
];

pub const REMOTE_FILE_SERVICES: &[ServiceType] = &[ServiceType::FileRepository, ServiceType::Ipfs];

pub const REAL_FILE_SERVICES: &[ServiceType] = &[
    ServiceType::LocalFileDomain,
    ServiceType::LocalFileUpdateDomain,
    ServiceType::LocalFileTrashDomain,
    ServiceType::HydrusLocalFileStorage,
    ServiceType::CombinedLocalFileDomains,
    ServiceType::CombinedDeletedFile,
    ServiceType::FileRepository,
    ServiceType::Ipfs,
];

pub const REAL_TAG_SERVICES: &[ServiceType] = &[ServiceType::LocalTag, ServiceType::TagRepository];

pub const ADDREMOVABLE_SERVICES: &[ServiceType] = &[
    ServiceType::LocalTag,
    ServiceType::LocalFileDomain,
    ServiceType::LocalRatingLike,
    ServiceType::LocalRatingNumerical,
    ServiceType::LocalRatingIncDec,
    ServiceType::FileRepository,
    ServiceType::TagRepository,
    ServiceType::ServerAdmin,
    ServiceType::Ipfs,
];

pub const MUST_HAVE_AT_LEAST_ONE_SERVICES: &[ServiceType] =
    &[ServiceType::LocalTag, ServiceType::LocalFileDomain];

pub const MUST_BE_EMPTY_OF_FILES_SERVICES: &[ServiceType] = &[ServiceType::LocalFileDomain];

pub const FILE_SERVICES_WITH_SPECIFIC_MAPPING_CACHES: &[ServiceType] = REAL_FILE_SERVICES;

pub const FILE_SERVICES_WITH_SPECIFIC_TAG_LOOKUP_CACHES: &[ServiceType] = &[
    ServiceType::HydrusLocalFileStorage,
    ServiceType::CombinedDeletedFile,
    ServiceType::FileRepository,
    ServiceType::Ipfs,
];

pub const FILE_SERVICES_COVERED_BY_COMBINED_LOCAL_FILE_DOMAINS: &[ServiceType] =
    &[ServiceType::LocalFileDomain];

pub const FILE_SERVICES_COVERED_BY_HYDRUS_LOCAL_FILE_STORAGE: &[ServiceType] = &[
    ServiceType::CombinedLocalFileDomains,
    ServiceType::LocalFileDomain,
    ServiceType::LocalFileUpdateDomain,
    ServiceType::LocalFileTrashDomain,
];

pub const FILE_SERVICES_COVERED_BY_COMBINED_DELETED_FILE: &[ServiceType] = &[
    ServiceType::CombinedLocalFileDomains,
    ServiceType::LocalFileDomain,
    ServiceType::LocalFileUpdateDomain,
    ServiceType::FileRepository,
    ServiceType::Ipfs,
];

pub const ALL_SERVICES: &[ServiceType] = &[
    ServiceType::TagRepository,
    ServiceType::FileRepository,
    ServiceType::RatingLikeRepository,
    ServiceType::RatingNumericalRepository,
    ServiceType::ServerAdmin,
    ServiceType::MessageDepot,
    ServiceType::Ipfs,
    ServiceType::LocalFileDomain,
    ServiceType::LocalFileUpdateDomain,
    ServiceType::LocalFileTrashDomain,
    ServiceType::HydrusLocalFileStorage,
    ServiceType::CombinedLocalFileDomains,
    ServiceType::LocalTag,
    ServiceType::LocalRatingLike,
    ServiceType::LocalRatingNumerical,
    ServiceType::LocalRatingIncDec,
    ServiceType::LocalNotes,
    ServiceType::ClientApiService,
    ServiceType::CombinedFile,
    ServiceType::CombinedTag,
    ServiceType::CombinedDeletedFile,
];

pub const ALL_TAG_SERVICES: &[ServiceType] = &[
    ServiceType::LocalTag,
    ServiceType::TagRepository,
    ServiceType::CombinedTag,
];

pub const ALL_FILE_SERVICES: &[ServiceType] = &[
    ServiceType::LocalFileDomain,
    ServiceType::LocalFileUpdateDomain,
    ServiceType::LocalFileTrashDomain,
    ServiceType::HydrusLocalFileStorage,
    ServiceType::CombinedLocalFileDomains,
    ServiceType::CombinedDeletedFile,
    ServiceType::FileRepository,
    ServiceType::Ipfs,
    ServiceType::CombinedFile,
];

pub const FILE_SERVICES_WITH_NO_DELETE_RECORD: &[ServiceType] = &[
    ServiceType::CombinedFile,
    ServiceType::LocalFileTrashDomain,
    ServiceType::CombinedDeletedFile,
];

pub fn file_services_with_delete_record() -> Vec<ServiceType> {
    REAL_FILE_SERVICES
        .iter()
        .copied()
        .filter(|service| !FILE_SERVICES_WITH_NO_DELETE_RECORD.contains(service))
        .collect()
}

pub const SERVICES_WITH_THUMBNAILS: &[ServiceType] =
    &[ServiceType::FileRepository, ServiceType::LocalFileDomain];

pub const SERVICE_TYPES_TO_CONTENT_TYPES: &[(ServiceType, &[ContentType])] = &[
    (ServiceType::FileRepository, &[ContentType::Files]),
    (ServiceType::LocalFileDomain, &[ContentType::Files]),
    (ServiceType::LocalFileUpdateDomain, &[ContentType::Files]),
    (ServiceType::LocalFileTrashDomain, &[ContentType::Files]),
    (ServiceType::CombinedLocalFileDomains, &[ContentType::Files]),
    (ServiceType::HydrusLocalFileStorage, &[ContentType::Files]),
    (ServiceType::Ipfs, &[ContentType::Files]),
    (
        ServiceType::TagRepository,
        &[
            ContentType::Mappings,
            ContentType::TagParents,
            ContentType::TagSiblings,
        ],
    ),
    (
        ServiceType::LocalTag,
        &[
            ContentType::Mappings,
            ContentType::TagParents,
            ContentType::TagSiblings,
        ],
    ),
    (ServiceType::LocalRatingLike, &[ContentType::Ratings]),
    (ServiceType::LocalRatingNumerical, &[ContentType::Ratings]),
    (ServiceType::LocalRatingIncDec, &[ContentType::Ratings]),
];

pub fn content_types_for_service(service_type: ServiceType) -> Option<&'static [ContentType]> {
    SERVICE_TYPES_TO_CONTENT_TYPES
        .iter()
        .find(|(service, _)| *service == service_type)
        .map(|(_, content_types)| *content_types)
}

// ============================================================================
// 9. SERVICE INFO
// ============================================================================

int_enum_numbers!(ServiceInfoType : u8 {
    NumFiles = 0,
    NumInbox = 1,
    NumLocal = 2,
    NumMappings = 3,
    NumDeletedMappings = 4,
    NumDeletedFiles = 5,
    NumThumbnails = 6,
    NumThumbnailsLocal = 7,
    TotalSize = 8,
    NumNamespaces = 9,
    NumTags = 10,
    NumPending = 11,
    NumConversations = 12,
    NumUnread = 13,
    NumDrafts = 14,
    NumPendingMappings = 15,
    NumPetitionedMappings = 16,
    NumPendingTagSiblings = 17,
    NumPetitionedTagSiblings = 18,
    NumPendingTagParents = 19,
    NumPetitionedTagParents = 20,
    NumShares = 21,
    NumViewableFiles = 22,
    NumTagSiblings = 23,
    NumTagParents = 24,
    NumDeletedTagSiblings = 25,
    NumDeletedTagParents = 26,
    NumActionableFileAddPetitions = 27,
    NumActionableFileDeletePetitions = 28,
    NumActionableMappingAddPetitions = 29,
    NumActionableMappingDeletePetitions = 30,
    NumActionableSiblingAddPetitions = 31,
    NumActionableSiblingDeletePetitions = 32,
    NumActionableParentAddPetitions = 33,
    NumActionableParentDeletePetitions = 34,
    NumFileHashes = 35,
    NumPendingFiles = 36,
    NumPetitionedFiles = 37,
});

impl ServiceInfoType {
    /// `service_info_enum_str_lookup`
    pub const fn as_str(self) -> &'static str {
        match self {
            ServiceInfoType::NumFileHashes => "number of file hashes",
            ServiceInfoType::NumFiles => "number of files",
            ServiceInfoType::NumInbox => "number in inbox",
            ServiceInfoType::NumLocal => "number of local files",
            ServiceInfoType::NumMappings => "number of mappings",
            ServiceInfoType::NumDeletedMappings => "number of deleted mappings",
            ServiceInfoType::NumDeletedFiles => "number of deleted files",
            ServiceInfoType::NumThumbnails => "number of thumbnails",
            ServiceInfoType::NumThumbnailsLocal => "number of local thumbnails",
            ServiceInfoType::TotalSize => "total size",
            ServiceInfoType::NumNamespaces => "number of namespaces",
            ServiceInfoType::NumTags => "number of tags",
            ServiceInfoType::NumPending => "number pending",
            ServiceInfoType::NumConversations => "number of conversations",
            ServiceInfoType::NumUnread => "number of unread conversations",
            ServiceInfoType::NumDrafts => "number of drafts",
            ServiceInfoType::NumPendingMappings => "number of pending mappings",
            ServiceInfoType::NumPetitionedMappings => "number of petitioned mappings",
            ServiceInfoType::NumPendingFiles => "number of pending files",
            ServiceInfoType::NumPetitionedFiles => "number of petitioned files",
            ServiceInfoType::NumPendingTagSiblings => "number of pending tag siblings",
            ServiceInfoType::NumPetitionedTagSiblings => "number of petitioned tag siblings",
            ServiceInfoType::NumPendingTagParents => "number of pending tag parents",
            ServiceInfoType::NumPetitionedTagParents => "number of petitioned tag parents",
            ServiceInfoType::NumShares => "number of shares",
            ServiceInfoType::NumViewableFiles => "number of viewable files",
            ServiceInfoType::NumTagSiblings => "number of tag siblings",
            ServiceInfoType::NumTagParents => "number of tag parents",
            ServiceInfoType::NumDeletedTagSiblings => "number of deleted tag siblings",
            ServiceInfoType::NumDeletedTagParents => "number of deleted tag parents",
            ServiceInfoType::NumActionableFileAddPetitions => {
                "number of actionable add-file petitions"
            }
            ServiceInfoType::NumActionableFileDeletePetitions => {
                "number of actionable delete-file petitions"
            }
            ServiceInfoType::NumActionableMappingAddPetitions => {
                "number of actionable add-mapping petitions"
            }
            ServiceInfoType::NumActionableMappingDeletePetitions => {
                "number of actionable delete-mapping petitions"
            }
            ServiceInfoType::NumActionableSiblingAddPetitions => {
                "number of actionable add-sibling petitions"
            }
            ServiceInfoType::NumActionableSiblingDeletePetitions => {
                "number of actionable delete-sibling petitions"
            }
            ServiceInfoType::NumActionableParentAddPetitions => {
                "number of actionable add-parent petitions"
            }
            ServiceInfoType::NumActionableParentDeletePetitions => {
                "number of actionable delete-parent petitions"
            }
        }
    }
}

impl fmt::Display for ServiceInfoType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

pub const FILE_REPOSITORY_SERVICE_INFO_TYPES: &[ServiceInfoType] = &[
    ServiceInfoType::NumFileHashes,
    ServiceInfoType::NumFiles,
    ServiceInfoType::NumDeletedFiles,
    ServiceInfoType::NumPendingFiles,
    ServiceInfoType::NumPetitionedFiles,
    ServiceInfoType::NumActionableFileAddPetitions,
    ServiceInfoType::NumActionableFileDeletePetitions,
];

pub const TAG_REPOSITORY_SERVICE_INFO_TYPES: &[ServiceInfoType] = &[
    ServiceInfoType::NumFileHashes,
    ServiceInfoType::NumTags,
    ServiceInfoType::NumMappings,
    ServiceInfoType::NumDeletedMappings,
    ServiceInfoType::NumPendingMappings,
    ServiceInfoType::NumPetitionedMappings,
    ServiceInfoType::NumActionableMappingAddPetitions,
    ServiceInfoType::NumActionableMappingDeletePetitions,
    ServiceInfoType::NumTagSiblings,
    ServiceInfoType::NumDeletedTagSiblings,
    ServiceInfoType::NumPendingTagSiblings,
    ServiceInfoType::NumPetitionedTagSiblings,
    ServiceInfoType::NumActionableSiblingAddPetitions,
    ServiceInfoType::NumActionableSiblingDeletePetitions,
    ServiceInfoType::NumTagParents,
    ServiceInfoType::NumDeletedTagParents,
    ServiceInfoType::NumPendingTagParents,
    ServiceInfoType::NumPetitionedTagParents,
    ServiceInfoType::NumActionableParentAddPetitions,
    ServiceInfoType::NumActionableParentDeletePetitions,
];

// ============================================================================
// 10. MISC ENUMS USED BY OTHER MODULES
// ============================================================================

// Python's `ADD`/`DELETE`/`EDIT`/`SET`.
int_enum_numbers!(ContentAction : u8 {
    Add = 0,
    Delete = 1,
    Edit = 2,
    Set = 3,
});

// ============================================================================
// 11. UNICODE
// ============================================================================

pub const UNICODE_APPROX_EQUAL: &str = "\u{2248}";
pub const UNICODE_BYTE_ORDER_MARK: &str = "\u{FEFF}";
pub const UNICODE_ELLIPSIS: &str = "\u{2026}";
pub const UNICODE_NOT_EQUAL: &str = "\u{2260}";
pub const UNICODE_REPLACEMENT_CHARACTER: &str = "\u{FFFD}";
pub const UNICODE_PLUS_OR_MINUS: &str = "\u{00B1}";
pub const UNICODE_LESS_THAN_OR_EQUAL_TO: &str = "\u{2264}";
pub const UNICODE_GREATER_THAN_OR_EQUAL_TO: &str = "\u{2265}";

pub const ROWS_S_TEXT: &str = " rows/s";

// ============================================================================
// 12. SQLITE / YAML NOTES
// ============================================================================
//
// Python registers adapters that YAML-dump dicts/lists/tuples into TEXT_YAML
// columns and convert INTEGER_BOOLEAN to bool. The Rust port has no YAML
// adapter layer: dynamic values travel as `serde_json::Value` (with
// `preserve_order`) and integers are stored directly.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numeric_values_match_python() {
        assert_eq!(Mime::Jpeg.as_int(), 1);
        assert_eq!(Mime::StaticGif.as_int(), 68);
        assert_eq!(Mime::Unknown.as_int(), 101);
        assert_eq!(ServiceType::LocalTag.as_int(), 5);
        assert_eq!(ServiceType::ServerAdmin.as_int(), 99);
        assert_eq!(ContentType::HttpHeaders.as_int(), 22);
        assert_eq!(ContentUpdate::MoveMerge.as_int(), 22);
        assert_eq!(DuplicateType::ConfirmedAlternate.as_int(), 10);
    }

    #[test]
    fn mime_lookups_work() {
        assert_eq!(
            Mime::from_mimetype("image/jpeg"),
            Some(MimeLookup::One(Mime::Jpeg))
        );
        assert_eq!(
            Mime::from_mimetype("image/gif"),
            Some(MimeLookup::One(Mime::AnimationGif))
        );
        assert!(matches!(
            Mime::from_mimetype("image"),
            Some(MimeLookup::Many(IMAGES))
        ));
        assert_eq!(
            Mime::from_nice_string("animated gif"),
            Some(Mime::AnimationGif)
        );
        assert_eq!(Mime::Jpeg.extension(), ".jpg");
        assert_eq!(Mime::Krita.extension(), ".kra");
        assert_eq!(Mime::AudioMkv.mimetype_str(), "audio/x-matroska");
    }

    #[test]
    fn service_helpers_work() {
        assert!(REAL_FILE_SERVICES.contains(&ServiceType::LocalFileDomain));
        assert!(ALL_TAG_SERVICES.contains(&ServiceType::CombinedTag));
        assert!(
            content_types_for_service(ServiceType::TagRepository)
                .unwrap()
                .contains(&ContentType::TagSiblings)
        );
        assert_eq!(
            permission_pair_description(ContentType::Files, Some(PermissionAction::Create)),
            Some("can upload new files and petition existing ones")
        );
    }

    #[test]
    fn general_mime_grouping_works() {
        assert_eq!(general_mimetype_for(Mime::Mp3), Some(Mime::GeneralAudio));
        assert_eq!(general_mimetype_for(Mime::Jpeg), Some(Mime::GeneralImage));
        assert_eq!(
            general_mimetype_for(Mime::Zip),
            Some(Mime::GeneralAppArchive)
        );
        assert_eq!(general_mimetype_for(Mime::Unknown), None);
    }

    #[test]
    fn extension_sets_are_populated() {
        assert!(IMAGE_FILE_EXTS.contains(".jpeg"));
        assert!(VIDEO_FILE_EXTS.contains(".webm"));
        assert!(allowed_mimes().contains(&Mime::UpdateContent));
    }
}
