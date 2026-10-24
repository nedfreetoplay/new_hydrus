
use super::*;

#[test]
fn default_and_hydrus_contexts_have_expected_flags() {
    assert!(NetworkContext::default_context().is_default());
    assert!(NetworkContext::hydrus().is_hydrus());
    assert!(!NetworkContext::for_domain("example.com").is_default());
}

#[test]
fn domain_contexts_render_the_domain() {
    let context = NetworkContext::for_domain("example.com");

    assert_eq!(context.to_string(), "domain example.com");
    assert_eq!(context.to_human_string(), "example.com");
}

#[test]
fn key_contexts_render_hex() {
    let context = NetworkContext::for_key(
        client_constants::NETWORK_CONTEXT_DOWNLOADER,
        vec![0xab, 0xcd],
    );

    assert_eq!(context.to_string(), "downloader abcd");
}

#[test]
fn serialisation_round_trips_domain_and_key() {
    for context in [
        NetworkContext::for_domain("example.com"),
        NetworkContext::for_key(client_constants::NETWORK_CONTEXT_DOWNLOADER, vec![1, 2]),
    ] {
        let info = context.get_serialisable_info();
        let mut loaded = NetworkContext::default_context();

        loaded.initialise_from_serialisable_info(&info).unwrap();

        assert_eq!(loaded, context);
    }
}

#[test]
fn page_contexts_are_ephemeral() {
    assert!(
        NetworkContext::for_key(client_constants::NETWORK_CONTEXT_DOWNLOADER_PAGE, vec![1])
            .is_ephemeral()
    );
    assert!(!NetworkContext::for_domain("example.com").is_ephemeral());
}
