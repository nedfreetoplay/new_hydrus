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
