//! Local ratings from `hydrus/client/db/ClientDBRatings.py`.

use hydrus_core::hydrus_db_base::DBBase;
use hydrus_core::hydrus_exceptions::HydrusResult;

use crate::metadata::Rating;

pub fn set_rating(db: &DBBase, service_id: i64, hash_id: i64, rating: Rating) -> HydrusResult<()> {
    match rating {
        Rating::Numerical(value) => {
            db.execute(
                "INSERT OR REPLACE INTO local_ratings
                 ( service_id, hash_id, rating ) VALUES ( ?, ?, ? )",
                &[&service_id, &hash_id, &value],
            )?;
        }
        Rating::Like => set_incdec_rating(db, service_id, hash_id, 1)?,
        Rating::Dislike => set_incdec_rating(db, service_id, hash_id, 0)?,
    }

    Ok(())
}

pub fn set_incdec_rating(
    db: &DBBase,
    service_id: i64,
    hash_id: i64,
    value: i64,
) -> HydrusResult<()> {
    db.execute(
        "INSERT OR REPLACE INTO local_incdec_ratings
         ( service_id, hash_id, rating ) VALUES ( ?, ?, ? )",
        &[&service_id, &hash_id, &value],
    )?;

    Ok(())
}

pub fn get_numerical_rating(
    db: &DBBase,
    service_id: i64,
    hash_id: i64,
) -> HydrusResult<Option<f64>> {
    db.query_one(
        "SELECT rating FROM local_ratings WHERE service_id = ? AND hash_id = ?",
        &[&service_id, &hash_id],
        |row| row.get(0),
    )
}

pub fn get_incdec_rating(db: &DBBase, service_id: i64, hash_id: i64) -> HydrusResult<Option<i64>> {
    db.query_one(
        "SELECT rating FROM local_incdec_ratings WHERE service_id = ? AND hash_id = ?",
        &[&service_id, &hash_id],
        |row| row.get(0),
    )
}

pub fn get_rating(db: &DBBase, service_id: i64, hash_id: i64) -> HydrusResult<Option<Rating>> {
    if let Some(value) = get_numerical_rating(db, service_id, hash_id)? {
        return Ok(Some(Rating::Numerical(value)));
    }

    Ok(get_incdec_rating(db, service_id, hash_id)?.map(|value| {
        if value > 0 {
            Rating::Like
        } else {
            Rating::Dislike
        }
    }))
}

pub fn delete_rating(db: &DBBase, service_id: i64, hash_id: i64) -> HydrusResult<()> {
    db.execute(
        "DELETE FROM local_ratings WHERE service_id = ? AND hash_id = ?",
        &[&service_id, &hash_id],
    )?;
    db.execute(
        "DELETE FROM local_incdec_ratings WHERE service_id = ? AND hash_id = ?",
        &[&service_id, &hash_id],
    )?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::ClientDB;
    use crate::db::master::get_hash_id;

    fn database() -> (ClientDB, std::path::PathBuf) {
        let directory = std::env::temp_dir().join(format!(
            "hydrus_client_ratings_{}_{}",
            std::process::id(),
            rand::random::<u64>()
        ));

        (ClientDB::new(&directory).unwrap(), directory)
    }

    #[test]
    fn numerical_ratings_round_trip() {
        let (db, directory) = database();
        let hash_id = get_hash_id(db.db(), &[1]).unwrap();

        set_rating(db.db(), 1, hash_id, Rating::Numerical(0.75)).unwrap();

        assert_eq!(
            get_rating(db.db(), 1, hash_id).unwrap(),
            Some(Rating::Numerical(0.75))
        );

        delete_rating(db.db(), 1, hash_id).unwrap();

        assert_eq!(get_rating(db.db(), 1, hash_id).unwrap(), None);

        drop(db);
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn like_dislike_use_incdec_table() {
        let (db, directory) = database();
        let hash_id = get_hash_id(db.db(), &[2]).unwrap();

        set_rating(db.db(), 3, hash_id, Rating::Like).unwrap();

        assert_eq!(get_rating(db.db(), 3, hash_id).unwrap(), Some(Rating::Like));

        set_rating(db.db(), 3, hash_id, Rating::Dislike).unwrap();

        assert_eq!(
            get_rating(db.db(), 3, hash_id).unwrap(),
            Some(Rating::Dislike)
        );

        drop(db);
        let _ = std::fs::remove_dir_all(directory);
    }
}
