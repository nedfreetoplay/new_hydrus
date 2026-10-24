//! Services table from `hydrus/client/db/ClientDBServices.py`.

use hydrus_core::hydrus_constants::ServiceType;
use hydrus_core::hydrus_db_base::DBBase;
use hydrus_core::hydrus_exceptions::{ErrorKind, HydrusException, HydrusResult};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Service {
    pub service_id: i64,
    pub service_key: Vec<u8>,
    pub service_type: ServiceType,
    pub name: String,
    pub dictionary_string: String,
}

pub fn get_or_add_service(
    db: &DBBase,
    service_key: &[u8],
    service_type: ServiceType,
    name: &str,
) -> HydrusResult<i64> {
    if let Some(service_id) = db.query_one(
        "SELECT service_id FROM services WHERE service_key = ?",
        &[&service_key],
        |row| row.get(0),
    )? {
        return Ok(service_id);
    }

    let service_type_int = service_type.as_int();

    db.execute(
        "INSERT INTO services ( service_key, service_type, name, dictionary_string )
         VALUES ( ?, ?, ?, '' )",
        &[&service_key, &service_type_int, &name],
    )?;

    db.query_one(
        "SELECT service_id FROM services WHERE service_key = ?",
        &[&service_key],
        |row| row.get(0),
    )?
    .ok_or_else(|| HydrusException::new(ErrorKind::DataMissing, "service_id disappeared"))
}

pub fn get_service(db: &DBBase, service_key: &[u8]) -> HydrusResult<Option<Service>> {
    db.query_one(
        "SELECT service_id, service_key, service_type, name, dictionary_string
         FROM services WHERE service_key = ?",
        &[&service_key],
        row_to_service,
    )
}

pub fn get_services(db: &DBBase) -> HydrusResult<Vec<Service>> {
    db.query_all(
        "SELECT service_id, service_key, service_type, name, dictionary_string FROM services",
        &[],
        row_to_service,
    )
}

pub fn update_service(
    db: &DBBase,
    service_key: &[u8],
    name: &str,
    dictionary_string: &str,
) -> HydrusResult<()> {
    db.execute(
        "UPDATE services SET name = ?, dictionary_string = ? WHERE service_key = ?",
        &[&name, &dictionary_string, &service_key],
    )?;

    Ok(())
}

pub fn get_service_info(
    db: &DBBase,
    service_id: i64,
    info_type: i64,
) -> HydrusResult<Option<Vec<u8>>> {
    db.query_one(
        "SELECT info FROM service_info WHERE service_id = ? AND info_type = ?",
        &[&service_id, &info_type],
        |row| row.get(0),
    )
}

pub fn set_service_info(
    db: &DBBase,
    service_id: i64,
    info_type: i64,
    info: &[u8],
) -> HydrusResult<()> {
    db.execute(
        "INSERT OR REPLACE INTO service_info ( service_id, info_type, info ) VALUES ( ?, ?, ? )",
        &[&service_id, &info_type, &info],
    )?;

    Ok(())
}

fn row_to_service(row: &rusqlite::Row<'_>) -> rusqlite::Result<Service> {
    let service_type_int: u16 = row.get(2)?;
    let service_type = ServiceType::try_from_int(service_type_int).ok_or_else(|| {
        rusqlite::Error::FromSqlConversionFailure(
            2,
            rusqlite::types::Type::Integer,
            Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "unknown service type",
            )),
        )
    })?;

    Ok(Service {
        service_id: row.get(0)?,
        service_key: row.get(1)?,
        service_type,
        name: row.get(3)?,
        dictionary_string: row.get(4)?,
    })
}
