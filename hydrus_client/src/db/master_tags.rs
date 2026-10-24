//! Tag master tables from `hydrus/client/db/ClientDBMaster.py`.

use hydrus_core::hydrus_db_base::DBBase;
use hydrus_core::hydrus_exceptions::HydrusResult;
use hydrus_core::hydrus_tags;

pub fn get_tag_id(db: &DBBase, tag: &str) -> HydrusResult<i64> {
    let (namespace, subtag) = hydrus_tags::split_tag(tag);

    let namespace_id = get_or_create(db, "namespaces", "namespace_id", "namespace", namespace)?;
    let subtag_id = get_or_create(db, "subtags", "subtag_id", "subtag", subtag)?;

    if let Some(tag_id) = db.query_one(
        "SELECT tag_id FROM tags WHERE namespace_id = ? AND subtag_id = ?",
        &[&namespace_id, &subtag_id],
        |row| row.get(0),
    )? {
        return Ok(tag_id);
    }

    db.execute(
        "INSERT INTO tags ( namespace_id, subtag_id ) VALUES ( ?, ? )",
        &[&namespace_id, &subtag_id],
    )?;

    db.query_one(
        "SELECT tag_id FROM tags WHERE namespace_id = ? AND subtag_id = ?",
        &[&namespace_id, &subtag_id],
        |row| row.get(0),
    )?
    .ok_or_else(|| hydrus_core::hydrus_exceptions::data_missing("tag_id disappeared"))
}

pub fn get_tag(db: &DBBase, tag_id: i64) -> HydrusResult<Option<String>> {
    let row: Option<(String, String)> = db.query_one(
        "SELECT namespaces.namespace, subtags.subtag FROM tags
         JOIN namespaces ON tags.namespace_id = namespaces.namespace_id
         JOIN subtags ON tags.subtag_id = subtags.subtag_id
         WHERE tags.tag_id = ?",
        &[&tag_id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;

    Ok(row.map(|(namespace, subtag)| hydrus_tags::combine_tag(&namespace, &subtag)))
}

pub fn get_tags(db: &DBBase, tag_ids: &[i64]) -> HydrusResult<Vec<String>> {
    let mut tags = Vec::with_capacity(tag_ids.len());

    for tag_id in tag_ids {
        if let Some(tag) = get_tag(db, *tag_id)? {
            tags.push(tag);
        }
    }

    Ok(tags)
}

pub fn has_tag(db: &DBBase, tag: &str) -> HydrusResult<bool> {
    let (namespace, subtag) = hydrus_tags::split_tag(tag);

    let count: i64 = db
        .query_one(
            "SELECT COUNT(*) FROM tags
             JOIN namespaces ON tags.namespace_id = namespaces.namespace_id
             JOIN subtags ON tags.subtag_id = subtags.subtag_id
             WHERE namespaces.namespace = ? AND subtags.subtag = ?",
            &[&namespace, &subtag],
            |row| row.get(0),
        )?
        .unwrap_or(0);

    Ok(count > 0)
}

fn get_or_create(
    db: &DBBase,
    table: &str,
    id_column: &str,
    value_column: &str,
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
    .ok_or_else(|| hydrus_core::hydrus_exceptions::data_missing("tag definition id disappeared"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::ClientDB;

    fn database() -> (ClientDB, std::path::PathBuf) {
        let directory = std::env::temp_dir().join(format!(
            "hydrus_client_mastertags_{}_{}",
            std::process::id(),
            rand::random::<u64>()
        ));

        (ClientDB::new(&directory).unwrap(), directory)
    }

    #[test]
    fn tag_ids_are_idempotent() {
        let (db, directory) = database();

        let first = get_tag_id(db.db(), "blue eyes").unwrap();
        let second = get_tag_id(db.db(), "blue eyes").unwrap();

        assert_eq!(first, second);
        assert_eq!(
            get_tag(db.db(), first).unwrap().as_deref(),
            Some("blue eyes")
        );

        drop(db);
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn namespaces_are_preserved() {
        let (db, directory) = database();
        let tag_id = get_tag_id(db.db(), "creator:someone").unwrap();

        assert_eq!(
            get_tag(db.db(), tag_id).unwrap().as_deref(),
            Some("creator:someone")
        );
        assert!(has_tag(db.db(), "creator:someone").unwrap());

        drop(db);
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn empty_namespace_tags_round_trip() {
        let (db, directory) = database();
        let tag_id = get_tag_id(db.db(), "solo").unwrap();

        assert_eq!(get_tag(db.db(), tag_id).unwrap().as_deref(), Some("solo"));

        drop(db);
        let _ = std::fs::remove_dir_all(directory);
    }
}
