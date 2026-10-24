//! Applying `ContentUpdatePackage`s to the database.

use hydrus_core::hydrus_constants::{ContentStatus, ContentType};
use hydrus_core::hydrus_db_base::DBBase;
use hydrus_core::hydrus_exceptions::HydrusResult;

use crate::db::{files_storage, mappings, master, master_tags, notes, ratings, services, url_map};
use crate::metadata::{ContentAction, ContentUpdate, ContentUpdatePackage, Rating};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ApplyResult {
    pub applied: usize,
    pub skipped: usize,
}

pub fn apply_content_update_package(
    db: &DBBase,
    package: &ContentUpdatePackage,
) -> HydrusResult<ApplyResult> {
    let mut result = ApplyResult::default();

    for update in &package.updates {
        if apply_update(db, update)? {
            result.applied += 1;
        } else {
            result.skipped += 1;
        }
    }

    Ok(result)
}

fn apply_update(db: &DBBase, update: &ContentUpdate) -> HydrusResult<bool> {
    let Some(service) = services::get_service(db, &update.service_key)? else {
        return Ok(false);
    };

    if update.hashes.is_empty() {
        return Ok(false);
    }

    let hash_ids: Vec<i64> = update
        .hashes
        .iter()
        .map(|hash| master::get_hash_id(db, hash))
        .collect::<HydrusResult<_>>()?;

    match update.content_type {
        ContentType::Mappings => apply_mappings(db, service.service_id, update, &hash_ids)?,
        ContentType::Files => apply_files(db, service.service_id, update, &hash_ids)?,
        ContentType::Urls => apply_urls(db, update, &hash_ids)?,
        ContentType::Ratings => apply_ratings(db, service.service_id, update, &hash_ids)?,
        ContentType::Notes => apply_notes(db, update, &hash_ids)?,
        _ => return Ok(false),
    }

    Ok(true)
}

fn apply_mappings(
    db: &DBBase,
    service_id: i64,
    update: &ContentUpdate,
    hash_ids: &[i64],
) -> HydrusResult<()> {
    let tag_ids: Vec<i64> = update
        .tags
        .iter()
        .map(|tag| master_tags::get_tag_id(db, tag))
        .collect::<HydrusResult<_>>()?;

    for hash_id in hash_ids {
        match status_for(update.action) {
            Some(status) => mappings::add_mappings(db, service_id, status, *hash_id, &tag_ids)?,
            None => {
                for status in [
                    ContentStatus::Current,
                    ContentStatus::Pending,
                    ContentStatus::Deleted,
                    ContentStatus::Petitioned,
                ] {
                    mappings::remove_mappings(db, service_id, status, *hash_id, &tag_ids)?;
                }
            }
        }
    }

    Ok(())
}

fn apply_files(
    db: &DBBase,
    service_id: i64,
    update: &ContentUpdate,
    hash_ids: &[i64],
) -> HydrusResult<()> {
    match status_for(update.action) {
        Some(status) => files_storage::add_files(db, service_id, status, hash_ids, None),
        None => {
            for status in [
                ContentStatus::Current,
                ContentStatus::Pending,
                ContentStatus::Deleted,
                ContentStatus::Petitioned,
            ] {
                files_storage::remove_files(db, service_id, status, hash_ids)?;
            }

            Ok(())
        }
    }
}

fn apply_urls(db: &DBBase, update: &ContentUpdate, hash_ids: &[i64]) -> HydrusResult<()> {
    for hash_id in hash_ids {
        if update.action == ContentAction::Add {
            for url in &update.urls {
                url_map::add_url(db, *hash_id, url)?;
            }
        } else if update.action == ContentAction::Delete {
            for url in &update.urls {
                url_map::delete_url(db, *hash_id, url)?;
            }
        }
    }

    Ok(())
}

fn apply_ratings(
    db: &DBBase,
    service_id: i64,
    update: &ContentUpdate,
    hash_ids: &[i64],
) -> HydrusResult<()> {
    let Some(value) = update
        .details
        .get("rating")
        .and_then(|value| value.as_f64())
    else {
        return Ok(());
    };

    let rating = Rating::Numerical(value.clamp(0.0, 1.0));

    for hash_id in hash_ids {
        if update.action == ContentAction::Set {
            ratings::set_rating(db, service_id, *hash_id, rating)?;
        } else if update.action == ContentAction::Delete {
            ratings::delete_rating(db, service_id, *hash_id)?;
        }
    }

    Ok(())
}

