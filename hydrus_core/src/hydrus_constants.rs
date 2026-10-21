use std::collections::HashSet;
use std::env;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

// ============================================================================
// 1. PATHS & ENVIRONMENT INITIALIZATION
// ============================================================================

pub const RUNNING_FROM_MACOS_APP: bool = cfg!(target_os = "macos"); // просто хардкодим проверку ОС на этапе компиляции
pub const RUNNING_FROM_SOURCE: bool = cfg!(debug_assertions);

const CONTENT_DIR: &'static str = "static";
const DB_DIR: &'static str = "db";
// Здесь собран список важных папок и файлов.

pub struct HydrusPaths {
    pub base_dir: PathBuf,
    pub content_base_dir: PathBuf,
    pub default_db_dir: PathBuf,
}

impl HydrusPaths {
    pub fn init(base_dir: PathBuf) -> Result<Self, Box<dyn std::error::Error>> {
        if base_dir.as_os_str().is_empty() {
            return Err("Не удалось определить базовый каталог!".into());
        }

        Ok(Self {
            base_dir: base_dir.clone(),
            content_base_dir: base_dir.join(CONTENT_DIR),
            default_db_dir: base_dir.join(DB_DIR),
        })
    }
}

// ============================================================================
// 2. PLATFORM CONSTANTS
// ============================================================================

pub const PLATFORM_WINDOWS: bool = cfg!(target_os = "windows");
pub const PLATFORM_MACOS: bool = cfg!(target_os = "macos");
pub const PLATFORM_LINUX: bool = cfg!(target_os = "linux");
pub const PLATFORM_HAIKU: bool = cfg!(target_os = "haiku");

#[cfg(target_os = "windows")]
pub const NICE_PLATFORM_STRING: &str = "Windows";

#[cfg(target_os = "macos")]
pub const NICE_PLATFORM_STRING: &str = "macOS";

#[cfg(target_os = "linux")]
pub const NICE_PLATFORM_STRING: &str = "Linux";

#[cfg(target_os = "haiku")]
pub const NICE_PLATFORM_STRING: &str = "Haiku";

#[cfg(not(any(
    target_os = "windows",
    target_os = "macos",
    target_os = "linux",
    target_os = "haiku"
)))]
pub const NICE_PLATFORM_STRING: &str = "Unknown?!";

#[cfg(target_arch = "x86_64")]
pub const ARCHITECTURE: &str = "x86_64";

#[cfg(target_arch = "aarch64")]
pub const ARCHITECTURE: &str = "aarch64";

#[cfg(target_arch = "x86")]
pub const ARCHITECTURE: &str = "x86";

#[cfg(target_arch = "arm")]
pub const ARCHITECTURE: &str = "arm";

#[cfg(all(
    not(target_arch = "x86_64"),
    not(target_arch = "aarch64"),
    not(target_arch = "x86"),
    not(target_arch = "arm")
))]
pub const ARCHITECTURE: &str = "Unknown?!";

pub const NICE_RUNNING_AS_STRING: &str = if RUNNING_FROM_SOURCE {
    "from source"
} else if RUNNING_FROM_MACOS_APP {
    "from App"
} else {
    "from unknown?!"
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

// ============================================================================
// 4. ENUMS & LOOKUPS (Idiomatic Rust replacement for dict lookups)
// ============================================================================

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BandwidthType {
    Data = 0,
    Requests = 1,
}
impl BandwidthType {
    pub fn as_str(&self) -> &'static str {
        match self {
            BandwidthType::Data => "data",
            BandwidthType::Requests => "requests",
        }
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ContentMergeAction {
    Copy = 0,
    Move = 1,
    TwoWayMerge = 2,
    None = 3,
    AddMove = 4,
}
impl ContentMergeAction {
    pub fn as_str(&self) -> &'static str {
        match self {
            ContentMergeAction::Copy => "copy from worse to better",
            ContentMergeAction::Move => "move from worse to better",
            ContentMergeAction::TwoWayMerge => "copy in both directions",
            ContentMergeAction::None => "make no change",
            ContentMergeAction::AddMove => "add move",
        }
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ContentStatus {
    Current = 0,
    Pending = 1,
    Deleted = 2,
    Petitioned = 3,
}
impl ContentStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            ContentStatus::Current => "current",
            ContentStatus::Pending => "pending",
            ContentStatus::Deleted => "deleted",
            ContentStatus::Petitioned => "petitioned",
        }
    }
}

