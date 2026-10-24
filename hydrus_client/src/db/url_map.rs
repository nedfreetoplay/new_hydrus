//! URL mapping from `hydrus/client/db/ClientDBURLMap.py`.

use hydrus_core::hydrus_db_base::DBBase;
use hydrus_core::hydrus_exceptions::HydrusResult;

pub fn add_url(db: &DBBase, hash_id: i64, url: &str) -> HydrusResult<()> {
    let domain = domain_of(url);
    let domain_id = get_or_create(db, "url_domains", "domain_id", "domain", &domain)?;
    let url_id = get_or_create_url(db, domain_id, url)?;

    db.execute(
        "INSERT OR IGNORE INTO url_map ( hash_id, url_id ) VALUES ( ?, ? )",
        &[&hash_id, &url_id],
    )?;

    Ok(())
}

pub fn get_urls(db: &DBBase, hash_id: i64) -> HydrusResult<Vec<String>> {
    db.query_all(
        "SELECT urls.url FROM url_map
         JOIN urls ON url_map.url_id = urls.url_id
         WHERE url_map.hash_id = ?",
        &[&hash_id],
        |row| row.get(0),
    )
}

pub fn delete_url(db: &DBBase, hash_id: i64, url: &str) -> HydrusResult<()> {
    db.execute(
        "DELETE FROM url_map WHERE hash_id = ? AND url_id IN
         ( SELECT url_id FROM urls WHERE url = ? )",
        &[&hash_id, &url],
    )?;

    Ok(())
}

pub fn get_num_urls(db: &DBBase, hash_id: i64) -> HydrusResult<i64> {
    Ok(db
        .query_one(
            "SELECT COUNT(*) FROM url_map WHERE hash_id = ?",
            &[&hash_id],
            |row| row.get(0),
        )?
        .unwrap_or(0))
}

fn domain_of(url: &str) -> String {
    url.split("://")
        .nth(1)
        .unwrap_or(url)
        .split('/')
        .next()
        .unwrap_or("")
        .to_lowercase()
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
    .ok_or_else(|| hydrus_core::hydrus_exceptions::data_missing("row id disappeared"))
}

fn get_or_create_url(db: &DBBase, domain_id: i64, url: &str) -> HydrusResult<i64> {
    if let Some(id) = db.query_one("SELECT url_id FROM urls WHERE url = ?", &[&url], |row| {
        row.get(0)
    })? {
        return Ok(id);
    }

    db.execute(
        "INSERT INTO urls ( domain_id, url ) VALUES ( ?, ? )",
        &[&domain_id, &url],
    )?;

    db.query_one("SELECT url_id FROM urls WHERE url = ?", &[&url], |row| {
        row.get(0)
    })?
    .ok_or_else(|| hydrus_core::hydrus_exceptions::data_missing("url id disappeared"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::master::get_hash_id;
    use crate::db::ClientDB;

    fn database() -> (ClientDB, std::path::PathBuf) {
        let directory = std::env::temp_dir().join(format!(
            "hydrus_client_urls_{}_{}",
            std::process::id(),
            rand::random::<u64>()
        ));

        (ClientDB::new(&directory).unwrap(), directory)
    }

    #[test]
    fn urls_are_deduped_and_domain_grouped() {
        let (db, directory) = database();
        let hash_id = get_hash_id(db.db(), &[1]).unwrap();

        add_url(db.db(), hash_id, "https://example.com/one").unwrap();
        add_url(db.db(), hash_id, "https://example.com/two").unwrap();
        add_url(db.db(), hash_id, "https://example.com/one").unwrap();

        let urls = get_urls(db.db(), hash_id).unwrap();

        assert_eq!(urls.len(), 2);
        assert_eq!(get_num_urls(db.db(), hash_id).unwrap(), 2);
        assert_eq!(
            db.db()
                .query_one("SELECT domain FROM url_domains", &[], |row| row
                    .get::<_, String>(0))
                .unwrap()
                .as_deref(),
            Some("example.com")
        );

        drop(db);
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn delete_removes_only_the_mapping() {
        let (db, directory) = database();
        let hash_id = get_hash_id(db.db(), &[2]).unwrap();

        add_url(db.db(), hash_id, "https://example.com/one").unwrap();
        delete_url(db.db(), hash_id, "https://example.com/one").unwrap();

        assert!(get_urls(db.db(), hash_id).unwrap().is_empty());

        drop(db);
        let _ = std::fs::remove_dir_all(directory);
    }
}
