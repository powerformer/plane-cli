use crate::{app::AppState, cli::execute, config::PlaneConfig};

fn state() -> AppState {
    AppState {
        config: PlaneConfig::default(),
        version: "0.1.0-test",
    }
}

fn args(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| value.to_string()).collect()
}

#[test]
fn no_args_prints_help() {
    let result = execute(&state(), &args(&[]));

    assert_eq!(result.status, 0);
    assert!(result.stdout.contains("Usage:"));
    assert!(result.stdout.contains("plane help"));
    assert!(result.stderr.is_empty());
}

#[test]
fn help_prints_help() {
    let result = execute(&state(), &args(&["help"]));

    assert_eq!(result.status, 0);
    assert!(result.stdout.contains("Commands:"));
    assert!(result.stderr.is_empty());
}

#[test]
fn version_prints_version() {
    let result = execute(&state(), &args(&["--version"]));

    assert_eq!(result.status, 0);
    assert_eq!(result.stdout, "plane 0.1.0-test\n");
    assert!(result.stderr.is_empty());
}

#[test]
fn unknown_command_fails_with_usage_hint() {
    let result = execute(&state(), &args(&["fly"]));

    assert_eq!(result.status, 2);
    assert!(result.stdout.is_empty());
    assert!(result.stderr.contains("unknown command"));
    assert!(result.stderr.contains("plane help"));
}
