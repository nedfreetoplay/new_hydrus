//! Per-service tag mappings from `hydrus/client/db/ClientDBMappingsStorage.py`.

use hydrus_core::hydrus_constants::ContentStatus;
use hydrus_core::hydrus_db_base::DBBase;
use hydrus_core::hydrus_exceptions::HydrusResult;

use crate::db::master_tags;

fn table_name(service_id: i64, status: ContentStatus) -> String {
    let prefix = match status {
        ContentStatus::Current => "current_mappings",
        ContentStatus::Pending => "pending_mappings",
        ContentStatus::Deleted => "deleted_mappings",
        ContentStatus::Petitioned => "petitioned_mappings",
    };

    format!("{prefix}_{service_id}")
}

pub fn create_tables(db: &DBBase, service_id: i64) -> HydrusResult<()> {
    for status in [
        ContentStatus::Current,
        ContentStatus::Pending,
        ContentStatus::Deleted,
        ContentStatus::Petitioned,
    ] {
        let table = table_name(service_id, status);
        db.execute_batch(&format!(
            "CREATE TABLE IF NOT EXISTS {table} (
                hash_id INTEGER,
                tag_id INTEGER,
                PRIMARY KEY ( hash_id, tag_id )
            );
            CREATE INDEX IF NOT EXISTS {table}_tag_id_index ON {table} ( tag_id );"
        ))?;
    }

    Ok(())
}

pub fn add_mappings(
    db: &DBBase,
    service_id: i64,
    status: ContentStatus,
    hash_id: i64,
    tag_ids: &[i64],
) -> HydrusResult<()> {
    create_tables(db, service_id)?;
    let table = table_name(service_id, status);

    for tag_id in tag_ids {
        db.execute(
            &format!("INSERT OR IGNORE INTO {table} ( hash_id, tag_id ) VALUES ( ?, ? )"),
            &[&hash_id, &tag_id],
        )?;
    }

    Ok(())
}

pub fn remove_mappings(
    db: &DBBase,
    service_id: i64,
    status: ContentStatus,
    hash_id: i64,
    tag_ids: &[i64],
) -> HydrusResult<()> {
    let table = table_name(service_id, status);

    for tag_id in tag_ids {
        db.execute(
            &format!("DELETE FROM {table} WHERE hash_id = ? AND tag_id = ?"),
            &[&hash_id, &tag_id],
        )?;
    }

    Ok(())
}

pub fn get_tag_ids(
    db: &DBBase,
    service_id: i64,
    status: ContentStatus,
    hash_id: i64,
) -> HydrusResult<Vec<i64>> {
    let table = table_name(service_id, status);

    if !db.table_exists(&table)? {
        return Ok(Vec::new());
    }

    db.query_all(
        &format!("SELECT tag_id FROM {table} WHERE hash_id = ?"),
        &[&hash_id],
        |row| row.get(0),
    )
}

pub fn get_hash_ids(
    db: &DBBase,
    service_id: i64,
    status: ContentStatus,
    tag_id: i64,
) -> HydrusResult<Vec<i64>> {
    let table = table_name(service_id, status);

    if !db.table_exists(&table)? {
        return Ok(Vec::new());
    }

    db.query_all(
        &format!("SELECT hash_id FROM {table} WHERE tag_id = ?"),
        &[&tag_id],
        |row| row.get(0),
    )
}

pub fn get_tags(
    db: &DBBase,
    service_id: i64,
    status: ContentStatus,
    hash_id: i64,
) -> HydrusResult<Vec<String>> {
    let tag_ids = get_tag_ids(db, service_id, status, hash_id)?;

    master_tags::get_tags(db, &tag_ids)
}

pub fn get_num_mappings(
    db: &DBBase,
    service_id: i64,
    status: ContentStatus,
) -> HydrusResult<i64> {
    let table = table_name(service_id, status);

    if !db.table_exists(&table)? {
        return Ok(0);
    }

    Ok(db
        .query_one(&format!("SELECT COUNT(*) FROM {table}"), &[], |row| {
            row.get(0)
        })?
        .unwrap_or(0))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::master::get_hash_id;
    use crate::db::master_tags::get_tag_id;
    use crate::db::ClientDB;

    fn database() -> (ClientDB, std::path::PathBuf) {
        let directory = std::env::temp_dir().join(format!(
            "hydrus_client_mappings_{}_{}",
            std::process::id(),
            rand::random::<u64>()
        ));

        (ClientDB::new(&directory).unwrap(), directory)
    }

    #[test]
    fn mappings_round_trip_both_ways() {
        let (db, directory) = database();
        let hash_id = get_hash_id(db.db(), &[1]).unwrap();
        let tag_id = get_tag_id(db.db(), "blue eyes").unwrap();

        add_mappings(
            db.db(),
            1,
            ContentStatus::Current,
            hash_id,
            &[tag_id],
        )
        .unwrap();

        assert_eq!(
            get_tag_ids(db.db(), 1, ContentStatus::Current, hash_id).unwrap(),
            vec![tag_id]
        );
        assert_eq!(
            get_hash_ids(db.db(), 1, ContentStatus::Current, tag_id).unwrap(),
            vec![hash_id]
        );
        assert_eq!(
            get_tags(db.db(), 1, ContentStatus::Current, hash_id).unwrap(),
            vec!["blue eyes".to_string()]
        );
        assert_eq!(
            get_num_mappings(db.db(), 1, ContentStatus::Current).unwrap(),
            1
        );

        drop(db);
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn statuses_and_services_are_independent() {
        let (db, directory) = database();
        let hash_id = get_hash_id(db.db(), &[2]).unwrap();
        let tag_id = get_tag_id(db.db(), "tag").unwrap();

        add_mappings(db.db(), 1, ContentStatus::Current, hash_id, &[tag_id]).unwrap();
        add_mappings(db.db(), 1, ContentStatus::Deleted, hash_id, &[tag_id]).unwrap();
        add_mappings(db.db(), 2, ContentStatus::Current, hash_id, &[tag_id]).unwrap();

        assert_eq!(
            get_tag_ids(db.db(), 1, ContentStatus::Current, hash_id).unwrap().len(),
            1
        );
        assert_eq!(
            get_tag_ids(db.db(), 1, ContentStatus::Deleted, hash_id).unwrap().len(),
            1
        );
        assert_eq!(
            get_tag_ids(db.db(), 2, ContentStatus::Current, hash_id).unwrap().len(),
            1
        );

        drop(db);
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn removing_mappings_deletes_rows() {
        let (db, directory) = database();
        let hash_id = get_hash_id(db.db(), &[3]).unwrap();
        let tag_id = get_tag_id(db.db(), "tag").unwrap();

        add_mappings(db.db(), 1, ContentStatus::Current, hash_id, &[tag_id]).unwrap();
        remove_mappings(db.db(), 1, ContentStatus::Current, hash_id, &[tag_id]).unwrap();

        assert!(get_tag_ids(db.db(), 1, ContentStatus::Current, hash_id)
            .unwrap()
            .is_empty());

        drop(db);
        let _ = std::fs::remove_dir_all(directory);
    }
}
