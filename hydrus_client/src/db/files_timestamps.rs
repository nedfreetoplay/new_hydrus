//! File timestamps from `hydrus/client/db/ClientDBFilesTimestamps.py`.

use hydrus_core::hydrus_constants::TimestampType;
use hydrus_core::hydrus_db_base::DBBase;
use hydrus_core::hydrus_exceptions::HydrusResult;

fn table_for(timestamp_type: TimestampType) -> Option<&'static str> {
    match timestamp_type {
        TimestampType::Archived => Some("archive_timestamps"),
        TimestampType::ModifiedFile => Some("file_modified_timestamps"),
        _ => None,
    }
}

pub fn set_file_timestamp(
    db: &DBBase,
    hash_id: i64,
    timestamp_type: TimestampType,
    timestamp: Option<i64>,
) -> HydrusResult<()> {
    let Some(table) = table_for(timestamp_type) else {
        return Ok(());
    };

    if let Some(timestamp) = timestamp {
        db.execute(
            &format!("INSERT OR REPLACE INTO {table} ( hash_id, timestamp ) VALUES ( ?, ? )"),
            &[&hash_id, &timestamp],
        )?;
    } else {
        db.execute(
            &format!("DELETE FROM {table} WHERE hash_id = ?"),
            &[&hash_id],
        )?;
    }

    Ok(())
}

pub fn get_file_timestamp(
    db: &DBBase,
    hash_id: i64,
    timestamp_type: TimestampType,
) -> HydrusResult<Option<i64>> {
    let Some(table) = table_for(timestamp_type) else {
        return Ok(None);
    };

    db.query_one(
        &format!("SELECT timestamp FROM {table} WHERE hash_id = ?"),
        &[&hash_id],
        |row| row.get(0),
    )
    .map(|value| value.flatten())
}

pub fn set_file_domain_timestamp(
    db: &DBBase,
    service_id: i64,
    hash_id: i64,
    timestamp: Option<i64>,
) -> HydrusResult<()> {
    if let Some(timestamp) = timestamp {
        db.execute(
            "INSERT OR REPLACE INTO file_domain_modified_timestamps
             ( service_id, hash_id, timestamp ) VALUES ( ?, ?, ? )",
            &[&service_id, &hash_id, &timestamp],
        )?;
    } else {
        db.execute(
            "DELETE FROM file_domain_modified_timestamps WHERE service_id = ? AND hash_id = ?",
            &[&service_id, &hash_id],
        )?;
    }

    Ok(())
}

pub fn get_file_domain_timestamp(
    db: &DBBase,
    service_id: i64,
    hash_id: i64,
) -> HydrusResult<Option<i64>> {
    db.query_one(
        "SELECT timestamp FROM file_domain_modified_timestamps
         WHERE service_id = ? AND hash_id = ?",
        &[&service_id, &hash_id],
        |row| row.get(0),
    )
    .map(|value| value.flatten())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::master::get_hash_id;
    use crate::db::ClientDB;

    fn database() -> (ClientDB, std::path::PathBuf) {
        let directory = std::env::temp_dir().join(format!(
            "hydrus_client_timestamps_{}_{}",
            std::process::id(),
            rand::random::<u64>()
        ));

        (ClientDB::new(&directory).unwrap(), directory)
    }

    #[test]
    fn archive_timestamps_round_trip() {
        let (db, directory) = database();
        let hash_id = get_hash_id(db.db(), &[1]).unwrap();

        set_file_timestamp(db.db(), hash_id, TimestampType::Archived, Some(1234)).unwrap();

        assert_eq!(
            get_file_timestamp(db.db(), hash_id, TimestampType::Archived).unwrap(),
            Some(1234)
        );

        set_file_timestamp(db.db(), hash_id, TimestampType::Archived, None).unwrap();

        assert_eq!(
            get_file_timestamp(db.db(), hash_id, TimestampType::Archived).unwrap(),
            None
        );

        drop(db);
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn domain_timestamps_are_per_service() {
        let (db, directory) = database();
        let hash_id = get_hash_id(db.db(), &[2]).unwrap();

        set_file_domain_timestamp(db.db(), 1, hash_id, Some(10)).unwrap();
        set_file_domain_timestamp(db.db(), 2, hash_id, Some(20)).unwrap();

        assert_eq!(
            get_file_domain_timestamp(db.db(), 1, hash_id).unwrap(),
            Some(10)
        );
        assert_eq!(
            get_file_domain_timestamp(db.db(), 2, hash_id).unwrap(),
            Some(20)
        );

        drop(db);
        let _ = std::fs::remove_dir_all(directory);
    }
}
