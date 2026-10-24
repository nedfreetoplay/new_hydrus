fn main() {
    if let Err(error) = hydrus_client::client_boot::boot() {
        eprintln!("Hydrus Client failed to start: {error}");
        return;
    }

    hydrus_client::gui::run();
}
