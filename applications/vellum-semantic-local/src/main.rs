fn main() {
    if std::env::args().nth(1).as_deref() == Some("--probe") {
        println!("vellum-semantic-local/0.1");
        return;
    }
    eprintln!("vellum-semantic-local: embed the library with an operator-installed profile");
    std::process::exit(2);
}
