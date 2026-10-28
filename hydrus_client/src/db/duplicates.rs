//! Duplicate file storage from `hydrus/client/db/ClientDBFilesDuplicatesStorage.py`.

use std::collections::BTreeMap;

use hydrus_core::hydrus_db_base::DBBase;
use hydrus_core::hydrus_exceptions::HydrusResult;

use super::similar_files;

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

pub fn is_king(db: &DBBase, hash_id: i64) -> HydrusResult<bool> {
    let count: i64 = db
        .query_one(
            "SELECT COUNT(*) FROM duplicate_files WHERE king_hash_id = ?",
            &[&hash_id],
            |row| row.get(0),
        )?
        .unwrap_or(0);

    Ok(count > 0)
}

/// Number of relationships of one kind for the file, matching Hydrus'
/// `num file relationships` predicate. Duplicate/alternate counts exclude the
/// file itself.
pub fn get_relationship_count(db: &DBBase, hash_id: i64, kind: &str) -> HydrusResult<i64> {
    let count = match kind {
        "alternates" => db.query_one(
            "SELECT COALESCE(SUM(cnt - 1), 0) FROM (
                 SELECT COUNT(*) AS cnt FROM alternate_file_group_members
                 WHERE alternates_group_id IN (
                     SELECT alternates_group_id FROM alternate_file_group_members
                     WHERE hash_id = ?
                 )
                 GROUP BY alternates_group_id
             )",
            &[&hash_id],
            |row| row.get(0),
        )?,
        "potential duplicates" => db.query_one(
            "SELECT COUNT(*) FROM potential_duplicate_pairs
             WHERE smaller_hash_id = ? OR larger_hash_id = ?",
            &[&hash_id, &hash_id],
            |row| row.get(0),
        )?,
        "not related/false positive" => db.query_one(
            "SELECT COUNT(*) FROM duplicate_false_positives
             WHERE smaller_hash_id = ? OR larger_hash_id = ?",
            &[&hash_id, &hash_id],
            |row| row.get(0),
        )?,
        _ => db.query_one(
            "SELECT COALESCE(MAX(cnt) - 1, 0) FROM (
                 SELECT COUNT(*) AS cnt FROM duplicate_file_members
                 WHERE media_id IN (
                     SELECT media_id FROM duplicate_file_members WHERE hash_id = ?
                 )
                 GROUP BY media_id
             )",
            &[&hash_id],
            |row| row.get(0),
        )?,
    };

    Ok(count.unwrap_or(0))
}

pub fn get_alternate_group_ids_for_hash(db: &DBBase, hash_id: i64) -> HydrusResult<Vec<i64>> {
    db.query_all(
        "SELECT alternates_group_id FROM alternate_file_group_members
         WHERE hash_id = ? ORDER BY alternates_group_id",
        &[&hash_id],
        |row| row.get(0),
    )
}

pub fn get_alternate_group_hashes(db: &DBBase, group_id: i64) -> HydrusResult<Vec<i64>> {
    get_alternate_group_members(db, group_id)
}

pub fn get_duplicate_media_ids_for_hash(db: &DBBase, hash_id: i64) -> HydrusResult<Vec<i64>> {
    db.query_all(
        "SELECT media_id FROM duplicate_file_members
         WHERE hash_id = ? ORDER BY media_id",
        &[&hash_id],
        |row| row.get(0),
    )
}

/// Removes every duplicate/alternate relation for one alternate group and the
/// group itself, matching `DissolveAlternatesGroupId`.
pub fn delete_alternate_group(db: &DBBase, group_id: i64) -> HydrusResult<()> {
    let hash_ids = get_alternate_group_members(db, group_id)?;

    db.execute(
        "DELETE FROM alternate_file_group_members WHERE alternates_group_id = ?",
        &[&group_id],
    )?;
    db.execute(
        "DELETE FROM alternate_file_groups WHERE alternates_group_id = ?",
        &[&group_id],
    )?;

    if !hash_ids.is_empty() {
        delete_false_positives_for_hashes(db, &hash_ids)?;
    }

    Ok(())
}

pub fn delete_duplicate_group(db: &DBBase, media_id: i64) -> HydrusResult<()> {
    db.execute(
        "DELETE FROM duplicate_file_members WHERE media_id = ?",
        &[&media_id],
    )?;
    db.execute(
        "DELETE FROM duplicate_files WHERE media_id = ?",
        &[&media_id],
    )?;

    Ok(())
}

/// Pulls one file out of its alternate group. An empty group is removed.
pub fn remove_from_alternate_group(db: &DBBase, hash_id: i64) -> HydrusResult<usize> {
    let group_ids = get_alternate_group_ids_for_hash(db, hash_id)?;

    for group_id in &group_ids {
        db.execute(
            "DELETE FROM alternate_file_group_members
             WHERE alternates_group_id = ? AND hash_id = ?",
            &[group_id, &hash_id],
        )?;

        let remaining: i64 = db
            .query_one(
                "SELECT COUNT(*) FROM alternate_file_group_members
                 WHERE alternates_group_id = ?",
                &[group_id],
                |row| row.get(0),
            )?
            .unwrap_or(0);

        if remaining <= 1 {
            db.execute(
                "DELETE FROM alternate_file_group_members WHERE alternates_group_id = ?",
                &[group_id],
            )?;
            db.execute(
                "DELETE FROM alternate_file_groups WHERE alternates_group_id = ?",
                &[group_id],
            )?;
            db.execute(
                "DELETE FROM duplicate_false_positives
                 WHERE smaller_hash_id = ? OR larger_hash_id = ?",
                &[&hash_id, &hash_id],
            )?;
        }
    }

    Ok(group_ids.len())
}

