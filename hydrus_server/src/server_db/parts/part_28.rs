
fn generate_registration_keys(
    db: &DBBase,
    service_id: i64,
    num: usize,
    account_type_id: i64,
    expires: Option<i64>,
    force_registration_key: Option<&[u8]>,
) -> HydrusResult<Vec<Vec<u8>>> {
    let account_type = get_account_type(db, service_id, account_type_id)?;

    if account_type.is_null_account() {
        let existing = db.query_one(
            "SELECT 1 FROM accounts WHERE account_type_id = ?;",
            &[&account_type_id],
            |row| row.get::<_, i64>(0),
        )?;

        if existing.is_some() {
            return Err(HydrusException::new(
                ErrorKind::BadRequest,
                "You cannot create new null accounts!",
            ));
        }
    }

    let mut registration_keys = Vec::with_capacity(num);

    for _ in 0..num {
        let registration_key = match force_registration_key {
            Some(key) => key.to_vec(),
            None => generate_key().to_vec(),
        };
        let account_key = generate_key().to_vec();
        let access_key = generate_key().to_vec();
        let registration_key_hash = sha256(&registration_key);

        db.execute(
            "INSERT INTO registration_keys ( registration_key, service_id, account_type_id, account_key, access_key, expires ) VALUES ( ?, ?, ?, ?, ?, ? );",
            &[
                &registration_key_hash,
                &service_id,
                &account_type_id,
                &account_key,
                &access_key,
                &expires,
            ],
        )?;

        registration_keys.push(registration_key);
    }

    Ok(registration_keys)
}

fn get_access_key(
    db: &DBBase,
    service_id: i64,
    registration_key: &[u8],
) -> HydrusResult<Vec<u8>> {
    let registration_key_hash = sha256(registration_key);

    let exists = db.query_one(
        "SELECT 1 FROM registration_keys WHERE service_id = ? AND registration_key = ?;",
        &[&service_id, &registration_key_hash],
        |row| row.get::<_, i64>(0),
    )?;

    if exists.is_none() {
        return Err(HydrusException::new(
            ErrorKind::InsufficientCredentials,
            "The service could not find that registration token in its database.",
        ));
    }

    let access_key = generate_key().to_vec();

    db.execute(
        "UPDATE registration_keys SET access_key = ? WHERE service_id = ? AND registration_key = ?;",
        &[&access_key, &service_id, &registration_key_hash],
    )?;

    Ok(access_key)
}

fn get_account_key_from_access_key(
    db: &DBBase,
    service_id: i64,
    access_key: &[u8],
) -> HydrusResult<Vec<u8>> {
    let hashed_access_key = sha256(access_key);

    if let Some(account_key) = db.query_one(
        "SELECT account_key FROM accounts WHERE service_id = ? AND hashed_access_key = ?;",
        &[&service_id, &hashed_access_key],
        |row| row.get::<_, Vec<u8>>(0),
    )? {
        return Ok(account_key);
    }

    let Some((account_type_id, account_key, expires)) = db.query_one(
        "SELECT account_type_id, account_key, expires FROM registration_keys WHERE access_key = ?;",
        &[&access_key],
        |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, Vec<u8>>(1)?,
                row.get::<_, Option<i64>>(2)?,
            ))
        },
    )?
    else {
        return Err(HydrusException::new(
            ErrorKind::InsufficientCredentials,
            "The service could not find that account in its database.",
        ));
    };

    db.execute(
        "DELETE FROM registration_keys WHERE access_key = ?;",
        &[&access_key],
    )?;

    let account_type = get_account_type(db, service_id, account_type_id)?;
    let created = hydrus_time::get_now();
    let account = NetworkAccount::new(account_key.clone(), account_type, created, expires);
    let (stored_key, _, _, _, dictionary) =
        NetworkAccount::generate_tuple_from_account(&account)?;

    db.execute(
        "INSERT INTO accounts ( service_id, account_key, hashed_access_key, account_type_id, created, expires, dictionary_string ) VALUES ( ?, ?, ?, ?, ?, ?, ? );",
        &[
            &service_id,
            &stored_key,
            &hashed_access_key,
            &account_type_id,
            &created,
            &expires,
            &dictionary.dump_to_string(),
        ],
    )?;

    Ok(stored_key)
}

fn get_account(db: &DBBase, service_id: i64, account_id: i64) -> HydrusResult<NetworkAccount> {
    let (account_key, account_type_id, created, expires, dictionary_string) = db
        .query_one(
            "SELECT account_key, account_type_id, created, expires, dictionary_string FROM accounts WHERE service_id = ? AND account_id = ?;",
            &[&service_id, &account_id],
            |row| {
                Ok((
                    row.get::<_, Vec<u8>>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, Option<i64>>(3)?,
                    row.get::<_, String>(4)?,
                ))
            },
        )?
        .ok_or_else(|| {
            HydrusException::new(
                ErrorKind::InsufficientCredentials,
                "The service could not find that account in its database.",
            )
        })?;

    let account_type = get_account_type(db, service_id, account_type_id)?;
    let dictionary = parse_dictionary(&dictionary_string)?;

    NetworkAccount::generate_account_from_tuple(
        account_key,
        account_type,
        created,
        expires,
        dictionary,
    )
}
