//! Generated simple-command display names.

use crate::client_application_command_full::*;

pub fn simple_command_name(command_type: i32) -> &'static str {
    match command_type {
        SIMPLE_ARCHIVE_DELETE_FILTER_BACK => "archive delete filter back",
        SIMPLE_ARCHIVE_DELETE_FILTER_DELETE => "archive delete filter delete",
        SIMPLE_ARCHIVE_DELETE_FILTER_KEEP => "archive delete filter keep",
        SIMPLE_ARCHIVE_DELETE_FILTER_SKIP => "archive delete filter skip",
        SIMPLE_ARCHIVE_FILE => "archive file",
        SIMPLE_CHECK_ALL_IMPORT_FOLDERS => "check all import folders",
        SIMPLE_CLOSE_MEDIA_VIEWER => "close media viewer",
        SIMPLE_CLOSE_PAGE => "close page",
        SIMPLE_COPY_FILES => "copy files",
        SIMPLE_COPY_FILE_PATHS => "copy file paths",
        SIMPLE_DELETE_FILE => "delete file",
        SIMPLE_DUPLICATE_FILTER_ALTERNATES => "duplicate filter alternates",
        SIMPLE_DUPLICATE_FILTER_BACK => "duplicate filter back",
        SIMPLE_DUPLICATE_FILTER_CUSTOM_ACTION => "duplicate filter custom action",
        SIMPLE_DUPLICATE_FILTER_EXACTLY_THE_SAME => "duplicate filter exactly the same",
        SIMPLE_DUPLICATE_FILTER_FALSE_POSITIVE => "duplicate filter false positive",
        SIMPLE_DUPLICATE_FILTER_SKIP => "duplicate filter skip",
        SIMPLE_DUPLICATE_FILTER_THIS_IS_BETTER_AND_DELETE_OTHER => {
            "duplicate filter this is better and delete other"
        }
        SIMPLE_DUPLICATE_FILTER_THIS_IS_BETTER_BUT_KEEP_BOTH => {
            "duplicate filter this is better but keep both"
        }
        SIMPLE_DUPLICATE_MEDIA_SET_ALTERNATE => "duplicate media set alternate",
        SIMPLE_DUPLICATE_MEDIA_SET_ALTERNATE_COLLECTIONS => {
            "duplicate media set alternate collections"
        }
        SIMPLE_DUPLICATE_MEDIA_SET_CUSTOM => "duplicate media set custom",
        SIMPLE_DUPLICATE_MEDIA_SET_FOCUSED_BETTER => "duplicate media set focused better",
        SIMPLE_DUPLICATE_MEDIA_SET_FOCUSED_KING => "duplicate media set focused king",
        SIMPLE_DUPLICATE_MEDIA_SET_SAME_QUALITY => "duplicate media set same quality",
        SIMPLE_EXIT_APPLICATION => "exit application",
        SIMPLE_EXPORT_FILES => "export files",
        SIMPLE_EXPORT_FILES_QUICK_AUTO_EXPORT => "export files quick auto export",
        SIMPLE_FLIP_DARKMODE => "flip darkmode",
        SIMPLE_FLIP_DEBUG_FORCE_IDLE_MODE_DO_NOT_SET_THIS => {
            "flip debug force idle mode do not set this"
        }
        SIMPLE_FOCUS_MEDIA_VIEWER => "focus media viewer",
        SIMPLE_GLOBAL_AUDIO_MUTE => "global audio mute",
        SIMPLE_GLOBAL_AUDIO_MUTE_FLIP => "global audio mute flip",
        SIMPLE_GLOBAL_AUDIO_UNMUTE => "global audio unmute",
        SIMPLE_HIDE_TO_SYSTEM_TRAY => "hide to system tray",
        SIMPLE_INBOX_FILE => "inbox file",
        SIMPLE_LAUNCH_MEDIA_VIEWER => "launch media viewer",
        SIMPLE_LAUNCH_THE_ARCHIVE_DELETE_FILTER => "launch the archive delete filter",
        SIMPLE_MANAGE_FILE_NOTES => "manage file notes",
        SIMPLE_MANAGE_FILE_RATINGS => "manage file ratings",
        SIMPLE_MANAGE_FILE_TAGS => "manage file tags",
        SIMPLE_MANAGE_FILE_URLS => "manage file urls",
        SIMPLE_MOVE_ANIMATION_TO_NEXT_FRAME => "move animation to next frame",
        SIMPLE_MOVE_ANIMATION_TO_PREVIOUS_FRAME => "move animation to previous frame",
        SIMPLE_NEW_DUPLICATE_FILTER_PAGE => "new duplicate filter page",
        SIMPLE_NEW_GALLERY_DOWNLOADER_PAGE => "new gallery downloader page",
        SIMPLE_NEW_PAGE => "new page",
        SIMPLE_NEW_PAGE_OF_PAGES => "new page of pages",
        SIMPLE_NEW_SIMPLE_DOWNLOADER_PAGE => "new simple downloader page",
        SIMPLE_NEW_URL_DOWNLOADER_PAGE => "new url downloader page",
        SIMPLE_NEW_WATCHER_DOWNLOADER_PAGE => "new watcher downloader page",
        SIMPLE_OPEN_FILE_IN_EXTERNAL_PROGRAM => "open file in external program",
        SIMPLE_OPEN_KNOWN_URL => "open known url",
        SIMPLE_OPEN_SELECTION_IN_NEW_PAGE => "open selection in new page",
        SIMPLE_PAN_BOTTOM_EDGE => "pan bottom edge",
        SIMPLE_PAN_DOWN => "pan down",
        SIMPLE_PAN_HORIZONTAL_CENTER => "pan horizontal center",
        SIMPLE_PAN_LEFT => "pan left",
        SIMPLE_PAN_LEFT_EDGE => "pan left edge",
        SIMPLE_PAN_RIGHT => "pan right",
        SIMPLE_PAN_RIGHT_EDGE => "pan right edge",
        SIMPLE_PAN_TOP_EDGE => "pan top edge",
        SIMPLE_PAN_UP => "pan up",
        SIMPLE_PAN_VERTICAL_CENTER => "pan vertical center",
        SIMPLE_PAUSE_MEDIA => "pause media",
        SIMPLE_PAUSE_PLAY_MEDIA => "pause play media",
        SIMPLE_PAUSE_PLAY_SLIDESHOW => "pause play slideshow",
        SIMPLE_REDO => "redo",
        SIMPLE_REFRESH => "refresh",
        SIMPLE_REFRESH_ALL_PAGES => "refresh all pages",
        _ => "unknown command",
    }
}