fn apply_notes(db: &DBBase, update: &ContentUpdate, hash_ids: &[i64]) -> HydrusResult<()> {
    let name = update
        .details
        .get("name")
        .and_then(|value| value.as_str())
        .unwrap_or("note");
    let note = update
        .details
        .get("note")
        .and_then(|value| value.as_str())
        .unwrap_or("");

    for hash_id in hash_ids {
        if update.action == ContentAction::Delete {
            notes::delete_note(db, *hash_id, name)?;
        } else {
            notes::set_note(db, *hash_id, name, note)?;
        }
    }

    Ok(())
}

fn status_for(action: ContentAction) -> Option<ContentStatus> {
    match action {
        ContentAction::Add | ContentAction::Set => Some(ContentStatus::Current),
        ContentAction::Pend => Some(ContentStatus::Pending),
        ContentAction::Petition => Some(ContentStatus::Petitioned),
        ContentAction::Delete => Some(ContentStatus::Deleted),
        ContentAction::Rescind | ContentAction::RescindPetition | ContentAction::Manage => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{ClientDB, master_tags};
    use hydrus_core::hydrus_constants::ServiceType;

    fn database() -> (ClientDB, std::path::PathBuf) {
        let directory = std::env::temp_dir().join(format!(
            "hydrus_client_content_updates_{}_{}",
            std::process::id(),
            rand::random::<u64>()
        ));

        let db = ClientDB::new(&directory).unwrap();
        services::get_or_add_service(db.db(), b"local tags", ServiceType::LocalTag, "local tags")
            .unwrap();
        services::get_or_add_service(
            db.db(),
            b"my files",
            ServiceType::LocalFileDomain,
            "my files",
        )
        .unwrap();

        (db, directory)
    }

    #[test]
    fn applies_mapping_updates() {
        let (db, directory) = database();
        let hash = vec![0xaa];
        let package = ContentUpdatePackage::from_updates(vec![
            ContentUpdate::new(
                ContentAction::Add,
                b"local tags".to_vec(),
                ContentType::Mappings,
            )
            .with_hashes([hash.clone()])
            .with_tags(["blue eyes"]),
        ]);

        let result = apply_content_update_package(db.db(), &package).unwrap();
        let hash_id = master::get_hash_id(db.db(), &hash).unwrap();
        let service = services::get_service(db.db(), b"local tags")
            .unwrap()
            .unwrap();
        let tag_id = master_tags::get_tag_id(db.db(), "blue eyes").unwrap();

        assert_eq!(result.applied, 1);
        assert_eq!(
            mappings::get_tag_ids(db.db(), service.service_id, ContentStatus::Current, hash_id)
                .unwrap(),
            vec![tag_id]
        );

        drop(db);
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn applies_file_and_url_updates() {
        let (db, directory) = database();
        let hash = vec![0xbb];
        let package = ContentUpdatePackage::from_updates(vec![
            ContentUpdate::new(ContentAction::Add, b"my files".to_vec(), ContentType::Files)
                .with_hashes([hash.clone()]),
            ContentUpdate::new(ContentAction::Add, b"my files".to_vec(), ContentType::Urls)
                .with_hashes([hash.clone()])
                .with_urls(["https://example.com/a"]),
        ]);

        apply_content_update_package(db.db(), &package).unwrap();
        let hash_id = master::get_hash_id(db.db(), &hash).unwrap();
        let service = services::get_service(db.db(), b"my files")
            .unwrap()
            .unwrap();

        assert_eq!(
            files_storage::get_hash_ids(db.db(), service.service_id, ContentStatus::Current)
                .unwrap(),
            vec![hash_id]
        );
        assert_eq!(url_map::get_urls(db.db(), hash_id).unwrap().len(), 1);

        drop(db);
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn missing_services_are_skipped() {
        let (db, directory) = database();
        let package = ContentUpdatePackage::from_updates(vec![
            ContentUpdate::new(
                ContentAction::Add,
                b"missing service".to_vec(),
                ContentType::Files,
            )
            .with_hashes([vec![1]]),
        ]);

        let result = apply_content_update_package(db.db(), &package).unwrap();

        assert_eq!(result.skipped, 1);

        drop(db);
        let _ = std::fs::remove_dir_all(directory);
    }
}
