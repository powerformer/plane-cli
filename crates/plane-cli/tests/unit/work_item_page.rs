use crate::{
    commands::execute,
    core::{
        app::AppState,
        config::{ConfigEnv, ConfigOverrides, PlaneConfig},
    },
};

fn args(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| value.to_string()).collect()
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
fn api_work_item_page_help_lists_subcommands() {
    let result = execute(
        &state_with_workspace("acme"),
        &args(&["api", "work-item", "page", "--help"]),
    );

    assert_eq!(result.status, 0);
    assert!(result.stdout.contains("list"));
    assert!(result.stdout.contains("link"));
    assert!(result.stdout.contains("unlink"));
    assert!(result.stdout.contains("work-items/<work_item>/pages"));
    assert!(result.stderr.is_empty());
}

#[test]
fn api_work_item_page_link_dry_run_prints_expected_path_and_body() {
    // dry-run with UUID references stays offline; human-readable references
    // resolve over the network first.
    let result = execute(
        &state_with_workspace("acme"),
        &args(&[
            "api",
            "work-item",
            "page",
            "link",
            "--project",
            "11111111-2222-4333-8444-555555555555",
            "--work-item",
            "aaaaaaaa-bbbb-4ccc-8ddd-eeeeeeeeeeee",
            "page-1",
            "page-2",
            "--dry-run",
        ]),
    );

    assert_eq!(result.status, 0);
    assert!(result
        .stdout
        .contains("DRY RUN POST /api/v1/workspaces/acme/projects/11111111-2222-4333-8444-555555555555/work-items/aaaaaaaa-bbbb-4ccc-8ddd-eeeeeeeeeeee/pages/"));
    assert!(result.stdout.contains("\"page_id\""));
    assert!(result.stdout.contains("page-1"));
    assert!(result.stdout.contains("page-2"));
    assert!(result.stderr.is_empty());
}

#[test]
fn api_work_item_page_unlink_dry_run_prints_expected_path() {
    let result = execute(
        &state_with_workspace("acme"),
        &args(&[
            "api",
            "work-item",
            "page",
            "unlink",
            "--project",
            "11111111-2222-4333-8444-555555555555",
            "--work-item",
            "aaaaaaaa-bbbb-4ccc-8ddd-eeeeeeeeeeee",
            "page-1",
            "--dry-run",
        ]),
    );

    assert_eq!(result.status, 0);
    assert_eq!(
        result.stdout,
        "DRY RUN DELETE /api/v1/workspaces/acme/projects/11111111-2222-4333-8444-555555555555/work-items/aaaaaaaa-bbbb-4ccc-8ddd-eeeeeeeeeeee/pages/page-1/\n"
    );
    assert!(result.stderr.is_empty());
}
