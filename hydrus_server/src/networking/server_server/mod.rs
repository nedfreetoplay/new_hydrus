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

include!("parts/part_01.rs");

include!("parts/part_02.rs");

include!("parts/part_03.rs");

include!("parts/part_04.rs");

#[cfg(test)]
mod tests;
