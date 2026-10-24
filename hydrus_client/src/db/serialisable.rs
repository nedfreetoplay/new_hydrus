//! Serialisable object storage from `hydrus/client/db/ClientDBSerialisable.py`.

use hydrus_core::hydrus_db_base::DBBase;
use hydrus_core::hydrus_exceptions::HydrusResult;
use hydrus_core::hydrus_serialisable::SerialisableObject;
use serde_json::Value;

pub fn set_json_dump(
    db: &DBBase,
    name: &str,
    dump_type: u16,
    version: u16,
    dump: &str,
) -> HydrusResult<()> {
    db.execute(
        "INSERT OR REPLACE INTO json_dumps ( name, dump_type, version, dump )
         VALUES ( ?, ?, ?, ? )",
        &[&name, &dump_type, &version, &dump],
    )?;

    Ok(())
}

pub fn get_json_dump(db: &DBBase, name: &str) -> HydrusResult<Option<String>> {
    db.query_one(
        "SELECT dump FROM json_dumps WHERE name = ?",
        &[&name],
        |row| row.get(0),
    )
}

pub fn set_serialisable(
    db: &DBBase,
    name: &str,
    object: &dyn SerialisableObject,
) -> HydrusResult<()> {
    set_json_dump(
        db,
        name,
        object.serialisable_type(),
        object.serialisable_version(),
        &object.dump_to_string(),
    )
}

pub fn get_serialisable(
    db: &DBBase,
    name: &str,
) -> HydrusResult<Option<Box<dyn SerialisableObject>>> {
    let Some(dump) = get_json_dump(db, name)? else {
        return Ok(None);
    };

    hydrus_core::hydrus_serialisable::create_from_string(&dump, false).map(Some)
}

pub fn get_json_value(db: &DBBase, name: &str) -> HydrusResult<Option<Value>> {
    let Some(dump) = get_json_dump(db, name)? else {
        return Ok(None);
    };

    Ok(serde_json::from_str(&dump).ok())
}

/// `SetJSONSimple` / `GetJSONSimple`
pub fn set_json_simple(db: &DBBase, name: &str, value: &Value) -> HydrusResult<()> {
    db.execute(
        "INSERT OR REPLACE INTO json_dict ( name, dump ) VALUES ( ?, ? )",
        &[&name, &serde_json::to_string(value).unwrap_or_default()],
    )?;

    Ok(())
}

pub fn get_json_simple(db: &DBBase, name: &str) -> HydrusResult<Option<Value>> {
    let dump: Option<String> = db.query_one(
        "SELECT dump FROM json_dict WHERE name = ?",
        &[&name],
        |row| row.get(0),
    )?;

    Ok(dump.and_then(|dump| serde_json::from_str(&dump).ok()))
}

/// `SetJSONComplex` with a named dump.
pub fn set_json_dump_named(
    db: &DBBase,
    dump_type: u16,
    dump_name: &str,
    version: u16,
    timestamp_ms: i64,
    dump: &str,
) -> HydrusResult<()> {
    db.execute(
        "INSERT OR REPLACE INTO json_dumps_named
         ( dump_type, dump_name, version, timestamp_ms, dump )
         VALUES ( ?, ?, ?, ?, ? )",
        &[&dump_type, &dump_name, &version, &timestamp_ms, &dump],
    )?;

    Ok(())
}

pub fn get_json_dump_named(
    db: &DBBase,
    dump_type: u16,
    dump_name: &str,
) -> HydrusResult<Option<(u16, i64, String)>> {
    db.query_one(
        "SELECT version, timestamp_ms, dump FROM json_dumps_named
         WHERE dump_type = ? AND dump_name = ?",
        &[&dump_type, &dump_name],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    )
}

pub fn get_json_dump_names(db: &DBBase, dump_type: u16) -> HydrusResult<Vec<String>> {
    db.query_all(
        "SELECT dump_name FROM json_dumps_named WHERE dump_type = ? ORDER BY dump_name",
        &[&dump_type],
        |row| row.get(0),
    )
}

/// `SetJSONDump( obj, force_timestamp_ms )` for hashed dumps.
pub fn set_json_dump_hashed(
    db: &DBBase,
    hash: &[u8],
    dump_type: u16,
    version: u16,
    dump: &str,
) -> HydrusResult<()> {
    db.execute(
        "INSERT OR REPLACE INTO json_dumps_hashed ( hash, dump_type, version, dump )
         VALUES ( ?, ?, ?, ? )",
        &[&hash, &dump_type, &version, &dump],
    )?;

    Ok(())
}

pub fn get_json_dump_hashed(
    db: &DBBase,
    hash: &[u8],
    dump_type: u16,
) -> HydrusResult<Option<(u16, String)>> {
    db.query_one(
        "SELECT version, dump FROM json_dumps_hashed WHERE hash = ? AND dump_type = ?",
        &[&hash, &dump_type],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::ClientDB;

    fn database() -> (ClientDB, std::path::PathBuf) {
        let directory = std::env::temp_dir().join(format!(
            "hydrus_client_serialisable_{}_{}",
            std::process::id(),
            rand::random::<u64>()
        ));

        (ClientDB::new(&directory).unwrap(), directory)
    }

    #[test]
    fn simple_json_round_trips() {
        let (db, directory) = database();
        let value = serde_json::json!({"a": 1});

        set_json_simple(db.db(), "simple", &value).unwrap();

        assert_eq!(get_json_simple(db.db(), "simple").unwrap(), Some(value));

        drop(db);
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn named_dumps_round_trip_and_list() {
        let (db, directory) = database();

        set_json_dump_named(db.db(), 22, "options", 8, 100, "{}").unwrap();
        set_json_dump_named(db.db(), 22, "backup", 8, 200, "{}").unwrap();

        assert_eq!(
            get_json_dump_names(db.db(), 22).unwrap(),
            vec!["backup", "options"]
        );
        assert_eq!(
            get_json_dump_named(db.db(), 22, "options").unwrap(),
            Some((8, 100, "{}".to_string()))
        );

        drop(db);
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn hashed_dumps_round_trip() {
        let (db, directory) = database();

        set_json_dump_hashed(db.db(), &[1, 2], 22, 8, "{}").unwrap();

        assert_eq!(
            get_json_dump_hashed(db.db(), &[1, 2], 22).unwrap(),
            Some((8, "{}".to_string()))
        );

        drop(db);
        let _ = std::fs::remove_dir_all(directory);
    }
}
