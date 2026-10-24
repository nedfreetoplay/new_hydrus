fn create_repository_file_tables(db: &DBBase, service_id: i64) -> HydrusResult<()> {
    let (current_files, deleted_files, pending_files, petitioned_files, ip_addresses) =
        generate_repository_files_table_names(service_id);

    db.execute(
        &format!("CREATE TABLE {current_files} ( service_hash_id INTEGER PRIMARY KEY, account_id INTEGER, file_timestamp INTEGER );"),
        &[],
    )?;
    db.execute(
        &format!("CREATE INDEX {current_files}_account_id_index ON {current_files} ( account_id );"),
        &[],
    )?;
    db.execute(
        &format!("CREATE INDEX {current_files}_file_timestamp_index ON {current_files} ( file_timestamp );"),
        &[],
    )?;

    db.execute(
        &format!("CREATE TABLE {deleted_files} ( service_hash_id INTEGER PRIMARY KEY, account_id INTEGER, file_timestamp INTEGER );"),
        &[],
    )?;
    db.execute(
        &format!("CREATE INDEX {deleted_files}_account_id_index ON {deleted_files} ( account_id );"),
        &[],
    )?;
    db.execute(
        &format!("CREATE INDEX {deleted_files}_file_timestamp_index ON {deleted_files} ( file_timestamp );"),
        &[],
    )?;

    db.execute(
        &format!("CREATE TABLE {pending_files} ( master_hash_id INTEGER, account_id INTEGER, reason_id INTEGER, PRIMARY KEY ( master_hash_id, account_id ) ) WITHOUT ROWID;"),
        &[],
    )?;
    db.execute(
        &format!("CREATE INDEX {pending_files}_account_id_reason_id_index ON {pending_files} ( account_id, reason_id );"),
        &[],
    )?;

    db.execute(
        &format!("CREATE TABLE {petitioned_files} ( service_hash_id INTEGER, account_id INTEGER, reason_id INTEGER, PRIMARY KEY ( service_hash_id, account_id ) ) WITHOUT ROWID;"),
        &[],
    )?;
    db.execute(
        &format!("CREATE INDEX {petitioned_files}_account_id_reason_id_index ON {petitioned_files} ( account_id, reason_id );"),
        &[],
    )?;

    db.execute(
        &format!("CREATE TABLE {ip_addresses} ( master_hash_id INTEGER, ip TEXT, ip_timestamp INTEGER );"),
        &[],
    )?;

    Ok(())
}
