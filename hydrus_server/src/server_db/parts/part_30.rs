fn create_repository_tables(db: &DBBase, service_id: i64) -> HydrusResult<()> {
    let (hash_map, tag_map) = generate_repository_master_map_table_names(service_id);

    db.execute(
        &format!("CREATE TABLE {hash_map} ( service_hash_id INTEGER PRIMARY KEY, master_hash_id INTEGER UNIQUE, hash_id_timestamp INTEGER );"),
        &[],
    )?;
    create_qualified_index(db, &hash_map, "hash_id_timestamp", "hash_id_timestamp")?;
    db.execute(
        &format!("CREATE TABLE {tag_map} ( service_tag_id INTEGER PRIMARY KEY, master_tag_id INTEGER UNIQUE, tag_id_timestamp INTEGER );"),
        &[],
    )?;
    create_qualified_index(db, &tag_map, "tag_id_timestamp", "tag_id_timestamp")?;

    create_repository_file_tables(db, service_id)?;
    create_repository_mapping_tables(db, service_id)?;
    create_repository_tag_parent_tables(db, service_id)?;
    create_repository_tag_sibling_tables(db, service_id)?;

    let update_table = generate_repository_update_table_name(service_id);

    db.execute(
        &format!("CREATE TABLE {update_table} ( master_hash_id INTEGER PRIMARY KEY );"),
        &[],
    )?;

    Ok(())
}
