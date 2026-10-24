//! Service path metadata from `hydrus/client/db/ClientDBServicePaths.py`.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use hydrus_core::hydrus_db_base::DBBase;
use hydrus_core::hydrus_exceptions::HydrusResult;

pub fn set_service_filename(
    db: &DBBase,
    service_id: i64,
    hash_id: i64,
    filename: &str,
) -> HydrusResult<()> {
    db.execute(
        "INSERT OR REPLACE INTO service_filenames ( service_id, hash_id, filename )
         VALUES ( ?, ?, ? )",
        &[&service_id, &hash_id, &filename],
    )?;

    Ok(())
}

pub fn get_service_filename(
    db: &DBBase,
    service_id: i64,
    hash_id: i64,
) -> HydrusResult<Option<String>> {
    db.query_one(
        "SELECT filename FROM service_filenames WHERE service_id = ? AND hash_id = ?",
        &[&service_id, &hash_id],
        |row| row.get(0),
    )
}

pub fn get_service_filenames(
    db: &DBBase,
    service_id: i64,
    hash_ids: &[i64],
) -> HydrusResult<Vec<(i64, String)>> {
    let mut filenames = Vec::new();

    for hash_id in hash_ids {
        if let Some(filename) = get_service_filename(db, service_id, *hash_id)? {
            filenames.push((*hash_id, filename));
        }
    }

    Ok(filenames)
}

pub fn set_service_directory(
    db: &DBBase,
    service_id: i64,
    hash_ids: &[i64],
    dirname: &str,
    total_size: i64,
    note: &str,
) -> HydrusResult<()> {
    let directory_id = directory_id(dirname);

    db.execute(
        "INSERT OR REPLACE INTO service_directories
         ( service_id, directory_id, dirname, num_files, total_size, note )
         VALUES ( ?, ?, ?, ?, ?, ? )",
        &[
            &service_id,
            &directory_id,
            &dirname,
            &(hash_ids.len() as i64),
            &total_size,
            &note,
        ],
    )?;

    for hash_id in hash_ids {
        db.execute(
            "INSERT OR IGNORE INTO service_directory_file_map
             ( service_id, directory_id, hash_id ) VALUES ( ?, ?, ? )",
            &[&service_id, &directory_id, &hash_id],
        )?;
    }

    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceDirectoryInfo {
    pub dirname: String,
    pub num_files: i64,
    pub total_size: i64,
    pub note: String,
}

pub fn get_service_directories_info(
    db: &DBBase,
    service_id: i64,
) -> HydrusResult<Vec<ServiceDirectoryInfo>> {
    db.query_all(
        "SELECT dirname, num_files, total_size, note FROM service_directories
         WHERE service_id = ? ORDER BY dirname",
        &[&service_id],
        |row| {
            Ok(ServiceDirectoryInfo {
                dirname: row.get(0)?,
                num_files: row.get(1)?,
                total_size: row.get(2)?,
                note: row.get(3)?,
            })
        },
    )
}

pub fn get_service_directory_hashes(
    db: &DBBase,
    service_id: i64,
    dirname: &str,
) -> HydrusResult<Vec<i64>> {
    db.query_all(
        "SELECT hash_id FROM service_directory_file_map
         WHERE service_id = ? AND directory_id = ?",
        &[&service_id, &directory_id(dirname)],
        |row| row.get(0),
    )
}

pub fn clear_service(db: &DBBase, service_id: i64) -> HydrusResult<()> {
    db.execute(
        "DELETE FROM service_filenames WHERE service_id = ?",
        &[&service_id],
    )?;
    db.execute(
        "DELETE FROM service_directory_file_map WHERE service_id = ?",
        &[&service_id],
    )?;
    db.execute(
        "DELETE FROM service_directories WHERE service_id = ?",
        &[&service_id],
    )?;

    Ok(())
}

fn directory_id(dirname: &str) -> i64 {
    let mut hasher = DefaultHasher::new();
    dirname.hash(&mut hasher);

    (hasher.finish() & 0x7fff_ffff_ffff_ffff) as i64
}

#[cfg(test)]
mod tests;
