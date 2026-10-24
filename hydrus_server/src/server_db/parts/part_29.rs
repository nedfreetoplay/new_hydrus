
fn get_account_id(db: &DBBase, account_key: &[u8]) -> HydrusResult<i64> {
    db.query_one(
        "SELECT account_id FROM accounts WHERE account_key = ?;",
        &[&account_key],
        |row| row.get::<_, i64>(0),
    )?
    .ok_or_else(|| {
        HydrusException::new(
            ErrorKind::InsufficientCredentials,
            "The service could not find that account id in its database.",
        )
    })
}

fn get_account_key_from_account_id(db: &DBBase, account_id: i64) -> HydrusResult<Vec<u8>> {
    db.query_one(
        "SELECT account_key FROM accounts WHERE account_id = ?;",
        &[&account_id],
        |row| row.get::<_, Vec<u8>>(0),
    )?
    .ok_or_else(|| {
        HydrusException::new(
            ErrorKind::InsufficientCredentials,
            "The service could not find that account_id in its database.",
        )
    })
}

fn verify_access_key(
    db: &DBBase,
    service_id: i64,
    access_key: &[u8],
) -> HydrusResult<bool> {
    let hashed_access_key = sha256(access_key);

    if db
        .query_one(
            "SELECT 1 FROM accounts WHERE service_id = ? AND hashed_access_key = ?;",
            &[&service_id, &hashed_access_key],
            |row| row.get::<_, i64>(0),
        )?
        .is_some()
    {
        return Ok(true);
    }

    Ok(db
        .query_one(
            "SELECT 1 FROM registration_keys WHERE service_id = ? AND access_key = ?;",
            &[&service_id, &access_key],
            |row| row.get::<_, i64>(0),
        )?
        .is_some())
}

fn parse_dictionary(dump: &str) -> HydrusResult<SerialisableDictionary> {
    let object = create_from_string(dump, false)?;

    let dictionary = object
        .as_any()
        .downcast_ref::<SerialisableDictionary>()
        .ok_or_else(|| {
            HydrusException::new(
                ErrorKind::Serialisation,
                "The account dictionary did not deserialise as a SerialisableDictionary.",
            )
        })?;

    Ok(dictionary.clone())
}

fn sha256(data: &[u8]) -> Vec<u8> {
    Sha256::digest(data).to_vec()
}
