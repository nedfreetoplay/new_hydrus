/// Start listeners for every hydrus service in the database.
pub async fn serve(controller: Arc<Controller>) -> HydrusResult<()> {
    let db = controller.open_database()?;
    let (cert_path, key_path) = db.db().get_ssl_paths()?;
    let tls_settings = generate_ssl_context_settings(&cert_path, &key_path)?;
    let acceptor = Arc::new(build_tls_acceptor(&tls_settings)?);

    let services = db.get_services(None)?;

    let mut handles = Vec::new();

    for service in services {
        if !matches!(
            service.get_service_type(),
            ServiceType::ServerAdmin | ServiceType::FileRepository | ServiceType::TagRepository
        ) {
            continue;
        }

        let port = service.get_port();
        let runtime = Arc::new(controller.resource_runtime(service.get_service_key())?);
        let context = Arc::new(ServiceContext { runtime });
        let listeners = bind_listeners(port).await?;

        for listener in listeners {
            let context = Arc::clone(&context);
            let acceptor = Arc::clone(&acceptor);

            handles.push(tokio::spawn(async move {
                serve_listener(listener, context, acceptor).await
            }));
        }
    }

    if handles.is_empty() {
        return Err(HydrusException::new(
            ErrorKind::Unknown,
            "There are no server services to listen on.",
        ));
    }

    let shutdown = controller.shutdown_requested();

    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                break;
            }
            _ = tokio::time::sleep(Duration::from_millis(200)) => {
                if shutdown.load(Ordering::Relaxed) {
                    break;
                }
            }
        }
    }

    for handle in handles {
        handle.abort();
    }

    Ok(())
}

async fn bind_listeners(port: u16) -> HydrusResult<Vec<TcpListener>> {
    let mut listeners = Vec::new();
    let mut errors = Vec::new();

    match bind_ipv6(port).await {
        Ok(listener) => listeners.push(listener),
        Err(error) => errors.push(format!("IPv6: {error}")),
    }

    match bind_ipv4(port).await {
        Ok(listener) => listeners.push(listener),
        Err(error) => errors.push(format!("IPv4: {error}")),
    }

    if listeners.is_empty() {
        return Err(HydrusException::new(
            ErrorKind::Network,
            format!(
                "Could not bind server service port {port} on either IPv4 or IPv6 ({})",
                errors.join("; ")
            ),
        ));
    }

    Ok(listeners)
}
