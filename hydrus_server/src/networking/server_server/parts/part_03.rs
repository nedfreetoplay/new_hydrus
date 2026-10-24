impl ServerResourceTree {
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
}
