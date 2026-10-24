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

pub const SYSTEM_SORT_TYPES: [i32; 28] = [
    SORT_FILES_BY_FILESIZE,
    SORT_FILES_BY_DURATION,
    SORT_FILES_BY_IMPORT_TIME,
    SORT_FILES_BY_MIME,
    SORT_FILES_BY_RANDOM,
    SORT_FILES_BY_WIDTH,
    SORT_FILES_BY_HEIGHT,
    SORT_FILES_BY_RATIO,
    SORT_FILES_BY_NUM_PIXELS,
    SORT_FILES_BY_NUM_TAGS,
    SORT_FILES_BY_MEDIA_VIEWS,
    SORT_FILES_BY_MEDIA_VIEWTIME,
    SORT_FILES_BY_APPROX_BITRATE,
    SORT_FILES_BY_HAS_AUDIO,
    SORT_FILES_BY_FILE_MODIFIED_TIMESTAMP,
    SORT_FILES_BY_FRAMERATE,
    SORT_FILES_BY_NUM_FRAMES,
    SORT_FILES_BY_NUM_COLLECTION_FILES,
    SORT_FILES_BY_LAST_VIEWED_TIME,
    SORT_FILES_BY_ARCHIVED_TIMESTAMP,
    SORT_FILES_BY_HASH,
    SORT_FILES_BY_PIXEL_HASH,
    SORT_FILES_BY_BLURHASH,
    SORT_FILES_BY_AVERAGE_COLOUR_LIGHTNESS,
    SORT_FILES_BY_AVERAGE_COLOUR_CHROMATIC_MAGNITUDE,
    SORT_FILES_BY_AVERAGE_COLOUR_CHROMATICITY_GREEN_RED,
    SORT_FILES_BY_AVERAGE_COLOUR_CHROMATICITY_BLUE_YELLOW,
    SORT_FILES_BY_AVERAGE_COLOUR_HUE,
];

pub fn sort_type_string(sort_type: i32) -> &'static str {
    match sort_type {
        SORT_FILES_BY_FILESIZE => "file size",
        SORT_FILES_BY_DURATION => "duration",
        SORT_FILES_BY_IMPORT_TIME => "import time",
        SORT_FILES_BY_MIME => "filetype",
        SORT_FILES_BY_RANDOM => "random",
        SORT_FILES_BY_WIDTH => "width",
        SORT_FILES_BY_HEIGHT => "height",
        SORT_FILES_BY_RATIO => "ratio",
        SORT_FILES_BY_NUM_PIXELS => "pixels",
        SORT_FILES_BY_NUM_TAGS => "number of tags",
        SORT_FILES_BY_MEDIA_VIEWS => "media views",
        SORT_FILES_BY_MEDIA_VIEWTIME => "media viewtime",
        SORT_FILES_BY_APPROX_BITRATE => "approx bitrate",
        SORT_FILES_BY_HAS_AUDIO => "has audio",
        SORT_FILES_BY_FILE_MODIFIED_TIMESTAMP => "file modified time",
        SORT_FILES_BY_FRAMERATE => "framerate",
        SORT_FILES_BY_NUM_FRAMES => "number of frames",
        SORT_FILES_BY_NUM_COLLECTION_FILES => "number of collection files",
        SORT_FILES_BY_LAST_VIEWED_TIME => "last viewed time",
        SORT_FILES_BY_ARCHIVED_TIMESTAMP => "archived time",
        SORT_FILES_BY_HASH => "hash",
        SORT_FILES_BY_PIXEL_HASH => "pixel hash",
        SORT_FILES_BY_BLURHASH => "blurhash",
        SORT_FILES_BY_AVERAGE_COLOUR_LIGHTNESS => "average colour lightness",
        SORT_FILES_BY_AVERAGE_COLOUR_CHROMATIC_MAGNITUDE => "average colour chromatic magnitude",
        SORT_FILES_BY_AVERAGE_COLOUR_CHROMATICITY_GREEN_RED => {
            "average colour chromaticity green-red"
        }
        SORT_FILES_BY_AVERAGE_COLOUR_CHROMATICITY_BLUE_YELLOW => {
            "average colour chromaticity blue-yellow"
        }
        SORT_FILES_BY_AVERAGE_COLOUR_HUE => "average colour hue",
        _ => "unknown",
    }
}

pub fn sort_order_string(sort_order: i32) -> &'static str {
    match sort_order {
        SORT_ASC => "ascending",
        SORT_DESC => "descending",
        _ => "unknown",
    }
}

pub fn network_context_type_string(context_type: i32) -> &'static str {
    match context_type {
        NETWORK_CONTEXT_GLOBAL => "global",
        NETWORK_CONTEXT_HYDRUS => "hydrus",
        NETWORK_CONTEXT_DOMAIN => "domain",
        NETWORK_CONTEXT_DOWNLOADER => "downloader",
        NETWORK_CONTEXT_DOWNLOADER_PAGE => "downloader page",
        NETWORK_CONTEXT_SUBSCRIPTION => "subscription",
        NETWORK_CONTEXT_WATCHER_PAGE => "watcher page",
        _ => "unknown",
    }
}

pub fn page_state_string(page_state: i32) -> &'static str {
    match page_state {
        PAGE_STATE_NORMAL => "normal",
        PAGE_STATE_INITIALISING => "initialising",
        PAGE_STATE_SEARCHING => "searching",
        PAGE_STATE_SEARCHING_CANCELLED => "searching cancelled",
        _ => "unknown",
    }
}
