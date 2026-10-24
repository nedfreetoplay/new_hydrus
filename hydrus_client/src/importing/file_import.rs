//! File import job from `hydrus/client/importing/ClientImportFiles.py`.

use std::path::{Path, PathBuf};

use hydrus_core::files::hydrus_file_handling::{self as core_files, FileInfo};
use hydrus_core::hydrus_constants::Mime;
use hydrus_core::hydrus_exceptions::{ErrorKind, HydrusException, HydrusResult};

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

#[derive(Debug)]
pub struct FileImportJob {
    pub temp_path: PathBuf,
    pub import_options: ImportOptionsContainer,
    pub human_file_description: Option<String>,
    pre_import_file_status: FileImportStatus,
    post_import_file_status: FileImportStatus,
    file_info: Option<FileInfo>,
    hash: Option<Vec<u8>>,
    file_modified_timestamp_ms: Option<i64>,
}

impl FileImportJob {
    pub fn new(
        temp_path: impl Into<PathBuf>,
        import_options: ImportOptionsContainer,
        human_file_description: Option<String>,
    ) -> Self {
        Self {
            temp_path: temp_path.into(),
            import_options,
            human_file_description,
            pre_import_file_status: FileImportStatus::unknown(),
            post_import_file_status: FileImportStatus::unknown(),
            file_info: None,
            hash: None,
            file_modified_timestamp_ms: None,
        }
    }

    pub fn get_pre_import_file_status(&self) -> &FileImportStatus {
        &self.pre_import_file_status
    }

    pub fn get_post_import_file_status(&self) -> &FileImportStatus {
        &self.post_import_file_status
    }

    pub fn get_file_info(&self) -> Option<&FileInfo> {
        self.file_info.as_ref()
    }

    pub fn get_hash(&self) -> Option<&[u8]> {
        self.hash.as_deref()
    }

    pub fn get_mime(&self) -> Option<Mime> {
        self.file_info.as_ref().map(|info| info.mime)
    }

    pub fn get_file_modified_timestamp_ms(&self) -> Option<i64> {
        self.file_modified_timestamp_ms
    }

    /// Hashes the temp file and asks the callback whether it is already known.
    pub fn generate_pre_import_hash_and_status(
        &mut self,
        is_in_db: impl Fn(&[u8]) -> HydrusResult<bool>,
    ) -> HydrusResult<FileImportStatus> {
        let hash = core_files::get_hash_from_path(&self.temp_path)?;
        self.hash = Some(hash.clone());

        self.pre_import_file_status = if is_in_db(&hash)? {
            redundant_status(hash, None)
        } else {
            FileImportStatus::new(STATUS_UNKNOWN, Some(hash), None, "")
        };

        Ok(self.pre_import_file_status.clone())
    }

    pub fn generate_info(&mut self) -> HydrusResult<&FileInfo> {
        if self.file_info.is_none() {
            self.file_info = Some(core_files::get_file_info(&self.temp_path, None, false)?);
        }

        self.file_modified_timestamp_ms =
            Some(core_files::get_file_modified_timestamp_ms(&self.temp_path)?);

        Ok(self.file_info.as_ref().expect("file info was just set"))
    }

    pub fn check_is_good_to_import(&self) -> HydrusResult<()> {
        let Some(info) = &self.file_info else {
            return Err(HydrusException::new(
                ErrorKind::DataMissing,
                "file info has not been generated",
            ));
        };

        if self.import_options.file_filtering.check_file_is_valid(
            info.size,
            info.mime,
            info.width,
            info.height,
        ) {
            Ok(())
        } else {
            Err(HydrusException::new(
                ErrorKind::Unknown,
                "file did not pass the import file filter",
            ))
        }
    }

    /// Runs the local half of an import. `is_in_db` normally wraps a client DB
    /// hash lookup.
    pub fn do_work(
        &mut self,
        is_in_db: impl Fn(&[u8]) -> HydrusResult<bool>,
    ) -> HydrusResult<FileImportStatus> {
        self.generate_pre_import_hash_and_status(is_in_db)?;

        if !self
            .pre_import_file_status
            .should_import(&self.import_options)
        {
            self.post_import_file_status = self.pre_import_file_status.duplicate();

            return Ok(self.post_import_file_status.clone());
        }

        self.generate_info()?;

        if self.check_is_good_to_import().is_err() {
            self.post_import_file_status = FileImportStatus::new(
                STATUS_ERROR,
                self.hash.clone(),
                self.get_mime(),
                "file did not pass the import file filter",
            );

            return Ok(self.post_import_file_status.clone());
        }

        self.post_import_file_status =
            FileImportStatus::new(STATUS_UNKNOWN, self.hash.clone(), self.get_mime(), "");

        Ok(self.post_import_file_status.clone())
    }
}

/// Reads the file info for an already-written import without a job object.
pub fn inspect_file(path: impl AsRef<Path>) -> HydrusResult<FileInfo> {
    core_files::get_file_info(path, None, false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::importing::options::FileFilteringImportOptions;
    use hydrus_core::hydrus_exceptions::unknown;
    use image::{Rgba, RgbaImage};

    fn png_file(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "hydrus_client_import_{name}_{}_{}.png",
            std::process::id(),
            rand::random::<u64>()
        ));
        RgbaImage::from_pixel(8, 8, Rgba([1, 2, 3, 255]))
            .save(&path)
            .unwrap();
        path
    }

    #[test]
    fn unknown_and_deleted_statuses_control_import() {
        let options = ImportOptionsContainer::default();

        assert!(FileImportStatus::unknown().should_import(&options));

        let deleted = FileImportStatus::new(STATUS_DELETED, None, None, "");
        assert!(!deleted.should_import(&options));

        let mut allow_deleted = options.clone();
        allow_deleted.file_filtering.excludes_deleted = false;
        assert!(deleted.should_import(&allow_deleted));
    }

    #[test]
    fn job_hashes_and_reads_file_info() {
        let path = png_file("info");
        let mut job = FileImportJob::new(&path, ImportOptionsContainer::default(), None);

        let status = job.do_work(|_| Ok(false)).unwrap();

        assert_eq!(status.status, STATUS_UNKNOWN);
        assert_eq!(status.mime, Some(Mime::Png));
        assert!(job.get_hash().is_some());
        assert_eq!(job.get_file_info().unwrap().width, Some(8));
        assert!(job.get_file_modified_timestamp_ms().is_some());

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn known_hashes_short_circuit_as_redundant() {
        let path = png_file("known");
        let mut job = FileImportJob::new(&path, ImportOptionsContainer::default(), None);

        let status = job.do_work(|_| Ok(true)).unwrap();

        assert!(status.already_in_db());
        assert!(job.get_file_info().is_none());

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn filters_reject_files_with_status_error() {
        let path = png_file("filtered");
        let options = ImportOptionsContainer {
            file_filtering: FileFilteringImportOptions {
                allowed_mimes: vec![Mime::Jpeg],
                ..Default::default()
            },
            ..Default::default()
        };
        let mut job = FileImportJob::new(&path, options, None);

        let status = job.do_work(|_| Ok(false)).unwrap();

        assert_eq!(status.status, STATUS_ERROR);
        assert!(status.note.contains("file filter"));

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn db_lookup_errors_propagate() {
        let path = png_file("error");
        let mut job = FileImportJob::new(&path, ImportOptionsContainer::default(), None);

        assert!(job.do_work(|_| Err(unknown("db unavailable"))).is_err());

        let _ = std::fs::remove_file(path);
    }
}