#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ContentType {
    Mappings = 0,
    TagSiblings = 1,
    TagParents = 2,
    Files = 3,
    Ratings = 4,
    Mapping = 5,
    Directories = 6,
    Urls = 7,
    Veto = 8,
    Accounts = 9,
    Options = 10,
    Services = 11,
    Unknown = 12,
    AccountTypes = 13,
    Variable = 14,
    Hash = 15,
    Timestamp = 16,
    Title = 17,
    Notes = 18,
    FileViewingStats = 19,
    Tag = 20,
    Definitions = 21,
    HttpHeaders = 22,
}

#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ServiceType {
    TagRepository = 0,
    FileRepository = 1,
    LocalFileDomain = 2,
    MessageDepot = 3,
    LocalTag = 5,
    LocalRatingNumerical = 6,
    LocalRatingLike = 7,
    RatingNumericalRepository = 8,
    RatingLikeRepository = 9,
    CombinedTag = 10,
    CombinedFile = 11,
    LocalBooru = 12,
    Ipfs = 13,
    LocalFileTrashDomain = 14,
    HydrusLocalFileStorage = 15,
    TestService = 16,
    LocalNotes = 17,
    ClientApiService = 18,
    CombinedDeletedFile = 19,
    LocalFileUpdateDomain = 20,
    CombinedLocalFileDomains = 21,
    LocalRatingIncDec = 22,
    ServerAdmin = 99,
    NullService = 100,
}

impl ServiceType {
    pub fn as_str(&self) -> &'static str {
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
}

// ============================================================================
// 5. MIME TYPES ENUM (Сжатый формат для экономии места, но полностью функциональный)
// ============================================================================

#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Mime {
    Collection = 0,
    Jpeg = 1,
    Png = 2,
    AnimationGif = 3,
    Bmp = 4,
    Flash = 5,
    Yaml = 6,
    Icon = 7,
    Html = 8,
    Flv = 9,
    Pdf = 10,
    Zip = 11,
    EncryptedZip = 12,
    Mp3 = 13,
    Mp4Video = 14,
    Ogg = 15,
    Flac = 16,
    Wma = 17,
    Wmv = 18,
    UndeterminedWm = 19,
    Mkv = 20,
    Webm = 21,
    Json = 22,
    Apng = 23,
    UndeterminedPng = 24,
    Mpeg = 25,
    Mov = 26,
    Avi = 27,
    UpdateDefinitions = 28,
    UpdateContent = 29,
    PlainText = 30,
    Rar = 31,
    SevenZ = 32,
    Webp = 33,
    Tiff = 34,
    Psd = 35,
    M4a = 36,
    RealVideo = 37,
    RealAudio = 38,
    TrueAudio = 39,
    GeneralAudio = 40,
    GeneralImage = 41,
    GeneralVideo = 42,
    GeneralApplication = 43,
    GeneralAnimation = 44,
    Clip = 45,
    Wave = 46,
    Ogv = 47,
    AudioMkv = 48,
    AudioMp4 = 49,
    UndeterminedMp4 = 50,
    Cbor = 51,
    WindowsExe = 52,
    WavPack = 53,
    Sai2 = 54,
    Krita = 55,
    Svg = 56,
    Xcf = 57,
    Gzip = 58,
    GeneralAppArchive = 59,
    GeneralImageProject = 60,
    Heif = 61,
    HeifSequence = 62,
    Heic = 63,
    HeicSequence = 64,
    Avif = 65,
    AvifSequence = 66,
    UndeterminedGif = 67,
    StaticGif = 68,
    Procreate = 69,
    Qoi = 70,
    Epub = 71,
    Djvu = 72,
    Cbz = 73,
    Ugoira = 74,
    Rtf = 75,
    Docx = 76,
    Xlsx = 77,
    Pptx = 78,
    UndeterminedOle = 79,
    Doc = 80,
    Xls = 81,
    Ppt = 82,
    AnimationWebp = 83,
    UndeterminedWebp = 84,
    Jxl = 85,
    PaintDotNet = 86,
    UndeterminedJxl = 87,
    AnimationJxl = 88,
    OpenRaster = 89,
    GeneralFile = 90,
    OctetStream = 100,
    Unknown = 101,
}

