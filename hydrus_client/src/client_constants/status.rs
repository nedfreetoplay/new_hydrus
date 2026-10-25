//! Status and sort constants.

use std::fmt;

use super::*;

/// File import status, mirroring the `STATUS_*` values from `ClientConstants.py`.
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Status {
    Unknown = 0,
    SuccessfulAndNew = 1,
    SuccessfulButRedundant = 2,
    Deleted = 3,
    Error = 4,
    New = 5,
    Paused = 6,
}

impl Status {
    pub const ALL: &'static [Status] = &[
        Status::Unknown,
        Status::SuccessfulAndNew,
        Status::SuccessfulButRedundant,
        Status::Deleted,
        Status::Error,
        Status::New,
        Status::Paused,
    ];

    pub const fn as_int(self) -> i32 {
        self as i32
    }

    pub fn try_from_int(value: i32) -> Option<Self> {
        match value {
            0 => Some(Status::Unknown),
            1 => Some(Status::SuccessfulAndNew),
            2 => Some(Status::SuccessfulButRedundant),
            3 => Some(Status::Deleted),
            4 => Some(Status::Error),
            5 => Some(Status::New),
            6 => Some(Status::Paused),
            _ => None,
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Status::Unknown => "unknown",
            Status::SuccessfulAndNew => "successful and new",
            Status::SuccessfulButRedundant => "successful but redundant",
            Status::Deleted => "deleted",
            Status::Error => "error",
            Status::New => "new",
            Status::Paused => "paused",
        }
    }
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// File sort type, mirroring the `SORT_FILES_BY_*` values from `ClientConstants.py`.
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SortType {
    FileSize = 0,
    Duration = 1,
    ImportTime = 2,
    Mime = 3,
    Random = 4,
    Width = 5,
    Height = 6,
    Ratio = 7,
    NumPixels = 8,
    NumTags = 9,
    MediaViews = 10,
    MediaViewtime = 11,
    ApproxBitrate = 12,
    HasAudio = 13,
    FileModifiedTimestamp = 14,
    Framerate = 15,
    NumFrames = 16,
    NumCollectionFiles = 17,
    LastViewedTime = 18,
    ArchivedTimestamp = 19,
    Hash = 20,
    PixelHash = 21,
    Blurhash = 22,
    AverageColourLightness = 23,
    AverageColourChromaticMagnitude = 24,
    AverageColourChromaticityGreenRed = 25,
    AverageColourChromaticityBlueYellow = 26,
    AverageColourHue = 27,
}

impl SortType {
    pub const ALL: &'static [SortType] = &[
        SortType::FileSize,
        SortType::Duration,
        SortType::ImportTime,
        SortType::Mime,
        SortType::Random,
        SortType::Width,
        SortType::Height,
        SortType::Ratio,
        SortType::NumPixels,
        SortType::NumTags,
        SortType::MediaViews,
        SortType::MediaViewtime,
        SortType::ApproxBitrate,
        SortType::HasAudio,
        SortType::FileModifiedTimestamp,
        SortType::Framerate,
        SortType::NumFrames,
        SortType::NumCollectionFiles,
        SortType::LastViewedTime,
        SortType::ArchivedTimestamp,
        SortType::Hash,
        SortType::PixelHash,
        SortType::Blurhash,
        SortType::AverageColourLightness,
        SortType::AverageColourChromaticMagnitude,
        SortType::AverageColourChromaticityGreenRed,
        SortType::AverageColourChromaticityBlueYellow,
        SortType::AverageColourHue,
    ];

    pub const fn as_int(self) -> i32 {
        self as i32
    }

    pub fn try_from_int(value: i32) -> Option<Self> {
        match value {
            0 => Some(SortType::FileSize),
            1 => Some(SortType::Duration),
            2 => Some(SortType::ImportTime),
            3 => Some(SortType::Mime),
            4 => Some(SortType::Random),
            5 => Some(SortType::Width),
            6 => Some(SortType::Height),
            7 => Some(SortType::Ratio),
            8 => Some(SortType::NumPixels),
            9 => Some(SortType::NumTags),
            10 => Some(SortType::MediaViews),
            11 => Some(SortType::MediaViewtime),
            12 => Some(SortType::ApproxBitrate),
            13 => Some(SortType::HasAudio),
            14 => Some(SortType::FileModifiedTimestamp),
            15 => Some(SortType::Framerate),
            16 => Some(SortType::NumFrames),
            17 => Some(SortType::NumCollectionFiles),
            18 => Some(SortType::LastViewedTime),
            19 => Some(SortType::ArchivedTimestamp),
            20 => Some(SortType::Hash),
            21 => Some(SortType::PixelHash),
            22 => Some(SortType::Blurhash),
            23 => Some(SortType::AverageColourLightness),
            24 => Some(SortType::AverageColourChromaticMagnitude),
            25 => Some(SortType::AverageColourChromaticityGreenRed),
            26 => Some(SortType::AverageColourChromaticityBlueYellow),
            27 => Some(SortType::AverageColourHue),
            _ => None,
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            SortType::FileSize => "file size",
            SortType::Duration => "duration",
            SortType::ImportTime => "import time",
            SortType::Mime => "filetype",
            SortType::Random => "random",
            SortType::Width => "width",
            SortType::Height => "height",
            SortType::Ratio => "ratio",
            SortType::NumPixels => "pixels",
            SortType::NumTags => "number of tags",
            SortType::MediaViews => "media views",
            SortType::MediaViewtime => "media viewtime",
            SortType::ApproxBitrate => "approx bitrate",
            SortType::HasAudio => "has audio",
            SortType::FileModifiedTimestamp => "file modified time",
            SortType::Framerate => "framerate",
            SortType::NumFrames => "number of frames",
            SortType::NumCollectionFiles => "number of collection files",
            SortType::LastViewedTime => "last viewed time",
            SortType::ArchivedTimestamp => "archived time",
            SortType::Hash => "hash",
            SortType::PixelHash => "pixel hash",
            SortType::Blurhash => "blurhash",
            SortType::AverageColourLightness => "average colour lightness",
            SortType::AverageColourChromaticMagnitude => "average colour chromatic magnitude",
            SortType::AverageColourChromaticityGreenRed => "average colour chromaticity green-red",
            SortType::AverageColourChromaticityBlueYellow => {
                "average colour chromaticity blue-yellow"
            }
            SortType::AverageColourHue => "average colour hue",
        }
    }
}

