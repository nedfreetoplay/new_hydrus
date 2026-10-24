//! A port of `hydrus.core.networking.HydrusServer`.
//!
//! Python subclasses Twisted's `Site` and `HTTPChannel`. The Rust port keeps
//! the parts that are Hydrus policy rather than Twisted plumbing:
//!
//! * the `FatHTTPChannel` size limits,
//! * the local/remote domain constants,
//! * the server version header,
//! * the root resource tree (`/`, `favicon.ico`, `robots.txt`),
//! * whether requests are logged.
//!
//! An HTTP integration layer turns [`ServiceRoot`] entries into responses with
//! [`HydrusService::render_resource`] and installs [`FatHttpChannel`] limits on
//! its own channel implementation.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use indexmap::IndexMap;

use crate::hydrus_constants::{
    CLIENT_API_VERSION, Mime, NETWORK_VERSION, SOFTWARE_VERSION, ServiceType,
};
use crate::hydrus_static_dir::get_static_path;
use crate::networking::hydrus_server_resources::{
    HydrusDomain, HydrusResourceService, ResponseContext, robots_txt_response, welcome_response,
};

/// `FatHTTPChannel.MAX_LENGTH`
pub const FAT_HTTP_CHANNEL_MAX_LENGTH: usize = 2 * 1048576;

/// `FatHTTPChannel.totalHeadersSize`
pub const FAT_HTTP_CHANNEL_TOTAL_HEADERS_SIZE: usize = 2 * 1048576;

/// `LOCAL_DOMAIN`
pub const LOCAL_DOMAIN: HydrusDomain = HydrusDomain::new(true);

/// `REMOTE_DOMAIN`
pub const REMOTE_DOMAIN: HydrusDomain = HydrusDomain::new(false);

/// `FatHTTPChannel`, reduced to the two limits the rest of the code reads.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FatHttpChannel;

impl FatHttpChannel {
    pub const MAX_LENGTH: usize = FAT_HTTP_CHANNEL_MAX_LENGTH;
    pub const TOTAL_HEADERS_SIZE: usize = FAT_HTTP_CHANNEL_TOTAL_HEADERS_SIZE;
}

/// One child of the root resource.
///
/// Twisted stores `Resource` objects here. These variants cover the resources
/// the core server creates; integrations can add their own files.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ServiceResource {
    Welcome,
    RobotsTxt,
    Favicon,
    File(PathBuf),
}

/// The root resource's child map.
///
/// Python uses byte paths (`b''`, `b'robots.txt'`) because Twisted's API is
/// bytes-based. Rust strings keep the same final path segments.
#[derive(Clone, Debug, Default)]
pub struct ServiceRoot {
    children: IndexMap<String, ServiceResource>,
}

impl ServiceRoot {
    pub fn new() -> Self {
        Self::default()
    }

    /// `root.putChild`
    pub fn put_child(&mut self, path: impl Into<String>, resource: ServiceResource) {
        self.children.insert(path.into(), resource);
    }

    pub fn get_child(&self, path: &str) -> Option<&ServiceResource> {
        self.children.get(path)
    }

    pub fn remove_child(&mut self, path: &str) -> Option<ServiceResource> {
        self.children.shift_remove(path)
    }

    pub fn children(&self) -> impl Iterator<Item = (&str, &ServiceResource)> {
        self.children
            .iter()
            .map(|(path, resource)| (path.as_str(), resource))
    }

    pub fn len(&self) -> usize {
        self.children.len()
    }

    pub fn is_empty(&self) -> bool {
        self.children.is_empty()
    }
}

/// Which request class Twisted would use.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RequestFactory {
    Plain,
    Logging,
}

/// A Python `HydrusService`.
pub struct HydrusService {
    service: Arc<dyn HydrusResourceService>,
    hydrus_favicon: PathBuf,
    server_version_string: String,
    root: ServiceRoot,
    logs_requests: bool,
}