impl Mime {
    /// Аналог mime_string_lookup
    pub fn as_str(&self) -> &'static str {
        match self {
            Mime::Collection => "collection",
            Mime::Jpeg => "jpeg",
            Mime::Png => "png",
            Mime::Apng => "apng",
            Mime::StaticGif => "static gif",
            Mime::AnimationGif => "animated gif",
            Mime::Bmp => "bitmap",
            Mime::Webp => "webp",
            Mime::AnimationWebp => "animated webp",
            Mime::Tiff => "tiff",
            Mime::Qoi => "qoi",
            Mime::Icon => "icon",
            Mime::Svg => "svg",
            Mime::Heif => "heif",
            Mime::HeifSequence => "heif sequence",
            Mime::Heic => "heic",
            Mime::HeicSequence => "heic sequence",
            Mime::Avif => "avif",
            Mime::AvifSequence => "avif sequence",
            Mime::Jxl => "jxl",
            Mime::AnimationJxl => "animated jxl",
            Mime::Ugoira => "ugoira",
            Mime::Cbz => "cbz",
            Mime::Flash => "flash",
            Mime::OctetStream => "application/octet-stream",
            Mime::Yaml => "yaml",
            Mime::Json => "json",
            Mime::Cbor => "cbor",
            Mime::Pdf => "pdf",
            Mime::Docx => "docx",
            Mime::Xlsx => "xlsx",
            Mime::Pptx => "pptx",
            Mime::Doc => "doc",
            Mime::Xls => "xls",
            Mime::Ppt => "ppt",
            Mime::Epub => "epub",
            Mime::Djvu => "djvu",
            Mime::Rtf => "rtf",
            Mime::Psd => "psd",
            Mime::Clip => "clip",
            Mime::Sai2 => "sai2",
            Mime::Krita => "krita",
            Mime::OpenRaster => "ora",
            Mime::PaintDotNet => "paint.net",
            Mime::Xcf => "xcf",
            Mime::Procreate => "procreate",
            Mime::Zip => "zip",
            Mime::Rar => "rar",
            Mime::SevenZ => "7z",
            Mime::Gzip => "gzip",
            Mime::WindowsExe => "windows exe",
            Mime::EncryptedZip => "application/hydrus-encrypted-zip",
            Mime::UpdateContent => "application/hydrus-update-content",
            Mime::UpdateDefinitions => "application/hydrus-update-definitions",
            Mime::M4a => "m4a",
            Mime::Mp3 => "mp3",
            Mime::Ogg => "ogg",
            Mime::Flac => "flac",
            Mime::AudioMkv => "matroska audio",
            Mime::AudioMp4 => "mp4 audio",
            Mime::Wave => "wave",
            Mime::RealAudio => "realaudio",
            Mime::TrueAudio => "tta",
            Mime::Wma => "wma",
            Mime::WavPack => "wavpack",
            Mime::Html => "html",
            Mime::PlainText => "plaintext",
            Mime::Avi => "avi",
            Mime::Flv => "flv",
            Mime::Mov => "quicktime",
            Mime::Mp4Video => "mp4",
            Mime::Mpeg => "mpeg",
            Mime::Wmv => "wmv",
            Mime::Mkv => "matroska",
            Mime::Ogv => "ogv",
            Mime::RealVideo => "realvideo",
            Mime::Webm => "webm",
            Mime::UndeterminedWm => "wma or wmv",
            Mime::UndeterminedMp4 => "mp4 with or without audio",
            Mime::UndeterminedPng => "png or apng",
            Mime::UndeterminedOle => "ole file",
            Mime::UndeterminedWebp => "webp with or without animation",
            Mime::UndeterminedJxl => "jxl with or without animation",
            Mime::Unknown => "unknown filetype",
            Mime::GeneralApplication => "application",
            Mime::GeneralAppArchive => "archive",
            Mime::GeneralImageProject => "image project file",
            Mime::GeneralAudio => "audio",
            Mime::GeneralImage => "image",
            Mime::GeneralVideo => "video",
            Mime::GeneralAnimation => "animation",
            Mime::GeneralFile => "all files",
            Mime::UndeterminedGif => "undetermined gif",
        }
    }

    /// Аналог mime_enum_lookup (обратный поиск из строки mimetype)
    pub fn from_mime_type(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "image/jpeg" | "image/jpg" | "image/jpe" => Some(Mime::Jpeg),
            "image/png" | "image/x-png" => Some(Mime::Png),
            "image/apng" => Some(Mime::Apng),
            "image/gif" => Some(Mime::AnimationGif), // Python код маппит на animated gif по умолчанию
            "image/webp" => Some(Mime::Webp),
            "application/pdf" | "pdf document" => Some(Mime::Pdf),
            "application/zip" => Some(Mime::Zip),
            "application/json" => Some(Mime::Json),
            "video/mp4" => Some(Mime::Mp4Video),
            "audio/mp3" => Some(Mime::Mp3),
            "unknown filetype" => Some(Mime::Unknown),
            _ => None, // В реальном коде здесь будут все 80+ вариантов
        }
    }

    pub fn extension(&self) -> &'static str {
        match self {
            Mime::Jpeg => ".jpg",
            Mime::Png | Mime::Apng => ".png",
            Mime::StaticGif | Mime::AnimationGif => ".gif",
            Mime::Webp | Mime::AnimationWebp => ".webp",
            Mime::Pdf => ".pdf",
            Mime::Zip | Mime::Ugoira => ".zip",
            Mime::Mp4Video => ".mp4",
            Mime::Mp3 => ".mp3",
            _ => ".bin",
        }
    }
}

