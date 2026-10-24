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

pub fn add_false_positive(db: &DBBase, first_hash_id: i64, second_hash_id: i64) -> HydrusResult<()> {
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
mod tests {
    use super::*;
    use crate::db::master::get_hash_id;
    use crate::db::ClientDB;

    fn database() -> (ClientDB, std::path::PathBuf) {
        let directory = std::env::temp_dir().join(format!(
            "hydrus_client_duplicates_{}_{}",
            std::process::id(),
            rand::random::<u64>()
        ));

        (ClientDB::new(&directory).unwrap(), directory)
    }

    #[test]
    fn potential_pairs_round_trip_and_filter() {
        let (db, directory) = database();
        let first = get_hash_id(db.db(), &[1]).unwrap();
        let second = get_hash_id(db.db(), &[2]).unwrap();

        add_potential_pair(db.db(), first, second, 4).unwrap();
        add_potential_pair(db.db(), second, first + 100, 10).unwrap();

        assert_eq!(get_potential_pairs(db.db(), Some(5)).unwrap(), vec![(first, second, 4)]);

        remove_potential_pair(db.db(), first, second).unwrap();

        assert_eq!(get_potential_pairs(db.db(), None).unwrap().len(), 1);

        drop(db);
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn false_positives_are_recorded() {
        let (db, directory) = database();
        let first = get_hash_id(db.db(), &[1]).unwrap();
        let second = get_hash_id(db.db(), &[2]).unwrap();

        assert!(!is_false_positive(db.db(), first, second).unwrap());

        add_false_positive(db.db(), first, second).unwrap();

        assert!(is_false_positive(db.db(), first, second).unwrap());

        drop(db);
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn alternate_groups_store_members() {
        let (db, directory) = database();
        let first = get_hash_id(db.db(), &[1]).unwrap();
        let second = get_hash_id(db.db(), &[2]).unwrap();

        let group = create_alternate_group(db.db(), &[first]).unwrap();
        add_to_alternate_group(db.db(), group, second).unwrap();

        assert_eq!(
            get_alternate_group_members(db.db(), group).unwrap(),
            vec![first, second]
        );

        drop(db);
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn duplicate_groups_have_a_king() {
        let (db, directory) = database();
        let king = get_hash_id(db.db(), &[1]).unwrap();
        let member = get_hash_id(db.db(), &[2]).unwrap();

        set_duplicate_group(db.db(), 1, king, &[king, member]).unwrap();

        assert_eq!(get_duplicate_king(db.db(), 1).unwrap(), Some(king));
        assert_eq!(
            get_duplicate_group_members(db.db(), 1).unwrap(),
            vec![king, member]
        );

        drop(db);
        let _ = std::fs::remove_dir_all(directory);
    }
}
