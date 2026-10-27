//! Tag search helpers from `hydrus/client/db/ClientDBTagSearch.py`.

use std::collections::BTreeSet;

use hydrus_core::hydrus_constants::ContentStatus;
use hydrus_core::hydrus_db_base::DBBase;
use hydrus_core::hydrus_exceptions::HydrusResult;
use hydrus_core::hydrus_tags;

use crate::search::predicate_constants::PredicateType;
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
        PredicateType::Tag => get_hash_ids_for_tag(db, service_id, status, text),
        PredicateType::Namespace => get_hash_ids_for_namespace(db, service_id, status, text),
        PredicateType::Wildcard => get_hash_ids_for_wildcard(db, service_id, status, text),
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

/// One autocomplete entry with the number of files currently mapped to it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagSuggestion {
    pub text: String,
    pub count: i64,
}

/// Tags whose text starts with `prefix`, most used first. `%` and `_` are
/// escaped so a user-typed wildcard cannot turn into a broader SQL pattern.
pub fn suggest_tags(
    db: &DBBase,
    service_id: i64,
    status: ContentStatus,
    prefix: &str,
    limit: usize,
) -> HydrusResult<Vec<TagSuggestion>> {
    let table = table_name(service_id, status);

    if !db.table_exists(&table)? {
        return Ok(Vec::new());
    }

    let pattern = format!("{}%", escape_like(prefix));

    db.query_all(
        &format!(
            "SELECT CASE WHEN n.namespace = '' THEN s.subtag
                         ELSE n.namespace || ':' || s.subtag END AS tag_text,
                    COUNT(DISTINCT m.hash_id) AS files
             FROM {table} m
             JOIN tags t ON m.tag_id = t.tag_id
             JOIN namespaces n ON t.namespace_id = n.namespace_id
             JOIN subtags s ON t.subtag_id = s.subtag_id
             WHERE tag_text LIKE ? ESCAPE '\\'
             GROUP BY t.tag_id
             ORDER BY files DESC, tag_text
             LIMIT ?"
        ),
        &[&pattern, &(limit as i64)],
        |row| {
            Ok(TagSuggestion {
                text: row.get(0)?,
                count: row.get(1)?,
            })
        },
    )
}

/// Namespaces whose name starts with `prefix`, with file counts.
pub fn suggest_namespaces(
    db: &DBBase,
    service_id: i64,
    status: ContentStatus,
    prefix: &str,
    limit: usize,
) -> HydrusResult<Vec<TagSuggestion>> {
    let table = table_name(service_id, status);

    if !db.table_exists(&table)? {
        return Ok(Vec::new());
    }

    let pattern = format!("{}%", escape_like(prefix));

    db.query_all(
        &format!(
            "SELECT n.namespace, COUNT(DISTINCT m.hash_id)
             FROM {table} m
             JOIN tags t ON m.tag_id = t.tag_id
             JOIN namespaces n ON t.namespace_id = n.namespace_id
             WHERE n.namespace <> '' AND n.namespace LIKE ? ESCAPE '\\'
             GROUP BY n.namespace
             ORDER BY COUNT(DISTINCT m.hash_id) DESC, n.namespace
             LIMIT ?"
        ),
        &[&pattern, &(limit as i64)],
        |row| {
            Ok(TagSuggestion {
                text: row.get(0)?,
                count: row.get(1)?,
            })
        },
    )
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
mod tests;
