async fn serve_listener(
    listener: TcpListener,
    context: Arc<ServiceContext>,
    acceptor: Arc<TlsAcceptor>,
) -> HydrusResult<()> {
    let local_address = listener.local_addr().map_err(|error| {
        HydrusException::new(
            ErrorKind::Network,
            format!("Could not read the listener address: {error}"),
        )
    })?;

    println!(
        "hydrus_server: listening for \"{}\" on https://{}",
        context.service_name(),
        local_address
    );

    loop {
        let (stream, peer_address) = match listener.accept().await {
            Ok(connection) => connection,
            Err(error) => {
                eprintln!(
                    "hydrus_server: accept failed on {}: {error}",
                    local_address
                );

                continue;
            }
        };

        let context = Arc::clone(&context);
        let acceptor = Arc::clone(&acceptor);

        tokio::spawn(async move {
            let tls_stream = match acceptor.accept(stream).await {
                Ok(stream) => stream,
                Err(error) => {
                    eprintln!(
                        "hydrus_server: TLS handshake from {} failed: {error}",
                        peer_address
                    );

                    return;
                }
            };

            let io = TokioIo::new(tls_stream);
            let client_ip = peer_address.ip().to_string();
            let service = service_fn(move |request| {
                let context = Arc::clone(&context);
                let client_ip = client_ip.clone();

                async move { handle_request(request, context, client_ip).await }
            });

            if let Err(error) = hyper::server::conn::http1::Builder::new()
                .serve_connection(io, service)
                .await
            {
                eprintln!("hydrus_server: HTTP connection failed: {error}");
            }
        });
    }
}
