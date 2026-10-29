use hydrus_core::hydrus_constants::Mime;

use crate::client_constants::Status;
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
    pub status: Status,
    pub hash: Option<Vec<u8>>,
    pub mime: Option<Mime>,
    pub note: String,
}

impl FileImportStatus {
    pub fn new(
        status: Status,
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
        Self::new(Status::Unknown, None, None, "")
    }

    pub fn already_in_db(&self) -> bool {
        self.status == Status::SuccessfulButRedundant
    }

    pub fn duplicate(&self) -> Self {
        self.clone()
    }

    /// `ShouldImport( import_options_container )`
    pub fn should_import(&self, container: &ImportOptionsContainer) -> bool {
        if self.status == Status::Unknown {
            return true;
        }

        if self.status == Status::Deleted && !container.file_filtering_or_default().excludes_deleted
        {
            return true;
        }

        false
    }

    pub fn to_string(&self) -> String {
        if self.note.is_empty() {
            self.status.to_string()
        } else {
            format!("{}, {}", self.status, self.note)
        }
    }
}

/// Builds the post-import status for a file that is already in the client.
pub fn redundant_status(hash: Vec<u8>, mime: Option<Mime>) -> FileImportStatus {
    FileImportStatus::new(Status::SuccessfulButRedundant, Some(hash), mime, "")
}
