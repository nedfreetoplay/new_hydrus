use std::path::PathBuf;
use std::sync::Arc;

use hydrus_client::client_boot;
use hydrus_client::controller::Controller;

fn main() {
    let arguments: Vec<String> = std::env::args().skip(1).collect();

    if arguments
        .iter()
        .any(|argument| argument == "--help" || argument == "-h")
    {
        println!("hydrus_client [--db_dir PATH] [--version]");
        return;
    }

    if arguments.iter().any(|argument| argument == "--version") {
        println!(
            "hydrus_client {} (Hydrus {})",
            env!("CARGO_PKG_VERSION"),
            hydrus_core::hydrus_constants::SOFTWARE_VERSION
        );
        return;
    }

    if let Err(error) = client_boot::boot() {
        eprintln!("Hydrus Client failed to start: {error}");
        return;
    }

    let db_dir = db_dir_from_arguments(&arguments)
        .unwrap_or_else(|| hydrus_core::hydrus_constants::dirs().default_db_dir.clone());

    let controller = match Controller::new(&db_dir) {
        Ok(controller) => Arc::new(controller),
        Err(error) => {
            eprintln!(
                "Could not open the client database at {}: {error}",
                db_dir.display()
            );
            return;
        }
    };

    if let Err(error) = controller.init_model() {
        eprintln!("Could not initialise the client model: {error}");
        return;
    }

    controller.install_global();

    hydrus_client::gui::run(controller.clone());

    if let Err(error) = controller.save_options() {
        eprintln!("Could not save client options: {error}");
    }
}

fn db_dir_from_arguments(arguments: &[String]) -> Option<PathBuf> {
    let position = arguments
        .iter()
        .position(|argument| argument == "--db_dir")?;

    arguments.get(position + 1).map(PathBuf::from)
}
