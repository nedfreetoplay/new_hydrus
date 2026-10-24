//! The runnable hydrus server.
//!
//! This binary is the counterpart of `hydrus_python/hydrus/server`. The port
//! starts with the controller/filesystem foundation and the service route map;
//! database and HTTP layers are added on top of these modules.

mod networking;
mod server_controller;
mod server_db;
mod server_files;
mod server_globals;

use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;

use hydrus_core::hydrus_constants::ServiceType;
use hydrus_core::hydrus_exceptions::{ErrorKind, HydrusException, HydrusResult};
use hydrus_core::networking::hydrus_server_request::HydrusRequest;

use networking::server_server::ServerResourceTree;
use server_controller::{Controller, StartingAction, process_starting_action};

#[tokio::main]
async fn main() -> ExitCode {
    hydrus_core::initialise();

    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();

    match run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{}", hydrus_core::hydrus_data::print_exception(&error, false));

            ExitCode::FAILURE
        }
    }
}

async fn run() -> HydrusResult<()> {
    let (db_dir, action) = parse_arguments();

    hydrus_core::hydrus_paths::make_sure_directory_exists(&db_dir)?;

    match process_starting_action(&db_dir, &action)? {
        StartingAction::Exit => return Ok(()),
        StartingAction::Stop => {
            server_controller::shutdown_sibling_instance(&db_dir)?;

            return Ok(());
        }
        StartingAction::Restart => {
            server_controller::shutdown_sibling_instance(&db_dir)?;
        }
        StartingAction::Start => {}
    }

    let controller = Arc::new(Controller::new(db_dir));

    controller.install();
    controller.prepare_files_dir()?;

    let db = controller.open_database()?;
    let services = db.get_services(None)?;

    controller.open_session_manager()?;

    let files = controller.files();

    let admin_routes = ServerResourceTree::for_service_type(ServiceType::ServerAdmin)?;
    let admin_key = services
        .iter()
        .find(|service| service.get_service_type() == ServiceType::ServerAdmin)
        .map(|service| service.get_service_key().to_vec())
        .ok_or_else(|| {
            HydrusException::new(
                ErrorKind::DataMissing,
                "No administration service was found.",
            )
        })?;

    let mut busy_request = HydrusRequest::new("GET", "/busy").with_client_ip("127.0.0.1");
    let busy_response = controller.render_service_request(&admin_key, "busy", &mut busy_request)?;

    println!(
        "hydrus_server: database directory at \"{}\"",
        controller.db_dir().display()
    );
    println!(
        "hydrus_server: file storage at \"{}\"",
        files.files_dir().display()
    );
    println!(
        "hydrus_server: {} service(s), first start: {}",
        services.len(),
        db.db().is_first_start()
    );
    println!(
        "hydrus_server: administration service has {} routes",
        admin_routes.len()
    );
    println!(
        "hydrus_server: /busy -> {}",
        String::from_utf8_lossy(busy_response.get_body_bytes().unwrap_or_default())
    );

    controller.run().await
}

fn parse_arguments() -> (PathBuf, String) {
    let mut db_dir = None;
    let mut action = "start".to_string();
    let mut arguments = std::env::args().skip(1);

    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--db-dir" => {
                if let Some(path) = arguments.next() {
                    db_dir = Some(PathBuf::from(path));
                }
            }
            "--start" => action = "start".to_string(),
            "--stop" => action = "stop".to_string(),
            "--restart" => action = "restart".to_string(),
            "--exit" => action = "exit".to_string(),
            value if value.starts_with("--db-dir=") => {
                db_dir = Some(PathBuf::from(&value["--db-dir=".len()..]));
            }
            value if !value.starts_with('-') => {
                db_dir = Some(PathBuf::from(value));
            }
            _ => {}
        }
    }

    let db_dir = db_dir
        .or_else(|| std::env::var_os("HYDRUS_SERVER_DB_DIR").map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from("server_db"));

    (db_dir, action)
}
