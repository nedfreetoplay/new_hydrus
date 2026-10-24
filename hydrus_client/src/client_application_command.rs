//! Application commands from `hydrus/client/ClientApplicationCommand.py`.
//!
//! The Python module defines 199 command ids plus a single `ApplicationCommand`
//! value object. This file ports the common subset used by the GUI boot shell;
//! the remaining ids are added as their handlers are ported.

use serde_json::Value;

pub const SIMPLE_ARCHIVE_DELETE_FILTER_BACK: i32 = 0;
pub const SIMPLE_ARCHIVE_DELETE_FILTER_DELETE: i32 = 1;
pub const SIMPLE_ARCHIVE_DELETE_FILTER_KEEP: i32 = 2;
pub const SIMPLE_ARCHIVE_DELETE_FILTER_SKIP: i32 = 3;
pub const SIMPLE_ARCHIVE_FILE: i32 = 4;
pub const SIMPLE_CHECK_ALL_IMPORT_FOLDERS: i32 = 5;
pub const SIMPLE_CLOSE_MEDIA_VIEWER: i32 = 6;
pub const SIMPLE_CLOSE_PAGE: i32 = 7;
pub const SIMPLE_COPY_FILES: i32 = 10;
pub const SIMPLE_COPY_FILE_PATHS: i32 = 12;
pub const SIMPLE_DELETE_FILE: i32 = 16;
pub const SIMPLE_EXIT_APPLICATION: i32 = 31;
pub const SIMPLE_EXPORT_FILES: i32 = 32;
pub const SIMPLE_FLIP_DARKMODE: i32 = 34;
pub const SIMPLE_FOCUS_MEDIA_VIEWER: i32 = 36;
pub const SIMPLE_HIDE_TO_SYSTEM_TRAY: i32 = 44;
pub const SIMPLE_INBOX_FILE: i32 = 45;
pub const SIMPLE_LAUNCH_MEDIA_VIEWER: i32 = 46;
pub const SIMPLE_LAUNCH_THE_ARCHIVE_DELETE_FILTER: i32 = 47;
pub const SIMPLE_MANAGE_FILE_NOTES: i32 = 48;
pub const SIMPLE_MANAGE_FILE_RATINGS: i32 = 49;
pub const SIMPLE_MANAGE_FILE_TAGS: i32 = 50;
pub const SIMPLE_MANAGE_FILE_URLS: i32 = 51;
pub const SIMPLE_MOVE_ANIMATION_TO_NEXT_FRAME: i32 = 52;
pub const SIMPLE_MOVE_ANIMATION_TO_PREVIOUS_FRAME: i32 = 53;
pub const SIMPLE_NEW_PAGE: i32 = 56;
pub const SIMPLE_NEW_PAGE_OF_PAGES: i32 = 57;
pub const SIMPLE_OPEN_FILE_IN_EXTERNAL_PROGRAM: i32 = 61;
pub const SIMPLE_OPEN_KNOWN_URL: i32 = 62;
pub const SIMPLE_OPEN_SELECTION_IN_NEW_PAGE: i32 = 63;
pub const SIMPLE_PAUSE_PLAY_MEDIA: i32 = 75;
pub const SIMPLE_PAUSE_PLAY_SLIDESHOW: i32 = 76;
pub const SIMPLE_REDO: i32 = 77;
pub const SIMPLE_REFRESH: i32 = 78;
pub const SIMPLE_REFRESH_ALL_PAGES: i32 = 79;
pub const SIMPLE_REMOVE_FILE_FROM_VIEW: i32 = 81;
pub const SIMPLE_RESTART_APPLICATION: i32 = 82;
pub const SIMPLE_SET_MEDIA_FOCUS: i32 = 83;
pub const SIMPLE_SET_SEARCH_FOCUS: i32 = 84;
pub const SIMPLE_SHOW_HIDE_SPLITTERS: i32 = 89;
pub const SIMPLE_SHOW_MENU: i32 = 90;
pub const SIMPLE_SWITCH_BETWEEN_100_PERCENT_AND_CANVAS_ZOOM: i32 = 91;
pub const SIMPLE_SWITCH_BETWEEN_FULLSCREEN_BORDERLESS_AND_REGULAR_FRAMED_WINDOW: i32 = 92;
pub const SIMPLE_UNCLOSE_PAGE: i32 = 94;
pub const SIMPLE_UNDELETE_FILE: i32 = 95;
pub const SIMPLE_UNDO: i32 = 96;
pub const SIMPLE_VIEW_FIRST: i32 = 97;
pub const SIMPLE_VIEW_LAST: i32 = 98;

