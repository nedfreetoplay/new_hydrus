use hydrus_core::hydrus_constants::Mime;
use hydrus_core::hydrus_exceptions::HydrusResult;

use crate::client_constants::{self, STATUS_DELETED, STATUS_ERROR, STATUS_UNKNOWN};
use crate::importing::options::ImportOptionsContainer;

pub const CHECKER_STATUS_OK: i32 = 0;
pub const CHECKER_STATUS_DEAD: i32 = 1;
pub const CHECKER_STATUS_404: i32 = 2;

pub const DOWNLOADER_SIMPLE_STATUS_DONE: i32 = 0;
pub const DOWNLOADER_SIMPLE_STATUS_WORKING: i32 = 1;
pub const DOWNLOADER_SIMPLE_STATUS_PENDING: i32 = 2;
pub const DOWNLOADER_SIMPLE_STATUS_PAUSED: i32 = 3;
pub const DOWNLOADER_SIMPLE_STATUS_DEFERRED: i32 = 4;
pub const DOWNLOADER_SIMPLE_STATUS_PAUSING: i32 = 5;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileImportStatus {
    pub status: i32,
    pub hash: Option<Vec<u8>>,
    pub mime: Option<Mime>,
    pub note: String,
}

impl FileImportStatus {
    pub fn new(
        status: i32,
        hash: Option<Vec<u8>>,
        mime: Option<Mime>,
        note: impl Into<String>,
    ) -> Self {
        Self {
            status,
            hash,
            mime,
            note: note.into(),
        }
    }

    pub fn unknown() -> Self {
        Self::new(STATUS_UNKNOWN, None, None, "")
    }

    pub fn already_in_db(&self) -> bool {
        self.status == client_constants::STATUS_SUCCESSFUL_BUT_REDUNDANT
    }

    pub fn duplicate(&self) -> Self {
        self.clone()
    }

    /// `ShouldImport( import_options_container )`
    pub fn should_import(&self, container: &ImportOptionsContainer) -> bool {
        if self.status == STATUS_UNKNOWN {
            return true;
        }

        if self.status == STATUS_DELETED && !container.file_filtering.excludes_deleted {
            return true;
        }

        false
    }

    pub fn to_string(&self) -> String {
        let status = status_string(self.status);

        if self.note.is_empty() {
            status.to_string()
        } else {
            format!("{status}, {}", self.note)
        }
    }
}

/// Builds the post-import status for a file that is already in the client.
pub fn redundant_status(hash: Vec<u8>, mime: Option<Mime>) -> FileImportStatus {
    FileImportStatus::new(
        client_constants::STATUS_SUCCESSFUL_BUT_REDUNDANT,
        Some(hash),
        mime,
        "",
    )
}

fn status_string(status: i32) -> &'static str {
    match status {
        STATUS_UNKNOWN => "unknown",
        client_constants::STATUS_SUCCESSFUL_AND_NEW => "successful and new",
        client_constants::STATUS_SUCCESSFUL_BUT_REDUNDANT => "successful but redundant",
        STATUS_DELETED => "deleted",
        STATUS_ERROR => "error",
        client_constants::STATUS_NEW => "new",
        client_constants::STATUS_PAUSED => "paused",
        _ => "unknown",
    }
}