impl fmt::Display for SortType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Sort direction, mirroring the `SORT_ASC`/`SORT_DESC` values from `ClientConstants.py`.
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SortOrder {
    Ascending = 0,
    Descending = 1,
}

impl SortOrder {
    pub const ALL: &'static [SortOrder] = &[SortOrder::Ascending, SortOrder::Descending];

    pub const fn as_int(self) -> i32 {
        self as i32
    }

    pub fn try_from_int(value: i32) -> Option<Self> {
        match value {
            0 => Some(SortOrder::Ascending),
            1 => Some(SortOrder::Descending),
            _ => None,
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            SortOrder::Ascending => "ascending",
            SortOrder::Descending => "descending",
        }
    }
}

impl fmt::Display for SortOrder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Page state, mirroring the `PAGE_STATE_*` values from `ClientConstants.py`.
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum PageState {
    Normal = 0,
    Initialising = 1,
    Searching = 2,
    SearchingCancelled = 3,
}

impl PageState {
    pub const ALL: &'static [PageState] = &[
        PageState::Normal,
        PageState::Initialising,
        PageState::Searching,
        PageState::SearchingCancelled,
    ];

    pub const fn as_int(self) -> i32 {
        self as i32
    }

    pub fn try_from_int(value: i32) -> Option<Self> {
        match value {
            0 => Some(PageState::Normal),
            1 => Some(PageState::Initialising),
            2 => Some(PageState::Searching),
            3 => Some(PageState::SearchingCancelled),
            _ => None,
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            PageState::Normal => "normal",
            PageState::Initialising => "initialising",
            PageState::Searching => "searching",
            PageState::SearchingCancelled => "searching cancelled",
        }
    }
}

impl fmt::Display for PageState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Network context type, mirroring the `NETWORK_CONTEXT_*` values from `ClientConstants.py`.
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum NetworkContextType {
    Global = 0,
    Hydrus = 1,
    Domain = 2,
    Downloader = 3,
    DownloaderPage = 4,
    Subscription = 5,
    WatcherPage = 6,
}

impl NetworkContextType {
    pub const ALL: &'static [NetworkContextType] = &[
        NetworkContextType::Global,
        NetworkContextType::Hydrus,
        NetworkContextType::Domain,
        NetworkContextType::Downloader,
        NetworkContextType::DownloaderPage,
        NetworkContextType::Subscription,
        NetworkContextType::WatcherPage,
    ];

    pub const fn as_int(self) -> i32 {
        self as i32
    }

    pub fn try_from_int(value: i32) -> Option<Self> {
        match value {
            0 => Some(NetworkContextType::Global),
            1 => Some(NetworkContextType::Hydrus),
            2 => Some(NetworkContextType::Domain),
            3 => Some(NetworkContextType::Downloader),
            4 => Some(NetworkContextType::DownloaderPage),
            5 => Some(NetworkContextType::Subscription),
            6 => Some(NetworkContextType::WatcherPage),
            _ => None,
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            NetworkContextType::Global => "global",
            NetworkContextType::Hydrus => "hydrus",
            NetworkContextType::Domain => "domain",
            NetworkContextType::Downloader => "downloader",
            NetworkContextType::DownloaderPage => "downloader page",
            NetworkContextType::Subscription => "subscription",
            NetworkContextType::WatcherPage => "watcher page",
        }
    }
}

impl fmt::Display for NetworkContextType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}
