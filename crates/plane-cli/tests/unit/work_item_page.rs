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
    assert!(result.stdout.contains("issues/<work_item>/pages"));
    assert!(result.stderr.is_empty());
}

#[test]
fn api_work_item_page_link_dry_run_prints_expected_path_and_body() {
    let result = execute(
        &state_with_workspace("acme"),
        &args(&[
            "api",
            "work-item",
            "page",
            "link",
            "--project",
            "p1",
            "--work-item",
            "wi1",
            "page-1",
            "page-2",
            "--dry-run",
        ]),
    );

    assert_eq!(result.status, 0);
    assert!(result
        .stdout
        .contains("DRY RUN POST /api/v1/workspaces/acme/projects/p1/issues/wi1/pages/"));
    assert!(result.stdout.contains("\"page_ids\""));
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
            "p1",
            "--work-item",
            "wi1",
            "page-1",
            "--dry-run",
        ]),
    );

    assert_eq!(result.status, 0);
    assert_eq!(
        result.stdout,
        "DRY RUN DELETE /api/v1/workspaces/acme/projects/p1/issues/wi1/pages/page-1/\n"
    );
    assert!(result.stderr.is_empty());
}
