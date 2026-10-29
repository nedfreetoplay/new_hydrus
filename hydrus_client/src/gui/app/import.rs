//! The local "file import" pipeline.
//!
//! The Python client imports files through `ClientImportFiles`; the Rust port
//! already had the hashing/metadata/thumbnail helpers but nothing that wrote a
//! file into the client database. This module performs that end-to-end step:
//! hash, inspect, copy into file storage, register metadata/service membership
//! and generate a thumbnail.

use std::path::{Path, PathBuf};

use hydrus_core::files::hydrus_file_handling as core_files;
use hydrus_core::hydrus_constants::{ContentStatus, Mime, ServiceType, TimestampType};
use hydrus_core::hydrus_db_base::DBBase;
use hydrus_core::hydrus_time;

use crate::controller::Controller;
use crate::db::{
    duplicates, files_metadata, files_storage, files_timestamps, master, similar_files, url_map,
};
use crate::files::ClientFilesManager;
use crate::files::images::{
    decode_image_path, discard_blank_perceptual_hashes, generate_shape_perceptual_hash,
};
use crate::importing::ImportOptionsContainer;

#[derive(Debug, Default)]
pub struct ImportSummary {
    pub hashes: Vec<Vec<u8>>,
    pub imported: usize,
    pub skipped: usize,
    pub errors: Vec<String>,
}

impl ImportSummary {
    pub fn message(&self) -> String {
        let mut message = format!("imported {} files", self.imported);

        if self.skipped > 0 {
            message.push_str(&format!(", skipped {}", self.skipped));
        }

        if !self.errors.is_empty() {
            message.push_str(&format!(", {} errors", self.errors.len()));
        }

        message
    }
}

/// The database and file-storage handles an import needs.
///
/// Keeping this separate from [`Controller`] lets background worker threads run
/// the same import path without sharing the whole controller.
#[derive(Clone)]
pub struct ImportContext {
    db: DBBase,
    files_manager: ClientFilesManager,
    file_service_ids: Vec<i64>,
    import_options: ImportOptionsContainer,
}

impl ImportContext {
    pub fn from_controller(controller: &Controller) -> Result<Self, String> {
        let import_options = super::import_options::local_import_options(controller);

        Self::new_with_options(
            controller.db().db().clone(),
            controller.files_manager(),
            import_options,
        )
    }

    pub fn new(db: DBBase, files_manager: ClientFilesManager) -> Result<Self, String> {
        Self::new_with_options(db, files_manager, ImportOptionsContainer::with_defaults())
    }

    pub fn new_with_options(
        db: DBBase,
        files_manager: ClientFilesManager,
        import_options: ImportOptionsContainer,
    ) -> Result<Self, String> {
        let services = crate::db::services::get_services(&db).unwrap_or_default();
        let default_service_id = services
            .iter()
            .find(|service| service.name == "my files")
            .or_else(|| {
                services
                    .iter()
                    .find(|service| service.service_type == ServiceType::LocalFileDomain)
            })
            .map(|service| service.service_id)
            .ok_or_else(|| "no local file domain".to_string())?;
        let locations = import_options.locations_or_default();
        let mut file_service_ids: Vec<i64> = locations
            .destination_services
            .iter()
            .filter_map(|name| {
                services
                    .iter()
                    .find(|service| {
                        &service.name == name
                            && service.service_type == ServiceType::LocalFileDomain
                    })
                    .map(|service| service.service_id)
            })
            .collect();

        if file_service_ids.is_empty() {
            file_service_ids.push(default_service_id);
        }

        Ok(Self {
            db,
            files_manager,
            file_service_ids,
            import_options,
        })
    }

    pub fn db(&self) -> &DBBase {
        &self.db
    }

    pub fn files_manager(&self) -> &ClientFilesManager {
        &self.files_manager
    }

    pub fn import_file(&self, path: &Path) -> Result<Vec<u8>, String> {
        self.import_file_from(path, None)
    }

    pub fn import_file_from(
        &self,
        path: &Path,
        source_url: Option<&str>,
    ) -> Result<Vec<u8>, String> {
        let hash = import_one(
            &self.db,
            &self.files_manager,
            &self.file_service_ids,
            &self.import_options,
            path,
            source_url,
        )?;

        Ok(hash)
    }
}

