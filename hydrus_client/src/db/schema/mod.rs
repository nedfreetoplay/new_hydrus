//! Initial client database schema.

use hydrus_core::hydrus_constants::SOFTWARE_VERSION;
use hydrus_core::hydrus_db_base::DBBase;
use hydrus_core::hydrus_exceptions::HydrusResult;

mod core;
mod duplicates;
mod files;
mod tags;

pub fn create_db(db: &DBBase) -> HydrusResult<()> {
    for tables in [
        core::TABLES,
        files::TABLES,
        tags::TABLES,
        duplicates::TABLES,
    ] {
        db.execute_batch(tables)?;
    }

    let count: i64 = db
        .query_one("SELECT COUNT(*) FROM version", &[], |row| row.get(0))?
        .unwrap_or(0);

    if count == 0 {
        db.execute(
            "INSERT INTO version ( version ) VALUES ( ? )",
            &[&SOFTWARE_VERSION],
        )?;
    }

    Ok(())
}
