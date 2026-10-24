/// `generate_repository_master_map_table_names( service_id )`
pub fn generate_repository_master_map_table_names(service_id: i64) -> (String, String) {
    let suffix = service_id.to_string();

    (
        format!("external_master.repository_hash_id_map_{suffix}"),
        format!("external_master.repository_tag_id_map_{suffix}"),
    )
}

/// `generate_repository_files_table_names( service_id )`
pub fn generate_repository_files_table_names(
    service_id: i64,
) -> (String, String, String, String, String) {
    let suffix = service_id.to_string();

    (
        format!("current_files_{suffix}"),
        format!("deleted_files_{suffix}"),
        format!("pending_files_{suffix}"),
        format!("petitioned_files_{suffix}"),
        format!("ip_addresses_{suffix}"),
    )
}

/// `generate_repository_mappings_table_names( service_id )`
pub fn generate_repository_mappings_table_names(
    service_id: i64,
) -> (String, String, String, String) {
    let suffix = service_id.to_string();

    (
        format!("external_mappings.current_mappings_{suffix}"),
        format!("external_mappings.deleted_mappings_{suffix}"),
        format!("external_mappings.pending_mappings_{suffix}"),
        format!("external_mappings.petitioned_mappings_{suffix}"),
    )
}

/// `generate_repository_tag_parents_table_names( service_id )`
pub fn generate_repository_tag_parents_table_names(
    service_id: i64,
) -> (String, String, String, String) {
    let suffix = service_id.to_string();

    (
        format!("current_tag_parents_{suffix}"),
        format!("deleted_tag_parents_{suffix}"),
        format!("pending_tag_parents_{suffix}"),
        format!("petitioned_tag_parents_{suffix}"),
    )
}

/// `generate_repository_tag_siblings_table_names( service_id )`
pub fn generate_repository_tag_siblings_table_names(
    service_id: i64,
) -> (String, String, String, String) {
    let suffix = service_id.to_string();

    (
        format!("current_tag_siblings_{suffix}"),
        format!("deleted_tag_siblings_{suffix}"),
        format!("pending_tag_siblings_{suffix}"),
        format!("petitioned_tag_siblings_{suffix}"),
    )
}

/// `generate_repository_update_table_name( service_id )`
pub fn generate_repository_update_table_name(service_id: i64) -> String {
    format!("updates_{service_id}")
}

/// One row from `read( 'sessions' )`.
pub struct ServerSessionRow {
    pub session_key: Vec<u8>,
    pub service_key: Vec<u8>,
    pub account: Arc<NetworkAccount>,
    pub hashed_access_key: Vec<u8>,
    pub expires: i64,
}

/// A server database, backed by the core `HydrusDB` job/database machinery.
pub struct ServerDB {
    db: Arc<HydrusDB>,
    db_dir: PathBuf,
    files: ServerFiles,
}
