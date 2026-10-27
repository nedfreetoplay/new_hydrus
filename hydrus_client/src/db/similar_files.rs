//! Similar files storage from `hydrus/client/db/ClientDBSimilarFiles.py`.

use hydrus_core::hydrus_data::get_64_bit_hamming_distance;
use hydrus_core::hydrus_db_base::DBBase;
use hydrus_core::hydrus_exceptions::HydrusResult;

pub fn set_perceptual_hashes(
    db: &DBBase,
    hash_id: i64,
    perceptual_hashes: &[Vec<u8>],
) -> HydrusResult<()> {
    db.execute(
        "DELETE FROM shape_perceptual_hashes WHERE hash_id = ?",
        &[&hash_id],
    )?;
    db.execute(
        "DELETE FROM shape_perceptual_hash_map WHERE hash_id = ?",
        &[&hash_id],
    )?;

    for perceptual_hash in perceptual_hashes {
        db.execute(
            "INSERT OR IGNORE INTO shape_perceptual_hashes ( hash_id, perceptual_hash )
             VALUES ( ?, ? )",
            &[&hash_id, &perceptual_hash],
        )?;
        db.execute(
            "INSERT OR IGNORE INTO shape_perceptual_hash_map ( perceptual_hash, hash_id )
             VALUES ( ?, ? )",
            &[&perceptual_hash, &hash_id],
        )?;
    }

    Ok(())
}

pub fn get_perceptual_hashes(db: &DBBase, hash_id: i64) -> HydrusResult<Vec<Vec<u8>>> {
    db.query_all(
        "SELECT perceptual_hash FROM shape_perceptual_hashes WHERE hash_id = ?",
        &[&hash_id],
        |row| row.get(0),
    )
}

pub fn set_pixel_hash(db: &DBBase, hash_id: i64, pixel_hash: &[u8]) -> HydrusResult<()> {
    db.execute("DELETE FROM pixel_hash_map WHERE hash_id = ?", &[&hash_id])?;
    db.execute(
        "INSERT OR IGNORE INTO pixel_hash_map ( pixel_hash, hash_id ) VALUES ( ?, ? )",
        &[&pixel_hash, &hash_id],
    )?;

    Ok(())
}

pub fn get_pixel_hash(db: &DBBase, hash_id: i64) -> HydrusResult<Option<Vec<u8>>> {
    db.query_one(
        "SELECT pixel_hash FROM pixel_hash_map WHERE hash_id = ?",
        &[&hash_id],
        |row| row.get(0),
    )
}

pub fn get_hash_ids_for_pixel_hash(db: &DBBase, pixel_hash: &[u8]) -> HydrusResult<Vec<i64>> {
    db.query_all(
        "SELECT hash_id FROM pixel_hash_map WHERE pixel_hash = ?",
        &[&pixel_hash],
        |row| row.get(0),
    )
}

pub fn get_hash_ids_for_perceptual_hash(
    db: &DBBase,
    perceptual_hash: &[u8],
) -> HydrusResult<Vec<i64>> {
    db.query_all(
        "SELECT hash_id FROM shape_perceptual_hash_map WHERE perceptual_hash = ?",
        &[&perceptual_hash],
        |row| row.get(0),
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SimilarFile {
    pub hash_id: i64,
    pub distance: u32,
}

/// Brute-force shape search. The Python implementation uses a vantage-point
/// tree; the storage layout here is ready for that index.
pub fn search_similar(
    db: &DBBase,
    perceptual_hash: &[u8],
    max_distance: u32,
    exclude_hash_id: Option<i64>,
) -> HydrusResult<Vec<SimilarFile>> {
    let rows: Vec<(Vec<u8>, i64)> = db.query_all(
        "SELECT perceptual_hash, hash_id FROM shape_perceptual_hash_map",
        &[],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;

    let mut matches: Vec<SimilarFile> = rows
        .into_iter()
        .filter(|(_, hash_id)| Some(*hash_id) != exclude_hash_id)
        .filter_map(|(candidate, hash_id)| {
            if candidate.len() != perceptual_hash.len() {
                return None;
            }

            let distance = if candidate.len() <= 8 {
                get_64_bit_hamming_distance(
                    &pad_to_eight(&candidate),
                    &pad_to_eight(perceptual_hash),
                )
            } else {
                candidate
                    .iter()
                    .zip(perceptual_hash.iter())
                    .map(|(a, b)| (a ^ b).count_ones())
                    .sum()
            };

            (distance <= max_distance).then_some(SimilarFile { hash_id, distance })
        })
        .collect();

    matches.sort_by_key(|file| (file.distance, file.hash_id));

    Ok(matches)
}

fn pad_to_eight(value: &[u8]) -> [u8; 8] {
    let mut padded = [0u8; 8];
    let length = value.len().min(8);
    padded[..length].copy_from_slice(&value[..length]);
    padded
}

pub fn set_search_distance(db: &DBBase, hash_id: i64, distance: i64) -> HydrusResult<()> {
    db.execute(
        "INSERT OR REPLACE INTO shape_search_cache ( hash_id, searched_distance )
         VALUES ( ?, ? )",
        &[&hash_id, &distance],
    )?;

    Ok(())
}

pub fn get_search_distance(db: &DBBase, hash_id: i64) -> HydrusResult<Option<i64>> {
    db.query_one(
        "SELECT searched_distance FROM shape_search_cache WHERE hash_id = ?",
        &[&hash_id],
        |row| row.get(0),
    )
}

#[cfg(test)]
mod tests;
