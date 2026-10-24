async fn bind_ipv4(port: u16) -> HydrusResult<TcpListener> {
    let socket = TcpSocket::new_v4().map_err(|error| {
        HydrusException::new(
            ErrorKind::Network,
            format!("Could not create an IPv4 socket: {error}"),
        )
    })?;

    socket.bind(SocketAddr::from((Ipv4Addr::UNSPECIFIED, port))).map_err(|error| {
        HydrusException::new(
            ErrorKind::Network,
            format!("Could not bind the IPv4 listener: {error}"),
        )
    })?;

    socket.listen(1024).map_err(|error| {
        HydrusException::new(
            ErrorKind::Network,
            format!("Could not listen on the IPv4 listener: {error}"),
        )
    })
}

async fn bind_ipv6(port: u16) -> HydrusResult<TcpListener> {
    let socket = socket2::Socket::new(
        socket2::Domain::IPV6,
        socket2::Type::STREAM,
        Some(socket2::Protocol::TCP),
    )
    .map_err(|error| {
        HydrusException::new(
            ErrorKind::Network,
            format!("Could not create an IPv6 socket: {error}"),
        )
    })?;

    // Keep IPv4 and IPv6 as separate listeners so a dual-stack bind cannot
    // conflict with the IPv4 socket on Windows.
    socket.set_only_v6(true).map_err(|error| {
        HydrusException::new(
            ErrorKind::Network,
            format!("Could not make the IPv6 socket v6-only: {error}"),
        )
    })?;

    socket
        .bind(&SocketAddr::from((Ipv6Addr::UNSPECIFIED, port)).into())
        .map_err(|error| {
            HydrusException::new(
                ErrorKind::Network,
                format!("Could not bind the IPv6 listener: {error}"),
            )
        })?;

    socket.listen(1024).map_err(|error| {
        HydrusException::new(
            ErrorKind::Network,
            format!("Could not listen on the IPv6 listener: {error}"),
        )
    })?;

    socket.set_nonblocking(true).map_err(|error| {
        HydrusException::new(
            ErrorKind::Network,
            format!("Could not make the IPv6 listener non-blocking: {error}"),
        )
    })?;

    let std_listener: std::net::TcpListener = socket.into();

    TcpListener::from_std(std_listener).map_err(|error| {
        HydrusException::new(
            ErrorKind::Network,
            format!("Could not register the IPv6 listener with tokio: {error}"),
        )
    })
}
