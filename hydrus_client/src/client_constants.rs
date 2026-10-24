//! Client-side constants mirrored from `hydrus/client/ClientConstants.py`.
//!
//! The Python module exposes loose integers; the Rust port keeps the same
//! values and adds small lookup helpers so presentation code does not hardcode
//! strings.

pub const CANVAS_MEDIA_VIEWER: i32 = 0;
pub const CANVAS_PREVIEW: i32 = 1;
pub const CANVAS_MEDIA_VIEWER_DUPLICATES: i32 = 2;
pub const CANVAS_MEDIA_VIEWER_ARCHIVE_DELETE: i32 = 3;
pub const CANVAS_CLIENT_API: i32 = 4;
pub const CANVAS_DIALOG: i32 = 5;

pub const CANVAS_MEDIA_VIEWER_TYPES: [i32; 3] = [
    CANVAS_MEDIA_VIEWER,
    CANVAS_MEDIA_VIEWER_DUPLICATES,
    CANVAS_MEDIA_VIEWER_ARCHIVE_DELETE,
];

pub const COLOUR_THUMB_BACKGROUND: i32 = 0;
pub const COLOUR_THUMB_BACKGROUND_SELECTED: i32 = 1;
pub const COLOUR_THUMB_BACKGROUND_REMOTE: i32 = 2;
pub const COLOUR_THUMB_BACKGROUND_REMOTE_SELECTED: i32 = 3;
pub const COLOUR_THUMB_BORDER: i32 = 4;
pub const COLOUR_THUMB_BORDER_SELECTED: i32 = 5;
pub const COLOUR_THUMB_BORDER_REMOTE: i32 = 6;
pub const COLOUR_THUMB_BORDER_REMOTE_SELECTED: i32 = 7;
pub const COLOUR_THUMBGRID_BACKGROUND: i32 = 8;
pub const COLOUR_AUTOCOMPLETE_BACKGROUND: i32 = 9;
pub const COLOUR_MEDIA_BACKGROUND: i32 = 10;
pub const COLOUR_MEDIA_TEXT: i32 = 11;
pub const COLOUR_TAGS_BOX: i32 = 12;

pub const COMMAND_PALETTE_PROVIDER_CALCULATOR: i32 = 0;
pub const COMMAND_PALETTE_PROVIDER_MAIN_MENU: i32 = 1;
pub const COMMAND_PALETTE_PROVIDER_MEDIA_MENU: i32 = 2;
pub const COMMAND_PALETTE_PROVIDER_PAGES: i32 = 3;
pub const COMMAND_PALETTE_PROVIDER_PAGES_HISTORY: i32 = 4;
pub const COMMAND_PALETTE_PROVIDER_FAVOURITE_SEARCH: i32 = 5;

pub const DISCRIMINANT_INBOX: i32 = 0;
pub const DISCRIMINANT_LOCAL: i32 = 1;
pub const DISCRIMINANT_NOT_LOCAL: i32 = 2;
pub const DISCRIMINANT_ARCHIVE: i32 = 3;
pub const DISCRIMINANT_DOWNLOADING: i32 = 4;
pub const DISCRIMINANT_LOCAL_BUT_NOT_IN_TRASH: i32 = 5;

pub const DIRECTION_UP: i32 = 0;
pub const DIRECTION_LEFT: i32 = 1;
pub const DIRECTION_RIGHT: i32 = 2;
pub const DIRECTION_DOWN: i32 = 3;

pub const FIELD_VERIFICATION_RECAPTCHA: i32 = 0;
pub const FIELD_COMMENT: i32 = 1;
pub const FIELD_TEXT: i32 = 2;
pub const FIELD_CHECKBOX: i32 = 3;
pub const FIELD_FILE: i32 = 4;
pub const FIELD_THREAD_ID: i32 = 5;
pub const FIELD_PASSWORD: i32 = 6;
pub const FIELDS: [i32; 7] = [
    FIELD_VERIFICATION_RECAPTCHA,
    FIELD_COMMENT,
    FIELD_TEXT,
    FIELD_CHECKBOX,
    FIELD_FILE,
    FIELD_THREAD_ID,
    FIELD_PASSWORD,
];

pub const FILE_VIEWING_STATS_MENU_DISPLAY_SUMMED_AND_THEN_SUBMENU: i32 = 2;
pub const FILE_VIEWING_STATS_MENU_DISPLAY_STACKED: i32 = 3;

