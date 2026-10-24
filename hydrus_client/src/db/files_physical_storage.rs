//! Physical storage configuration from `hydrus/client/db/ClientDBFilesPhysicalStorage.py`.

use hydrus_core::hydrus_db_base::DBBase;
use hydrus_core::hydrus_exceptions::HydrusResult;

use crate::files::{FilesStorageBaseLocation, FilesStorageSubfolder};

pub fn set_current_locations(
    db: &DBBase,
    locations: &[FilesStorageBaseLocation],
) -> HydrusResult<()> {
    db.execute("DELETE FROM current_client_files_locations", &[])?;

    for location in locations {
        db.execute(
            "INSERT INTO current_client_files_locations
             ( path, weight, max_num_bytes ) VALUES ( ?, ?, ? )",
            &[
                &location.path.to_string_lossy().to_string(),
                &location.ideal_weight,
                &location.max_num_bytes.map(|value| value as i64),
            ],
        )?;
    }

    Ok(())
}

pub fn get_current_locations(db: &DBBase) -> HydrusResult<Vec<FilesStorageBaseLocation>> {
    get_locations(db, "current_client_files_locations")
}

pub fn set_ideal_locations(
    db: &DBBase,
    locations: &[FilesStorageBaseLocation],
) -> HydrusResult<()> {
    db.execute("DELETE FROM ideal_client_files_locations", &[])?;

    for location in locations {
        db.execute(
            "INSERT INTO ideal_client_files_locations
             ( path, weight, max_num_bytes ) VALUES ( ?, ?, ? )",
            &[
                &location.path.to_string_lossy().to_string(),
                &location.ideal_weight,
                &location.max_num_bytes.map(|value| value as i64),
            ],
        )?;
    }

    Ok(())
}

pub fn get_ideal_locations(db: &DBBase) -> HydrusResult<Vec<FilesStorageBaseLocation>> {
    get_locations(db, "ideal_client_files_locations")
}

fn get_locations(db: &DBBase, table: &str) -> HydrusResult<Vec<FilesStorageBaseLocation>> {
    db.query_all(
        &format!("SELECT path, weight, max_num_bytes FROM {table}"),
        &[],
        |row| {
            Ok(FilesStorageBaseLocation::new(
                row.get::<_, String>(0)?,
                row.get(1)?,
                row.get::<_, Option<i64>>(2)?.map(|value| value as u64),
            ))
        },
    )
}

pub fn set_subfolders(db: &DBBase, subfolders: &[FilesStorageSubfolder]) -> HydrusResult<()> {
    db.execute("DELETE FROM client_files_subfolders", &[])?;

    for subfolder in subfolders {
        db.execute(
            "INSERT INTO client_files_subfolders ( prefix, location ) VALUES ( ?, ? )",
            &[
                &subfolder.prefix,
                &subfolder.base_location.path.to_string_lossy().to_string(),
            ],
        )?;
    }

    Ok(())
}

pub fn get_subfolders(
    db: &DBBase,
    locations: &[FilesStorageBaseLocation],
) -> HydrusResult<Vec<FilesStorageSubfolder>> {
    let rows: Vec<(String, String)> = db.query_all(
        "SELECT prefix, location FROM client_files_subfolders",
        &[],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;

    let mut subfolders = Vec::new();

    for (prefix, location_path) in rows {
        if let Some(location) = locations
            .iter()
            .find(|location| location.path.to_string_lossy() == location_path)
        {
            subfolders.push(FilesStorageSubfolder::new(prefix, location.clone()));
        }
    }

    Ok(subfolders)
}

pub fn set_ideal_thumbnail_override_location(db: &DBBase, path: Option<&str>) -> HydrusResult<()> {
    db.execute("DELETE FROM ideal_thumbnail_override_location", &[])?;

    if let Some(path) = path {
        db.execute(
            "INSERT INTO ideal_thumbnail_override_location ( path ) VALUES ( ? )",
            &[&path],
        )?;
    }

    Ok(())
}

pub fn get_ideal_thumbnail_override_location(db: &DBBase) -> HydrusResult<Option<String>> {
    db.query_one(
        "SELECT path FROM ideal_thumbnail_override_location LIMIT 1",
        &[],
        |row| row.get(0),
    )
}

pub fn set_storage_granularity(db: &DBBase, granularity: i64) -> HydrusResult<()> {
    db.execute("DELETE FROM current_storage_granularity", &[])?;
    db.execute(
        "INSERT INTO current_storage_granularity ( granularity ) VALUES ( ? )",
        &[&granularity],
    )?;

    Ok(())
}

pub fn get_storage_granularity(db: &DBBase) -> HydrusResult<Option<i64>> {
    db.query_one(
        "SELECT granularity FROM current_storage_granularity LIMIT 1",
        &[],
        |row| row.get(0),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::ClientDB;

    fn database() -> (ClientDB, std::path::PathBuf) {
        let directory = std::env::temp_dir().join(format!(
            "hydrus_client_physical_{}_{}",
            std::process::id(),
            rand::random::<u64>()
        ));

        (ClientDB::new(&directory).unwrap(), directory)
    }

    #[test]
    fn locations_round_trip() {
        let (db, directory) = database();
        let locations = vec![
            FilesStorageBaseLocation::new("C:/hydrus/a", 3, None),
            FilesStorageBaseLocation::new("C:/hydrus/b", 7, Some(1024)),
        ];

        set_current_locations(db.db(), &locations).unwrap();
        set_ideal_locations(db.db(), &locations).unwrap();

        assert_eq!(get_current_locations(db.db()).unwrap(), locations);
        assert_eq!(get_ideal_locations(db.db()).unwrap(), locations);

        drop(db);
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn subfolders_resolve_base_locations() {
        let (db, directory) = database();
        let location = FilesStorageBaseLocation::new("C:/hydrus/a", 1, None);
        let subfolders = vec![FilesStorageSubfolder::new("fa", location.clone())];

        set_subfolders(db.db(), &subfolders).unwrap();

        let loaded = get_subfolders(db.db(), &[location]).unwrap();

        assert_eq!(loaded, subfolders);

        drop(db);
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn granularity_and_thumbnail_override_round_trip() {
        let (db, directory) = database();

        set_storage_granularity(db.db(), 5).unwrap();
        set_ideal_thumbnail_override_location(db.db(), Some("C:/thumbs")).unwrap();

        assert_eq!(get_storage_granularity(db.db()).unwrap(), Some(5));
        assert_eq!(
            get_ideal_thumbnail_override_location(db.db())
                .unwrap()
                .as_deref(),
            Some("C:/thumbs")
        );

        drop(db);
        let _ = std::fs::remove_dir_all(directory);
    }
}
