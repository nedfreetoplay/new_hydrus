//! Master hash table from `hydrus/client/db/ClientDBMaster.py`.

use hydrus_core::hydrus_db_base::DBBase;
use hydrus_core::hydrus_exceptions::HydrusResult;

pub fn get_hash_id(db: &DBBase, hash: &[u8]) -> HydrusResult<i64> {
    if let Some(hash_id) = db.query_one(
        "SELECT hash_id FROM hashes WHERE hash = ?",
        &[&hash],
        |row| row.get(0),
    )? {
        return Ok(hash_id);
    }

    db.execute("INSERT INTO hashes ( hash ) VALUES ( ? )", &[&hash])?;

    db.query_one(
        "SELECT hash_id FROM hashes WHERE hash = ?",
        &[&hash],
        |row| row.get(0),
    )?
    .ok_or_else(|| hydrus_core::hydrus_exceptions::data_missing("hash_id disappeared"))
}

pub fn get_hash(db: &DBBase, hash_id: i64) -> HydrusResult<Option<Vec<u8>>> {
    db.query_one(
        "SELECT hash FROM hashes WHERE hash_id = ?",
        &[&hash_id],
        |row| row.get(0),
    )
}

pub fn has_hash(db: &DBBase, hash: &[u8]) -> HydrusResult<bool> {
    let count: i64 = db.query_one(
        "SELECT COUNT(*) FROM hashes WHERE hash = ?",
        &[&hash],
        |row| row.get(0),
    )?.unwrap_or(0);

    Ok(count > 0)
}