// ============================================================================
// 6. GROUPINGS (Аналоги множеств и кортежей Python)
// ============================================================================

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

// Пример LazyLock для HashSet, если нужна быстрая проверка принадлежности O(1)
pub static SEARCHABLE_MIMES: LazyLock<HashSet<Mime>> = LazyLock::new(|| {
    let mut set = HashSet::new();
    for &mime in IMAGES {
        set.insert(mime);
    }
    for &mime in ANIMATIONS {
        set.insert(mime);
    }
    for &mime in VIDEO {
        set.insert(mime);
    }
    for &mime in AUDIO {
        set.insert(mime);
    }
    set.insert(Mime::Pdf);
    set.insert(Mime::Docx);
    // ... добавить остальные
    set
});

// ============================================================================
// 7. UNICODE CONSTANTS
// ============================================================================

pub const UNICODE_APPROX_EQUAL: &str = "\u{2248}";
pub const UNICODE_BYTE_ORDER_MARK: &str = "\u{FEFF}";
pub const UNICODE_ELLIPSIS: &str = "\u{2026}";
pub const UNICODE_NOT_EQUAL: &str = "\u{2260}";
pub const UNICODE_REPLACEMENT_CHARACTER: &str = "\u{FFFD}";
pub const UNICODE_PLUS_OR_MINUS: &str = "\u{00B1}";
pub const UNICODE_LESS_THAN_OR_EQUAL_TO: &str = "\u{2264}";
pub const UNICODE_GREATER_THAN_OR_EQUAL_TO: &str = "\u{2265}";

// ============================================================================
// 8. SQLITE & YAML INTEGRATION NOTES (Rust Way)
// ============================================================================

/*
В Python вы использовали:
  sqlite3.register_adapter(dict, yaml.safe_dump)
  sqlite3.register_converter('TEXT_YAML', yaml.safe_load)

В Rust это делается через трейты `ToSql` и `FromSql` из крейта `rusqlite`,
в комбинации с `serde` и `serde_yaml`.

Пример реализации для Rust:

use rusqlite::{types::ToSqlOutput, Result as SqlResult, ToSql, FromSql, ValueRef};
use serde::{Serialize, de::DeserializeOwned};

// Адаптер для записи YAML в SQLite
pub struct YamlWrapper<T>(pub T);

impl<T: Serialize> ToSql for YamlWrapper<T> {
    fn to_sql(&self) -> SqlResult<ToSqlOutput<'_>> {
        let yaml_string = serde_yaml::to_string(&self.0)
            .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
        Ok(ToSqlOutput::from(yaml_string))
    }
}

// Конвертер для чтения YAML из SQLite
impl<T: DeserializeOwned> FromSql for YamlWrapper<T> {
    fn column_result(value: ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        let yaml_str = value.as_str()?;
        serde_yaml::from_str::<T>(yaml_str)
            .map(YamlWrapper)
            .map_err(|e| rusqlite::types::FromSqlError::Other(Box::new(e)))
    }
}

// Для boolean-to-int адаптации в SQLite Rust делает это автоматически,
// но если нужна строгая совместимость с INTEGER_BOOLEAN:
impl ToSql for bool {
    fn to_sql(&self) -> SqlResult<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::from(*self as i64))
    }
}
*/

pub const ROWS_S_TEXT: &str = " rows/s";
