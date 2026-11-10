use std::io::{BufWriter, Write};

#[cfg(unix)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use std::{ffi::OsString, os::unix::ffi::OsStrExt, path::PathBuf};
    use vellum_client::Client;
    use vellum_mcp::{BoundedLine, McpLimits, McpServer, read_bounded_line};
    use vellum_transport::{RemoteTransport, TransportLimits, UnixConnector};

    #[derive(Debug)]
    struct CliError(&'static str);
    impl std::fmt::Display for CliError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str(self.0)
        }
    }
    impl std::error::Error for CliError {}

    let args: Vec<OsString> = std::env::args_os().skip(1).collect();
    if args.len() == 1 && args[0] == "--help" {
        println!("vellum-mcp --socket PRIVATE_SOCKET");
        println!(
            "Runs the current MCP stdio projection for an already-running local Vellum daemon. MCP client identity is descriptive, not authentication."
        );
        return Ok(());
    }
    if args.len() != 2 || args[0] != "--socket" {
        return Err(Box::new(CliError(
            "usage: vellum-mcp --socket PRIVATE_SOCKET",
        )));
    }
    let socket = PathBuf::from(&args[1]);
    if socket.as_os_str().as_bytes().len() > 100 {
        return Err(Box::new(CliError("socket path exceeds reference limit")));
    }

    let connector = UnixConnector::new(socket);
    let transport = RemoteTransport::new(connector, TransportLimits::reference())
        .map_err(|_| CliError("transport construction refused"))?;
    let client = Client::connect(transport).map_err(|_| CliError("daemon connection refused"))?;
    let limits = McpLimits::reference();
    let mut server =
        McpServer::new(client, limits).map_err(|_| CliError("MCP construction refused"))?;
    let stdin = std::io::stdin();
    let mut input = stdin.lock();
    let stdout = std::io::stdout();
    let mut output = BufWriter::new(stdout.lock());
    loop {
        let response = match read_bounded_line(&mut input, limits.max_request_bytes)? {
            BoundedLine::Eof => break,
            BoundedLine::Limit(observed) => Some(server.limit_response(observed)),
            BoundedLine::Truncated(observed) => Some(server.truncated_response(observed)),
            BoundedLine::Line(line) => server.handle(&line),
        };
        if let Some(response) = response {
            output.write_all(&response)?;
            output.write_all(b"\n")?;
            output.flush()?;
        }
    }
    if server.into_client().disconnect().is_err() {
        return Err(Box::new(CliError("daemon disconnect failed")));
    }
    Ok(())
}

#[cfg(not(unix))]
fn main() {
    eprintln!("vellum-mcp: the qualified local daemon transport requires Unix");
    std::process::exit(2);
}
