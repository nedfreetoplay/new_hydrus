//! Display/suggestion helpers from `ClientDBTagDisplay.py` and
//! `ClientDBTagSuggestions.py`.

use hydrus_core::hydrus_db_base::DBBase;
use hydrus_core::hydrus_exceptions::HydrusResult;
use hydrus_core::hydrus_tags;

use crate::db::tag_relations;

pub fn get_display_tag(db: &DBBase, tag_service_id: i64, tag: &str) -> HydrusResult<String> {
    tag_relations::resolve_siblings(db, tag_service_id, tag)
}

pub fn get_display_tags(
    db: &DBBase,
    tag_service_id: i64,
    tags: impl IntoIterator<Item = String>,
) -> HydrusResult<Vec<String>> {
    let mut display = Vec::new();

    for tag in tags {
        let resolved = get_display_tag(db, tag_service_id, &tag)?;

        if !display.contains(&resolved) {
            display.push(resolved);
        }
    }

    display.sort();

    Ok(display)
}

/// Returns display tags that start with the requested prefix.
pub fn get_suggested_tags(db: &DBBase, prefix: &str, limit: usize) -> HydrusResult<Vec<String>> {
    let pattern = format!("{}%", escape_like(prefix));

    let rows: Vec<(String, String)> = db.query_all(
        "SELECT namespaces.namespace, subtags.subtag
         FROM tags
         JOIN namespaces ON tags.namespace_id = namespaces.namespace_id
         JOIN subtags ON tags.subtag_id = subtags.subtag_id
         WHERE (CASE WHEN namespaces.namespace = '' THEN subtags.subtag
                     ELSE namespaces.namespace || ':' || subtags.subtag END) LIKE ? ESCAPE '\\'
         ORDER BY subtags.subtag
         LIMIT ?",
        &[&pattern, &(limit as i64)],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;

    Ok(rows
        .into_iter()
        .map(|(namespace, subtag)| hydrus_tags::combine_tag(&namespace, &subtag))
        .collect())
}

pub fn get_namespaces(db: &DBBase) -> HydrusResult<Vec<String>> {
    db.query_all(
        "SELECT namespace FROM namespaces ORDER BY namespace",
        &[],
        |row| row.get(0),
    )
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
    use crate::db::ClientDB;
    use crate::db::master_tags::get_tag_id;
    use crate::db::tag_relations::{self, TagRelation};
    use hydrus_core::hydrus_constants::ContentStatus;

    fn database() -> (ClientDB, std::path::PathBuf) {
        let directory = std::env::temp_dir().join(format!(
            "hydrus_client_tagdisplay_{}_{}",
            std::process::id(),
            rand::random::<u64>()
        ));

        (ClientDB::new(&directory).unwrap(), directory)
    }

    #[test]
    fn display_tags_resolve_siblings() {
        let (db, directory) = database();
        let bad = get_tag_id(db.db(), "blue eyes").unwrap();
        let good = get_tag_id(db.db(), "eyes:blue").unwrap();

        tag_relations::add_pair(
            db.db(),
            TagRelation::Siblings,
            1,
            ContentStatus::Current,
            bad,
            good,
            None,
        )
        .unwrap();

        assert_eq!(
            get_display_tag(db.db(), 1, "blue eyes").unwrap(),
            "eyes:blue"
        );
        assert_eq!(
            get_display_tags(db.db(), 1, ["blue eyes".to_string()]).unwrap(),
            vec!["eyes:blue"]
        );

        drop(db);
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn suggestions_match_prefix() {
        let (db, directory) = database();
        get_tag_id(db.db(), "blue eyes").unwrap();
        get_tag_id(db.db(), "blonde hair").unwrap();
        get_tag_id(db.db(), "green eyes").unwrap();

        let suggestions = get_suggested_tags(db.db(), "bl", 10).unwrap();

        assert_eq!(suggestions, vec!["blonde hair", "blue eyes"]);

        drop(db);
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn namespace_list_is_sorted() {
        let (db, directory) = database();
        get_tag_id(db.db(), "creator:someone").unwrap();
        get_tag_id(db.db(), "series:thing").unwrap();

        let namespaces = get_namespaces(db.db()).unwrap();

        assert_eq!(namespaces, vec!["creator", "series"]);

        drop(db);
        let _ = std::fs::remove_dir_all(directory);
    }
}
