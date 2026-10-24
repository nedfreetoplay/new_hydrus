//! Tag search helpers from `hydrus/client/db/ClientDBTagSearch.py`.

use std::collections::BTreeSet;

use hydrus_core::hydrus_constants::ContentStatus;
use hydrus_core::hydrus_db_base::DBBase;
use hydrus_core::hydrus_exceptions::HydrusResult;
use hydrus_core::hydrus_tags;

use crate::search::predicate_constants::{
    PREDICATE_TYPE_NAMESPACE, PREDICATE_TYPE_TAG, PREDICATE_TYPE_WILDCARD,
};
use crate::search::{Predicate, PredicateValue};

pub fn search_hash_ids(
    db: &DBBase,
    service_id: i64,
    status: ContentStatus,
    predicate: &Predicate,
) -> HydrusResult<BTreeSet<i64>> {
    let PredicateValue::Text(text) = &predicate.value else {
        return Ok(BTreeSet::new());
    };

    if !predicate.inclusive {
        return Ok(BTreeSet::new());
    }

    match predicate.predicate_type {
        PREDICATE_TYPE_TAG => get_hash_ids_for_tag(db, service_id, status, text),
        PREDICATE_TYPE_NAMESPACE => get_hash_ids_for_namespace(db, service_id, status, text),
        PREDICATE_TYPE_WILDCARD => get_hash_ids_for_wildcard(db, service_id, status, text),
        _ => Ok(BTreeSet::new()),
    }
}

pub fn get_hash_ids_for_tag(
    db: &DBBase,
    service_id: i64,
    status: ContentStatus,
    tag: &str,
) -> HydrusResult<BTreeSet<i64>> {
    let (namespace, subtag) = hydrus_tags::split_tag(tag);
    let table = table_name(service_id, status);

    if !db.table_exists(&table)? {
        return Ok(BTreeSet::new());
    }

    let rows = db.query_all(
        &format!(
            "SELECT m.hash_id FROM {table} m
             JOIN tags t ON m.tag_id = t.tag_id
             JOIN namespaces n ON t.namespace_id = n.namespace_id
             JOIN subtags s ON t.subtag_id = s.subtag_id
             WHERE n.namespace = ? AND s.subtag = ?"
        ),
        &[&namespace, &subtag],
        |row| row.get(0),
    )?;

    Ok(rows.into_iter().collect())
}

pub fn get_hash_ids_for_namespace(
    db: &DBBase,
    service_id: i64,
    status: ContentStatus,
    namespace: &str,
) -> HydrusResult<BTreeSet<i64>> {
    let table = table_name(service_id, status);

    if !db.table_exists(&table)? {
        return Ok(BTreeSet::new());
    }

    let rows = db.query_all(
        &format!(
            "SELECT m.hash_id FROM {table} m
             JOIN tags t ON m.tag_id = t.tag_id
             JOIN namespaces n ON t.namespace_id = n.namespace_id
             WHERE n.namespace = ?"
        ),
        &[&namespace],
        |row| row.get(0),
    )?;

    Ok(rows.into_iter().collect())
}

pub fn get_hash_ids_for_wildcard(
    db: &DBBase,
    service_id: i64,
    status: ContentStatus,
    wildcard: &str,
) -> HydrusResult<BTreeSet<i64>> {
    let table = table_name(service_id, status);

    if !db.table_exists(&table)? {
        return Ok(BTreeSet::new());
    }

    let pattern = format!("%{}%", escape_like(wildcard.trim_matches('*')));

    let rows = db.query_all(
        &format!(
            "SELECT m.hash_id FROM {table} m
             JOIN tags t ON m.tag_id = t.tag_id
             JOIN namespaces n ON t.namespace_id = n.namespace_id
             JOIN subtags s ON t.subtag_id = s.subtag_id
             WHERE (CASE WHEN n.namespace = '' THEN s.subtag
                         ELSE n.namespace || ':' || s.subtag END) LIKE ? ESCAPE '\\'"
        ),
        &[&pattern],
        |row| row.get(0),
    )?;

    Ok(rows.into_iter().collect())
}

pub fn get_num_files_for_tag(
    db: &DBBase,
    service_id: i64,
    status: ContentStatus,
    tag: &str,
) -> HydrusResult<i64> {
    Ok(get_hash_ids_for_tag(db, service_id, status, tag)?.len() as i64)
}

fn table_name(service_id: i64, status: ContentStatus) -> String {
    let prefix = match status {
        ContentStatus::Current => "current_mappings",
        ContentStatus::Pending => "pending_mappings",
        ContentStatus::Deleted => "deleted_mappings",
        ContentStatus::Petitioned => "petitioned_mappings",
    };

    format!("{prefix}_{service_id}")
}

fn escape_like(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::master::get_hash_id;
    use crate::db::master_tags::get_tag_id;
    use crate::db::{mappings, ClientDB};

    fn database() -> (ClientDB, std::path::PathBuf) {
        let directory = std::env::temp_dir().join(format!(
            "hydrus_client_tagsearch_{}_{}",
            std::process::id(),
            rand::random::<u64>()
        ));

        (ClientDB::new(&directory).unwrap(), directory)
    }

    fn seed(db: &DBBase) -> (i64, i64, i64) {
        let first = get_hash_id(db, &[1]).unwrap();
        let second = get_hash_id(db, &[2]).unwrap();
        let blue = get_tag_id(db, "blue eyes").unwrap();
        let blonde = get_tag_id(db, "blonde hair").unwrap();
        let creator = get_tag_id(db, "creator:someone").unwrap();

        mappings::add_mappings(db, 1, ContentStatus::Current, first, &[blue, creator]).unwrap();
        mappings::add_mappings(db, 1, ContentStatus::Current, second, &[blonde]).unwrap();

        (first, second, blue)
    }

    #[test]
    fn tag_search_returns_matching_hashes() {
        let (db, directory) = database();
        let (first, _, _) = seed(db.db());

        let hashes = get_hash_ids_for_tag(db.db(), 1, ContentStatus::Current, "blue eyes").unwrap();

        assert_eq!(hashes, BTreeSet::from([first]));

        drop(db);
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn namespace_search_returns_all_in_namespace() {
        let (db, directory) = database();
        let (first, _, _) = seed(db.db());

        let hashes =
            get_hash_ids_for_namespace(db.db(), 1, ContentStatus::Current, "creator").unwrap();

        assert_eq!(hashes, BTreeSet::from([first]));

        drop(db);
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn wildcard_search_matches_substrings_and_namespaces() {
        let (db, directory) = database();
        let (first, second, _) = seed(db.db());

        let hashes = get_hash_ids_for_wildcard(db.db(), 1, ContentStatus::Current, "*hair*").unwrap();

        assert_eq!(hashes, BTreeSet::from([second]));

        let all = get_hash_ids_for_wildcard(db.db(), 1, ContentStatus::Current, "*e*").unwrap();

        assert!(all.contains(&first));

        drop(db);
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn predicate_search_dispatches_by_type() {
        let (db, directory) = database();
        let (first, _, _) = seed(db.db());

        let predicate = Predicate::tag("blue eyes");
        let hashes = search_hash_ids(db.db(), 1, ContentStatus::Current, &predicate).unwrap();

        assert_eq!(hashes, BTreeSet::from([first]));

        drop(db);
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn missing_tables_are_empty_results() {
        let (db, directory) = database();

        assert!(get_hash_ids_for_tag(db.db(), 99, ContentStatus::Current, "x")
            .unwrap()
            .is_empty());

        drop(db);
        let _ = std::fs::remove_dir_all(directory);
    }
}
