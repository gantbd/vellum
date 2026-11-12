use std::io::{BufWriter, Write};

#[cfg(unix)]
#[derive(Debug)]
struct CliError(&'static str);

#[cfg(unix)]
impl std::fmt::Display for CliError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.0)
    }
}

#[cfg(unix)]
impl std::error::Error for CliError {}

#[cfg(unix)]
fn serve_connector<X>(connector: X) -> Result<(), Box<dyn std::error::Error>>
where
    X: vellum_sdk_client::transport::FrameExchange,
{
    use crate::{McpLimits, McpServer};
    use vellum_sdk_client::{
        client::Client,
        transport::{RemoteTransport, TransportLimits},
    };

    let transport = RemoteTransport::new(connector, TransportLimits::reference())
        .map_err(|_| CliError("transport construction refused"))?;
    let client = Client::connect(transport).map_err(|_| CliError("daemon connection refused"))?;
    let server = McpServer::new(client, McpLimits::reference())
        .map_err(|_| CliError("MCP construction refused"))?;
    serve(server)
}

#[cfg(unix)]
fn run_authenticated(
    connector: vellum_sdk_client::transport::AuthenticatedUnixConnector,
) -> Result<(), Box<dyn std::error::Error>> {
    use crate::{McpLimits, McpServer};
    use vellum_sdk_client::{
        client::Client,
        transport::{RemoteTransport, TransportLimits},
    };

    let transport = RemoteTransport::new(connector, TransportLimits::reference())
        .map_err(|_| CliError("transport construction refused"))?;
    let client = Client::connect(transport).map_err(|_| CliError("daemon connection refused"))?;
    let server = McpServer::new_authenticated(client, McpLimits::reference())
        .map_err(|_| CliError("MCP construction refused"))?;
    serve(server)
}

#[cfg(unix)]
fn serve<T>(mut server: crate::McpServer<T>) -> Result<(), Box<dyn std::error::Error>>
where
    T: vellum_sdk_client::protocol::ServicePort,
{
    use crate::{BoundedLine, McpLimits, read_bounded_line};

    let limits = McpLimits::reference();
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

#[cfg(unix)]
pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    use std::{ffi::OsString, os::unix::ffi::OsStrExt, path::PathBuf};
    use vellum_adapter_local_authentication::{
        LocalCredentialClaimProviderV1, LocalCredentialStoreV1,
    };
    use vellum_sdk_client::transport::{AuthenticatedUnixConnector, UnixConnector};

    let args: Vec<OsString> = std::env::args_os().skip(1).collect();
    if args.len() == 1 && args[0] == "--help" {
        println!("vellum-mcp --socket PRIVATE_SOCKET [--credential PRIVATE_CREDENTIAL]");
        println!(
            "Runs the current MCP stdio projection for an already-running local Vellum daemon. A configured private credential authenticates a stable subject but grants no Vellum action. Credential material is never accepted through argv or environment."
        );
        return Ok(());
    }
    if !matches!(args.len(), 2 | 4) || args[0] != "--socket" {
        return Err(Box::new(CliError(
            "usage: vellum-mcp --socket PRIVATE_SOCKET [--credential PRIVATE_CREDENTIAL]",
        )));
    }
    let credential = if args.len() == 4 && args[2] == "--credential" {
        if args[3].as_bytes().starts_with(b"--") {
            return Err(Box::new(CliError("credential option refused")));
        }
        Some(PathBuf::from(&args[3]))
    } else if args.len() == 4 {
        return Err(Box::new(CliError("unknown MCP option")));
    } else {
        None
    };
    let socket = PathBuf::from(&args[1]);
    if socket.as_os_str().as_bytes().len() > 100 {
        return Err(Box::new(CliError("socket path exceeds reference limit")));
    }

    if let Some(path) = credential {
        let record = LocalCredentialStoreV1::default()
            .load(&path)
            .map_err(|_| CliError("credential loading refused"))?;
        run_authenticated(AuthenticatedUnixConnector::with_provider(
            socket,
            LocalCredentialClaimProviderV1::new(record),
        ))
    } else {
        serve_connector(UnixConnector::new(socket))
    }
}

#[cfg(not(unix))]
pub fn run() {
    eprintln!("vellum-mcp: the qualified local daemon transport requires Unix");
    std::process::exit(2);
}
