//! Mapping counts from `hydrus/client/db/ClientDBMappingsCounts.py`.

use std::collections::BTreeMap;

use hydrus_core::hydrus_db_base::DBBase;
use hydrus_core::hydrus_exceptions::HydrusResult;

fn table_name(file_service_id: i64, tag_service_id: i64) -> String {
    format!("mapping_counts_{file_service_id}_{tag_service_id}")
}

fn create_table(db: &DBBase, table: &str) -> HydrusResult<()> {
    db.execute_batch(&format!(
        "CREATE TABLE IF NOT EXISTS {table} (
            tag_id INTEGER PRIMARY KEY,
            current_count INTEGER,
            pending_count INTEGER
        );"
    ))
}

pub fn add_counts(
    db: &DBBase,
    file_service_id: i64,
    tag_service_id: i64,
    tag_id: i64,
    current_delta: i64,
    pending_delta: i64,
) -> HydrusResult<()> {
    let table = table_name(file_service_id, tag_service_id);
    create_table(db, &table)?;

    db.execute(
        &format!(
            "INSERT INTO {table} ( tag_id, current_count, pending_count )
             VALUES ( ?, ?, ? )
             ON CONFLICT(tag_id) DO UPDATE SET
                current_count = current_count + excluded.current_count,
                pending_count = pending_count + excluded.pending_count"
        ),
        &[&tag_id, &current_delta, &pending_delta],
    )?;

    Ok(())
}

pub fn set_counts(
    db: &DBBase,
    file_service_id: i64,
    tag_service_id: i64,
    tag_id: i64,
    current_count: i64,
    pending_count: i64,
) -> HydrusResult<()> {
    let table = table_name(file_service_id, tag_service_id);
    create_table(db, &table)?;

    db.execute(
        &format!(
            "INSERT OR REPLACE INTO {table} ( tag_id, current_count, pending_count )
             VALUES ( ?, ?, ? )"
        ),
        &[&tag_id, &current_count, &pending_count],
    )?;

    Ok(())
}