/// `ApplicationCommand`
#[derive(Debug, Clone, PartialEq)]
pub struct ApplicationCommand {
    pub command_type: i32,
    pub data: Option<Value>,
}

impl ApplicationCommand {
    pub fn new(command_type: i32, data: Option<Value>) -> Self {
        Self { command_type, data }
    }

    pub fn simple(command_type: i32) -> Self {
        Self::new(command_type, None)
    }

    pub fn is_simple(&self) -> bool {
        self.data.is_none()
    }

    /// `ToString()`
    pub fn to_string(&self) -> String {
        let name = command_type_string(self.command_type);

        if let Some(data) = &self.data {
            format!("{name}: {data}")
        } else {
            name.to_string()
        }
    }

    pub fn duplicate(&self) -> Self {
        self.clone()
    }
}

pub fn command_type_string(command_type: i32) -> &'static str {
    match command_type {
        SIMPLE_ARCHIVE_DELETE_FILTER_BACK => "archive/delete filter: back",
        SIMPLE_ARCHIVE_DELETE_FILTER_DELETE => "archive/delete filter: delete",
        SIMPLE_ARCHIVE_DELETE_FILTER_KEEP => "archive/delete filter: keep",
        SIMPLE_ARCHIVE_DELETE_FILTER_SKIP => "archive/delete filter: skip",
        SIMPLE_ARCHIVE_FILE => "archive file",
        SIMPLE_CHECK_ALL_IMPORT_FOLDERS => "check all import folders",
        SIMPLE_CLOSE_MEDIA_VIEWER => "close media viewer",
        SIMPLE_CLOSE_PAGE => "close page",
        SIMPLE_COPY_FILES => "copy files",
        SIMPLE_COPY_FILE_PATHS => "copy file paths",
        SIMPLE_DELETE_FILE => "delete file",
        SIMPLE_EXIT_APPLICATION => "exit application",
        SIMPLE_EXPORT_FILES => "export files",
        SIMPLE_FLIP_DARKMODE => "flip darkmode",
        SIMPLE_FOCUS_MEDIA_VIEWER => "focus media viewer",
        SIMPLE_HIDE_TO_SYSTEM_TRAY => "hide to system tray",
        SIMPLE_INBOX_FILE => "inbox file",
        SIMPLE_LAUNCH_MEDIA_VIEWER => "launch media viewer",
        SIMPLE_LAUNCH_THE_ARCHIVE_DELETE_FILTER => "launch the archive/delete filter",
        SIMPLE_MANAGE_FILE_NOTES => "manage file notes",
        SIMPLE_MANAGE_FILE_RATINGS => "manage file ratings",
        SIMPLE_MANAGE_FILE_TAGS => "manage file tags",
        SIMPLE_MANAGE_FILE_URLS => "manage file urls",
        SIMPLE_MOVE_ANIMATION_TO_NEXT_FRAME => "move animation to next frame",
        SIMPLE_MOVE_ANIMATION_TO_PREVIOUS_FRAME => "move animation to previous frame",
        SIMPLE_NEW_PAGE => "new page",
        SIMPLE_NEW_PAGE_OF_PAGES => "new page of pages",
        SIMPLE_OPEN_FILE_IN_EXTERNAL_PROGRAM => "open file in external program",
        SIMPLE_OPEN_KNOWN_URL => "open known url",
        SIMPLE_OPEN_SELECTION_IN_NEW_PAGE => "open selection in new page",
        SIMPLE_PAUSE_PLAY_MEDIA => "pause/play media",
        SIMPLE_PAUSE_PLAY_SLIDESHOW => "pause/play slideshow",
        SIMPLE_REDO => "redo",
        SIMPLE_REFRESH => "refresh",
        SIMPLE_REFRESH_ALL_PAGES => "refresh all pages",
        SIMPLE_REMOVE_FILE_FROM_VIEW => "remove file from view",
        SIMPLE_RESTART_APPLICATION => "restart application",
        SIMPLE_SET_MEDIA_FOCUS => "set media focus",
        SIMPLE_SET_SEARCH_FOCUS => "set search focus",
        SIMPLE_SHOW_HIDE_SPLITTERS => "show/hide splitters",
        SIMPLE_SHOW_MENU => "show menu",
        SIMPLE_SWITCH_BETWEEN_100_PERCENT_AND_CANVAS_ZOOM => "switch between 100% and canvas zoom",
        SIMPLE_SWITCH_BETWEEN_FULLSCREEN_BORDERLESS_AND_REGULAR_FRAMED_WINDOW => {
            "switch between fullscreen borderless and regular framed window"
        }
        SIMPLE_UNCLOSE_PAGE => "unclose page",
        SIMPLE_UNDELETE_FILE => "undelete file",
        SIMPLE_UNDO => "undo",
        SIMPLE_VIEW_FIRST => "view first",
        SIMPLE_VIEW_LAST => "view last",
        _ => crate::client_application_command_full::simple_command_name(command_type),
    }
}

