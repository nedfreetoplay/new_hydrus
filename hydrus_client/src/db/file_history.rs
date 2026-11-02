//! File import history (`view file history`) from
//! `hydrus/client/db/ClientDB.py::_GetFileHistoryFromTable`.
//!
//! Domain is "all known files" (`system:everything`): `current_files_*` of the
//! local file service, `deleted_files_*` of the trash service, `file_inbox`
//! and `archive_timestamps`. The Python stepping loops are ported 1:1, but the
//! port schema stores `timestamp` in whole seconds (not `timestamp_ms`) and has
//! no `original_timestamp_ms` column, so the `current` series counts imports
//! from the current table only and the `deleted` series counts rows by their
//! delete timestamp. The per-query `file search context` of Python (arbitrary
//! predicates, `TooComplicatedM8`) is not ported: see the plan residual.

use hydrus_core::hydrus_db_base::DBBase;
use hydrus_core::hydrus_exceptions::HydrusResult;

/// Python `ReviewFileHistory` always requests this many steps.
pub const NUM_STEPS: i64 = 7680;

/// Cumulative `(timestamp_secs, num_files)` series, in ascending time order.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FileHistory {
    pub current: Vec<(i64, i64)>,
    pub inbox: Vec<(i64, i64)>,
    pub archive: Vec<(i64, i64)>,
    pub deleted: Vec<(i64, i64)>,
}

fn timestamps(db: &DBBase, table: &str, column: &str, ordered: bool) -> HydrusResult<Vec<i64>> {
    if !db.table_exists(table)? {
        return Ok(Vec::new());
    }

    let order = if ordered { " ORDER BY 1 ASC" } else { "" };
    let sql = format!("SELECT {column} FROM {table} WHERE {column} IS NOT NULL{order}");

    Ok(db.query_all(&sql, &[], |row| row.get(0))?)
}

fn count_all(db: &DBBase, table: &str) -> HydrusResult<i64> {
    if !db.table_exists(table)? {
        return Ok(0);
    }

    Ok(db
        .query_one(&format!("SELECT COUNT(*) FROM {table}"), &[], |row| row.get(0))?
        .unwrap_or(0))
}

