fn create_repository_mapping_tables(db: &DBBase, service_id: i64) -> HydrusResult<()> {
    let (current_mappings, deleted_mappings, pending_mappings, petitioned_mappings) =
        generate_repository_mappings_table_names(service_id);

    db.execute(
        &format!("CREATE TABLE {current_mappings} ( service_tag_id INTEGER, service_hash_id INTEGER, account_id INTEGER, mapping_timestamp INTEGER, PRIMARY KEY ( service_tag_id, service_hash_id ) ) WITHOUT ROWID;"),
        &[],
    )?;
    create_qualified_index(db, &current_mappings, "account_id", "account_id")?;
    create_qualified_index(
        db,
        &current_mappings,
        "mapping_timestamp",
        "mapping_timestamp",
    )?;

    db.execute(
        &format!("CREATE TABLE {deleted_mappings} ( service_tag_id INTEGER, service_hash_id INTEGER, account_id INTEGER, mapping_timestamp INTEGER, PRIMARY KEY ( service_tag_id, service_hash_id ) ) WITHOUT ROWID;"),
        &[],
    )?;
    create_qualified_index(db, &deleted_mappings, "account_id", "account_id")?;
    create_qualified_index(
        db,
        &deleted_mappings,
        "mapping_timestamp",
        "mapping_timestamp",
    )?;

    db.execute(
        &format!("CREATE TABLE {pending_mappings} ( master_tag_id INTEGER, master_hash_id INTEGER, account_id INTEGER, reason_id INTEGER, PRIMARY KEY ( master_tag_id, master_hash_id, account_id ) ) WITHOUT ROWID;"),
        &[],
    )?;
    create_qualified_index(
        db,
        &pending_mappings,
        "account_id_reason_id",
        "account_id, reason_id",
    )?;

    db.execute(
        &format!("CREATE TABLE {petitioned_mappings} ( service_tag_id INTEGER, service_hash_id INTEGER, account_id INTEGER, reason_id INTEGER, PRIMARY KEY ( service_tag_id, service_hash_id, account_id ) ) WITHOUT ROWID;"),
        &[],
    )?;
    create_qualified_index(
        db,
        &petitioned_mappings,
        "account_id_reason_id",
        "account_id, reason_id",
    )?;

    Ok(())
}