pub const FLAGS_NONE: i32 = 0;
pub const FLAGS_CENTER: i32 = 3;
pub const FLAGS_EXPAND_PERPENDICULAR: i32 = 4;
pub const FLAGS_EXPAND_BOTH_WAYS: i32 = 5;
pub const FLAGS_EXPAND_BOTH_WAYS_POLITE: i32 = 7;
pub const FLAGS_EXPAND_BOTH_WAYS_SHY: i32 = 8;
pub const FLAGS_EXPAND_SIZER_PERPENDICULAR: i32 = 10;
pub const FLAGS_EXPAND_SIZER_BOTH_WAYS: i32 = 11;
pub const FLAGS_ON_LEFT: i32 = 12;
pub const FLAGS_ON_RIGHT: i32 = 13;
pub const FLAGS_CENTER_PERPENDICULAR: i32 = 15;
pub const FLAGS_SIZER_CENTER: i32 = 16;
pub const FLAGS_CENTER_PERPENDICULAR_EXPAND_DEPTH: i32 = 17;

pub const DAY: i32 = 0;
pub const WEEK: i32 = 1;
pub const MONTH: i32 = 2;

pub const RESTRICTION_MIN_RESOLUTION: i32 = 0;
pub const RESTRICTION_MAX_RESOLUTION: i32 = 1;
pub const RESTRICTION_MAX_FILE_SIZE: i32 = 2;
pub const RESTRICTION_ALLOWED_MIMES: i32 = 3;

pub const HAMMING_EXACT_MATCH: i32 = 0;
pub const HAMMING_VERY_SIMILAR: i32 = 2;
pub const HAMMING_SIMILAR: i32 = 4;
pub const HAMMING_SPECULATIVE: i32 = 8;

pub const IDLE_NOT_ON_SHUTDOWN: i32 = 0;
pub const IDLE_ON_SHUTDOWN: i32 = 1;
pub const IDLE_ON_SHUTDOWN_ASK_FIRST: i32 = 2;

pub const IMPORT_FOLDER_DELETE: i32 = 0;
pub const IMPORT_FOLDER_IGNORE: i32 = 1;
pub const IMPORT_FOLDER_MOVE: i32 = 2;

pub const EXIT_SESSION_SESSION_NAME: &str = "exit session";
pub const LAST_SESSION_SESSION_NAME: &str = "last session";

pub const MEDIA_VIEWER_ACTION_SHOW_WITH_NATIVE: i32 = 0;
pub const MEDIA_VIEWER_ACTION_SHOW_WITH_NATIVE_PAUSED: i32 = 1;
pub const MEDIA_VIEWER_ACTION_SHOW_BEHIND_EMBED: i32 = 2;
pub const MEDIA_VIEWER_ACTION_SHOW_BEHIND_EMBED_PAUSED: i32 = 3;
pub const MEDIA_VIEWER_ACTION_SHOW_OPEN_EXTERNALLY_BUTTON: i32 = 4;
pub const MEDIA_VIEWER_ACTION_DO_NOT_SHOW_ON_ACTIVATION_OPEN_EXTERNALLY: i32 = 5;
pub const MEDIA_VIEWER_ACTION_DO_NOT_SHOW: i32 = 6;
pub const MEDIA_VIEWER_ACTION_SHOW_WITH_MPV: i32 = 7;
pub const MEDIA_VIEWER_ACTION_SHOW_WITH_QMEDIAPLAYER_VIDEO_WIDGET: i32 = 8;
pub const MEDIA_VIEWER_ACTION_SHOW_WITH_QTMEDIAPLAYER: i32 = 9;

pub const MEDIA_VIEWER_SCALE_100: i32 = 0;
pub const MEDIA_VIEWER_SCALE_MAX_REGULAR: i32 = 1;
pub const MEDIA_VIEWER_SCALE_TO_CANVAS: i32 = 2;

pub const MEDIA_VIEWER_TAGS_SCROLLING_BEHAVIOUR_NEVER_PROPAGATE: i32 = 0;
pub const MEDIA_VIEWER_TAGS_SCROLLING_BEHAVIOUR_ONLY_PROPAGATE_ON_NO_SCROLLBAR: i32 = 1;
pub const MEDIA_VIEWER_TAGS_SCROLLING_BEHAVIOUR_ONLY_PROPAGATE_AFTER_DELAY: i32 = 2;
pub const MEDIA_VIEWER_TAGS_SCROLLING_BEHAVIOUR_QT_DEFAULT: i32 = 3;

pub const NEW_PAGE_GOES_FAR_LEFT: i32 = 0;
pub const NEW_PAGE_GOES_LEFT_OF_CURRENT: i32 = 1;
pub const NEW_PAGE_GOES_RIGHT_OF_CURRENT: i32 = 2;
pub const NEW_PAGE_GOES_FAR_RIGHT: i32 = 3;

pub const CLOSED_PAGE_FOCUS_GOES_LEFT: i32 = 0;
pub const CLOSED_PAGE_FOCUS_GOES_RIGHT: i32 = 1;