/// Builds the four cumulative series for the "all known files" domain.
///
/// `my_files_id` is the local file service, `trash_id` the trash service.
pub fn collect(db: &DBBase, my_files_id: i64, trash_id: i64, num_steps: i64) -> HydrusResult<FileHistory> {
    let current_table = format!("current_files_{my_files_id}");
    let deleted_table = format!("deleted_files_{trash_id}");

    let current_timestamps = timestamps(db, &current_table, "timestamp", false)?;
    let mut deleted_timestamps = timestamps(db, &deleted_table, "timestamp", true)?;

    let mut all_known: Vec<i64> = current_timestamps.clone();
    all_known.sort_unstable();

    let total_deleted = count_all(db, &deleted_table)?;

    let mut history = FileHistory::default();

    // Current: +1 on import, -1 on delete.
    // Port simplification (documented residual): the port schema stores a single
    // `timestamp` (seconds) per deleted row and has no `original_timestamp_ms`
    // column, so the original import times of deleted files are unknown.
    // Python adds them to the import side; the port counts current-table
    // imports only and subtracts deletes at delete time. The endpoint still
    // lands on the present file count.
    let mut with_delta: Vec<(i64, i64)> = current_timestamps.iter().map(|t| (*t, 1)).collect();
    with_delta.extend(deleted_timestamps.iter().map(|t| (*t, -1)));
    with_delta.sort_unstable();

    if !with_delta.is_empty() {
        history.current.push((with_delta[0].0, 0));

        let step_gap = if with_delta.len() < 2 {
            1
        } else {
            ((with_delta[with_delta.len() - 1].0 - with_delta[0].0) / num_steps).max(1)
        };

        let mut total = 0_i64;
        let mut step_ts = with_delta[0].0;

        for (timestamp, delta) in with_delta {
            while timestamp > step_ts + step_gap {
                history.current.push((step_ts, total));
                step_ts += step_gap;
            }

            total += delta;
        }
    }

    // Deleted: cumulative count by delete time. Rows without a timestamp were
    // already counted via `total_deleted`.
    if !deleted_timestamps.is_empty() {
        deleted_timestamps.sort_unstable();

        let step_gap = if deleted_timestamps.len() < 2 {
            1
        } else {
            ((deleted_timestamps[deleted_timestamps.len() - 1] - deleted_timestamps[0]) / num_steps).max(1)
        };

        let mut step_ts = deleted_timestamps[0];
        let nulls = total_deleted - deleted_timestamps.len() as i64;
        let mut total = nulls.max(0);

        for ts in &deleted_timestamps {
            while *ts > step_ts + step_gap {
                history.deleted.push((step_ts, total));
                step_ts += step_gap;
            }

            total += 1;
        }
    }

    // Inbox/archive walk backwards from now: archive +1/-1 on the inbox count,
    // import -1 on inbox, delete +1 on archive; mirrored from Python.
    let total_inbox: i64 = db
        .query_one("SELECT COUNT(*) FROM file_inbox", &[], |row| row.get(0))?
        .unwrap_or(0);
    let mut archive_timestamps = timestamps(db, "archive_timestamps", "timestamp", true)?;

    if !archive_timestamps.is_empty() {
        archive_timestamps.sort_unstable();

        let first_archive = archive_timestamps[0];
        let total_archiveable = current_timestamps.len() as i64;
        let mut total_archive = total_archiveable - total_inbox;

        // (timestamp, inbox_delta, archive_delta), walked newest-first.
        let mut combined: Vec<(i64, i64, i64)> = archive_timestamps.iter().map(|t| (*t, 1, -1)).collect();
        combined.extend(
            all_known
                .iter()
                .filter(|t| **t >= first_archive)
                .map(|t| (*t, -1, 0)),
        );
        combined.extend(
            deleted_timestamps
                .iter()
                .filter(|t| **t >= first_archive)
                .map(|t| (*t, 0, 1)),
        );
        combined.sort_by(|a, b| b.cmp(a));

        if !combined.is_empty() {
            let step_gap = if combined.len() < 2 {
                1
            } else {
                ((combined[0].0 - combined[combined.len() - 1].0) / num_steps).max(1)
            };

            let mut step_ts = combined[0].0;
            let mut inbox_total = total_inbox;

            for (timestamp, inbox_delta, archive_delta) in combined {
                while timestamp < step_ts - step_gap {
                    history.inbox.push((timestamp, inbox_total));
                    history.archive.push((timestamp, total_archive));
                    step_ts -= step_gap;
                }

                inbox_total += inbox_delta;
                total_archive += archive_delta;
            }

            history.inbox.reverse();
            history.archive.reverse();
        }
    }

    Ok(history)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::files_storage;
    use crate::db::master::get_hash_id;
    use crate::db::ClientDB;
    use hydrus_core::hydrus_constants::ContentStatus;

    fn database() -> (ClientDB, std::path::PathBuf) {
        let directory = std::env::temp_dir().join(format!(
            "hydrus_client_filehistory_{}_{}",
            std::process::id(),
            rand::random::<u64>()
        ));

        (ClientDB::new(&directory).unwrap(), directory)
    }

    #[test]
    fn num_steps_matches_python() {
        assert_eq!(NUM_STEPS, 7680);
    }

    #[test]
    fn empty_database_gives_empty_series() {
        let (db, directory) = database();
        let history = collect(db.db(), 1, 2, NUM_STEPS).unwrap();

        assert_eq!(history, FileHistory::default());

        drop(db);
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn current_and_deleted_series_step_like_python() {
        let (db, directory) = database();
        let base = 1_700_000_000_i64;

        // Three imports a day apart, two deletes on different days.
        let hashes: Vec<i64> = (0..3)
            .map(|i| get_hash_id(db.db(), &[10 + i as u8]).unwrap())
            .collect();
        for (index, hash_id) in hashes.iter().enumerate() {
            files_storage::add_files(
                db.db(),
                1,
                ContentStatus::Current,
                &[*hash_id],
                Some(base + index as i64 * 86_400),
            )
            .unwrap();
        }
        files_storage::add_files(
            db.db(),
            2,
            ContentStatus::Deleted,
            &[hashes[2]],
            Some(base + 2 * 86_400),
        )
        .unwrap();
        files_storage::add_files(
            db.db(),
            2,
            ContentStatus::Deleted,
            &[hashes[1]],
            Some(base + 3 * 86_400),
        )
        .unwrap();

        let history = collect(db.db(), 1, 2, NUM_STEPS).unwrap();

        assert!(!history.current.is_empty());
        assert_eq!(history.current[0], (base, 0));
        // Python records the pre-delta value at each step, so the pushed peak
        // is 2: the third import and the first delete share a timestamp.
        let peak = history.current.iter().map(|(_, n)| *n).max().unwrap();
        assert_eq!(peak, 2);
        // Two deletes on different days give the deleted series real steps.
        assert!(!history.deleted.is_empty());
        assert_eq!(
            history.deleted.iter().map(|(_, n)| *n).max().unwrap(),
            1
        );

        drop(db);
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn inbox_and_archive_walk_backwards_from_now() {
        let (db, directory) = database();
        let base = 1_700_000_000_i64;
        let day = 86_400_i64;

        // A stays in the inbox; B and C are archived a day and three days later.
        let inbox_hash = get_hash_id(db.db(), &[20]).unwrap();
        let archived_b = get_hash_id(db.db(), &[21]).unwrap();
        let archived_c = get_hash_id(db.db(), &[22]).unwrap();
        files_storage::add_files(db.db(), 1, ContentStatus::Current, &[inbox_hash], Some(base)).unwrap();
        files_storage::add_files(
            db.db(),
            1,
            ContentStatus::Current,
            &[archived_b],
            Some(base),
        )
        .unwrap();
        files_storage::add_files(
            db.db(),
            1,
            ContentStatus::Current,
            &[archived_c],
            Some(base + 2 * day),
        )
        .unwrap();
        files_storage::set_inbox(db.db(), &[inbox_hash], true).unwrap();
        db.db()
            .execute(
                "INSERT OR REPLACE INTO archive_timestamps ( hash_id, timestamp ) VALUES ( ?, ? )",
                &[&archived_b, &(base + day)],
            )
            .unwrap();
        db.db()
            .execute(
                "INSERT OR REPLACE INTO archive_timestamps ( hash_id, timestamp ) VALUES ( ?, ? )",
                &[&archived_c, &(base + 3 * day)],
            )
            .unwrap();

        let history = collect(db.db(), 1, 2, NUM_STEPS).unwrap();

        assert!(!history.inbox.is_empty());
        assert!(!history.archive.is_empty());
        // Backwards walk, exactly like Python: the oldest event's pre-delta
        // values come first, the newest event's steps come last.
        assert_eq!(*history.inbox.first().unwrap(), (base + day, 1));
        assert_eq!(*history.inbox.last().unwrap(), (base + 2 * day, 2));
        assert_eq!(*history.archive.first().unwrap(), (base + day, 1));
        assert_eq!(*history.archive.last().unwrap(), (base + 2 * day, 1));

        drop(db);
        let _ = std::fs::remove_dir_all(directory);
    }
}
