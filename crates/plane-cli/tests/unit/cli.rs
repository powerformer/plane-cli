use crate::{
    commands::execute,
    core::{
        app::AppState,
        config::{ConfigEnv, ConfigOverrides, PlaneConfig},
    },
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
    assert!(result.stdout.contains("--api-base-url"));
    assert!(result.stdout.contains("api"));
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
fn api_help_is_self_describing() {
    let result = execute(&state(), &args(&["api", "--help"]));

    assert_eq!(result.status, 0);
    assert!(result.stdout.contains("X-API-Key"));
    assert!(result.stdout.contains("api_base_url"));
    assert!(result.stdout.contains("me"));
    assert!(result.stderr.is_empty());
}

#[test]
fn api_me_help_explains_smoke_path() {
    let result = execute(&state(), &args(&["api", "me", "--help"]));

    assert_eq!(result.status, 0);
    assert!(result.stdout.contains("/api/v1/users/me/"));
    assert!(result.stdout.contains("--json"));
    assert!(result.stderr.is_empty());
}

#[test]
fn api_me_requires_api_key() {
    // api_base_url falls back to the default backend, so api_key is the only
    // setting that must be present.
    let result = execute(&state(), &args(&["api", "me"]));

    assert_eq!(result.status, 1);
    assert!(result.stdout.is_empty());
    assert!(result.stderr.contains("api_key is required"));
}

#[test]
fn dep_help_lists_subcommands() {
    let result = execute(&state(), &args(&["dep", "--help"]));

    assert_eq!(result.status, 0);
    assert!(result.stdout.contains("add"));
    assert!(result.stdout.contains("rm"));
    assert!(result.stdout.contains("ls"));
    assert!(result.stdout.contains("gc"));
    assert!(result.stderr.is_empty());
}

#[test]
fn dep_add_help_explains_target() {
    let result = execute(&state(), &args(&["dep", "add", "--help"]));

    assert_eq!(result.status, 0);
    assert!(result.stdout.contains("--on"));
    assert!(result.stdout.contains("KEY:SEQ"));
    assert!(result.stderr.is_empty());
}

#[test]
fn upgrade_help_explains_report_only() {
    let result = execute(&state(), &args(&["upgrade", "--help"]));

    assert_eq!(result.status, 0);
    assert!(result.stdout.contains("upgrade"));
    assert!(result.stdout.contains("does not download or replace"));
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

#[test]
fn api_page_help_lists_crud_verbs() {
    let result = execute(&state(), &args(&["api", "page", "--help"]));

    assert_eq!(result.status, 0);
    assert!(result.stdout.contains("create"));
    assert!(result.stdout.contains("update"));
    assert!(result.stdout.contains("delete"));
    assert!(result.stdout.contains("Markdown"));
    assert!(result.stderr.is_empty());
}

fn state_with_workspace(slug: &str) -> AppState {
    let config = PlaneConfig::resolve(
        ConfigOverrides::default(),
        ConfigEnv::new(
            std::env::temp_dir(),
            [
                (
                    "HOME".to_string(),
                    std::env::temp_dir()
                        .join("plane-cli-test-home")
                        .to_string_lossy()
                        .to_string(),
                ),
                ("PLANE_WORKSPACE_SLUG".to_string(), slug.to_string()),
            ],
        ),
    )
    .expect("test config");
    AppState {
        config,
        version: "0.1.0-test",
    }
}

#[test]
fn api_page_create_dry_run_converts_markdown() {
    // dry-run returns before any network call; it only needs a workspace.
    let result = execute(
        &state_with_workspace("acme"),
        &args(&[
            "api",
            "page",
            "create",
            "--project",
            "p1",
            "--name",
            "Doc",
            "--body",
            "# Title",
            "--dry-run",
        ]),
    );

    assert_eq!(result.status, 0);
    assert!(result.stdout.contains("DRY RUN POST"));
    assert!(result.stdout.contains("workspaces/acme/projects/p1/pages/"));
    assert!(result.stdout.contains("<h1>Title</h1>"));
    assert!(result.stderr.is_empty());
}

#[test]
fn api_page_create_rejects_unknown_access() {
    let result = execute(
        &state(),
        &args(&[
            "api",
            "page",
            "create",
            "--project",
            "p1",
            "--name",
            "Doc",
            "--access",
            "secret",
        ]),
    );

    assert_eq!(result.status, 2);
    assert!(result.stderr.contains("--access"));
}

#[test]
fn api_comment_help_lists_verbs() {
    let result = execute(&state(), &args(&["api", "comment", "--help"]));

    assert_eq!(result.status, 0);
    assert!(result.stdout.contains("list"));
    assert!(result.stdout.contains("create"));
    assert!(result.stdout.contains("delete"));
    assert!(result.stderr.is_empty());
}

#[test]
fn api_comment_create_help_explains_body_and_identifier() {
    let result = execute(&state(), &args(&["api", "comment", "create", "--help"]));

    assert_eq!(result.status, 0);
    assert!(result.stdout.contains("--work-item"));
    assert!(result.stdout.contains("--from-file"));
    assert!(result.stdout.contains("OPEND-7")); // identifier example
    assert!(result.stderr.is_empty());
}

#[test]
fn api_comment_create_dry_run_converts_markdown() {
    // A UUID work item needs no lookup, so dry-run stays fully offline.
    let result = execute(
        &state_with_workspace("acme"),
        &args(&[
            "api",
            "comment",
            "create",
            "--project",
            "p1",
            "--work-item",
            "11111111-1111-1111-1111-111111111111",
            "--body",
            "looks **good**",
            "--dry-run",
        ]),
    );

    assert_eq!(result.status, 0);
    assert!(result.stdout.contains("DRY RUN POST"));
    assert!(result
        .stdout
        .contains("work-items/11111111-1111-1111-1111-111111111111/comments/"));
    assert!(result.stdout.contains("comment_html"));
    assert!(result.stdout.contains("<strong>good</strong>"));
    assert!(result.stderr.is_empty());
}

#[test]
fn api_comment_create_requires_a_body() {
    let result = execute(
        &state_with_workspace("acme"),
        &args(&[
            "api",
            "comment",
            "create",
            "--project",
            "p1",
            "--work-item",
            "11111111-1111-1111-1111-111111111111",
            "--dry-run",
        ]),
    );

    assert_eq!(result.status, 1);
    assert!(result.stderr.contains("comment body is required"));
}