pub fn import_paths(controller: &Controller, paths: &[PathBuf]) -> ImportSummary {
    match ImportContext::from_controller(controller) {
        Ok(context) => import_paths_with(&context, paths),
        Err(error) => ImportSummary {
            errors: vec![error],
            skipped: paths.len(),
            ..ImportSummary::default()
        },
    }
}

/// Imports files with an explicit context. Used by the synchronous test helper
/// and by the background import queue.
pub fn import_paths_with(context: &ImportContext, paths: &[PathBuf]) -> ImportSummary {
    let mut summary = ImportSummary::default();

    for path in paths {
        match context.import_file(path) {
            Ok(hash) => {
                summary.imported += 1;
                summary.hashes.push(hash);
            }
            Err(error) => {
                summary.skipped += 1;
                summary.errors.push(format!("{}: {error}", path.display()));
            }
        }
    }

    summary
}

fn import_one(
    db: &DBBase,
    files_manager: &ClientFilesManager,
    file_service_ids: &[i64],
    import_options: &ImportOptionsContainer,
    path: &Path,
    source_url: Option<&str>,
) -> Result<Vec<u8>, String> {
    if !path.is_file() {
        return Err("not a file".to_string());
    }

    let hash = core_files::get_hash_from_path(path).map_err(|error| error.to_string())?;
    let mime = core_files::get_mime(path, false).map_err(|error| error.to_string())?;

    if mime == Mime::Unknown {
        return Err("unknown file type".to_string());
    }

    let info = core_files::get_file_info(path, Some(mime), false)
        .unwrap_or_else(|_| fallback_info(path, mime));

    if !import_options
        .file_filtering_or_default()
        .check_file_is_valid(info.size, info.mime, info.width, info.height)
    {
        return Err("file did not pass the import file filter".to_string());
    }

    let destination = files_manager
        .get_file_path(&hash, mime, false)
        .map_err(|error| error.to_string())?;

    if !destination.exists() {
        if let Some(parent) = destination.parent() {
            std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }

        std::fs::copy(path, &destination).map_err(|error| error.to_string())?;
    }

    let hash_id = master::get_hash_id(db, &hash).map_err(|error| error.to_string())?;

    files_metadata::set_file_info(db, hash_id, &info).map_err(|error| error.to_string())?;

    let locations = import_options.locations_or_default();

    for file_service_id in file_service_ids {
        files_storage::add_files(
            db,
            *file_service_id,
            ContentStatus::Current,
            &[hash_id],
            Some(hydrus_time::get_now()),
        )
        .map_err(|error| error.to_string())?;
    }

    if locations.automatically_archives {
        files_storage::set_inbox(db, &[hash_id], false).map_err(|error| error.to_string())?;
    } else {
        files_storage::set_inbox(db, &[hash_id], true).map_err(|error| error.to_string())?;
    }

    if let Some(source_url) = source_url {
        if locations.associate_primary_urls || locations.associate_source_urls {
            let _ = url_map::add_url(db, hash_id, source_url);
        }
    }

    if let Ok(timestamp_ms) = core_files::get_file_modified_timestamp_ms(path) {
        let _ = files_timestamps::set_file_timestamp(
            db,
            hash_id,
            TimestampType::ModifiedFile,
            Some(timestamp_ms / 1000),
        );
    }

    write_thumbnail(files_manager, &hash, path, &info);
    index_similar_files(db, hash_id, path, info.mime);

    Ok(hash)
}

/// Stores the perceptual/pixel hashes used by the duplicate filter and queues
/// potential duplicate pairs, as Python does during import.
fn index_similar_files(
    db: &hydrus_core::hydrus_db_base::DBBase,
    hash_id: i64,
    path: &Path,
    mime: Mime,
) {
    if hydrus_core::hydrus_constants::files_that_have_perceptual_hash().contains(&mime) {
        if let Ok(image) = decode_image_path(path) {
            let perceptual_hashes =
                discard_blank_perceptual_hashes([generate_shape_perceptual_hash(&image)]);

            if similar_files::set_perceptual_hashes(db, hash_id, &perceptual_hashes).is_ok() {
                for perceptual_hash in &perceptual_hashes {
                    let Ok(matches) =
                        similar_files::search_similar(db, perceptual_hash, 10, Some(hash_id))
                    else {
                        continue;
                    };

                    for similar in matches {
                        let (smaller, larger) = if similar.hash_id < hash_id {
                            (similar.hash_id, hash_id)
                        } else {
                            (hash_id, similar.hash_id)
                        };

                        let _ = duplicates::add_potential_pair(
                            db,
                            smaller,
                            larger,
                            similar.distance as i64,
                        );
                    }
                }
            }
        }
    }

    if let Ok(pixel_hash) =
        hydrus_core::files::images::hydrus_image_handling::get_image_pixel_hash(path, mime)
    {
        if similar_files::set_pixel_hash(db, hash_id, &pixel_hash).is_ok() {
            if let Ok(identical) = similar_files::get_hash_ids_for_pixel_hash(db, &pixel_hash) {
                for other in identical {
                    if other != hash_id {
                        let (smaller, larger) = if other < hash_id {
                            (other, hash_id)
                        } else {
                            (hash_id, other)
                        };

                        let _ = duplicates::add_potential_pair(db, smaller, larger, 0);
                    }
                }
            }
        }
    }
}

