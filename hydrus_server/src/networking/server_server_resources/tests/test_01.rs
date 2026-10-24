use super::*;
use super::support::*;
use hydrus_core::networking::hydrus_server::LOCAL_DOMAIN;

    #[test]
    fn busy_check_reports_the_runtime_flag() {
        let fixture = Fixture::new();
        let mut request = fixture.request("GET");

        let response = fixture
            .render(ServerResourceKind::BusyCheck, &mut request)
            .unwrap();

        assert_eq!(response.get_status_code(), 200);
        assert_eq!(response.get_body_bytes(), Some(&b"0"[..]));
    }

    #[test]
    fn access_key_endpoint_issues_a_key_for_the_init_token() {
        let fixture = Fixture::new();
        let mut request = fixture.request("GET");

        request
            .parsed_request_args
            .insert("registration_key", ArgValue::from(b"init".as_slice()));

        let response = fixture
            .render(ServerResourceKind::AccessKey, &mut request)
            .unwrap();

        let args = body_args(&response);
        let access_key = args.get_bytes("access_key").unwrap();

        assert_eq!(access_key.len(), 32);

        let mut verification = fixture.request("GET");
        verification
            .request_headers
            .set("Hydrus-Key", hex_encode(&access_key));

        let response = fixture
            .render(
                ServerResourceKind::AccessKeyVerification,
                &mut verification,
            )
            .unwrap();

        assert!(body_args(&response).get_bool("verified").unwrap());
    }

    #[test]
    fn session_key_endpoint_sets_a_cookie() {
        let fixture = Fixture::new();
        let access_key = fixture.db.get_access_key(&fixture.admin_key, b"init").unwrap();

        let mut request = fixture.request("GET");
        request
            .request_headers
            .set("Hydrus-Key", hex_encode(&access_key));

        let response = fixture
            .render(ServerResourceKind::SessionKey, &mut request)
            .unwrap();

        assert_eq!(response.get_status_code(), 200);
        assert_eq!(response.get_cookies().len(), 1);
        assert_eq!(response.get_cookies()[0].name, "session_key");
    }

    #[test]
    fn account_endpoint_returns_the_established_account() {
        let fixture = Fixture::new();
        let access_key = fixture.db.get_access_key(&fixture.admin_key, b"init").unwrap();

        let mut request = fixture.request("GET");
        request
            .request_headers
            .set("Hydrus-Key", hex_encode(&access_key));

        let response = fixture
            .render(ServerResourceKind::RestrictedAccount, &mut request)
            .unwrap();

        assert_eq!(response.get_status_code(), 200);
        assert!(response.get_body_bytes().is_some());
        assert!(request.hydrus_account.is_some());
    }

    #[test]
    fn account_types_endpoint_returns_both_default_types() {
        let fixture = Fixture::new();
        let access_key = fixture.db.get_access_key(&fixture.admin_key, b"init").unwrap();

        let mut request = fixture.request("GET");
        request
            .request_headers
            .set("Hydrus-Key", hex_encode(&access_key));

        let response = fixture
            .render(ServerResourceKind::RestrictedAccountTypes, &mut request)
            .unwrap();

        let args = body_args(&response);

        assert_eq!(args.get_list("account_types").unwrap().len(), 2);
    }

    #[test]
    fn shutdown_endpoint_sets_the_shutdown_flag() {
        let fixture = Fixture::new();
        let mut request = fixture.request("POST");

        let response = render_resource(
            ServerResourceKind::Shutdown,
            ServerRoute {
                kind: ServerResourceKind::Shutdown,
                domain: LOCAL_DOMAIN,
            },
            &fixture.runtime,
            &mut request,
        )
        .unwrap();

        assert_eq!(response.get_status_code(), 200);
        assert!(fixture.runtime.shutdown_requested.load(Ordering::Relaxed));
    }

    #[test]
    fn update_resources_are_explicitly_not_ported_yet() {
        let fixture = Fixture::new();
        let access_key = fixture.db.get_access_key(&fixture.admin_key, b"init").unwrap();

        let mut request = fixture.request("GET");
        request
            .request_headers
            .set("Hydrus-Key", hex_encode(&access_key));

        let response = fixture
            .render(ServerResourceKind::RestrictedUpdate, &mut request)
            .unwrap();

        assert_eq!(response.get_status_code(), 501);
    }
