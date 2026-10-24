use super::*;
use super::support::*;

#[test]
fn adding_a_repository_creates_its_tables() {
    let dir = TestDir::new();
    let db = make_db(&dir);

    let service = generate_service(
        vec![0xAB; 32],
        ServiceType::FileRepository,
        "test file repository",
        DEFAULT_SERVICE_PORT,
        None,
    )
    .unwrap();

    let service_id = db.add_service(&service).unwrap();

    assert_eq!(
        db.get_service_type(service_id).unwrap(),
        ServiceType::FileRepository
    );

    for table in [
        "current_files_2",
        "deleted_files_2",
        "pending_files_2",
        "petitioned_files_2",
        "ip_addresses_2",
        "current_tag_parents_2",
        "current_tag_siblings_2",
        "updates_2",
    ] {
        assert!(db.table_exists(table).unwrap(), "missing table {table}");
    }

    assert!(REPOSITORIES.contains(&ServiceType::FileRepository));
}
