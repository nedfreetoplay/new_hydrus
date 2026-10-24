fn create_repository_tag_parent_tables(db: &DBBase, service_id: i64) -> HydrusResult<()> {
    let (current_parents, deleted_parents, pending_parents, petitioned_parents) =
        generate_repository_tag_parents_table_names(service_id);

    db.execute(
        &format!("CREATE TABLE {current_parents} ( child_service_tag_id INTEGER, parent_service_tag_id INTEGER, account_id INTEGER, parent_timestamp INTEGER, PRIMARY KEY ( child_service_tag_id, parent_service_tag_id ) ) WITHOUT ROWID;"),
        &[],
    )?;
    db.execute(
        &format!("CREATE INDEX {current_parents}_account_id_index ON {current_parents} ( account_id );"),
        &[],
    )?;
    db.execute(
        &format!("CREATE INDEX {current_parents}_parent_timestamp_index ON {current_parents} ( parent_timestamp );"),
        &[],
    )?;

    db.execute(
        &format!("CREATE TABLE {deleted_parents} ( child_service_tag_id INTEGER, parent_service_tag_id INTEGER, account_id INTEGER, parent_timestamp INTEGER, PRIMARY KEY ( child_service_tag_id, parent_service_tag_id ) ) WITHOUT ROWID;"),
        &[],
    )?;
    db.execute(
        &format!("CREATE INDEX {deleted_parents}_account_id_index ON {deleted_parents} ( account_id );"),
        &[],
    )?;
    db.execute(
        &format!("CREATE INDEX {deleted_parents}_parent_timestamp_index ON {deleted_parents} ( parent_timestamp );"),
        &[],
    )?;

    db.execute(
        &format!("CREATE TABLE {pending_parents} ( child_master_tag_id INTEGER, parent_master_tag_id INTEGER, account_id INTEGER, reason_id INTEGER, PRIMARY KEY ( child_master_tag_id, parent_master_tag_id, account_id ) ) WITHOUT ROWID;"),
        &[],
    )?;
    db.execute(
        &format!("CREATE INDEX {pending_parents}_account_id_reason_id_index ON {pending_parents} ( account_id, reason_id );"),
        &[],
    )?;

    db.execute(
        &format!("CREATE TABLE {petitioned_parents} ( child_service_tag_id INTEGER, parent_service_tag_id INTEGER, account_id INTEGER, reason_id INTEGER, PRIMARY KEY ( child_service_tag_id, parent_service_tag_id, account_id ) ) WITHOUT ROWID;"),
        &[],
    )?;
    db.execute(
        &format!("CREATE INDEX {petitioned_parents}_account_id_reason_id_index ON {petitioned_parents} ( account_id, reason_id );"),
        &[],
    )?;

    Ok(())
}
