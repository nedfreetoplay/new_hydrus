impl ServerResourceTree {
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
