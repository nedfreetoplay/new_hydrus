//! File viewing statistics from `hydrus/client/db/ClientDBFilesViewingStats.py`.

use hydrus_core::hydrus_db_base::DBBase;
use hydrus_core::hydrus_exceptions::HydrusResult;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ViewingStats {
    pub last_viewed_timestamp_ms: Option<i64>,
    pub views: i64,
    pub viewtime_ms: i64,
}

pub fn get_viewing_stats(
    db: &DBBase,
    hash_id: i64,
    canvas_type: i64,
) -> HydrusResult<ViewingStats> {
    let row = db.query_one(
        "SELECT last_viewed_timestamp_ms, views, viewtime_ms
         FROM file_viewing_stats WHERE hash_id = ? AND canvas_type = ?",
        &[&hash_id, &canvas_type],
        |row| {
            Ok(ViewingStats {
                last_viewed_timestamp_ms: row.get(0)?,
                views: row.get(1)?,
                viewtime_ms: row.get(2)?,
            })
        },
    )?;

    Ok(row.unwrap_or_default())
}

pub fn set_viewing_stats(
    db: &DBBase,
    hash_id: i64,
    canvas_type: i64,
    stats: ViewingStats,
) -> HydrusResult<()> {
    db.execute(
        "INSERT OR REPLACE INTO file_viewing_stats
         ( hash_id, canvas_type, last_viewed_timestamp_ms, views, viewtime_ms )
         VALUES ( ?, ?, ?, ?, ? )",
        &[
            &hash_id,
            &canvas_type,
            &stats.last_viewed_timestamp_ms,
            &stats.views,
            &stats.viewtime_ms,
        ],
    )?;

    Ok(())
}

pub fn add_viewing_stats(
    db: &DBBase,
    hash_id: i64,
    canvas_type: i64,
    views: i64,
    viewtime_ms: i64,
    last_viewed_timestamp_ms: Option<i64>,
) -> HydrusResult<ViewingStats> {
    let mut stats = get_viewing_stats(db, hash_id, canvas_type)?;

    stats.views += views;
    stats.viewtime_ms += viewtime_ms;

    if last_viewed_timestamp_ms.is_some() {
        stats.last_viewed_timestamp_ms = last_viewed_timestamp_ms;
    }

    set_viewing_stats(db, hash_id, canvas_type, stats)?;

    Ok(stats)
}

pub fn clear_viewing_stats(db: &DBBase) -> HydrusResult<()> {
    db.execute("DELETE FROM file_viewing_stats", &[])?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::ClientDB;
    use crate::db::master::get_hash_id;

    fn database() -> (ClientDB, std::path::PathBuf) {
        let directory = std::env::temp_dir().join(format!(
            "hydrus_client_viewstats_{}_{}",
            std::process::id(),
            rand::random::<u64>()
        ));

        (ClientDB::new(&directory).unwrap(), directory)
    }

    #[test]
    fn stats_start_at_zero_and_accumulate() {
        let (db, directory) = database();
        let hash_id = get_hash_id(db.db(), &[9]).unwrap();

        assert_eq!(
            get_viewing_stats(db.db(), hash_id, 0).unwrap(),
            ViewingStats::default()
        );

        add_viewing_stats(db.db(), hash_id, 0, 2, 500, Some(1000)).unwrap();
        let stats = add_viewing_stats(db.db(), hash_id, 0, 1, 250, Some(2000)).unwrap();

        assert_eq!(stats.views, 3);
        assert_eq!(stats.viewtime_ms, 750);
        assert_eq!(stats.last_viewed_timestamp_ms, Some(2000));

        drop(db);
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn canvas_types_are_independent() {
        let (db, directory) = database();
        let hash_id = get_hash_id(db.db(), &[10]).unwrap();

        add_viewing_stats(db.db(), hash_id, 0, 5, 100, Some(1)).unwrap();
        add_viewing_stats(db.db(), hash_id, 1, 2, 50, Some(2)).unwrap();

        assert_eq!(get_viewing_stats(db.db(), hash_id, 0).unwrap().views, 5);
        assert_eq!(get_viewing_stats(db.db(), hash_id, 1).unwrap().views, 2);

        drop(db);
        let _ = std::fs::remove_dir_all(directory);
    }
}