/// Pulls one file out of its duplicate group. If it was the king the whole
/// group is dissolved, like `RemoveMediaIdMember`.
pub fn remove_from_duplicate_group(db: &DBBase, hash_id: i64) -> HydrusResult<usize> {
    let media_ids = get_duplicate_media_ids_for_hash(db, hash_id)?;
    let mut changed = 0usize;

    for media_id in &media_ids {
        let king = get_duplicate_king(db, *media_id)?.unwrap_or(0);

        if king == hash_id {
            delete_duplicate_group(db, *media_id)?;
        } else {
            db.execute(
                "DELETE FROM duplicate_file_members WHERE media_id = ? AND hash_id = ?",
                &[media_id, &hash_id],
            )?;
        }

        changed += 1;
    }

    Ok(changed)
}

/// Deletes every false-positive row that mentions one of the files.
pub fn delete_false_positives_for_hashes(db: &DBBase, hash_ids: &[i64]) -> HydrusResult<usize> {
    let mut deleted = 0usize;

    for hash_id in hash_ids {
        let before: i64 = db
            .query_one(
                "SELECT COUNT(*) FROM duplicate_false_positives
                 WHERE smaller_hash_id = ? OR larger_hash_id = ?",
                &[hash_id, hash_id],
                |row| row.get(0),
            )?
            .unwrap_or(0);

        db.execute(
            "DELETE FROM duplicate_false_positives
             WHERE smaller_hash_id = ? OR larger_hash_id = ?",
            &[hash_id, hash_id],
        )?;

        deleted += before as usize;
    }

    Ok(deleted)
}

/// Deletes false-positive rows where both sides are in the supplied set.
pub fn delete_internal_false_positives(db: &DBBase, hash_ids: &[i64]) -> HydrusResult<usize> {
    if hash_ids.len() < 2 {
        return Ok(0);
    }

    let mut deleted = 0usize;

    for (index, first) in hash_ids.iter().enumerate() {
        for second in hash_ids.iter().skip(index + 1) {
            let (smaller, larger) = if first < second {
                (*first, *second)
            } else {
                (*second, *first)
            };

            let before: i64 = db
                .query_one(
                    "SELECT COUNT(*) FROM duplicate_false_positives
                     WHERE smaller_hash_id = ? AND larger_hash_id = ?",
                    &[&smaller, &larger],
                    |row| row.get(0),
                )?
                .unwrap_or(0);

            db.execute(
                "DELETE FROM duplicate_false_positives
                 WHERE smaller_hash_id = ? AND larger_hash_id = ?",
                &[&smaller, &larger],
            )?;

            deleted += before as usize;
        }
    }

    Ok(deleted)
}

pub fn delete_potentials_for_hashes(db: &DBBase, hash_ids: &[i64]) -> HydrusResult<usize> {
    let mut deleted = 0usize;

    for hash_id in hash_ids {
        let before: i64 = db
            .query_one(
                "SELECT COUNT(*) FROM potential_duplicate_pairs
                 WHERE smaller_hash_id = ? OR larger_hash_id = ?",
                &[hash_id, hash_id],
                |row| row.get(0),
            )?
            .unwrap_or(0);

        db.execute(
            "DELETE FROM potential_duplicate_pairs
             WHERE smaller_hash_id = ? OR larger_hash_id = ?",
            &[hash_id, hash_id],
        )?;

        deleted += before as usize;
    }

    Ok(deleted)
}

/// Re-runs the shape/pixel comparison for one file and queues new pairs.
pub fn reset_potential_search(db: &DBBase, hash_id: i64, max_distance: i64) -> HydrusResult<usize> {
    let mut found: BTreeMap<(i64, i64), i64> = BTreeMap::new();

    for perceptual_hash in similar_files::get_perceptual_hashes(db, hash_id)? {
        for similar in similar_files::search_similar(
            db,
            &perceptual_hash,
            max_distance.max(0) as u32,
            Some(hash_id),
        )? {
            let (smaller, larger) = if similar.hash_id < hash_id {
                (similar.hash_id, hash_id)
            } else {
                (hash_id, similar.hash_id)
            };
            let distance = similar.distance as i64;

            found
                .entry((smaller, larger))
                .and_modify(|current| *current = (*current).min(distance))
                .or_insert(distance);
        }
    }

    if let Some(pixel_hash) = similar_files::get_pixel_hash(db, hash_id)? {
        for other in similar_files::get_hash_ids_for_pixel_hash(db, &pixel_hash)? {
            if other != hash_id {
                let (smaller, larger) = if other < hash_id {
                    (other, hash_id)
                } else {
                    (hash_id, other)
                };

                found.insert((smaller, larger), 0);
            }
        }
    }

    for ((smaller, larger), distance) in &found {
        add_potential_pair(db, *smaller, *larger, *distance)?;
    }

    Ok(found.len())
}

#[cfg(test)]
mod tests;
