mod commands;
mod core;

fn main() {
    crate::core::logger::init();
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let result = commands::execute_from_env(&args);
    result.emit();
    std::process::exit(result.status);
}
