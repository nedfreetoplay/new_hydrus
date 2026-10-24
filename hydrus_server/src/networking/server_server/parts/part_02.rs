impl ServerResourceTree {
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
}
