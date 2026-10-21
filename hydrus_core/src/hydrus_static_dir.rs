//! Rust port of `hydrus/core/HydrusStaticDir.py`.
//!
//! Python asks `HG.controller` for the db dir so a user can override installed
//! icons. The controller is above this crate, so the db dir comes from
//! [`crate::hydrus_globals::db_dir`], set during boot.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::LazyLock;

use crate::hydrus_constants::{content_base_dir, USERPATH_SVG_ICON};
use crate::hydrus_exceptions::{not_found, HydrusResult};
use crate::hydrus_globals::db_dir;

/// `INSTALL_STATIC_DIR`
pub static INSTALL_STATIC_DIR: LazyLock<PathBuf> =
    LazyLock::new(|| content_base_dir().join("static"));

static USE_USER_STATIC_DIR: AtomicBool = AtomicBool::new(true);

/// `USE_USER_STATIC_DIR`
pub fn use_user_static_dir() -> bool {
    USE_USER_STATIC_DIR.load(Ordering::Relaxed)
}

pub fn set_use_user_static_dir(value: bool) {
    USE_USER_STATIC_DIR.store(value, Ordering::Relaxed);
}

/// `get_static_icon_path( name, force_install_dir = False, force_svg = False )`
pub fn get_static_icon_path(
    name: &str,
    force_install_dir: bool,
    force_svg: bool,
) -> HydrusResult<PathBuf> {
    let (svg_path, was_userdir) =
        get_static_path_with_result(&format!("{name}.svg"), force_install_dir);

    if !force_svg && !was_userdir {
        // A little ugly, but user png has preference over install svg.
        let (png_path, was_userdir) =
            get_static_path_with_result(&format!("{name}.png"), force_install_dir);

        if was_userdir {
            return Ok(png_path);
        }
    }

    if svg_path.exists() {
        Ok(svg_path)
    } else if force_svg {
        Err(not_found(format!("No SVG with the name \"{name}\"!")))
    } else {
        Ok(get_static_path(&format!("{name}.png"), force_install_dir))
    }
}

/// `get_static_path( sub_path, force_install_dir = False )`
pub fn get_static_path(sub_path: &str, force_install_dir: bool) -> PathBuf {
    get_static_path_with_result(sub_path, force_install_dir).0
}

/// `get_static_path_with_result( sub_path, force_install_dir = False )`
///
/// Returns `(path, was_userdir)`.
pub fn get_static_path_with_result(
    sub_path: &str,
    force_install_dir: bool,
) -> (PathBuf, bool) {
    if !force_install_dir && use_user_static_dir() {
        if let Some(db_dir) = db_dir() {
            let user_path = db_dir.join("static").join(sub_path);

            if user_path.exists() {
                return (user_path, true);
            }
        }
    }

    (INSTALL_STATIC_DIR.join(sub_path), false)
}

/// `get_rating_svg_path( name )`
pub fn get_rating_svg_path(name: &str) -> HydrusResult<PathBuf> {
    let name = Path::new(USERPATH_SVG_ICON).join(name);

    let name_string = name.to_string_lossy().to_string();

    let path = get_static_icon_path(&name_string, false, true)?;

    if path.exists() {
        Ok(path)
    } else {
        Err(not_found(format!(
            "Did not have a rating SVG called \"{name_string}\"!"
        )))
    }
}

/// `List_static_dir_file_paths( sub_dir_path )`
pub fn list_static_dir_file_paths(sub_dir_path: &str) -> Vec<PathBuf> {
    let user_path = get_static_path(sub_dir_path, false);
    let install_path = get_static_path(sub_dir_path, true);

    let mut dirs_to_do = Vec::new();

    if user_path != install_path {
        dirs_to_do.push(user_path);
    }

    dirs_to_do.push(install_path);

    let mut seen: HashSet<PathBuf> = HashSet::new();
    let mut file_paths = Vec::new();

    for dir_to_do in dirs_to_do {
        let Ok(entries) = std::fs::read_dir(&dir_to_do) else {
            continue;
        };

        let mut paths: Vec<PathBuf> = entries
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .filter(|path| path.is_file())
            .collect();

        paths.sort();

        for path in paths {
            if seen.insert(path.clone()) {
                file_paths.push(path);
            }
        }
    }

    file_paths
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn install_path_is_the_fallback() {
        let (path, was_userdir) = get_static_path_with_result("definitely-not-a-real-file.png", true);

        assert!(!was_userdir);
        assert_eq!(path, INSTALL_STATIC_DIR.join("definitely-not-a-real-file.png"));
    }

    #[test]
    fn force_svg_errors_when_missing() {
        let error = get_static_icon_path("definitely-not-a-real-icon", true, true).unwrap_err();

        assert!(error.to_string().contains("No SVG"));
    }

    #[test]
    fn missing_directories_list_nothing() {
        assert!(list_static_dir_file_paths("definitely-not-a-real-dir").is_empty());
    }
}
