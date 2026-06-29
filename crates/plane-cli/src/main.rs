mod app;
mod cli;
mod config;
mod output;

fn main() {
    let state = app::AppState::from_env();
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let result = cli::execute(&state, &args);
    result.emit();
    std::process::exit(result.status);
}
