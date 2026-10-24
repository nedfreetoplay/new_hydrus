//! Environment preparation from `hydrus/client/ClientEnvironment.py`.

use std::path::{Path, PathBuf};

use hydrus_core::hydrus_paths;

pub const REQUESTS_CA_BUNDLE: &str = "REQUESTS_CA_BUNDLE";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CaBundleOutcome {
    /// The variable was already present and was left untouched.
    AlreadySet(String),
    /// The variable now points at this existing file.
    Set(PathBuf),
    /// A path was proposed but does not exist.
    Missing(PathBuf),
    /// Neither the environment nor the caller supplied a bundle.
    NotProvided,
}

/// `set_requests_ca_bundle_env`
///
/// Python falls back to `certifi.where()`; the Rust port accepts an explicit
/// path (usually provided by the packaging layer) and never overwrites an
/// existing `REQUESTS_CA_BUNDLE`.
pub fn set_requests_ca_bundle_env(pem_path: Option<&Path>) -> CaBundleOutcome {
    let existing = std::env::var(REQUESTS_CA_BUNDLE).ok();
    let outcome = decide(existing.as_deref(), pem_path, |path| path.exists());

    if let CaBundleOutcome::Set(path) = &outcome {
        // SAFETY: called during single-threaded boot, before worker threads.
        unsafe { std::env::set_var(REQUESTS_CA_BUNDLE, path) };
    }

    outcome
}

fn decide(
    existing: Option<&str>,
    pem_path: Option<&Path>,
    exists: impl Fn(&Path) -> bool,
) -> CaBundleOutcome {
    if let Some(existing) = existing {
        return CaBundleOutcome::AlreadySet(existing.to_string());
    }

    let Some(pem_path) = pem_path else {
        return CaBundleOutcome::NotProvided;
    };

    if exists(pem_path) {
        CaBundleOutcome::Set(pem_path.to_path_buf())
    } else {
        CaBundleOutcome::Missing(pem_path.to_path_buf())
    }
}

/// Convenience wrapper used by the boot path: points requests at the
/// certificate bundle shipped with the hydrus static files, when present.
pub fn prepare_default_bundle() -> CaBundleOutcome {
    let candidate =
        hydrus_core::hydrus_static_dir::get_static_path("certificates/cacert.pem", false);

    set_requests_ca_bundle_env(Some(&candidate))
}

pub fn ca_bundle_path() -> Option<PathBuf> {
    std::env::var_os(REQUESTS_CA_BUNDLE).map(PathBuf::from)
}

pub fn ensure_parent_dir(path: &Path) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        hydrus_paths::make_sure_directory_exists(parent)
            .map_err(|error| std::io::Error::other(error.to_string()))?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn existing_environment_wins() {
        let outcome = decide(
            Some("/custom/bundle.pem"),
            Some(Path::new("/other.pem")),
            |_| true,
        );

        assert_eq!(
            outcome,
            CaBundleOutcome::AlreadySet("/custom/bundle.pem".to_string())
        );
    }

    #[test]
    fn missing_file_is_reported() {
        let outcome = decide(None, Some(Path::new("missing.pem")), |_| false);

        assert_eq!(
            outcome,
            CaBundleOutcome::Missing(PathBuf::from("missing.pem"))
        );
    }

    #[test]
    fn no_path_means_not_provided() {
        assert_eq!(decide(None, None, |_| true), CaBundleOutcome::NotProvided);
    }
}
