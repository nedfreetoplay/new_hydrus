fn create_repository_tag_sibling_tables(db: &DBBase, service_id: i64) -> HydrusResult<()> {
    let (current_siblings, deleted_siblings, pending_siblings, petitioned_siblings) =
        generate_repository_tag_siblings_table_names(service_id);

    db.execute(
        &format!("CREATE TABLE {current_siblings} ( bad_service_tag_id INTEGER PRIMARY KEY, good_service_tag_id INTEGER, account_id INTEGER, sibling_timestamp INTEGER );"),
        &[],
    )?;
    db.execute(
        &format!("CREATE INDEX {current_siblings}_account_id_index ON {current_siblings} ( account_id );"),
        &[],
    )?;
    db.execute(
        &format!("CREATE INDEX {current_siblings}_sibling_timestamp_index ON {current_siblings} ( sibling_timestamp );"),
        &[],
    )?;

    db.execute(
        &format!("CREATE TABLE {deleted_siblings} ( bad_service_tag_id INTEGER PRIMARY KEY, good_service_tag_id INTEGER, account_id INTEGER, sibling_timestamp INTEGER );"),
        &[],
    )?;
    db.execute(
        &format!("CREATE INDEX {deleted_siblings}_account_id_index ON {deleted_siblings} ( account_id );"),
        &[],
    )?;
    db.execute(
        &format!("CREATE INDEX {deleted_siblings}_sibling_timestamp_index ON {deleted_siblings} ( sibling_timestamp );"),
        &[],
    )?;

    db.execute(
        &format!("CREATE TABLE {pending_siblings} ( bad_master_tag_id INTEGER, good_master_tag_id INTEGER, account_id INTEGER, reason_id INTEGER, PRIMARY KEY ( bad_master_tag_id, account_id ) ) WITHOUT ROWID;"),
        &[],
    )?;
    db.execute(
        &format!("CREATE INDEX {pending_siblings}_account_id_reason_id_index ON {pending_siblings} ( account_id, reason_id );"),
        &[],
    )?;

    db.execute(
        &format!("CREATE TABLE {petitioned_siblings} ( bad_service_tag_id INTEGER, good_service_tag_id INTEGER, account_id INTEGER, reason_id INTEGER, PRIMARY KEY ( bad_service_tag_id, account_id ) ) WITHOUT ROWID;"),
        &[],
    )?;
    db.execute(
        &format!("CREATE INDEX {petitioned_siblings}_account_id_reason_id_index ON {petitioned_siblings} ( account_id, reason_id );"),
        &[],
    )?;

    Ok(())
}
