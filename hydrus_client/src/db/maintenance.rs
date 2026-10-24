//! Database maintenance bookkeeping from `hydrus/client/db/ClientDBMaintenance.py`.

use hydrus_core::hydrus_db_base::DBBase;
use hydrus_core::hydrus_exceptions::HydrusResult;

pub fn set_last_shutdown_work_time(db: &DBBase, name: &str, timestamp: i64) -> HydrusResult<()> {
    db.execute(
        "INSERT OR REPLACE INTO last_shutdown_work_time ( name, timestamp ) VALUES ( ?, ? )",
        &[&name, &timestamp],
    )?;

    Ok(())
}

pub fn get_last_shutdown_work_time(db: &DBBase, name: &str) -> HydrusResult<Option<i64>> {
    db.query_one(
        "SELECT timestamp FROM last_shutdown_work_time WHERE name = ?",
        &[&name],
        |row| row.get(0),
    )
}

pub fn set_analyze_timestamp(db: &DBBase, table_name: &str, timestamp: i64) -> HydrusResult<()> {
    db.execute(
        "INSERT OR REPLACE INTO analyze_timestamps ( table_name, timestamp ) VALUES ( ?, ? )",
        &[&table_name, &timestamp],
    )?;

    Ok(())
}

pub fn get_analyze_timestamp(db: &DBBase, table_name: &str) -> HydrusResult<Option<i64>> {
    db.query_one(
        "SELECT timestamp FROM analyze_timestamps WHERE table_name = ?",
        &[&table_name],
        |row| row.get(0),
    )
}

pub fn set_vacuum_timestamp(db: &DBBase, table_name: &str, timestamp: i64) -> HydrusResult<()> {
    db.execute(
        "INSERT OR REPLACE INTO vacuum_timestamps ( table_name, timestamp ) VALUES ( ?, ? )",
        &[&table_name, &timestamp],
    )?;

    Ok(())
}

pub fn get_vacuum_timestamp(db: &DBBase, table_name: &str) -> HydrusResult<Option<i64>> {
    db.query_one(
        "SELECT timestamp FROM vacuum_timestamps WHERE table_name = ?",
        &[&table_name],
        |row| row.get(0),
    )
}

pub fn schedule_table_delete(db: &DBBase, table_name: &str, priority: i64) -> HydrusResult<()> {
    db.execute(
        "INSERT OR REPLACE INTO deferred_delete_tables ( table_name, priority ) VALUES ( ?, ? )",
        &[&table_name, &priority],
    )?;

    Ok(())
}

pub fn cancel_table_delete(db: &DBBase, table_name: &str) -> HydrusResult<()> {
    db.execute(
        "DELETE FROM deferred_delete_tables WHERE table_name = ?",
        &[&table_name],
    )?;

    Ok(())
}

pub fn get_deferred_table_deletes(db: &DBBase) -> HydrusResult<Vec<(String, i64)>> {
    db.query_all(
        "SELECT table_name, priority FROM deferred_delete_tables
         ORDER BY priority DESC, table_name ASC",
        &[],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )
}

pub fn pop_deferred_table_delete(db: &DBBase) -> HydrusResult<Option<String>> {
    let Some((table_name, _)) = get_deferred_table_deletes(db)?.into_iter().next() else {
        return Ok(None);
    };

    cancel_table_delete(db, &table_name)?;

    Ok(Some(table_name))
}

pub fn drop_table_if_deferred(db: &DBBase) -> HydrusResult<Option<String>> {
    let Some(table_name) = pop_deferred_table_delete(db)? else {
        return Ok(None);
    };

    if db.table_exists(&table_name)? {
        db.execute_batch(&format!("DROP TABLE {table_name};"))?;
    }

    Ok(Some(table_name))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::ClientDB;

    fn database() -> (ClientDB, std::path::PathBuf) {
        let directory = std::env::temp_dir().join(format!(
            "hydrus_client_maintenance_{}_{}",
            std::process::id(),
            rand::random::<u64>()
        ));

        (ClientDB::new(&directory).unwrap(), directory)
    }

    #[test]
    fn shutdown_work_time_round_trips() {
        let (db, directory) = database();

        set_last_shutdown_work_time(db.db(), "vacuum", 1234).unwrap();

        assert_eq!(
            get_last_shutdown_work_time(db.db(), "vacuum").unwrap(),
            Some(1234)
        );

        drop(db);
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn analyze_and_vacuum_timestamps_are_per_table() {
        let (db, directory) = database();

        set_analyze_timestamp(db.db(), "files_info", 10).unwrap();
        set_vacuum_timestamp(db.db(), "files_info", 20).unwrap();

        assert_eq!(
            get_analyze_timestamp(db.db(), "files_info").unwrap(),
            Some(10)
        );
        assert_eq!(
            get_vacuum_timestamp(db.db(), "files_info").unwrap(),
            Some(20)
        );

        drop(db);
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn deferred_deletes_are_priority_ordered() {
        let (db, directory) = database();

        schedule_table_delete(db.db(), "low", 1).unwrap();
        schedule_table_delete(db.db(), "high", 10).unwrap();

        assert_eq!(
            get_deferred_table_deletes(db.db()).unwrap(),
            vec![("high".to_string(), 10), ("low".to_string(), 1)]
        );
        assert_eq!(
            pop_deferred_table_delete(db.db()).unwrap().as_deref(),
            Some("high")
        );
        assert_eq!(
            get_deferred_table_deletes(db.db()).unwrap(),
            vec![("low".to_string(), 1)]
        );

        drop(db);
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn deferred_table_is_dropped() {
        let (db, directory) = database();

        db.db()
            .execute_batch("CREATE TABLE doomed ( x INTEGER );")
            .unwrap();
        schedule_table_delete(db.db(), "doomed", 0).unwrap();

        assert_eq!(
            drop_table_if_deferred(db.db()).unwrap().as_deref(),
            Some("doomed")
        );
        assert!(!db.db().table_exists("doomed").unwrap());

        drop(db);
        let _ = std::fs::remove_dir_all(directory);
    }
}