pub const NETWORK_CONTEXT_GLOBAL: i32 = 0;
pub const NETWORK_CONTEXT_HYDRUS: i32 = 1;
pub const NETWORK_CONTEXT_DOMAIN: i32 = 2;
pub const NETWORK_CONTEXT_DOWNLOADER: i32 = 3;
pub const NETWORK_CONTEXT_DOWNLOADER_PAGE: i32 = 4;
pub const NETWORK_CONTEXT_SUBSCRIPTION: i32 = 5;
pub const NETWORK_CONTEXT_WATCHER_PAGE: i32 = 6;

pub const PAGE_FILE_COUNT_DISPLAY_ALL: i32 = 0;
pub const PAGE_FILE_COUNT_DISPLAY_NONE: i32 = 1;
pub const PAGE_FILE_COUNT_DISPLAY_ONLY_IMPORTERS: i32 = 2;
pub const PAGE_FILE_COUNT_DISPLAY_ALL_BUT_ONLY_IF_GREATER_THAN_ZERO: i32 = 3;

pub const PAGE_STATE_NORMAL: i32 = 0;
pub const PAGE_STATE_INITIALISING: i32 = 1;
pub const PAGE_STATE_SEARCHING: i32 = 2;
pub const PAGE_STATE_SEARCHING_CANCELLED: i32 = 3;

pub const SHUTDOWN_TIMESTAMP_VACUUM: i32 = 0;
pub const SHUTDOWN_TIMESTAMP_FATTEN_AC_CACHE: i32 = 1;
pub const SHUTDOWN_TIMESTAMP_DELETE_ORPHANS: i32 = 2;

pub const SORT_FILES_BY_FILESIZE: i32 = 0;
pub const SORT_FILES_BY_DURATION: i32 = 1;
pub const SORT_FILES_BY_IMPORT_TIME: i32 = 2;
pub const SORT_FILES_BY_MIME: i32 = 3;
pub const SORT_FILES_BY_RANDOM: i32 = 4;
pub const SORT_FILES_BY_WIDTH: i32 = 5;
pub const SORT_FILES_BY_HEIGHT: i32 = 6;
pub const SORT_FILES_BY_RATIO: i32 = 7;
pub const SORT_FILES_BY_NUM_PIXELS: i32 = 8;
pub const SORT_FILES_BY_NUM_TAGS: i32 = 9;
pub const SORT_FILES_BY_MEDIA_VIEWS: i32 = 10;
pub const SORT_FILES_BY_MEDIA_VIEWTIME: i32 = 11;
pub const SORT_FILES_BY_APPROX_BITRATE: i32 = 12;
pub const SORT_FILES_BY_HAS_AUDIO: i32 = 13;
pub const SORT_FILES_BY_FILE_MODIFIED_TIMESTAMP: i32 = 14;
pub const SORT_FILES_BY_FRAMERATE: i32 = 15;
pub const SORT_FILES_BY_NUM_FRAMES: i32 = 16;
pub const SORT_FILES_BY_NUM_COLLECTION_FILES: i32 = 17;
pub const SORT_FILES_BY_LAST_VIEWED_TIME: i32 = 18;
pub const SORT_FILES_BY_ARCHIVED_TIMESTAMP: i32 = 19;
pub const SORT_FILES_BY_HASH: i32 = 20;
pub const SORT_FILES_BY_PIXEL_HASH: i32 = 21;
pub const SORT_FILES_BY_BLURHASH: i32 = 22;
pub const SORT_FILES_BY_AVERAGE_COLOUR_LIGHTNESS: i32 = 23;
pub const SORT_FILES_BY_AVERAGE_COLOUR_CHROMATIC_MAGNITUDE: i32 = 24;
pub const SORT_FILES_BY_AVERAGE_COLOUR_CHROMATICITY_GREEN_RED: i32 = 25;
pub const SORT_FILES_BY_AVERAGE_COLOUR_CHROMATICITY_BLUE_YELLOW: i32 = 26;
pub const SORT_FILES_BY_AVERAGE_COLOUR_HUE: i32 = 27;

pub const SORT_ASC: i32 = 0;
pub const SORT_DESC: i32 = 1;

pub const STATUS_UNKNOWN: i32 = 0;
pub const STATUS_SUCCESSFUL_AND_NEW: i32 = 1;
pub const STATUS_SUCCESSFUL_BUT_REDUNDANT: i32 = 2;
pub const STATUS_DELETED: i32 = 3;
pub const STATUS_ERROR: i32 = 4;
pub const STATUS_NEW: i32 = 5;
pub const STATUS_PAUSED: i32 = 6;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sort_type_lookup_covers_every_system_sort() {
        for sort_type in SYSTEM_SORT_TYPES {
            assert_ne!(sort_type_string(sort_type), "unknown");
        }
    }

    #[test]
    fn media_viewer_canvas_types_match_python() {
        assert_eq!(
            CANVAS_MEDIA_VIEWER_TYPES,
            [0, 2, 3],
            "Python set is {{0, 2, 3}}"
        );
    }
}
