//! Maintenance job execution for `ClientFilesMaintenanceManager`.

use hydrus_core::files::hydrus_file_handling as core_files;
use hydrus_core::files::images::hydrus_blurhash;
use hydrus_core::files::images::hydrus_image_colours;
use hydrus_core::hydrus_constants::TimestampType;
use hydrus_core::hydrus_db_base::DBBase;
use hydrus_core::hydrus_exceptions::{ErrorKind, HydrusException, HydrusResult};

use crate::db::{files_metadata, files_timestamps, master};
use crate::files::maintenance::*;
use crate::files::{ClientFilesManager, write_file};

pub struct FilesMaintenanceExecutor {
    files_manager: ClientFilesManager,
}

impl FilesMaintenanceExecutor {
    pub fn new(files_manager: ClientFilesManager) -> Self {
        Self { files_manager }
    }

    /// Runs one regeneration job. Returns `false` for jobs that are not yet
    /// implemented (integrity/url maintenance, exif/iptc flags, etc.).
    pub fn run_job(&self, db: &DBBase, hash_id: i64, job_type: i32) -> HydrusResult<bool> {
        match job_type {
            REGENERATE_FILE_DATA_JOB_FILE_METADATA => {
                let Some(info) = files_metadata::get_file_info(db, hash_id)? else {
                    return Ok(false);
                };
                let path = self.file_path(db, hash_id, info.mime)?;
                let new_info = core_files::get_file_info(&path, Some(info.mime), false)?;
                files_metadata::set_file_info(db, hash_id, &new_info)?;

                Ok(true)
            }
            REGENERATE_FILE_DATA_JOB_FORCE_THUMBNAIL | REGENERATE_FILE_DATA_JOB_REFIT_THUMBNAIL => {
                let Some(info) = files_metadata::get_file_info(db, hash_id)? else {
                    return Ok(false);
                };
                let path = self.file_path(db, hash_id, info.mime)?;
                let thumbnail = core_files::generate_thumbnail_bytes(
                    &path,
                    (200, 200),
                    info.mime,
                    None,
                    None,
                    35.0,
                )?;
                let thumbnail_path = self
                    .files_manager
                    .get_thumbnail_path(&self.hash(db, hash_id)?)?;
                write_file(&thumbnail_path, &thumbnail)?;

                Ok(true)
            }
            REGENERATE_FILE_DATA_JOB_BLURHASH => {
                let Some(info) = files_metadata::get_file_info(db, hash_id)? else {
                    return Ok(false);
                };
                let path = self.file_path(db, hash_id, info.mime)?;
                let image =
                    hydrus_core::files::images::hydrus_image_handling::decode_image_path(&path)?;
                let blurhash = hydrus_blurhash::get_blur_hash_from_numpy(&image);
                files_metadata::set_blurhash(db, hash_id, &blurhash)?;

                Ok(true)
            }
            REGENERATE_FILE_DATA_JOB_FILE_HAS_TRANSPARENCY => {
                let Some(info) = files_metadata::get_file_info(db, hash_id)? else {
                    return Ok(false);
                };
                let path = self.file_path(db, hash_id, info.mime)?;
                let image =
                    hydrus_core::files::images::hydrus_image_handling::decode_image_path(&path)?;
                let has_transparency =
                    hydrus_image_colours::image_has_human_perceptible_alpha_channel(&image);
                files_metadata::set_metadata_flag(
                    db,
                    hash_id,
                    files_metadata::MetadataFlag::Transparency,
                    has_transparency,
                )?;

                Ok(true)
            }
            REGENERATE_FILE_DATA_JOB_FILE_MODIFIED_TIMESTAMP => {
                let Some(info) = files_metadata::get_file_info(db, hash_id)? else {
                    return Ok(false);
                };
                let path = self.file_path(db, hash_id, info.mime)?;
                let timestamp_ms = core_files::get_file_modified_timestamp_ms(&path)?;
                files_timestamps::set_file_timestamp(
                    db,
                    hash_id,
                    TimestampType::ModifiedFile,
                    Some(timestamp_ms / 1000),
                )?;

                Ok(true)
            }
            _ => Ok(false),
        }
    }

    fn file_path(
        &self,
        db: &DBBase,
        hash_id: i64,
        mime: hydrus_core::hydrus_constants::Mime,
    ) -> HydrusResult<std::path::PathBuf> {
        let hash = self.hash(db, hash_id)?;
        self.files_manager.get_file_path(&hash, mime, false)
    }

    fn hash(&self, db: &DBBase, hash_id: i64) -> HydrusResult<Vec<u8>> {
        master::get_hash(db, hash_id)?
            .ok_or_else(|| HydrusException::new(ErrorKind::DataMissing, "hash id is missing"))
    }
}

#[cfg(test)]
mod tests;
