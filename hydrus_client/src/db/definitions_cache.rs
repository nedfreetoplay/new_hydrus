//! Definitions cache from `hydrus/client/db/ClientDBDefinitionsCache.py`.

use hydrus_core::hydrus_db_base::DBBase;
use hydrus_core::hydrus_exceptions::HydrusResult;

use crate::db::master_tags;

pub fn add_hash_ids_to_cache(db: &DBBase, hash_ids: &[i64]) -> HydrusResult<()> {
    for hash_id in hash_ids {
        db.execute(
            "INSERT OR REPLACE INTO local_hashes_cache ( hash_id, hash )
             SELECT hash_id, hash FROM hashes WHERE hash_id = ?",
            &[hash_id],
        )?;
    }

    Ok(())
}

pub fn add_tag_ids_to_cache(db: &DBBase, tag_ids: &[i64]) -> HydrusResult<()> {
    for tag_id in tag_ids {
        if let Some(tag) = master_tags::get_tag(db, *tag_id)? {
            db.execute(
                "INSERT OR REPLACE INTO local_tags_cache ( tag_id, tag ) VALUES ( ?, ? )",
                &[tag_id, &tag],
            )?;
        }
    }

    Ok(())
}

pub fn get_hash(db: &DBBase, hash_id: i64) -> HydrusResult<Option<Vec<u8>>> {
    if let Some(hash) = db.query_one(
        "SELECT hash FROM local_hashes_cache WHERE hash_id = ?",
        &[&hash_id],
        |row| row.get(0),
    )? {
        return Ok(Some(hash));
    }

    add_hash_ids_to_cache(db, &[hash_id])?;

    db.query_one(
        "SELECT hash FROM local_hashes_cache WHERE hash_id = ?",
        &[&hash_id],
        |row| row.get(0),
    )
}

pub fn get_tag(db: &DBBase, tag_id: i64) -> HydrusResult<Option<String>> {
    if let Some(tag) = db.query_one(
        "SELECT tag FROM local_tags_cache WHERE tag_id = ?",
        &[&tag_id],
        |row| row.get(0),
    )? {
        return Ok(Some(tag));
    }

    let tag = master_tags::get_tag(db, tag_id)?;

    if let Some(tag) = &tag {
        db.execute(
            "INSERT OR REPLACE INTO local_tags_cache ( tag_id, tag ) VALUES ( ?, ? )",
            &[&tag_id, &tag],
        )?;
    }

    Ok(tag)
}

pub fn get_hashes(db: &DBBase, hash_ids: &[i64]) -> HydrusResult<Vec<Vec<u8>>> {
    let mut hashes = Vec::with_capacity(hash_ids.len());

    for hash_id in hash_ids {
        if let Some(hash) = get_hash(db, *hash_id)? {
            hashes.push(hash);
        }
    }

    Ok(hashes)
}

pub fn get_tags(db: &DBBase, tag_ids: &[i64]) -> HydrusResult<Vec<String>> {
    let mut tags = Vec::with_capacity(tag_ids.len());

    for tag_id in tag_ids {
        if let Some(tag) = get_tag(db, *tag_id)? {
            tags.push(tag);
        }
    }

    Ok(tags)
}

pub fn drop_hash_ids_from_cache(db: &DBBase, hash_ids: &[i64]) -> HydrusResult<()> {
    for hash_id in hash_ids {
        db.execute(
            "DELETE FROM local_hashes_cache WHERE hash_id = ?",
            &[hash_id],
        )?;
    }

    Ok(())
}

pub fn drop_tag_ids_from_cache(db: &DBBase, tag_ids: &[i64]) -> HydrusResult<()> {
    for tag_id in tag_ids {
        db.execute("DELETE FROM local_tags_cache WHERE tag_id = ?", &[tag_id])?;
    }

    Ok(())
}

pub fn clear_hash_cache(db: &DBBase) -> HydrusResult<()> {
    db.execute("DELETE FROM local_hashes_cache", &[])?;
    Ok(())
}

pub fn clear_tag_cache(db: &DBBase) -> HydrusResult<()> {
    db.execute("DELETE FROM local_tags_cache", &[])?;
    Ok(())
}

pub fn repopulate_tag_cache(db: &DBBase) -> HydrusResult<()> {
    clear_tag_cache(db)?;
    db.execute(
        "INSERT INTO local_tags_cache ( tag_id, tag )
         SELECT tags.tag_id,
                CASE WHEN namespaces.namespace = '' THEN subtags.subtag
                     ELSE namespaces.namespace || ':' || subtags.subtag END
         FROM tags
         JOIN namespaces ON tags.namespace_id = namespaces.namespace_id
         JOIN subtags ON tags.subtag_id = subtags.subtag_id",
        &[],
    )?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::ClientDB;
    use crate::db::master::get_hash_id;
    use crate::db::master_tags::get_tag_id;

    fn database() -> (ClientDB, std::path::PathBuf) {
        let directory = std::env::temp_dir().join(format!(
            "hydrus_client_defcache_{}_{}",
            std::process::id(),
            rand::random::<u64>()
        ));

        (ClientDB::new(&directory).unwrap(), directory)
    }

    #[test]
    fn hash_cache_populates_on_demand() {
        let (db, directory) = database();
        let hash_id = get_hash_id(db.db(), &[1, 2, 3]).unwrap();

        assert_eq!(get_hash(db.db(), hash_id).unwrap(), Some(vec![1, 2, 3]));

        drop_hash_ids_from_cache(db.db(), &[hash_id]).unwrap();
        clear_hash_cache(db.db()).unwrap();

        assert_eq!(get_hash(db.db(), hash_id).unwrap(), Some(vec![1, 2, 3]));

        drop(db);
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn tag_cache_round_trips_namespaced_tags() {
        let (db, directory) = database();
        let tag_id = get_tag_id(db.db(), "creator:someone").unwrap();

        assert_eq!(
            get_tag(db.db(), tag_id).unwrap().as_deref(),
            Some("creator:someone")
        );

        repopulate_tag_cache(db.db()).unwrap();

        assert_eq!(
            get_tag(db.db(), tag_id).unwrap().as_deref(),
            Some("creator:someone")
        );

        drop(db);
        let _ = std::fs::remove_dir_all(directory);
    }
}
