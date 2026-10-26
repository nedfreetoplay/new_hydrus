//! Duplicate file storage from `hydrus/client/db/ClientDBFilesDuplicatesStorage.py`.

use hydrus_core::hydrus_db_base::DBBase;
use hydrus_core::hydrus_exceptions::HydrusResult;

pub fn add_potential_pair(
    db: &DBBase,
    smaller_hash_id: i64,
    larger_hash_id: i64,
    distance: i64,
) -> HydrusResult<()> {
    db.execute(
        "INSERT OR REPLACE INTO potential_duplicate_pairs
         ( smaller_hash_id, larger_hash_id, distance ) VALUES ( ?, ?, ? )",
        &[&smaller_hash_id, &larger_hash_id, &distance],
    )?;

    Ok(())
}

pub fn remove_potential_pair(
    db: &DBBase,
    smaller_hash_id: i64,
    larger_hash_id: i64,
) -> HydrusResult<()> {
    db.execute(
        "DELETE FROM potential_duplicate_pairs
         WHERE smaller_hash_id = ? AND larger_hash_id = ?",
        &[&smaller_hash_id, &larger_hash_id],
    )?;

    Ok(())
}

pub fn get_potential_pairs(
    db: &DBBase,
    max_distance: Option<i64>,
) -> HydrusResult<Vec<(i64, i64, i64)>> {
    match max_distance {
        Some(max_distance) => db.query_all(
            "SELECT smaller_hash_id, larger_hash_id, distance
             FROM potential_duplicate_pairs WHERE distance <= ?
             ORDER BY distance, smaller_hash_id",
            &[&max_distance],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        ),
        None => db.query_all(
            "SELECT smaller_hash_id, larger_hash_id, distance
             FROM potential_duplicate_pairs ORDER BY distance, smaller_hash_id",
            &[],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        ),
    }
}

/// Candidate duplicate pairs that still need a decision. False positives and
/// pairs without metadata on both sides are skipped.
pub fn get_candidate_pairs(
    db: &DBBase,
    max_distance: Option<i64>,
) -> HydrusResult<Vec<(i64, i64, i64)>> {
    let sql = match max_distance {
        Some(_) => {
            "SELECT p.smaller_hash_id, p.larger_hash_id, p.distance
             FROM potential_duplicate_pairs p
             WHERE p.distance <= ?
               AND NOT EXISTS (
                   SELECT 1 FROM duplicate_false_positives f
                   WHERE f.smaller_hash_id = p.smaller_hash_id
                     AND f.larger_hash_id = p.larger_hash_id
               )
               AND EXISTS ( SELECT 1 FROM files_info a WHERE a.hash_id = p.smaller_hash_id )
               AND EXISTS ( SELECT 1 FROM files_info b WHERE b.hash_id = p.larger_hash_id )
             ORDER BY p.distance, p.smaller_hash_id"
        }
        None => {
            "SELECT p.smaller_hash_id, p.larger_hash_id, p.distance
             FROM potential_duplicate_pairs p
             WHERE NOT EXISTS (
                   SELECT 1 FROM duplicate_false_positives f
                   WHERE f.smaller_hash_id = p.smaller_hash_id
                     AND f.larger_hash_id = p.larger_hash_id
               )
               AND EXISTS ( SELECT 1 FROM files_info a WHERE a.hash_id = p.smaller_hash_id )
               AND EXISTS ( SELECT 1 FROM files_info b WHERE b.hash_id = p.larger_hash_id )
             ORDER BY p.distance, p.smaller_hash_id"
        }
    };

    match max_distance {
        Some(max_distance) => db.query_all(sql, &[&max_distance], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        }),
        None => db.query_all(sql, &[], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?))),
    }
}

pub fn add_false_positive(
    db: &DBBase,
    first_hash_id: i64,
    second_hash_id: i64,
) -> HydrusResult<()> {
    db.execute(
        "INSERT OR IGNORE INTO duplicate_false_positives
         ( smaller_hash_id, larger_hash_id ) VALUES ( ?, ? )",
        &[&first_hash_id, &second_hash_id],
    )?;

    Ok(())
}

pub fn is_false_positive(
    db: &DBBase,
    first_hash_id: i64,
    second_hash_id: i64,
) -> HydrusResult<bool> {
    let count: i64 = db
        .query_one(
            "SELECT COUNT(*) FROM duplicate_false_positives
             WHERE smaller_hash_id = ? AND larger_hash_id = ?",
            &[&first_hash_id, &second_hash_id],
            |row| row.get(0),
        )?
        .unwrap_or(0);

    Ok(count > 0)
}

pub fn create_alternate_group(db: &DBBase, hash_ids: &[i64]) -> HydrusResult<i64> {
    db.execute("INSERT INTO alternate_file_groups DEFAULT VALUES", &[])?;
    let group_id: i64 = db
        .query_one("SELECT last_insert_rowid()", &[], |row| row.get(0))?
        .unwrap_or(0);

    for hash_id in hash_ids {
        db.execute(
            "INSERT OR IGNORE INTO alternate_file_group_members
             ( alternates_group_id, hash_id ) VALUES ( ?, ? )",
            &[&group_id, &hash_id],
        )?;
    }

    Ok(group_id)
}

pub fn add_to_alternate_group(db: &DBBase, group_id: i64, hash_id: i64) -> HydrusResult<()> {
    db.execute(
        "INSERT OR IGNORE INTO alternate_file_group_members
         ( alternates_group_id, hash_id ) VALUES ( ?, ? )",
        &[&group_id, &hash_id],
    )?;

    Ok(())
}

pub fn get_alternate_group_members(db: &DBBase, group_id: i64) -> HydrusResult<Vec<i64>> {
    db.query_all(
        "SELECT hash_id FROM alternate_file_group_members
         WHERE alternates_group_id = ? ORDER BY hash_id",
        &[&group_id],
        |row| row.get(0),
    )
}

pub fn set_duplicate_group(
    db: &DBBase,
    media_id: i64,
    king_hash_id: i64,
    hash_ids: &[i64],
) -> HydrusResult<()> {
    db.execute(
        "INSERT OR REPLACE INTO duplicate_files ( media_id, king_hash_id ) VALUES ( ?, ? )",
        &[&media_id, &king_hash_id],
    )?;

    for hash_id in hash_ids {
        db.execute(
            "INSERT OR IGNORE INTO duplicate_file_members ( media_id, hash_id ) VALUES ( ?, ? )",
            &[&media_id, &hash_id],
        )?;
    }

    Ok(())
}

pub fn get_duplicate_group_members(db: &DBBase, media_id: i64) -> HydrusResult<Vec<i64>> {
    db.query_all(
        "SELECT hash_id FROM duplicate_file_members WHERE media_id = ? ORDER BY hash_id",
        &[&media_id],
        |row| row.get(0),
    )
}

pub fn get_duplicate_king(db: &DBBase, media_id: i64) -> HydrusResult<Option<i64>> {
    db.query_one(
        "SELECT king_hash_id FROM duplicate_files WHERE media_id = ?",
        &[&media_id],
        |row| row.get(0),
    )
}

#[cfg(test)]
mod tests;
