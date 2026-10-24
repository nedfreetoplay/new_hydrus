//! Repository storage from `hydrus/client/db/ClientDBRepositories.py`.

use hydrus_core::hydrus_db_base::DBBase;
use hydrus_core::hydrus_exceptions::HydrusResult;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositoryUpdate {
    pub service_id: i64,
    pub update_index: i64,
    pub update_hash: Vec<u8>,
    pub begin: i64,
    pub end: i64,
    pub processed: bool,
}

pub fn set_remote_thumbnail(
    db: &DBBase,
    service_id: i64,
    hash_id: i64,
    thumbnail: &[u8],
) -> HydrusResult<()> {
    db.execute(
        "INSERT OR REPLACE INTO remote_thumbnails ( service_id, hash_id, thumbnail )
         VALUES ( ?, ?, ? )",
        &[&service_id, &hash_id, &thumbnail],
    )?;

    Ok(())
}

pub fn get_remote_thumbnail(
    db: &DBBase,
    service_id: i64,
    hash_id: i64,
) -> HydrusResult<Option<Vec<u8>>> {
    db.query_one(
        "SELECT thumbnail FROM remote_thumbnails WHERE service_id = ? AND hash_id = ?",
        &[&service_id, &hash_id],
        |row| row.get(0),
    )
}

pub fn has_remote_thumbnail(db: &DBBase, service_id: i64, hash_id: i64) -> HydrusResult<bool> {
    let count: i64 = db
        .query_one(
            "SELECT COUNT(*) FROM remote_thumbnails WHERE service_id = ? AND hash_id = ?",
            &[&service_id, &hash_id],
            |row| row.get(0),
        )?
        .unwrap_or(0);

    Ok(count > 0)
}

pub fn add_update(
    db: &DBBase,
    service_id: i64,
    update_index: i64,
    update_hash: &[u8],
    begin: i64,
    end: i64,
) -> HydrusResult<()> {
    db.execute(
        "INSERT OR REPLACE INTO repository_updates
         ( service_id, update_index, update_hash, begin_timestamp, end_timestamp, processed )
         VALUES ( ?, ?, ?, ?, ?, 0 )",
        &[&service_id, &update_index, &update_hash, &begin, &end],
    )?;

    Ok(())
}

pub fn get_updates(db: &DBBase, service_id: i64) -> HydrusResult<Vec<RepositoryUpdate>> {
    db.query_all(
        "SELECT service_id, update_index, update_hash, begin_timestamp, end_timestamp, processed
         FROM repository_updates WHERE service_id = ? ORDER BY update_index",
        &[&service_id],
        |row| {
            Ok(RepositoryUpdate {
                service_id: row.get(0)?,
                update_index: row.get(1)?,
                update_hash: row.get(2)?,
                begin: row.get(3)?,
                end: row.get(4)?,
                processed: row.get::<_, i64>(5)? != 0,
            })
        },
    )
}

pub fn get_next_unprocessed_update(
    db: &DBBase,
    service_id: i64,
) -> HydrusResult<Option<RepositoryUpdate>> {
    db.query_one(
        "SELECT service_id, update_index, update_hash, begin_timestamp, end_timestamp, processed
         FROM repository_updates WHERE service_id = ? AND processed = 0
         ORDER BY update_index LIMIT 1",
        &[&service_id],
        |row| {
            Ok(RepositoryUpdate {
                service_id: row.get(0)?,
                update_index: row.get(1)?,
                update_hash: row.get(2)?,
                begin: row.get(3)?,
                end: row.get(4)?,
                processed: row.get::<_, i64>(5)? != 0,
            })
        },
    )
}

pub fn mark_update_processed(db: &DBBase, service_id: i64, update_index: i64) -> HydrusResult<()> {
    db.execute(
        "UPDATE repository_updates SET processed = 1
         WHERE service_id = ? AND update_index = ?",
        &[&service_id, &update_index],
    )?;

    Ok(())
}

pub fn delete_updates(db: &DBBase, service_id: i64) -> HydrusResult<()> {
    db.execute(
        "DELETE FROM repository_updates WHERE service_id = ?",
        &[&service_id],
    )?;

    Ok(())
}

#[cfg(test)]
mod tests;
