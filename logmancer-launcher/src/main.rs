fn main() {
    if let Err(error) = logmancer_launcher::run_from_env() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
