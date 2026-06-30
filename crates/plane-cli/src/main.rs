mod app;
mod cli;
mod config;
mod output;
mod skill;

fn main() {
    init_tracing();
    let state = app::AppState::from_env();
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let result = cli::execute(&state, &args);
    result.emit();
    std::process::exit(result.status);
}

fn init_tracing() {
    let verbose = std::env::args().any(|arg| arg == "--verbose");
    let default_level = if verbose { "debug" } else { "info" };
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(default_level));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .without_time()
        .with_writer(std::io::stderr)
        .init();
}
