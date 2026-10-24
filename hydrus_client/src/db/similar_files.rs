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
mod tests {
    use super::*;
    use crate::db::ClientDB;
    use crate::db::master::get_hash_id;

    fn database() -> (ClientDB, std::path::PathBuf) {
        let directory = std::env::temp_dir().join(format!(
            "hydrus_client_similar_{}_{}",
            std::process::id(),
            rand::random::<u64>()
        ));

        (ClientDB::new(&directory).unwrap(), directory)
    }

    #[test]
    fn perceptual_hashes_round_trip() {
        let (db, directory) = database();
        let hash_id = get_hash_id(db.db(), &[1]).unwrap();

        set_perceptual_hashes(db.db(), hash_id, &[vec![0xaa, 0xbb]]).unwrap();

        assert_eq!(
            get_perceptual_hashes(db.db(), hash_id).unwrap(),
            vec![vec![0xaa, 0xbb]]
        );

        drop(db);
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn similar_search_respects_distance() {
        let (db, directory) = database();
        let first = get_hash_id(db.db(), &[1]).unwrap();
        let second = get_hash_id(db.db(), &[2]).unwrap();
        let third = get_hash_id(db.db(), &[3]).unwrap();

        set_perceptual_hashes(db.db(), first, &[vec![0, 0, 0, 0, 0, 0, 0, 0]]).unwrap();
        set_perceptual_hashes(db.db(), second, &[vec![0, 0, 0, 0, 0, 0, 0, 1]]).unwrap();
        set_perceptual_hashes(db.db(), third, &[vec![255, 0, 0, 0, 0, 0, 0, 0]]).unwrap();

        let matches = search_similar(db.db(), &[0, 0, 0, 0, 0, 0, 0, 0], 2, Some(first)).unwrap();

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].hash_id, second);
        assert_eq!(matches[0].distance, 1);

        drop(db);
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn pixel_hashes_are_indexed() {
        let (db, directory) = database();
        let hash_id = get_hash_id(db.db(), &[1]).unwrap();

        set_pixel_hash(db.db(), hash_id, &[9, 9]).unwrap();

        assert_eq!(get_pixel_hash(db.db(), hash_id).unwrap(), Some(vec![9, 9]));
        assert_eq!(
            get_hash_ids_for_pixel_hash(db.db(), &[9, 9]).unwrap(),
            vec![hash_id]
        );

        drop(db);
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn search_distance_is_cached() {
        let (db, directory) = database();
        let hash_id = get_hash_id(db.db(), &[1]).unwrap();

        assert_eq!(get_search_distance(db.db(), hash_id).unwrap(), None);

        set_search_distance(db.db(), hash_id, 4).unwrap();

        assert_eq!(get_search_distance(db.db(), hash_id).unwrap(), Some(4));

        drop(db);
        let _ = std::fs::remove_dir_all(directory);
    }
}
