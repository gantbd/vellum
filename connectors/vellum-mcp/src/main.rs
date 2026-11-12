#[cfg(unix)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    vellum_mcp::cli::run()
}

#[cfg(not(unix))]
fn main() {
    vellum_mcp::cli::run();
}
