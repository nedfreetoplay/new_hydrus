//! Generated simple-command display names.

use crate::client_application_command_full::*;

pub fn simple_command_name(command_type: i32) -> &'static str {
    match command_type {
        SIMPLE_REFRESH_PAGE_OF_PAGES_PAGES => "refresh page of pages pages",
        SIMPLE_REMOVE_FILE_FROM_VIEW => "remove file from view",
        SIMPLE_RESTART_APPLICATION => "restart application",
        SIMPLE_SET_MEDIA_FOCUS => "set media focus",
        SIMPLE_SET_SEARCH_FOCUS => "set search focus",
        SIMPLE_SHOW_AND_FOCUS_MANAGE_TAGS_FAVOURITE_TAGS => {
            "show and focus manage tags favourite tags"
        }
        SIMPLE_SHOW_AND_FOCUS_MANAGE_TAGS_FILE_LOOKUP_SCRIPT_TAGS => {
            "show and focus manage tags file lookup script tags"
        }
        SIMPLE_SHOW_AND_FOCUS_MANAGE_TAGS_RECENT_TAGS => "show and focus manage tags recent tags",
        SIMPLE_SHOW_AND_FOCUS_MANAGE_TAGS_RELATED_TAGS => "show and focus manage tags related tags",
        SIMPLE_SHOW_HIDE_SPLITTERS => "show hide splitters",
        SIMPLE_SHOW_MENU => "show menu",
        SIMPLE_SWITCH_BETWEEN_100_PERCENT_AND_CANVAS_ZOOM => {
            "switch between 100 percent and canvas zoom"
        }
        SIMPLE_SWITCH_BETWEEN_FULLSCREEN_BORDERLESS_AND_REGULAR_FRAMED_WINDOW => {
            "switch between fullscreen borderless and regular framed window"
        }
        SIMPLE_SYNCHRONISED_WAIT_SWITCH => "synchronised wait switch",
        SIMPLE_UNCLOSE_PAGE => "unclose page",
        SIMPLE_UNDELETE_FILE => "undelete file",
        SIMPLE_UNDO => "undo",
        SIMPLE_VIEW_FIRST => "view first",
        SIMPLE_VIEW_LAST => "view last",
        SIMPLE_VIEW_NEXT => "view next",
        SIMPLE_VIEW_PREVIOUS => "view previous",
        SIMPLE_ZOOM_IN => "zoom in",
        SIMPLE_ZOOM_OUT => "zoom out",
        SIMPLE_EXIT_APPLICATION_FORCE_MAINTENANCE => "exit application force maintenance",
        SIMPLE_ZOOM_IN_VIEWER_CENTER => "zoom in viewer center",
        SIMPLE_ZOOM_OUT_VIEWER_CENTER => "zoom out viewer center",
        SIMPLE_SWITCH_BETWEEN_100_PERCENT_AND_CANVAS_ZOOM_VIEWER_CENTER => {
            "switch between 100 percent and canvas zoom viewer center"
        }
        SIMPLE_DUPLICATE_MEDIA_DISSOLVE_FOCUSED_ALTERNATE_GROUP => {
            "duplicate media dissolve focused alternate group"
        }
        SIMPLE_DUPLICATE_MEDIA_DISSOLVE_ALTERNATE_GROUP => {
            "duplicate media dissolve alternate group"
        }
        SIMPLE_DUPLICATE_MEDIA_DISSOLVE_FOCUSED_DUPLICATE_GROUP => {
            "duplicate media dissolve focused duplicate group"
        }
        SIMPLE_DUPLICATE_MEDIA_DISSOLVE_DUPLICATE_GROUP => {
            "duplicate media dissolve duplicate group"
        }
        SIMPLE_DUPLICATE_MEDIA_REMOVE_FOCUSED_FROM_ALTERNATE_GROUP => {
            "duplicate media remove focused from alternate group"
        }
        SIMPLE_DUPLICATE_MEDIA_REMOVE_FOCUSED_FROM_DUPLICATE_GROUP => {
            "duplicate media remove focused from duplicate group"
        }
        SIMPLE_DUPLICATE_MEDIA_RESET_FOCUSED_POTENTIAL_SEARCH => {
            "duplicate media reset focused potential search"
        }
        SIMPLE_DUPLICATE_MEDIA_RESET_POTENTIAL_SEARCH => "duplicate media reset potential search",
        SIMPLE_DUPLICATE_MEDIA_REMOVE_FOCUSED_POTENTIALS => {
            "duplicate media remove focused potentials"
        }
        SIMPLE_DUPLICATE_MEDIA_REMOVE_POTENTIALS => "duplicate media remove potentials",
        SIMPLE_DUPLICATE_MEDIA_SET_POTENTIAL => "duplicate media set potential",
        SIMPLE_DUPLICATE_MEDIA_CLEAR_ALL_FALSE_POSITIVES => {
            "duplicate media clear all false positives"
        }
        SIMPLE_DUPLICATE_MEDIA_CLEAR_ALL_FOCUSED_FALSE_POSITIVES => {
            "duplicate media clear all focused false positives"
        }
        SIMPLE_RUN_ALL_EXPORT_FOLDERS => "run all export folders",
        SIMPLE_MOVE_PAGES_SELECTION_LEFT => "move pages selection left",
        SIMPLE_MOVE_PAGES_SELECTION_RIGHT => "move pages selection right",
        SIMPLE_MOVE_PAGES_SELECTION_HOME => "move pages selection home",
        SIMPLE_MOVE_PAGES_SELECTION_END => "move pages selection end",
        SIMPLE_REFRESH_RELATED_TAGS => "refresh related tags",
        SIMPLE_AUTOCOMPLETE_FORCE_FETCH => "autocomplete force fetch",
        SIMPLE_AUTOCOMPLETE_IME_MODE => "autocomplete ime mode",
        SIMPLE_AUTOCOMPLETE_IF_EMPTY_TAB_LEFT => "autocomplete if empty tab left",
        SIMPLE_AUTOCOMPLETE_IF_EMPTY_TAB_RIGHT => "autocomplete if empty tab right",
        SIMPLE_AUTOCOMPLETE_IF_EMPTY_PAGE_LEFT => "autocomplete if empty page left",
        SIMPLE_AUTOCOMPLETE_IF_EMPTY_PAGE_RIGHT => "autocomplete if empty page right",
        SIMPLE_AUTOCOMPLETE_IF_EMPTY_MEDIA_PREVIOUS => "autocomplete if empty media previous",
        SIMPLE_AUTOCOMPLETE_IF_EMPTY_MEDIA_NEXT => "autocomplete if empty media next",
        SIMPLE_MEDIA_SEEK_DELTA => "media seek delta",
        SIMPLE_GLOBAL_PROFILE_MODE_FLIP => "global profile mode flip",
        SIMPLE_GLOBAL_FORCE_ANIMATION_SCANBAR_SHOW => "global force animation scanbar show",
        SIMPLE_OPEN_COMMAND_PALETTE => "open command palette",
        SIMPLE_SWITCH_BETWEEN_100_PERCENT_AND_MAX_ZOOM => "switch between 100 percent and max zoom",
        SIMPLE_SWITCH_BETWEEN_CANVAS_AND_MAX_ZOOM => "switch between canvas and max zoom",
        SIMPLE_ZOOM_MAX => "zoom max",
        SIMPLE_ZOOM_CANVAS => "zoom canvas",
        SIMPLE_ZOOM_100 => "zoom 100",
        SIMPLE_ZOOM_DEFAULT => "zoom default",
        SIMPLE_SHOW_DUPLICATES => "show duplicates",
        SIMPLE_MANAGE_FILE_TIMESTAMPS => "manage file timestamps",
        SIMPLE_OPEN_FILE_IN_FILE_EXPLORER => "open file in file explorer",
        SIMPLE_MOVE_THUMBNAIL_FOCUS => "move thumbnail focus",
        SIMPLE_SELECT_FILES => "select files",
        SIMPLE_REARRANGE_THUMBNAILS => "rearrange thumbnails",
        SIMPLE_MAC_QUICKLOOK => "mac quicklook",
        SIMPLE_COPY_URLS => "copy urls",
        SIMPLE_OPEN_FILE_IN_WEB_BROWSER => "open file in web browser",
        SIMPLE_OPEN_SELECTION_IN_NEW_DUPLICATES_FILTER_PAGE => {
            "open selection in new duplicates filter page"
        }
        SIMPLE_OPEN_SIMILAR_LOOKING_FILES => "open similar looking files",
        SIMPLE_COPY_FILE_HASHES => "copy file hashes",
        SIMPLE_COPY_FILE_BITMAP => "copy file bitmap",
        SIMPLE_COPY_FILE_ID => "copy file id",
        SIMPLE_COPY_FILE_SERVICE_FILENAMES => "copy file service filenames",
        SIMPLE_NATIVE_OPEN_FILE_PROPERTIES => "native open file properties",
        SIMPLE_NATIVE_OPEN_FILE_WITH_DIALOG => "native open file with dialog",
        SIMPLE_RELOAD_CURRENT_STYLESHEET => "reload current stylesheet",
        SIMPLE_FLIP_ICC_PROFILE_APPLICATION => "flip icc profile application",
        SIMPLE_ZOOM_100_CENTER => "zoom 100 center",
        SIMPLE_ZOOM_CANVAS_VIEWER_CENTER => "zoom canvas viewer center",
        SIMPLE_ZOOM_CANVAS_FILL_X => "zoom canvas fill x",
        SIMPLE_ZOOM_CANVAS_FILL_X_VIEWER_CENTER => "zoom canvas fill x viewer center",
        SIMPLE_ZOOM_CANVAS_FILL_Y => "zoom canvas fill y",
        SIMPLE_ZOOM_CANVAS_FILL_Y_VIEWER_CENTER => "zoom canvas fill y viewer center",
        SIMPLE_ZOOM_CANVAS_FILL_AUTO => "zoom canvas fill auto",
        SIMPLE_ZOOM_CANVAS_FILL_AUTO_VIEWER_CENTER => "zoom canvas fill auto viewer center",
        SIMPLE_SWITCH_BETWEEN_100_PERCENT_AND_CANVAS_FIT_AND_FILL_ZOOM => {
            "switch between 100 percent and canvas fit and fill zoom"
        }
        SIMPLE_SWITCH_BETWEEN_100_PERCENT_AND_CANVAS_FIT_AND_FILL_ZOOM_VIEWER_CENTER => {
            "switch between 100 percent and canvas fit and fill zoom viewer center"
        }
        SIMPLE_ZOOM_DEFAULT_VIEWER_CENTER => "zoom default viewer center",
        SIMPLE_RESET_PAN_TO_CENTER => "reset pan to center",
        SIMPLE_CLOSE_MEDIA_VIEWER_AND_FOCUS_TAB => "close media viewer and focus tab",
        SIMPLE_ZOOM_TO_PERCENTAGE => "zoom to percentage",
        SIMPLE_ZOOM_TO_PERCENTAGE_CENTER => "zoom to percentage center",
        SIMPLE_RESIZE_WINDOW_TO_MEDIA => "resize window to media",
        SIMPLE_RESIZE_WINDOW_TO_MEDIA_VIEWER_CENTER => "resize window to media viewer center",
        SIMPLE_RESIZE_WINDOW_TO_MEDIA_ZOOMED => "resize window to media zoomed",
        SIMPLE_RESIZE_WINDOW_TO_MEDIA_ZOOMED_VIEWER_CENTER => {
            "resize window to media zoomed viewer center"
        }
        SIMPLE_OPEN_OPTIONS => "open options",
        SIMPLE_DUPLICATE_MEDIA_CLEAR_INTERNAL_FALSE_POSITIVES => {
            "duplicate media clear internal false positives"
        }
        SIMPLE_CLOSE_MEDIA_VIEWER_AND_FOCUS_TAB_AND_FOCUS_MEDIA => {
            "close media viewer and focus tab and focus media"
        }
        SIMPLE_VIEW_RANDOM => "view random",
        SIMPLE_UNDO_RANDOM => "undo random",
        SIMPLE_FOCUS_TAB_AND_MEDIA => "focus tab and media",
        SIMPLE_DUPLICATE_FILTER_APPROVE_AUTO_RESOLUTION => {
            "duplicate filter approve auto resolution"
        }
        SIMPLE_DUPLICATE_FILTER_DENY_AUTO_RESOLUTION => "duplicate filter deny auto resolution",
        SIMPLE_FLIP_TRANSPARENCY_CHECKERBOARD_MEDIA_VIEWER => {
            "flip transparency checkerboard media viewer"
        }
        SIMPLE_FLIP_TRANSPARENCY_CHECKERBOARD_MEDIA_VIEWER_DUPLICATE_FILTER => {
            "flip transparency checkerboard media viewer duplicate filter"
        }
        SIMPLE_FLIP_TRANSPARENCY_CHECKERBOARD_GREENSCREEN => {
            "flip transparency checkerboard greenscreen"
        }
        SIMPLE_START_SLIDESHOW => "start slideshow",
        SIMPLE_WINDOW_ALWAYS_ON_TOP_FLIP => "window always on top flip",
        SIMPLE_WINDOW_ALWAYS_ON_TOP_ON => "window always on top on",
        SIMPLE_WINDOW_ALWAYS_ON_TOP_OFF => "window always on top off",
        SIMPLE_WINDOW_FRAMELESS_FLIP => "window frameless flip",
        SIMPLE_RENAME_PAGE => "rename page",
        SIMPLE_FLIP_THISWINDOW_SLIDESHOW_SHUFFLE => "flip thiswindow slideshow shuffle",
        SIMPLE_FLIP_GLOBAL_SLIDESHOW_SHUFFLE => "flip global slideshow shuffle",
        SIMPLE_FLIP_THISWINDOW_SLIDESHOW_ALWAYS_PLAY_DURATION_MEDIA_ONCE_THROUGH => {
            "flip thiswindow slideshow always play duration media once through"
        }
        SIMPLE_FLIP_GLOBAL_SLIDESHOW_ALWAYS_PLAY_DURATION_MEDIA_ONCE_THROUGH => {
            "flip global slideshow always play duration media once through"
        }
        SIMPLE_PER_PLAYER_AUDIO_MUTE => "per player audio mute",
        SIMPLE_PER_PLAYER_AUDIO_MUTE_FLIP => "per player audio mute flip",
        SIMPLE_PER_PLAYER_AUDIO_UNMUTE => "per player audio unmute",
        SIMPLE_WINDOW_ALWAYS_ON_TOP_WHILE_PLAYING_FLIP => "window always on top while playing flip",
        SIMPLE_SHOW_DETAILED_EMBEDDED_FILE_METADATA_WINDOW => {
            "show detailed embedded file metadata window"
        }
        _ => "unknown command",
    }
}
