
use super::*;

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
    assert_eq!(controller.services().get_num_services(), 3);
    assert_eq!(controller.files_manager().locations().len(), 2);
    assert_eq!(controller.shortcuts().len(), 2);

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