impl HydrusService {
    /// `HydrusService.__init__`, without the Twisted `Site` constructor.
    pub fn new(service: Arc<dyn HydrusResourceService>) -> Self {
        let hydrus_favicon = get_static_path("hydrus.ico", false);

        let service_type = service.get_service_type();

        let server_version_string = if service_type == ServiceType::ClientApiService {
            format!(
                "{}/{CLIENT_API_VERSION} ({SOFTWARE_VERSION})",
                service_type.long_str()
            )
        } else {
            format!("{}/{NETWORK_VERSION}", service_type.long_str())
        };

        let logs_requests = service.logs_requests();

        let mut root = ServiceRoot::new();

        root.put_child("", ServiceResource::Welcome);
        root.put_child("favicon.ico", ServiceResource::Favicon);
        root.put_child("robots.txt", ServiceResource::RobotsTxt);

        Self {
            service,
            hydrus_favicon,
            server_version_string,
            root,
            logs_requests,
        }
    }

    pub fn get_service(&self) -> &Arc<dyn HydrusResourceService> {
        &self.service
    }

    pub fn get_hydrus_favicon_path(&self) -> &Path {
        &self.hydrus_favicon
    }

    /// The value written to both `Server` and `Hydrus-Server`.
    pub fn get_server_version_string(&self) -> &str {
        &self.server_version_string
    }

    pub fn get_root(&self) -> &ServiceRoot {
        &self.root
    }

    /// Subclasses in the Python code add repository/client-API resources here.
    pub fn get_root_mut(&mut self) -> &mut ServiceRoot {
        &mut self.root
    }

    pub fn logs_requests(&self) -> bool {
        self.logs_requests
    }

    /// `requestFactory`
    pub fn request_factory(&self) -> RequestFactory {
        if self.logs_requests {
            RequestFactory::Logging
        } else {
            RequestFactory::Plain
        }
    }

    /// `_protocol_factory`
    pub fn protocol_factory(&self) -> FatHttpChannel {
        FatHttpChannel
    }