pub fn get_counts(
    db: &DBBase,
    file_service_id: i64,
    tag_service_id: i64,
    tag_id: i64,
) -> HydrusResult<(i64, i64)> {
    let table = table_name(file_service_id, tag_service_id);

    if !db.table_exists(&table)? {
        return Ok((0, 0));
    }

    Ok(db
        .query_one(
            &format!("SELECT current_count, pending_count FROM {table} WHERE tag_id = ?"),
            &[&tag_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?
        .unwrap_or((0, 0)))
}

pub fn get_all_counts(
    db: &DBBase,
    file_service_id: i64,
    tag_service_id: i64,
) -> HydrusResult<BTreeMap<i64, (i64, i64)>> {
    let table = table_name(file_service_id, tag_service_id);

    if !db.table_exists(&table)? {
        return Ok(BTreeMap::new());
    }

    let rows: Vec<(i64, i64, i64)> = db.query_all(
        &format!("SELECT tag_id, current_count, pending_count FROM {table}"),
        &[],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    )?;

    Ok(rows
        .into_iter()
        .map(|(tag_id, current, pending)| (tag_id, (current, pending)))
        .collect())
}

pub fn get_total_current_count(
    db: &DBBase,
    file_service_id: i64,
    tag_service_id: i64,
) -> HydrusResult<i64> {
    let table = table_name(file_service_id, tag_service_id);

    if !db.table_exists(&table)? {
        return Ok(0);
    }

    Ok(db
        .query_one(
            &format!("SELECT SUM(current_count) FROM {table}"),
            &[],
            |row| row.get::<_, Option<i64>>(0),
        )?
        .flatten()
        .unwrap_or(0))
}

pub fn clear_counts(
    db: &DBBase,
    file_service_id: i64,
    tag_service_id: i64,
    keep_current: bool,
    keep_pending: bool,
) -> HydrusResult<()> {
    let table = table_name(file_service_id, tag_service_id);

    if !db.table_exists(&table)? {
        return Ok(());
    }

    match (keep_current, keep_pending) {
        (true, true) => {}
        (true, false) => {
            db.execute(&format!("UPDATE {table} SET pending_count = 0"), &[])?;
        }
        (false, true) => {
            db.execute(&format!("UPDATE {table} SET current_count = 0"), &[])?;
        }
        (false, false) => {
            db.execute(&format!("DELETE FROM {table}"), &[])?;
        }
    };

    Ok(())
}

/// In-memory layer used while a tag service is being edited in bulk.
#[derive(Debug, Default)]
pub struct MappingCountsCache {
    entries: BTreeMap<(i64, i64, i64), (i64, i64)>,
    dirty: bool,
}

impl MappingCountsCache {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get(&self, file_service_id: i64, tag_service_id: i64, tag_id: i64) -> (i64, i64) {
        self.entries
            .get(&(file_service_id, tag_service_id, tag_id))
            .copied()
            .unwrap_or((0, 0))
    }

    pub fn set(
        &mut self,
        file_service_id: i64,
        tag_service_id: i64,
        tag_id: i64,
        current: i64,
        pending: i64,
    ) {
        self.entries.insert(
            (file_service_id, tag_service_id, tag_id),
            (current, pending),
        );
        self.dirty = true;
    }

    pub fn add(
        &mut self,
        file_service_id: i64,
        tag_service_id: i64,
        tag_id: i64,
        current_delta: i64,
        pending_delta: i64,
    ) {
        let (current, pending) = self.get(file_service_id, tag_service_id, tag_id);
        self.set(
            file_service_id,
            tag_service_id,
            tag_id,
            current + current_delta,
            pending + pending_delta,
        );
    }

    pub fn remove(&mut self, file_service_id: i64, tag_service_id: i64, tag_id: i64) -> bool {
        self.dirty = true;
        self.entries
            .remove(&(file_service_id, tag_service_id, tag_id))
            .is_some()
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.dirty = true;
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub fn set_clean(&mut self) {
        self.dirty = false;
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::ClientDB;
    use crate::db::master_tags::get_tag_id;

    fn database() -> (ClientDB, std::path::PathBuf) {
        let directory = std::env::temp_dir().join(format!(
            "hydrus_client_counts_{}_{}",
            std::process::id(),
            rand::random::<u64>()
        ));

        (ClientDB::new(&directory).unwrap(), directory)
    }

    #[test]
    fn counts_accumulate() {
        let (db, directory) = database();
        let tag_id = get_tag_id(db.db(), "blue eyes").unwrap();

        add_counts(db.db(), 1, 2, tag_id, 3, 1).unwrap();
        add_counts(db.db(), 1, 2, tag_id, 2, -1).unwrap();

        assert_eq!(get_counts(db.db(), 1, 2, tag_id).unwrap(), (5, 0));
        assert_eq!(get_total_current_count(db.db(), 1, 2).unwrap(), 5);

        drop(db);
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn counts_are_per_service_pair() {
        let (db, directory) = database();
        let tag_id = get_tag_id(db.db(), "tag").unwrap();

        set_counts(db.db(), 1, 1, tag_id, 10, 2).unwrap();
        set_counts(db.db(), 1, 2, tag_id, 20, 4).unwrap();

        assert_eq!(get_counts(db.db(), 1, 1, tag_id).unwrap(), (10, 2));
        assert_eq!(get_counts(db.db(), 1, 2, tag_id).unwrap(), (20, 4));

        drop(db);
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn clearing_can_keep_current_counts() {
        let (db, directory) = database();
        let tag_id = get_tag_id(db.db(), "tag").unwrap();

        set_counts(db.db(), 1, 1, tag_id, 10, 2).unwrap();
        clear_counts(db.db(), 1, 1, true, false).unwrap();

        assert_eq!(get_counts(db.db(), 1, 1, tag_id).unwrap(), (10, 0));

        drop(db);
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn in_memory_cache_accumulates_and_tracks_dirty() {
        let mut cache = MappingCountsCache::new();

        cache.add(1, 2, 3, 5, 1);
        cache.add(1, 2, 3, -2, 4);
        cache.add(1, 2, 4, 1, 0);

        assert_eq!(cache.get(1, 2, 3), (3, 5));
        assert_eq!(cache.len(), 2);
        assert!(cache.is_dirty());

        cache.set_clean();
        assert!(!cache.is_dirty());

        cache.remove(1, 2, 3);
        assert_eq!(cache.get(1, 2, 3), (0, 0));
        assert!(cache.is_dirty());
    }

    #[test]
    fn cache_clear_empties_entries() {
        let mut cache = MappingCountsCache::new();
        cache.set(1, 1, 1, 2, 3);
        cache.clear();

        assert!(cache.is_empty());
    }
}
