use super::*;

#[test]
fn restricted_tree_matches_the_python_children() {
    let tree = ServerResourceTree::restricted();

    assert_eq!(tree.len(), 24);
    assert_eq!(
        tree.get_child("busy"),
        Some(&ServerRoute::remote(ServerResourceKind::BusyCheck))
    );
    assert_eq!(
        tree.get_child("access_key_verification"),
        Some(&ServerRoute::remote(ServerResourceKind::AccessKeyVerification))
    );
    assert_eq!(
        tree.get_child("maintenance_regen_service_info"),
        Some(&ServerRoute::remote(
            ServerResourceKind::RestrictedMaintenanceRegenServiceInfo
        ))
    );
    assert!(tree.get_child("file").is_none());
    assert!(tree.get_child("shutdown").is_none());
}

#[test]
fn admin_adds_local_shutdown_and_admin_routes() {
    let tree = ServerResourceTree::admin();

    assert_eq!(
        tree.get_child("shutdown"),
        Some(&ServerRoute::local(ServerResourceKind::Shutdown))
    );
    assert_eq!(
        tree.get_child("services"),
        Some(&ServerRoute::remote(ServerResourceKind::RestrictedServices))
    );
    assert!(tree.get_child("file").is_none());
}

#[test]
fn file_repository_extends_the_common_repository_routes() {
    let tree = ServerResourceTree::repository_file();

    assert_eq!(
        tree.get_child("update"),
        Some(&ServerRoute::remote(ServerResourceKind::RestrictedUpdate))
    );
    assert_eq!(
        tree.get_child("file"),
        Some(&ServerRoute::remote(
            ServerResourceKind::RestrictedRepositoryFile
        ))
    );
    assert!(tree.get_child("tag_filter").is_none());
}

#[test]
fn tag_repository_adds_only_the_tag_filter() {
    let tree = ServerResourceTree::repository_tag();

    assert_eq!(
        tree.get_child("tag_filter"),
        Some(&ServerRoute::remote(
            ServerResourceKind::RestrictedTagFilter
        ))
    );
    assert!(tree.get_child("file").is_none());
    assert!(tree.get_child("thumbnail").is_none());
}

#[test]
fn non_server_service_types_are_rejected() {
    let error = ServerResourceTree::for_service_type(ServiceType::ClientApiService)
        .unwrap_err();

    assert!(error.is_a(ErrorKind::Unknown));
    assert!(error.to_string().contains("not a hydrus server service"));
}
