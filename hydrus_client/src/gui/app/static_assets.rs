//! Resolver for the vendored Hydrus `static/` asset directory.
//!
//! The Python client loads icons, stylesheets and other resources from a
//! `static/` directory next to the executable (or inside the source tree in
//! development). The Rust port mirrors that: it searches a small list of
//! locations and returns the first one that actually exists, so the app keeps
//! working even when `static/` was not deployed (callers fall back to painted
//! icons).

use std::path::{Path, PathBuf};

/// Environment variable that overrides every other location.
pub const ENV_VAR: &str = "HYDRUS_STATIC_DIR";

/// Resolve the static directory using the production search order:
/// 1. `HYDRUS_STATIC_DIR`
/// 2. `<exe dir>/static`
/// 3. `<repo root>/static` (development builds, via `CARGO_MANIFEST_DIR`)
pub fn resolve_static_dir() -> Option<PathBuf> {
    let env_override = std::env::var_os(ENV_VAR).map(PathBuf::from);
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(PathBuf::from));
    let repo_root = option_env!("CARGO_MANIFEST_DIR").map(|dir| {
        PathBuf::from(dir)
            .parent()
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(dir))
    });
    resolve_from(env_override.as_deref(), exe_dir.as_deref(), repo_root.as_deref())
}

/// Pure resolution helper, exposed for tests. Returns the first existing
/// candidate in the documented order.
pub fn resolve_from(
    env_override: Option<&Path>,
    exe_dir: Option<&Path>,
    repo_root: Option<&Path>,
) -> Option<PathBuf> {
    if let Some(path) = env_override {
        if path.is_dir() {
            return Some(path.to_path_buf());
        }
    }
    if let Some(dir) = exe_dir {
        let candidate = dir.join("static");
        if candidate.is_dir() {
            return Some(candidate);
        }
    }
    if let Some(root) = repo_root {
        let candidate = root.join("static");
        if candidate.is_dir() {
            return Some(candidate);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn make_dir(path: &Path) {
        fs::create_dir_all(path).unwrap();
    }

    #[test]
    fn env_override_wins() {
        let base = std::env::temp_dir().join("hydrus_static_test_env");
        let _ = fs::remove_dir_all(&base);
        let env_dir = base.join("env_static");
        let exe_dir = base.join("exe");
        let repo_dir = base.join("repo");
        make_dir(&env_dir);
        make_dir(&exe_dir.join("static"));
        make_dir(&repo_dir.join("static"));

        let resolved = resolve_from(Some(&env_dir), Some(&exe_dir), Some(&repo_dir));
        assert_eq!(resolved, Some(env_dir.clone()));

        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn exe_dir_fallback() {
        let base = std::env::temp_dir().join("hydrus_static_test_exe");
        let _ = fs::remove_dir_all(&base);
        let exe_dir = base.join("exe");
        make_dir(&exe_dir.join("static"));

        let resolved = resolve_from(None, Some(&exe_dir), None);
        assert_eq!(resolved, Some(exe_dir.join("static")));

        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn repo_root_fallback() {
        let base = std::env::temp_dir().join("hydrus_static_test_repo");
        let _ = fs::remove_dir_all(&base);
        let repo_dir = base.join("repo");
        make_dir(&repo_dir.join("static"));

        let resolved = resolve_from(None, None, Some(&repo_dir));
        assert_eq!(resolved, Some(repo_dir.join("static")));

        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn missing_returns_none() {
        let base = std::env::temp_dir().join("hydrus_static_test_none");
        let _ = fs::remove_dir_all(&base);
        let resolved = resolve_from(
            Some(&base.join("nope")),
            Some(&base.join("exe")),
            Some(&base.join("repo")),
        );
        assert!(resolved.is_none());
    }
}