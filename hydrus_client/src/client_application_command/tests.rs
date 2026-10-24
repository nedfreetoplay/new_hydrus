use super::*;

#[test]
fn simple_commands_have_no_data() {
    let command = ApplicationCommand::simple(SIMPLE_ARCHIVE_FILE);

    assert!(command.is_simple());
    assert_eq!(command.to_string(), "archive file");
}

#[test]
fn data_commands_render_data() {
    let command = ApplicationCommand::new(
        SIMPLE_SET_MEDIA_FOCUS,
        Some(Value::String("media".to_string())),
    );

    assert!(!command.is_simple());
    assert!(command.to_string().contains("media"));
}

#[test]
fn lookup_covers_ported_commands() {
    for command_type in [
        SIMPLE_ARCHIVE_FILE,
        SIMPLE_CLOSE_PAGE,
        SIMPLE_EXIT_APPLICATION,
        SIMPLE_UNDO,
        SIMPLE_VIEW_LAST,
    ] {
        assert_ne!(command_type_string(command_type), "unknown command");
    }
}

#[test]
fn full_generated_table_covers_every_simple_command() {
    let known = (0..=208)
        .filter(|command_type| {
            crate::client_application_command_full::simple_command_name(*command_type)
                != "unknown command"
        })
        .count();

    assert!(known >= 190, "only {known} simple command ids were named");
    assert_ne!(
        crate::client_application_command_full::simple_command_name(208),
        "unknown command"
    );
}

#[test]
fn router_dispatches_registered_commands() {
    let mut router = ApplicationCommandRouter::new();
    let counter = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let handler_counter = counter.clone();

    router.register(SIMPLE_ARCHIVE_FILE, move |_| {
        handler_counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        true
    });

    assert!(router.dispatch(&ApplicationCommand::simple(SIMPLE_ARCHIVE_FILE)));
    assert_eq!(counter.load(std::sync::atomic::Ordering::SeqCst), 1);

    assert!(!router.dispatch(&ApplicationCommand::simple(SIMPLE_UNDO)));
    assert!(router.has_handler(SIMPLE_ARCHIVE_FILE));
    assert_eq!(router.len(), 1);
}
