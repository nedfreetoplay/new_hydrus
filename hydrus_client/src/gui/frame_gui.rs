//! Description of the boot GUI shell.
//!
//! Keeping the static shell content separate from the Qt bridge makes it easy
//! to test and lets later milestones replace individual pieces with real pages.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MenuModel {
    pub title: &'static str,
    pub actions: &'static [&'static str],
    pub has_exit: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameGuiModel {
    pub title: &'static str,
    pub status: &'static str,
    pub sidebar: &'static [&'static str],
    pub pages: &'static [&'static str],
    pub menus: &'static [MenuModel],
}

pub fn default_frame_gui() -> FrameGuiModel {
    FrameGuiModel {
        title: "Hydrus Client",
        status: "Hydrus Client — Rust/Qt 6",
        sidebar: &[
            "Pages",
            "Files",
            "Downloads",
            "Subscriptions",
            "Gallery",
            "Watchers",
            "Duplicate Filter",
        ],
        pages: &["Welcome"],
        menus: &[
            MenuModel {
                title: "File",
                actions: &["New Page", "Open File", "Import Folder"],
                has_exit: true,
            },
            MenuModel {
                title: "Pages",
                actions: &["Refresh", "Undo", "Redo", "Close Page"],
                has_exit: false,
            },
            MenuModel {
                title: "Help",
                actions: &["About Hydrus", "Debug Info"],
                has_exit: false,
            },
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_shell_has_navigation_and_menus() {
        let model = default_frame_gui();

        assert!(!model.sidebar.is_empty());
        assert!(!model.pages.is_empty());
        assert!(model.menus.iter().any(|menu| menu.title == "File"));
    }

    #[test]
    fn exactly_one_menu_exits_the_application() {
        let model = default_frame_gui();
        let exits = model.menus.iter().filter(|menu| menu.has_exit).count();

        assert_eq!(exits, 1);
    }
}
