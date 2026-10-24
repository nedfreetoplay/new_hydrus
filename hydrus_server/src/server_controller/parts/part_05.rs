/// The result of `process_starting_action`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StartingAction {
    Start,
    Restart,
    Stop,
    Exit,
}

#[allow(dead_code)] // Exposed for future CLI/diagnostic output.
impl StartingAction {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Start => "start",
            Self::Restart => "restart",
            Self::Stop => "stop",
            Self::Exit => "exit",
        }
    }
}

/// `process_starting_action( db_dir, action )`
pub fn process_starting_action(
    db_dir: &Path,
    action: &str,
) -> HydrusResult<StartingAction> {
    let already_running = hydrus_process::is_already_running(db_dir, "server");

    match action {
        "start" => {
            if !already_running {
                return Ok(StartingAction::Start);
            }

            println!(
                "The server is already running. Would you like to [s]top it, [r]estart it here, or e[x]it?"
            );

            let mut answer = String::new();

            if std::io::stdin().read_line(&mut answer).is_ok() {
                match answer.chars().next() {
                    Some('s') => return Ok(StartingAction::Stop),
                    Some('r') => return Ok(StartingAction::Restart),
                    _ => {}
                }
            }

            Ok(StartingAction::Exit)
        }
        "stop" => {
            if already_running {
                Ok(StartingAction::Stop)
            } else {
                Err(HydrusException::new(
                    ErrorKind::Shutdown,
                    "The server is not running, so it cannot be stopped!",
                ))
            }
        }
        "restart" => {
            if already_running {
                Ok(StartingAction::Restart)
            } else {
                println!(
                    "Did not find an already running instance of the server--changing boot command from 'restart' to 'start'."
                );

                Ok(StartingAction::Start)
            }
        }
        "exit" => Ok(StartingAction::Exit),
        _ => Err(HydrusException::new(
            ErrorKind::BadRequest,
            format!("Unknown server action: {action}"),
        )),
    }
}

/// `shutdown_sibling_instance( db_dir )`
pub fn shutdown_sibling_instance(db_dir: &Path) -> HydrusResult<()> {
    let Some(ports) = hydrus_process::get_sibling_process_ports(db_dir, "server")? else {
        return Err(HydrusException::new(
            ErrorKind::Shutdown,
            "Could not figure out the existing server's ports, so could not shut it down!",
        ));
    };

    let client = reqwest::blocking::Client::builder()
        .danger_accept_invalid_certs(true)
        .build()
        .map_err(|error| {
            HydrusException::new(
                ErrorKind::Network,
                format!("Could not create the shutdown client: {error}"),
            )
        })?;

    let mut port_found = false;

    for port in ports {
        let response = client
            .get(format!("https://127.0.0.1:{port}/"))
            .send()
            .map_err(|error| {
                HydrusException::new(
                    ErrorKind::Shutdown,
                    format!("Could not contact existing server's port {port}: {error}"),
                )
            })?;

        let server_name = response
            .headers()
            .get("Server")
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default();

        if !server_name.contains("server administration") {
            continue;
        }

        port_found = true;

        println!("Sending shut down instruction...");

        let response = client
            .post(format!("https://127.0.0.1:{port}/shutdown"))
            .send()
            .map_err(|error| {
                HydrusException::new(
                    ErrorKind::Shutdown,
                    format!("The existing server did not accept the shutdown: {error}"),
                )
            })?;

        if !response.status().is_success() {
            return Err(HydrusException::new(
                ErrorKind::Shutdown,
                "When told to shut down, the existing server gave an error!",
            ));
        }
    }

    if !port_found {
        return Err(HydrusException::new(
            ErrorKind::Shutdown,
            "Could not find the existing server's administration port!",
        ));
    }

    Ok(())
}
