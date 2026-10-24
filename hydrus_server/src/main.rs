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
use hydrus_core::hydrus_exceptions::HydrusResult;

use networking::server_server::ServerResourceTree;
use server_controller::Controller;

fn main() -> ExitCode {
    hydrus_core::initialise();

    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{}", hydrus_core::hydrus_data::print_exception(&error, false));

            ExitCode::FAILURE
        }
    }
}

fn run() -> HydrusResult<()> {
    let db_dir = server_db_dir();

    hydrus_core::hydrus_paths::make_sure_directory_exists(&db_dir)?;

    let controller = Arc::new(Controller::new(db_dir));

    controller.install();
    controller.prepare_files_dir()?;

    let db = controller.open_database()?;
    let services = db.get_services(None)?;
    let files = controller.files();

    let admin_routes = ServerResourceTree::for_service_type(ServiceType::ServerAdmin)?;

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

    Ok(())
}

fn server_db_dir() -> PathBuf {
    if let Some(db_dir) = std::env::args().nth(1) {
        return PathBuf::from(db_dir);
    }

    if let Some(db_dir) = std::env::var_os("HYDRUS_SERVER_DB_DIR") {
        return PathBuf::from(db_dir);
    }

    PathBuf::from("server_db")
}
