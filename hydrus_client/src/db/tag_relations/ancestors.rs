//! Split out from the parent module.

use super::*;

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

pub(super) fn master_tags_lookup(db: &DBBase, tag: &str) -> HydrusResult<Option<i64>> {
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
