//! Per-service file storage tables from `hydrus/client/db/ClientDBFilesStorage.py`.

use hydrus_core::hydrus_constants::ContentStatus;
use hydrus_core::hydrus_db_base::DBBase;
use hydrus_core::hydrus_exceptions::{ErrorKind, HydrusException, HydrusResult};

fn table_name(service_id: i64, status: ContentStatus) -> String {
    let suffix = match status {
        ContentStatus::Current => "current_files",
        ContentStatus::Pending => "pending_files",
        ContentStatus::Deleted => "deleted_files",
        ContentStatus::Petitioned => "petitioned_files",
    };

    format!("{suffix}_{service_id}")
}

pub fn create_tables(db: &DBBase, service_id: i64) -> HydrusResult<()> {
    for status in [
        ContentStatus::Current,
        ContentStatus::Pending,
        ContentStatus::Deleted,
        ContentStatus::Petitioned,
    ] {
        let table = table_name(service_id, status);
        let timestamp = matches!(status, ContentStatus::Current | ContentStatus::Deleted);

        if timestamp {
            db.execute_batch(&format!(
                "CREATE TABLE IF NOT EXISTS {table} (
                    hash_id INTEGER PRIMARY KEY,
                    timestamp INTEGER
                );"
            ))?;
        } else {
            db.execute_batch(&format!(
                "CREATE TABLE IF NOT EXISTS {table} (
                    hash_id INTEGER PRIMARY KEY
                );"
            ))?;
        }
    }

    Ok(())
}

pub fn add_files(
    db: &DBBase,
    service_id: i64,
    status: ContentStatus,
    hash_ids: &[i64],
    timestamp: Option<i64>,
) -> HydrusResult<()> {
    create_tables(db, service_id)?;
    let table = table_name(service_id, status);

    for hash_id in hash_ids {
        if matches!(status, ContentStatus::Current | ContentStatus::Deleted) {
            db.execute(
                &format!("INSERT OR REPLACE INTO {table} ( hash_id, timestamp ) VALUES ( ?, ? )"),
                &[
                    hash_id,
                    &timestamp.unwrap_or_else(hydrus_core::hydrus_time::get_now),
                ],
            )?;
        } else {
            db.execute(
                &format!("INSERT OR IGNORE INTO {table} ( hash_id ) VALUES ( ? )"),
                &[hash_id],
            )?;
        }
    }

    Ok(())
}

pub fn remove_files(
    db: &DBBase,
    service_id: i64,
    status: ContentStatus,
    hash_ids: &[i64],
) -> HydrusResult<()> {
    let table = table_name(service_id, status);

    for hash_id in hash_ids {
        db.execute(
            &format!("DELETE FROM {table} WHERE hash_id = ?"),
            &[hash_id],
        )?;
    }

    Ok(())
}

pub fn get_hash_ids(db: &DBBase, service_id: i64, status: ContentStatus) -> HydrusResult<Vec<i64>> {
    let table = table_name(service_id, status);

    if !db.table_exists(&table)? {
        return Ok(Vec::new());
    }

    db.query_all(&format!("SELECT hash_id FROM {table}"), &[], |row| {
        row.get(0)
    })
}

pub fn get_hashes(
    db: &DBBase,
    service_id: i64,
    status: ContentStatus,
) -> HydrusResult<Vec<Vec<u8>>> {
    let table = table_name(service_id, status);

    if !db.table_exists(&table)? {
        return Ok(Vec::new());
    }

    db.query_all(
        &format!(
            "SELECT hashes.hash FROM {table} files, hashes
             WHERE files.hash_id = hashes.hash_id"
        ),
        &[],
        |row| row.get(0),
    )
}

pub fn set_inbox(db: &DBBase, hash_ids: &[i64], inbox: bool) -> HydrusResult<()> {
    for hash_id in hash_ids {
        if inbox {
            db.execute(
                "INSERT OR IGNORE INTO file_inbox ( hash_id ) VALUES ( ? )",
                &[hash_id],
            )?;
        } else {
            db.execute("DELETE FROM file_inbox WHERE hash_id = ?", &[hash_id])?;
        }
    }

    Ok(())
}

pub fn get_inbox_hash_ids(db: &DBBase) -> HydrusResult<Vec<i64>> {
    db.query_all("SELECT hash_id FROM file_inbox", &[], |row| row.get(0))
}

pub fn get_timestamp(
    db: &DBBase,
    service_id: i64,
    status: ContentStatus,
    hash_id: i64,
) -> HydrusResult<Option<i64>> {
    let table = table_name(service_id, status);

    if !db.table_exists(&table)? {
        return Ok(None);
    }

    db.query_one(
        &format!("SELECT timestamp FROM {table} WHERE hash_id = ?"),
        &[&hash_id],
        |row| row.get(0),
    )
    .map(|value| value.flatten())
}

pub fn is_inbox(db: &DBBase, hash_id: i64) -> HydrusResult<bool> {
    let count: i64 = db
        .query_one(
            "SELECT COUNT(*) FROM file_inbox WHERE hash_id = ?",
            &[&hash_id],
            |row| row.get(0),
        )?
        .unwrap_or(0);

    Ok(count > 0)
}

pub fn get_num_inbox(db: &DBBase) -> HydrusResult<i64> {
    Ok(db
        .query_one("SELECT COUNT(*) FROM file_inbox", &[], |row| row.get(0))?
        .unwrap_or(0))
}

pub fn get_num_archive(db: &DBBase, service_id: i64) -> HydrusResult<i64> {
    let current = get_hash_ids(db, service_id, ContentStatus::Current)?.len() as i64;
    let inbox = get_num_inbox(db)?;

    Ok((current - inbox).max(0))
}

pub fn delete_for_service(db: &DBBase, service_id: i64) -> HydrusResult<()> {
    for status in [
        ContentStatus::Current,
        ContentStatus::Pending,
        ContentStatus::Deleted,
        ContentStatus::Petitioned,
    ] {
        let table = table_name(service_id, status);

        if !db.table_exists(&table)? {
            return Err(HydrusException::new(
                ErrorKind::DataMissing,
                format!("file table {table} does not exist"),
            ));
        }

        db.execute(&format!("DELETE FROM {table}"), &[])?;
    }

    Ok(())
}

#[cfg(test)]
mod tests;