    /// The headers set by `getResourceFor`.
    pub fn server_headers(&self) -> [(&'static str, &str); 2] {
        [
            ("Server", self.server_version_string.as_str()),
            ("Hydrus-Server", self.server_version_string.as_str()),
        ]
    }

    /// Look up a child of the root resource.
    pub fn get_resource_for(&self, path: &str) -> Option<&ServiceResource> {
        self.root.get_child(path)
    }

    /// Produce the response body for a core root resource.
    ///
    /// `None` means the path is not one of this server's core children. The
    /// HTTP layer can then try its own route table.
    pub fn render_resource(
        &self,
        path: &str,
    ) -> crate::hydrus_exceptions::HydrusResult<Option<ResponseContext>> {
        let Some(resource) = self.root.get_child(path) else {
            return Ok(None);
        };

        let response_context = match resource {
            ServiceResource::Welcome => welcome_response(self.service.as_ref()),
            ServiceResource::RobotsTxt => robots_txt_response(),
            ServiceResource::Favicon => {
                ResponseContext::from_path(200, Mime::Icon, self.hydrus_favicon.clone())?
            }
            ServiceResource::File(path) => {
                // Twisted's static File resource guesses the MIME type from the
                // extension. The integration layer can do the same; the core
                // fallback is a binary stream.
                ResponseContext::from_path(200, Mime::OctetStream, path.clone())?
            }
        };

        Ok(Some(response_context))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::hydrus_constants::ServiceType;

    struct TestService {
        name: &'static str,
        service_type: ServiceType,
        logs_requests: bool,
    }

    impl TestService {
        fn new(name: &'static str, service_type: ServiceType) -> Self {
            Self {
                name,
                service_type,
                logs_requests: false,
            }
        }
    }

    impl HydrusResourceService for TestService {
        fn get_name(&self) -> String {
            self.name.to_string()
        }

        fn get_service_type(&self) -> ServiceType {
            self.service_type
        }

        fn allows_non_local_connections(&self) -> bool {
            false
        }

        fn logs_requests(&self) -> bool {
            self.logs_requests
        }
    }

    #[test]
    fn fat_http_channel_limits_match_python() {
        assert_eq!(FatHttpChannel::MAX_LENGTH, 2 * 1048576);
        assert_eq!(FatHttpChannel::TOTAL_HEADERS_SIZE, 2 * 1048576);
        assert_eq!(FAT_HTTP_CHANNEL_MAX_LENGTH, 2 * 1048576);
        assert_eq!(FAT_HTTP_CHANNEL_TOTAL_HEADERS_SIZE, 2 * 1048576);
    }

    #[test]
    fn server_version_strings_match_the_service_type() {
        let api = HydrusService::new(Arc::new(TestService::new(
            "my api",
            ServiceType::ClientApiService,
        )));

        assert_eq!(
            api.get_server_version_string(),
            format!("client api/{CLIENT_API_VERSION} ({SOFTWARE_VERSION})")
        );

        let repository = HydrusService::new(Arc::new(TestService::new(
            "repo",
            ServiceType::FileRepository,
        )));

        assert_eq!(
            repository.get_server_version_string(),
            format!("hydrus file repository/{NETWORK_VERSION}")
        );
    }

    #[test]
    fn root_has_the_three_core_children() {
        let service = HydrusService::new(Arc::new(TestService::new(
            "my api",
            ServiceType::ClientApiService,
        )));
        let root = service.get_root();

        assert_eq!(root.len(), 3);
        assert_eq!(root.get_child(""), Some(&ServiceResource::Welcome));
        assert_eq!(
            root.get_child("favicon.ico"),
            Some(&ServiceResource::Favicon)
        );
        assert_eq!(
            root.get_child("robots.txt"),
            Some(&ServiceResource::RobotsTxt)
        );

        let paths: Vec<&str> = root.children().map(|(path, _)| path).collect();

        assert_eq!(paths, vec!["", "favicon.ico", "robots.txt"]);
    }

    #[test]
    fn custom_children_can_be_added() {
        let mut service = HydrusService::new(Arc::new(TestService::new(
            "repo",
            ServiceType::FileRepository,
        )));

        service
            .get_root_mut()
            .put_child("file", ServiceResource::File(PathBuf::from("x.bin")));

        assert_eq!(
            service.get_resource_for("file"),
            Some(&ServiceResource::File(PathBuf::from("x.bin")))
        );
        assert!(service.get_resource_for("missing").is_none());
    }

    #[test]
    fn core_resources_render_without_twisted() {
        let service = HydrusService::new(Arc::new(TestService::new(
            "my api",
            ServiceType::ClientApiService,
        )));

        let welcome = service.render_resource("").unwrap().expect("welcome");
        assert_eq!(welcome.get_mime(), Mime::Html);
        assert!(
            std::str::from_utf8(welcome.get_body_bytes().unwrap())
                .unwrap()
                .contains("my api")
        );

        let robots = service
            .render_resource("robots.txt")
            .unwrap()
            .expect("robots");
        assert_eq!(robots.get_mime(), Mime::PlainText);
        assert_eq!(
            robots.get_body_bytes(),
            Some(crate::networking::hydrus_server_resources::ROBOTS_TXT.as_bytes())
        );

        assert!(service.render_resource("nope").unwrap().is_none());
    }

    #[test]
    fn headers_and_request_factory_follow_the_service() {
        let mut test_service = TestService::new("my api", ServiceType::ClientApiService);
        test_service.logs_requests = true;

        let service = HydrusService::new(Arc::new(test_service));

        assert_eq!(
            service.server_headers(),
            [
                ("Server", service.get_server_version_string()),
                ("Hydrus-Server", service.get_server_version_string())
            ]
        );
        assert_eq!(service.request_factory(), RequestFactory::Logging);
        assert!(service.logs_requests());

        let quiet = HydrusService::new(Arc::new(TestService::new(
            "repo",
            ServiceType::FileRepository,
        )));

        assert_eq!(quiet.request_factory(), RequestFactory::Plain);
    }
}
