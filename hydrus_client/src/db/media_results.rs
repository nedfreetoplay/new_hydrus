//! Builds `MediaResult`s from the database (`ClientDBMediaResults.py`).

use hydrus_core::hydrus_constants::{ContentStatus, ServiceType, TimestampType};
use hydrus_core::hydrus_db_base::DBBase;
use hydrus_core::hydrus_exceptions::HydrusResult;

use crate::db::{
    definitions_cache, files_metadata, files_storage, files_timestamps, mappings, master,
    notes as db_notes, ratings, services,
};
use crate::media::MediaResult;
use crate::metadata::TagStatus;

pub fn get_media_result(db: &DBBase, hash_id: i64) -> HydrusResult<Option<MediaResult>> {
    let Some(file_info) = files_metadata::get_file_info(db, hash_id)? else {
        return Ok(None);
    };

    let Some(hash) = definitions_cache::get_hash(db, hash_id)?
        .or(master::get_hash(db, hash_id)?)
    else {
        return Ok(None);
    };

    let mut media_result = MediaResult::new(hash, file_info);
    media_result.locations_manager.inbox = files_storage::is_inbox(db, hash_id)?;

    let all_services = services::get_services(db)?;

    for service in &all_services {
        match service.service_type {
            ServiceType::LocalFileDomain | ServiceType::LocalFileTrashDomain => {
                if let Some(timestamp) = files_storage::get_timestamp(
                    db,
                    service.service_id,
                    ContentStatus::Current,
                    hash_id,
                )? {
                    media_result
                        .locations_manager
                        .set_current(service.service_key.clone(), timestamp);
                }

                if let Some(timestamp) = files_storage::get_timestamp(
                    db,
                    service.service_id,
                    ContentStatus::Deleted,
                    hash_id,
                )? {
                    media_result
                        .locations_manager
                        .set_deleted(service.service_key.clone(), timestamp);
                }
            }
            ServiceType::LocalTag => {
                let tags = mappings::get_tags(
                    db,
                    service.service_id,
                    ContentStatus::Current,
                    hash_id,
                )?;

                media_result.tags_manager.tags.add_tags(
                    service.service_key.clone(),
                    tags,
                    TagStatus::Current,
                );
            }
            ServiceType::LocalRatingNumerical
            | ServiceType::LocalRatingLike
            | ServiceType::LocalRatingIncDec => {
                if let Some(rating) = ratings::get_rating(db, service.service_id, hash_id)? {
                    media_result
                        .ratings_manager
                        .set_rating(service.service_key.clone(), rating);
                }
            }
            _ => {}
        }
    }

    for (name, note) in db_notes::get_notes(db, hash_id)? {
        media_result.notes_manager.set_note(name, note);
    }

    if let Some(timestamp) =
        files_timestamps::get_file_timestamp(db, hash_id, TimestampType::Archived)?
    {
        media_result
            .times_manager
            .set_time(TimestampType::Archived.as_int() as i32, Some(timestamp));
    }

    Ok(Some(media_result))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::master_tags::get_tag_id;
    use crate::db::{files_metadata, notes, ratings};
    use crate::db::ClientDB;
    use crate::metadata::Rating;
    use hydrus_core::files::hydrus_file_handling::FileInfo;
    use hydrus_core::hydrus_constants::Mime;

    fn database() -> (ClientDB, std::path::PathBuf, i64, i64, i64) {
        let directory = std::env::temp_dir().join(format!(
            "hydrus_client_media_results_{}_{}",
            std::process::id(),
            rand::random::<u64>()
        ));
        let db = ClientDB::new(&directory).unwrap();

        let file_service = services::get_or_add_service(
            db.db(),
            b"my files",
            ServiceType::LocalFileDomain,
            "my files",
        )
        .unwrap();
        let tag_service = services::get_or_add_service(
            db.db(),
            b"local tags",
            ServiceType::LocalTag,
            "local tags",
        )
        .unwrap();
        let rating_service = services::get_or_add_service(
            db.db(),
            b"rating",
            ServiceType::LocalRatingNumerical,
            "rating",
        )
        .unwrap();

        (db, directory, file_service, tag_service, rating_service)
    }

    #[test]
    fn builds_media_result_from_db_managers() {
        let (db, directory, file_service, tag_service, rating_service) = database();
        let hash_id = master::get_hash_id(db.db(), &[1, 2, 3]).unwrap();
        let file_info = FileInfo {
            size: 1024,
            mime: Mime::Png,
            width: Some(10),
            height: Some(20),
            duration_ms: None,
            num_frames: None,
            has_audio: false,
            num_words: None,
        };
        files_metadata::set_file_info(db.db(), hash_id, &file_info).unwrap();
        files_storage::add_files(
            db.db(),
            file_service,
            ContentStatus::Current,
            &[hash_id],
            Some(100),
        )
        .unwrap();
        files_storage::set_inbox(db.db(), &[hash_id], true).unwrap();

        let tag_id = get_tag_id(db.db(), "blue eyes").unwrap();
        mappings::add_mappings(
            db.db(),
            tag_service,
            ContentStatus::Current,
            hash_id,
            &[tag_id],
        )
        .unwrap();
        ratings::set_rating(db.db(), rating_service, hash_id, Rating::Numerical(0.5)).unwrap();
        notes::set_note(db.db(), hash_id, "page 1", "hello").unwrap();

        let media = get_media_result(db.db(), hash_id).unwrap().unwrap();

        assert_eq!(media.get_size(), 1024);
        assert!(media.get_inbox());
        assert!(media.locations_manager.is_current(b"my files"));
        assert_eq!(media.tags_manager.get_tags(b"local tags"), vec!["blue eyes"]);
        assert_eq!(
            media.ratings_manager.get_rating(b"rating"),
            Some(Rating::Numerical(0.5))
        );
        assert_eq!(media.notes_manager.get_note("page 1"), Some("hello"));

        drop(db);
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn missing_file_info_returns_none() {
        let (db, directory, _, _, _) = database();

        assert!(get_media_result(db.db(), 999).unwrap().is_none());

        drop(db);
        let _ = std::fs::remove_dir_all(directory);
    }
}
