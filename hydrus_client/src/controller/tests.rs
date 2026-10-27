use super::*;

use crate::db::services;
use crate::gui::pages::{Page, PageType, PagesNotebook};

fn temporary_controller() -> (Controller, PathBuf) {
    let directory = std::env::temp_dir().join(format!(
        "hydrus_client_controller_{}_{}",
        std::process::id(),
        rand::random::<u64>()
    ));

    let _ = crate::client_options::register_serialisable_type();

    let controller = Controller::new(&directory).unwrap();
    controller.init_model().unwrap();

    (controller, directory)
}

#[test]
fn init_model_creates_local_services_and_loads_options() {
    let (controller, directory) = temporary_controller();
    let services = services::get_services(controller.db().db()).unwrap();

    assert!(services.iter().any(|service| service.name == "my files"));
    assert!(services.iter().any(|service| service.name == "local tags"));
    assert!(!controller.options().get_boolean("advanced_mode"));
    assert_eq!(controller.status_line().contains("services"), true);
    assert_eq!(controller.session().pages.len(), 1);
    assert_eq!(controller.session().pages[0].name, "Welcome");
    assert_eq!(controller.services().get_num_services(), 5);
    assert_eq!(controller.files_manager().locations().len(), 2);
    assert_eq!(controller.shortcuts().len(), 2);

    drop(controller);
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn services_can_be_added_renamed_and_deleted() {
    let (controller, directory) = temporary_controller();

    let service_id = controller
        .add_service("test tags", ServiceType::LocalTag)
        .unwrap();
    let service = controller
        .services()
        .get_services(None)
        .into_iter()
        .find(|service| service.service_id == service_id)
        .cloned()
        .unwrap();
    let key: String = service
        .service_key
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();

    assert_eq!(service.name, "test tags");
    assert_eq!(controller.services().get_num_services(), 6);

    controller.rename_service(&key, "renamed tags").unwrap();
    assert!(
        controller
            .services()
            .names()
            .iter()
            .any(|name| *name == "renamed tags")
    );

    controller.delete_service(&key).unwrap();
    assert_eq!(controller.services().get_num_services(), 5);

    drop(controller);
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn network_settings_are_saved_and_reloaded() {
    let (controller, directory) = temporary_controller();

    controller
        .domain_manager()
        .write()
        .unwrap()
        .set_custom_header(
            "X-Test",
            CustomHeader {
                value: "yes".to_string(),
                approved: true,
                note: String::new(),
            },
        );

    controller.replace_bandwidth_rule_entries(vec![BandwidthRuleEntry {
        context_type: "domain".to_string(),
        context: "example.com".to_string(),
        bandwidth_type: "requests".to_string(),
        time_delta: Some(60),
        max_allowed: 4,
    }]);

    let mut pools = DownloadPoolSettings::new();
    pools.set_limit("example.com", 3);
    controller.replace_download_pool_settings(pools);

    controller.save_network_settings().unwrap();

    let reloaded = Controller::new(&directory).unwrap();
    reloaded.init_model().unwrap();

    let header = reloaded
        .domain_manager()
        .read()
        .unwrap()
        .get_custom_header("X-Test")
        .cloned()
        .unwrap();

    assert_eq!(header.value, "yes");
    assert_eq!(
        reloaded
            .download_pool_settings()
            .limit_for("example.com", 1),
        3
    );

    let rules = reloaded.bandwidth_rule_entries();

    assert!(rules.iter().any(|rule| {
        rule.context_type == "domain" && rule.context == "example.com" && rule.max_allowed == 4
    }));

    drop(reloaded);
    drop(controller);
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn options_are_saved_and_reloaded() {
    let (controller, directory) = temporary_controller();

    {
        let mut options = controller.options.write().unwrap();
        options.set_boolean("advanced_mode", true);
    }

    controller.save().unwrap();

    let reloaded = Controller::new(&directory).unwrap();
    reloaded.init_model().unwrap();

    assert!(reloaded.options().get_boolean("advanced_mode"));

    drop(reloaded);
    drop(controller);
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn session_is_saved_and_restored() {
    let (controller, directory) = temporary_controller();

    let mut notebook = PagesNotebook::new();
    notebook.add_page(Page::new("files", PageType::Import));
    controller.set_session(GuiSession::from_notebook(&notebook));
    controller.save().unwrap();

    let reloaded = Controller::new(&directory).unwrap();

    assert_eq!(reloaded.session().pages.len(), 1);
    assert_eq!(reloaded.session().pages[0].name, "files");

    drop(reloaded);
    drop(controller);
    let _ = std::fs::remove_dir_all(directory);
}
