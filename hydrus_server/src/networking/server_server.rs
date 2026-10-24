//! Rust port of `hydrus/server/networking/ServerServer.py`.
//!
//! Twisted owns the actual `Resource` classes and the reactor. This module
//! ports the service hierarchy instead: which paths each service type exposes,
//! which resource handles them, and whether the path is local-only or remote.
//! The HTTP layer can turn a [`ServerRoute`] into a concrete handler.

use hydrus_core::hydrus_constants::ServiceType;
use hydrus_core::hydrus_exceptions::{ErrorKind, HydrusException, HydrusResult};
use hydrus_core::networking::hydrus_server_resources::HydrusDomain;
use hydrus_core::networking::hydrus_server::{LOCAL_DOMAIN, REMOTE_DOMAIN};

/// One concrete resource class from `ServerServerResources.py`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ServerResourceKind {
    BusyCheck,
    AccessKey,
    AccessKeyVerification,
    AutoCreateAccountTypes,
    RestrictedAutoCreateRegistrationKey,
    SessionKey,
    Shutdown,

    RestrictedAccount,
    RestrictedOptions,
    RestrictedOptionsModifyNullificationPeriod,
    RestrictedOptionsModifyUpdatePeriod,
    RestrictedAllAccounts,
    RestrictedAccountOtherAccount,
    RestrictedAccountModifyAccountType,
    RestrictedAccountModifyBan,
    RestrictedAccountModifyDeleteAllContent,
    RestrictedAccountModifyExpires,
    RestrictedAccountModifySetMessage,
    RestrictedAccountModifyUnban,
    RestrictedAccountInfo,
    RestrictedAccountKeyFromContent,
    RestrictedAccountTypes,
    RestrictedRegistrationKeys,
    RestrictedServiceInfo,
    RestrictedMaintenanceRegenServiceInfo,

    RestrictedBackup,
    RestrictedLockOn,
    RestrictedLockOff,
    RestrictedRestartServices,
    RestrictedServices,
    RestrictedVacuum,

    RestrictedNumPetitions,
    RestrictedPetition,
    RestrictedPetitionsSummary,
    RestrictedUpdate,
    RestrictedMetadataUpdate,

    RestrictedRepositoryFile,
    RestrictedIp,
    RestrictedRepositoryThumbnail,
    RestrictedTagFilter,
}

/// A child entry equivalent to one `root.putChild( path, resource )`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ServerRoute {
    pub kind: ServerResourceKind,
    pub domain: HydrusDomain,
}

impl ServerRoute {
    fn remote(kind: ServerResourceKind) -> Self {
        Self {
            kind,
            domain: REMOTE_DOMAIN,
        }
    }

    fn local(kind: ServerResourceKind) -> Self {
        Self {
            kind,
            domain: LOCAL_DOMAIN,
        }
    }
}

/// The service's root child map.
///
/// Insertion order is preserved so a future HTTP layer can expose a stable
/// route listing, just as Twisted's `putChild` calls define it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ServerResourceTree {
    children: Vec<(&'static str, ServerRoute)>,
}

impl ServerResourceTree {
    pub fn new() -> Self {
        Self::default()
    }

    /// `root.putChild`
    pub fn put_child(&mut self, path: &'static str, route: ServerRoute) {
        if let Some((_, existing)) = self
            .children
            .iter_mut()
            .find(|(existing_path, _)| *existing_path == path)
        {
            *existing = route;
        } else {
            self.children.push((path, route));
        }
    }

    #[allow(dead_code)]
    pub fn get_child(&self, path: &str) -> Option<&ServerRoute> {
        self.children
            .iter()
            .find(|(existing_path, _)| *existing_path == path)
            .map(|(_, route)| route)
    }

    #[allow(dead_code)]
    pub fn children(&self) -> impl Iterator<Item = (&'static str, &ServerRoute)> {
        self.children
            .iter()
            .map(|(path, route)| (*path, route))
    }