/// Maps command ids to the GUI/controller handlers that consume them.
#[derive(Default)]
pub struct ApplicationCommandRouter {
    handlers: std::collections::BTreeMap<
        i32,
        Box<dyn Fn(&ApplicationCommand) -> bool + Send + Sync>,
    >,
}

impl ApplicationCommandRouter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(
        &mut self,
        command_type: i32,
        handler: impl Fn(&ApplicationCommand) -> bool + Send + Sync + 'static,
    ) {
        self.handlers.insert(command_type, Box::new(handler));
    }

    /// Returns `true` when a handler consumed the command.
    pub fn dispatch(&self, command: &ApplicationCommand) -> bool {
        self.handlers
            .get(&command.command_type)
            .is_some_and(|handler| handler(command))
    }

    pub fn has_handler(&self, command_type: i32) -> bool {
        self.handlers.contains_key(&command_type)
    }

    pub fn len(&self) -> usize {
        self.handlers.len()
    }

    pub fn is_empty(&self) -> bool {
        self.handlers.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_commands_have_no_data() {
        let command = ApplicationCommand::simple(SIMPLE_ARCHIVE_FILE);

        assert!(command.is_simple());
        assert_eq!(command.to_string(), "archive file");
    }

    #[test]
    fn data_commands_render_data() {
        let command = ApplicationCommand::new(
            SIMPLE_SET_MEDIA_FOCUS,
            Some(Value::String("media".to_string())),
        );

        assert!(!command.is_simple());
        assert!(command.to_string().contains("media"));
    }

    #[test]
    fn lookup_covers_ported_commands() {
        for command_type in [
            SIMPLE_ARCHIVE_FILE,
            SIMPLE_CLOSE_PAGE,
            SIMPLE_EXIT_APPLICATION,
            SIMPLE_UNDO,
            SIMPLE_VIEW_LAST,
        ] {
            assert_ne!(command_type_string(command_type), "unknown command");
        }
    }

    #[test]
    fn full_generated_table_covers_every_simple_command() {
        let known = (0..=208)
            .filter(|command_type| {
                crate::client_application_command_full::simple_command_name(*command_type)
                    != "unknown command"
            })
            .count();

        assert!(known >= 190, "only {known} simple command ids were named");
        assert_ne!(
            crate::client_application_command_full::simple_command_name(208),
            "unknown command"
        );
    }

    #[test]
    fn router_dispatches_registered_commands() {
        let mut router = ApplicationCommandRouter::new();
        let counter = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let handler_counter = counter.clone();

        router.register(SIMPLE_ARCHIVE_FILE, move |_| {
            handler_counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            true
        });

        assert!(router.dispatch(&ApplicationCommand::simple(SIMPLE_ARCHIVE_FILE)));
        assert_eq!(counter.load(std::sync::atomic::Ordering::SeqCst), 1);

        assert!(!router.dispatch(&ApplicationCommand::simple(SIMPLE_UNDO)));
        assert!(router.has_handler(SIMPLE_ARCHIVE_FILE));
        assert_eq!(router.len(), 1);
    }
}
