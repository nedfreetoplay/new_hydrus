//! Tag siblings and parents storage from `ClientDBTagSiblings.py` and
//! `ClientDBTagParents.py`.

use std::collections::BTreeSet;

use hydrus_core::hydrus_constants::ContentStatus;
use hydrus_core::hydrus_db_base::DBBase;
use hydrus_core::hydrus_exceptions::HydrusResult;

use crate::db::master_tags;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TagRelation {
    Siblings,
    Parents,
}

fn prefix(relation: TagRelation) -> &'static str {
    match relation {
        TagRelation::Siblings => "tag_siblings",
        TagRelation::Parents => "tag_parents",
    }
}

fn columns(relation: TagRelation) -> (&'static str, &'static str) {
    match relation {
        TagRelation::Siblings => ("bad_tag_id", "good_tag_id"),
        TagRelation::Parents => ("child_tag_id", "parent_tag_id"),
    }
}

fn table_name(relation: TagRelation, service_id: i64, status: ContentStatus) -> String {
    let status = match status {
        ContentStatus::Current => "current",
        ContentStatus::Pending => "pending",
        ContentStatus::Deleted => "deleted",
        ContentStatus::Petitioned => "petitioned",
    };

    format!("{}_{}_{}", prefix(relation), status, service_id)
}

pub fn create_tables(db: &DBBase, relation: TagRelation, service_id: i64) -> HydrusResult<()> {
    for status in [
        ContentStatus::Current,
        ContentStatus::Deleted,
        ContentStatus::Pending,
        ContentStatus::Petitioned,
    ] {
        let table = table_name(relation, service_id, status);
        let (first, second) = columns(relation);

        if matches!(status, ContentStatus::Pending | ContentStatus::Petitioned) {
            db.execute_batch(&format!(
                "CREATE TABLE IF NOT EXISTS {table} (
                    {first} INTEGER,
                    {second} INTEGER,
                    reason_id INTEGER,
                    PRIMARY KEY ( {first}, {second} )
                );"
            ))?;
        } else {
            db.execute_batch(&format!(
                "CREATE TABLE IF NOT EXISTS {table} (
                    {first} INTEGER,
                    {second} INTEGER,
                    PRIMARY KEY ( {first}, {second} )
                );"
            ))?;
        }
    }

    Ok(())
}

pub fn add_pair(
    db: &DBBase,
    relation: TagRelation,
    service_id: i64,
    status: ContentStatus,
    first: i64,
    second: i64,
    reason_id: Option<i64>,
) -> HydrusResult<()> {
    create_tables(db, relation, service_id)?;
    let table = table_name(relation, service_id, status);
    let (first_column, second_column) = columns(relation);

    if matches!(status, ContentStatus::Pending | ContentStatus::Petitioned) {
        db.execute(
            &format!(
                "INSERT OR REPLACE INTO {table} ( {first_column}, {second_column}, reason_id )
                 VALUES ( ?, ?, ? )"
            ),
            &[&first, &second, &reason_id],
        )?;
    } else {
        db.execute(
            &format!(
                "INSERT OR IGNORE INTO {table} ( {first_column}, {second_column} ) VALUES ( ?, ? )"
            ),
            &[&first, &second],
        )?;
    }

    Ok(())
}

pub fn remove_pair(
    db: &DBBase,
    relation: TagRelation,
    service_id: i64,
    status: ContentStatus,
    first: i64,
    second: i64,
) -> HydrusResult<()> {
    let table = table_name(relation, service_id, status);

    if !db.table_exists(&table)? {
        return Ok(());
    }

    let (first_column, second_column) = columns(relation);

    db.execute(
        &format!("DELETE FROM {table} WHERE {first_column} = ? AND {second_column} = ?"),
        &[&first, &second],
    )?;

    Ok(())
}

pub fn get_pairs(
    db: &DBBase,
    relation: TagRelation,
    service_id: i64,
    status: ContentStatus,
) -> HydrusResult<Vec<(i64, i64)>> {
    let table = table_name(relation, service_id, status);

    if !db.table_exists(&table)? {
        return Ok(Vec::new());
    }

    let (first_column, second_column) = columns(relation);

    db.query_all(
        &format!("SELECT {first_column}, {second_column} FROM {table}"),
        &[],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )
}

pub fn get_pairs_with_tags(
    db: &DBBase,
    relation: TagRelation,
    service_id: i64,
    status: ContentStatus,
) -> HydrusResult<Vec<(String, String)>> {
    let mut pairs = Vec::new();

    for (first, second) in get_pairs(db, relation, service_id, status)? {
        let first_tag = master_tags::get_tag(db, first)?;
        let second_tag = master_tags::get_tag(db, second)?;

        if let (Some(first_tag), Some(second_tag)) = (first_tag, second_tag) {
            pairs.push((first_tag, second_tag));
        }
    }

    Ok(pairs)
}

/// `STATICResolveSiblings` — follows the current sibling chain until stable.
pub fn resolve_siblings(db: &DBBase, service_id: i64, tag: &str) -> HydrusResult<String> {
    let mut current = tag.to_string();
    let mut seen = BTreeSet::new();

    loop {
        if !seen.insert(current.clone()) {
            return Ok(current);
        }

        let Some(tag_id) = master_tags_lookup(db, &current)? else {
            return Ok(current);
        };

        let table = table_name(TagRelation::Siblings, service_id, ContentStatus::Current);

        if !db.table_exists(&table)? {
            return Ok(current);
        }

        let next: Option<i64> = db.query_one(
            &format!("SELECT good_tag_id FROM {table} WHERE bad_tag_id = ?"),
            &[&tag_id],
            |row| row.get(0),
        )?;

        let Some(next) = next else {
            return Ok(current);
        };

        let Some(next_tag) = master_tags::get_tag(db, next)? else {
            return Ok(current);
        };

        current = next_tag;
    }
}

/// `STATICGetDescendants` for parents: returns all transitive ancestors.
pub fn get_ancestors(db: &DBBase, service_id: i64, tag: &str) -> HydrusResult<BTreeSet<String>> {
    let mut ancestors = BTreeSet::new();
    let mut frontier = vec![tag.to_string()];

    while let Some(current) = frontier.pop() {
        let Some(tag_id) = master_tags_lookup(db, &current)? else {
            continue;
        };

        let table = table_name(TagRelation::Parents, service_id, ContentStatus::Current);

        if !db.table_exists(&table)? {
            continue;
        }

        let parents: Vec<i64> = db.query_all(
            &format!("SELECT parent_tag_id FROM {table} WHERE child_tag_id = ?"),
            &[&tag_id],
            |row| row.get(0),
        )?;

        for parent_id in parents {
            if let Some(parent_tag) = master_tags::get_tag(db, parent_id)? {
                if ancestors.insert(parent_tag.clone()) {
                    frontier.push(parent_tag);
                }
            }
        }
    }

    Ok(ancestors)
}

fn master_tags_lookup(db: &DBBase, tag: &str) -> HydrusResult<Option<i64>> {
    let (namespace, subtag) = hydrus_core::hydrus_tags::split_tag(tag);

    db.query_one(
        "SELECT tags.tag_id FROM tags
         JOIN namespaces ON tags.namespace_id = namespaces.namespace_id
         JOIN subtags ON tags.subtag_id = subtags.subtag_id
         WHERE namespaces.namespace = ? AND subtags.subtag = ?",
        &[&namespace, &subtag],
        |row| row.get(0),
    )
}

#[cfg(test)]
mod tests;