fn fallback_info(path: &Path, mime: Mime) -> hydrus_core::files::hydrus_file_handling::FileInfo {
    hydrus_core::files::hydrus_file_handling::FileInfo {
        size: std::fs::metadata(path)
            .map(|metadata| metadata.len())
            .unwrap_or(0),
        mime,
        width: None,
        height: None,
        duration_ms: None,
        num_frames: None,
        has_audio: false,
        num_words: None,
    }
}

fn write_thumbnail(
    files_manager: &ClientFilesManager,
    hash: &[u8],
    path: &Path,
    info: &hydrus_core::files::hydrus_file_handling::FileInfo,
) {
    let Ok(thumbnail_path) = files_manager.get_thumbnail_path(hash) else {
        return;
    };

    if let Some(parent) = thumbnail_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    let generated = core_files::generate_thumbnail_bytes(
        path,
        (200, 200),
        info.mime,
        info.duration_ms,
        info.num_frames,
        35.0,
    )
    .or_else(|_| {
        let image = core_files::generate_default_thumbnail(info.mime, (200, 200))?;
        hydrus_core::files::images::hydrus_image_handling::generate_thumbnail_bytes_from_numpy(
            &image,
        )
    });

    if let Ok(bytes) = generated {
        let _ = std::fs::write(thumbnail_path, bytes);
    }
}

/// Downloads one file URL and imports it. This is the small synchronous core
/// of the URL downloader; a queue/thread port will replace it later.
pub fn import_url(controller: &Controller, url: &str) -> ImportSummary {
    let mut summary = ImportSummary::default();

    if !(url.starts_with("http://") || url.starts_with("https://")) {
        summary
            .errors
            .push("url must start with http:// or https://".to_string());
        summary.skipped = 1;
        return summary;
    }

    let client = match reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .user_agent("Hydrus Client (Rust port)")
        .build()
    {
        Ok(client) => client,
        Err(error) => {
            summary.errors.push(error.to_string());
            summary.skipped = 1;
            return summary;
        }
    };

    let bytes = match client
        .get(url)
        .send()
        .and_then(|response| response.error_for_status())
    {
        Ok(response) => match response.bytes() {
            Ok(bytes) => bytes,
            Err(error) => {
                summary.errors.push(error.to_string());
                summary.skipped = 1;
                return summary;
            }
        },
        Err(error) => {
            summary.errors.push(error.to_string());
            summary.skipped = 1;
            return summary;
        }
    };

    let (mut file, path) = match hydrus_core::hydrus_temp::get_temp_path("url_import", ".bin", None)
    {
        Ok(value) => value,
        Err(error) => {
            summary.errors.push(error.to_string());
            summary.skipped = 1;
            return summary;
        }
    };

    use std::io::Write;

    if let Err(error) = file.write_all(&bytes) {
        summary.errors.push(error.to_string());
        summary.skipped = 1;
        return summary;
    }

    drop(file);

    summary = import_paths(controller, &[path.clone()]);
    let _ = std::fs::remove_file(path);

    summary
}

/// Collects every file under a directory. Used by import folders later.
pub fn walk_files(root: &Path, recursive: bool) -> Vec<PathBuf> {
    let mut result = Vec::new();
    let mut stack = vec![root.to_path_buf()];

    while let Some(directory) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&directory) else {
            continue;
        };

        for entry in entries.flatten() {
            let path = entry.path();

            if path.is_dir() {
                if recursive {
                    stack.push(path);
                }
            } else if path.is_file() {
                result.push(path);
            }
        }
    }

    result
}

#[cfg(test)]
mod tests;
