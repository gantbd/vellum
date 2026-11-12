#[cfg(target_os = "macos")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    vellum_daemon::cli::run()
}

#[cfg(not(target_os = "macos"))]
fn main() {
    vellum_daemon::cli::run();
}
