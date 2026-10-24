
fn add_service_full(
    db: &DBBase,
    service: &ServerService,
) -> HydrusResult<(i64, Vec<u8>)> {
    let service_id = insert_service_row(db, service)?;

    let null_account_type = AccountType::generate_null_account_type();
    let null_account_type_id = add_account_type(db, service_id, &null_account_type)?;

    let null_registration_keys =
        generate_registration_keys(db, service_id, 1, null_account_type_id, None, None)?;
    let null_access_key = get_access_key(db, service_id, &null_registration_keys[0])?;

    // The null access key is consumed here and never shown again.
    let _ = get_account_key_from_access_key(db, service_id, &null_access_key)?;

    let admin_account_type = AccountType::generate_admin_account_type(service.get_service_type())?;
    let admin_account_type_id = add_account_type(db, service_id, &admin_account_type)?;

    let force_registration_key = if service.get_service_type() == ServiceType::ServerAdmin {
        Some(b"init".as_slice())
    } else {
        None
    };

    let admin_registration_keys = generate_registration_keys(
        db,
        service_id,
        1,
        admin_account_type_id,
        None,
        force_registration_key,
    )?;
    let admin_access_key = get_access_key(db, service_id, &admin_registration_keys[0])?;

    if REPOSITORIES.contains(&service.get_service_type()) {
        create_repository_tables(db, service_id)?;
    }

    Ok((service_id, admin_access_key))
}

fn insert_service_row(db: &DBBase, service: &ServerService) -> HydrusResult<i64> {
    let (service_key, service_type, name, port, dictionary) = service.to_tuple()?;
    let dictionary_string = dictionary.dump_to_string();

    db.execute(
        "INSERT INTO services ( service_key, service_type, name, port, dictionary_string ) VALUES ( ?, ?, ?, ?, ? );",
        &[
            &service_key,
            &i64::from(service_type.as_int()),
            &name,
            &i64::from(port),
            &dictionary_string,
        ],
    )?;

    Ok(db.get_last_row_id())
}

fn add_account_type(
    db: &DBBase,
    service_id: i64,
    account_type: &AccountType,
) -> HydrusResult<i64> {
    db.execute(
        "INSERT INTO account_types ( service_id, dump ) VALUES ( ?, ? );",
        &[&service_id, &account_type.dump_to_string()],
    )?;

    Ok(db.get_last_row_id())
}

fn parse_account_type(dump: &str) -> HydrusResult<Arc<AccountType>> {
    let object = create_from_string(dump, false)?;

    let account_type = object
        .as_any()
        .downcast_ref::<AccountType>()
        .ok_or_else(|| {
            HydrusException::new(
                ErrorKind::Serialisation,
                "The account type did not deserialise as an AccountType.",
            )
        })?;

    Ok(Arc::new(account_type.clone()))
}

fn get_account_type(
    db: &DBBase,
    service_id: i64,
    account_type_id: i64,
) -> HydrusResult<Arc<AccountType>> {
    let dump = db
        .query_one(
            "SELECT dump FROM account_types WHERE service_id = ? AND account_type_id = ?;",
            &[&service_id, &account_type_id],
            |row| row.get::<_, String>(0),
        )?
        .ok_or_else(|| {
            HydrusException::new(
                ErrorKind::DataMissing,
                "Could not find the given account type for that service!",
            )
        })?;

    parse_account_type(&dump)
}

fn get_account_types(db: &DBBase, service_id: i64) -> HydrusResult<Vec<Arc<AccountType>>> {
    let dumps = db.query_all(
        "SELECT dump FROM account_types WHERE service_id = ? ORDER BY account_type_id;",
        &[&service_id],
        |row| row.get::<_, String>(0),
    )?;

    dumps.iter().map(|dump| parse_account_type(dump)).collect()
}

fn get_account_type_id(
    db: &DBBase,
    service_id: i64,
    account_type_key: &[u8],
) -> HydrusResult<i64> {
    let rows = db.query_all(
        "SELECT account_type_id, dump FROM account_types WHERE service_id = ?;",
        &[&service_id],
        |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)),
    )?;

    for (account_type_id, dump) in rows {
        let account_type = parse_account_type(&dump)?;

        if account_type.get_account_type_key() == account_type_key {
            return Ok(account_type_id);
        }
    }

    Err(HydrusException::new(
        ErrorKind::DataMissing,
        "Could not find the given account type key!",
    ))
}
