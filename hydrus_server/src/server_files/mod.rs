//! Rust port of `hydrus/server/ServerFiles.py`.
//!
//! Server files live in the same two-hex-digit prefix layout as the client's
//! physical storage. The Python module reads the active controller from a
//! global; here [`ServerFiles`] can be constructed with an explicit directory
//! for tests, while the free functions preserve the Python API by resolving the
//! global controller.

use std::collections::HashSet;
use std::ffi::OsStr;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use hydrus_core::files::hydrus_files_physical_storage;
use hydrus_core::hydrus_exceptions::{ErrorKind, HydrusException, HydrusResult};

use crate::server_globals;

/// The Python `file_type` string.
#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ServerFileType {
    File,
    Thumbnail,
}

impl ServerFileType {
    fn includes(self, path: &Path) -> bool {
        let name = path.file_name().and_then(OsStr::to_str).unwrap_or_default();
        let is_thumbnail = name.ends_with(".thumbnail");

        match self {
            Self::File => !is_thumbnail,
            Self::Thumbnail => is_thumbnail,
        }
    }
}

/// A server file store rooted at `server_files`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServerFiles {
    files_dir: PathBuf,
}

include!("parts/part_01.rs");

include!("parts/part_02.rs");

include!("parts/part_03.rs");

#[cfg(test)]
mod tests;
