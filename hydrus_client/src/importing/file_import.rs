//! File import job from `hydrus/client/importing/ClientImportFiles.py`.

use std::path::{Path, PathBuf};

use hydrus_core::files::hydrus_file_handling::{self as core_files, FileInfo};
use hydrus_core::hydrus_constants::Mime;
use hydrus_core::hydrus_exceptions::{ErrorKind, HydrusException, HydrusResult};

use crate::client_constants::{self, STATUS_DELETED, STATUS_ERROR, STATUS_UNKNOWN};
use crate::importing::options::ImportOptionsContainer;

mod status;

pub use status::*;

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
mod tests;
