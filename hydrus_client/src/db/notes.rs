//! File notes from `hydrus/client/db/ClientDBNotesMap.py`.

use std::collections::BTreeMap;

use hydrus_core::hydrus_db_base::DBBase;
use hydrus_core::hydrus_exceptions::HydrusResult;

pub fn set_note(db: &DBBase, hash_id: i64, name: &str, note: &str) -> HydrusResult<()> {
    let name_id = get_or_create(db, "labels", "label", "label_id", name)?;
    let note_id = get_or_create(db, "notes", "note", "note_id", note)?;

    db.execute(
        "INSERT OR REPLACE INTO file_notes ( hash_id, name_id, note_id ) VALUES ( ?, ?, ? )",
        &[&hash_id, &name_id, &note_id],
    )?;

    Ok(())
}

pub fn get_note(db: &DBBase, hash_id: i64, name: &str) -> HydrusResult<Option<String>> {
    db.query_one(
        "SELECT notes.note FROM file_notes
         JOIN labels ON file_notes.name_id = labels.label_id
         JOIN notes ON file_notes.note_id = notes.note_id
         WHERE file_notes.hash_id = ? AND labels.label = ?",
        &[&hash_id, &name],
        |row| row.get(0),
    )
}

pub fn get_notes(db: &DBBase, hash_id: i64) -> HydrusResult<BTreeMap<String, String>> {
    let rows: Vec<(String, String)> = db.query_all(
        "SELECT labels.label, notes.note FROM file_notes
         JOIN labels ON file_notes.name_id = labels.label_id
         JOIN notes ON file_notes.note_id = notes.note_id
         WHERE file_notes.hash_id = ?",
        &[&hash_id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;

    Ok(rows.into_iter().collect())
}

pub fn delete_note(db: &DBBase, hash_id: i64, name: &str) -> HydrusResult<()> {
    db.execute(
        "DELETE FROM file_notes WHERE hash_id = ? AND name_id IN
         ( SELECT label_id FROM labels WHERE label = ? )",
        &[&hash_id, &name],
    )?;

    Ok(())
}

pub fn has_notes(db: &DBBase, hash_id: i64) -> HydrusResult<bool> {
    let count: i64 = db
        .query_one(
            "SELECT COUNT(*) FROM file_notes WHERE hash_id = ?",
            &[&hash_id],
            |row| row.get(0),
        )?
        .unwrap_or(0);

    Ok(count > 0)
}

fn get_or_create(
    db: &DBBase,
    table: &str,
    value_column: &str,
    id_column: &str,
    value: &str,
) -> HydrusResult<i64> {
    if let Some(id) = db.query_one(
        &format!("SELECT {id_column} FROM {table} WHERE {value_column} = ?"),
        &[&value],
        |row| row.get(0),
    )? {
        return Ok(id);
    }

    db.execute(
        &format!("INSERT INTO {table} ( {value_column} ) VALUES ( ? )"),
        &[&value],
    )?;

    db.query_one(
        &format!("SELECT {id_column} FROM {table} WHERE {value_column} = ?"),
        &[&value],
        |row| row.get(0),
    )?
    .ok_or_else(|| hydrus_core::hydrus_exceptions::data_missing("row id disappeared"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::ClientDB;
    use crate::db::master::get_hash_id;

    fn database() -> (ClientDB, std::path::PathBuf) {
        let directory = std::env::temp_dir().join(format!(
            "hydrus_client_notes_{}_{}",
            std::process::id(),
            rand::random::<u64>()
        ));

        (ClientDB::new(&directory).unwrap(), directory)
    }

    #[test]
    fn notes_round_trip_and_overwrite() {
        let (db, directory) = database();
        let hash_id = get_hash_id(db.db(), &[1]).unwrap();

        set_note(db.db(), hash_id, "page 1", "hello").unwrap();
        set_note(db.db(), hash_id, "page 2", "world").unwrap();
        set_note(db.db(), hash_id, "page 1", "updated").unwrap();

        let notes = get_notes(db.db(), hash_id).unwrap();

        assert_eq!(notes.len(), 2);
        assert_eq!(notes["page 1"], "updated");
        assert!(has_notes(db.db(), hash_id).unwrap());

        delete_note(db.db(), hash_id, "page 1").unwrap();

        assert_eq!(get_note(db.db(), hash_id, "page 1").unwrap(), None);

        drop(db);
        let _ = std::fs::remove_dir_all(directory);
    }
}