    pub fn len(&self) -> usize {
        self.children.len()
    }

    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.children.is_empty()
    }

    /// `HydrusServiceRestricted._init_root`.
    pub fn restricted() -> Self {
        let mut tree = Self::new();

        let mut add = |path, kind| tree.put_child(path, ServerRoute::remote(kind));

        add("busy", ServerResourceKind::BusyCheck);
        add("access_key", ServerResourceKind::AccessKey);
        add(
            "access_key_verification",
            ServerResourceKind::AccessKeyVerification,
        );
        add(
            "auto_create_account_types",
            ServerResourceKind::AutoCreateAccountTypes,
        );
        add(
            "auto_create_registration_key",
            ServerResourceKind::RestrictedAutoCreateRegistrationKey,
        );
        add("session_key", ServerResourceKind::SessionKey);

        add("account", ServerResourceKind::RestrictedAccount);
        add("options", ServerResourceKind::RestrictedOptions);

        add("all_accounts", ServerResourceKind::RestrictedAllAccounts);
        add(
            "other_account",
            ServerResourceKind::RestrictedAccountOtherAccount,
        );

        add(
            "modify_account_account_type",
            ServerResourceKind::RestrictedAccountModifyAccountType,
        );
        add(
            "modify_account_ban",
            ServerResourceKind::RestrictedAccountModifyBan,
        );
        add(
            "modify_account_delete_all_content",
            ServerResourceKind::RestrictedAccountModifyDeleteAllContent,
        );
        add(
            "modify_account_expires",
            ServerResourceKind::RestrictedAccountModifyExpires,
        );
        add(
            "modify_account_set_message",
            ServerResourceKind::RestrictedAccountModifySetMessage,
        );
        add(
            "modify_account_unban",
            ServerResourceKind::RestrictedAccountModifyUnban,
        );

        add("account_info", ServerResourceKind::RestrictedAccountInfo);
        add(
            "account_key_from_content",
            ServerResourceKind::RestrictedAccountKeyFromContent,
        );
        add("account_types", ServerResourceKind::RestrictedAccountTypes);

        add(
            "options_nullification_period",
            ServerResourceKind::RestrictedOptionsModifyNullificationPeriod,
        );
        add(
            "options_update_period",
            ServerResourceKind::RestrictedOptionsModifyUpdatePeriod,
        );

        add(
            "registration_keys",
            ServerResourceKind::RestrictedRegistrationKeys,
        );

        add("service_info", ServerResourceKind::RestrictedServiceInfo);
        add(
            "maintenance_regen_service_info",
            ServerResourceKind::RestrictedMaintenanceRegenServiceInfo,
        );

        tree
    }

    /// `HydrusServiceAdmin._init_root`.
    pub fn admin() -> Self {
        let mut tree = Self::restricted();

        tree.put_child(
            "backup",
            ServerRoute::remote(ServerResourceKind::RestrictedBackup),
        );
        tree.put_child(
            "lock_on",
            ServerRoute::remote(ServerResourceKind::RestrictedLockOn),
        );
        tree.put_child(
            "lock_off",
            ServerRoute::remote(ServerResourceKind::RestrictedLockOff),
        );
        tree.put_child(
            "restart_services",
            ServerRoute::remote(ServerResourceKind::RestrictedRestartServices),
        );
        tree.put_child(
            "services",
            ServerRoute::remote(ServerResourceKind::RestrictedServices),
        );
        tree.put_child("shutdown", ServerRoute::local(ServerResourceKind::Shutdown));
        tree.put_child(
            "vacuum",
            ServerRoute::remote(ServerResourceKind::RestrictedVacuum),
        );

        tree
    }

    /// `HydrusServiceRepository._init_root`.
    pub fn repository() -> Self {
        let mut tree = Self::restricted();

        tree.put_child(
            "num_petitions",
            ServerRoute::remote(ServerResourceKind::RestrictedNumPetitions),
        );
        tree.put_child(
            "petition",
            ServerRoute::remote(ServerResourceKind::RestrictedPetition),
        );
        tree.put_child(
            "petitions_summary",
            ServerRoute::remote(ServerResourceKind::RestrictedPetitionsSummary),
        );
        tree.put_child(
            "update",
            ServerRoute::remote(ServerResourceKind::RestrictedUpdate),
        );
        tree.put_child(
            "metadata",
            ServerRoute::remote(ServerResourceKind::RestrictedMetadataUpdate),
        );

        tree
    }

    /// `HydrusServiceRepositoryFile._init_root`.
    pub fn repository_file() -> Self {
        let mut tree = Self::repository();

        tree.put_child(
            "file",
            ServerRoute::remote(ServerResourceKind::RestrictedRepositoryFile),
        );
        tree.put_child(
            "ip",
            ServerRoute::remote(ServerResourceKind::RestrictedIp),
        );
        tree.put_child(
            "thumbnail",
            ServerRoute::remote(ServerResourceKind::RestrictedRepositoryThumbnail),
        );

        tree
    }

    /// `HydrusServiceRepositoryTag._init_root`.
    pub fn repository_tag() -> Self {
        let mut tree = Self::repository();

        tree.put_child(
            "tag_filter",
            ServerRoute::remote(ServerResourceKind::RestrictedTagFilter),
        );

        tree
    }

    /// Select the Python subclass for a server service type.
    pub fn for_service_type(service_type: ServiceType) -> HydrusResult<Self> {
        match service_type {
            ServiceType::ServerAdmin => Ok(Self::admin()),
            ServiceType::FileRepository => Ok(Self::repository_file()),
            ServiceType::TagRepository => Ok(Self::repository_tag()),
            _ => Err(HydrusException::new(
                ErrorKind::Unknown,
                format!(
                    "The service type \"{}\" is not a hydrus server service.",
                    service_type.long_str()
                ),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
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
}
