use crate::{
    app::AppState,
    cli::execute,
    config::{ConfigEnv, ConfigOverrides, PlaneConfig},
};

fn state() -> AppState {
    AppState {
        config: test_config(),
        version: "0.1.0-test",
    }
}

fn test_config() -> PlaneConfig {
    PlaneConfig::resolve(
        ConfigOverrides::default(),
        ConfigEnv::new(
            std::env::temp_dir(),
            [("HOME", std::env::temp_dir().join("plane-cli-test-home"))],
        ),
    )
    .expect("test config")
}

fn args(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| value.to_string()).collect()
}

#[test]
fn no_args_prints_help() {
    let result = execute(&state(), &args(&[]));

    assert_eq!(result.status, 0);
    assert!(result.stdout.contains("Usage:"));
    assert!(result.stdout.contains("Commands:"));
    assert!(result.stdout.contains("--config"));
    assert!(result.stdout.contains("--home"));
    assert!(result.stdout.contains("skill"));
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
fn version_command_prints_version() {
    let result = execute(&state(), &args(&["version"]));

    assert_eq!(result.status, 0);
    assert_eq!(result.stdout, "plane 0.1.0-test\n");
    assert!(result.stderr.is_empty());
}

#[test]
fn skill_help_is_self_describing() {
    let result = execute(&state(), &args(&["skill", "--help"]));

    assert_eq!(result.status, 0);
    assert!(result.stdout.contains("Install, upgrade, list"));
    assert!(result.stdout.contains("install"));
    assert!(result.stdout.contains("upgrade"));
    assert!(result.stdout.contains("uninstall"));
    assert!(result.stderr.is_empty());
}

#[test]
fn skill_install_help_explains_path() {
    let result = execute(&state(), &args(&["skill", "install", "--help"]));

    assert_eq!(result.status, 0);
    assert!(result.stdout.contains("--path"));
    assert!(result.stdout.contains("final skill directory"));
    assert!(result.stdout.contains("--channel"));
    assert!(result.stderr.is_empty());
}

#[test]
fn unknown_command_fails_with_usage_hint() {
    let result = execute(&state(), &args(&["fly"]));

    assert_eq!(result.status, 2);
    assert!(result.stdout.is_empty());
    assert!(result.stderr.contains("unrecognized subcommand"));
    assert!(result.stderr.contains("Usage:"));
}
