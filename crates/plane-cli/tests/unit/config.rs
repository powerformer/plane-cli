use crate::config::{ConfigEnv, ConfigOverrides, PlaneConfig};
use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

#[test]
fn default_config_uses_home_plane_dir() {
    let root = unique_test_dir();
    let home = root.join("home");
    let config = resolve(&root, [("HOME", home.clone())], ConfigOverrides::default());

    assert_eq!(config.workspace_root, root);
    assert_eq!(config.config_path, home.join(".plane/plane.toml"));
    assert_eq!(config.plane_home, home.join(".plane"));
    assert_eq!(config.state_dir, home.join(".plane/state"));
    assert_eq!(
        config.skills_state_path,
        home.join(".plane/state/skills.json")
    );
    assert_eq!(config.api_base_url, None);
    assert_eq!(config.api_key, None);
    assert_eq!(config.workspace_slug, None);
}

#[test]
fn plane_home_env_bootstraps_default_config_path() {
    let root = unique_test_dir();
    let home = root.join("home");
    let plane_home = root.join("env-plane-home");
    let config = resolve(
        &root,
        [("HOME", home), ("PLANE_HOME", plane_home.clone())],
        ConfigOverrides::default(),
    );

    assert_eq!(config.config_path, plane_home.join("plane.toml"));
    assert_eq!(config.plane_home, plane_home);
}

#[test]
fn plane_config_env_selects_config_file() {
    let root = unique_test_dir();
    let config_dir = root.join("etc");
    fs::create_dir_all(&config_dir).expect("config dir");
    let config_path = config_dir.join("plane.toml");
    fs::write(&config_path, r#"home = "from-plane-config""#).expect("write config");

    let config = resolve(
        &root,
        [
            ("HOME", root.join("home")),
            ("PLANE_HOME", root.join("env-home")),
            ("PLANE_CONFIG", config_path.clone()),
        ],
        ConfigOverrides::default(),
    );

    assert_eq!(config.config_path, config_path);
    assert_eq!(config.plane_home, config_dir.join("from-plane-config"));
}

#[test]
fn explicit_config_file_overrides_env_paths() {
    let root = unique_test_dir();
    let config_dir = root.join("config");
    fs::create_dir_all(&config_dir).expect("config dir");
    let config_path = config_dir.join("plane.toml");
    fs::write(
        &config_path,
        r#"
home = "file-home"
state_dir = "file-state"
skills_state_path = "file-state/skills.custom.json"
releases_public_url = "https://mirror.example.test/"
api_base_url = "https://plane.example.test/"
api_key = "file-token"
workspace_slug = "file-workspace"
codex_home = "codex-config"
"#,
    )
    .expect("write config");

    let config = resolve(
        &root,
        [
            ("HOME", root.join("home")),
            ("PLANE_HOME", root.join("env-home")),
            ("PLANE_STATE_DIR", root.join("env-state")),
            (
                "PLANE_SKILLS_STATE_PATH",
                root.join("env-state/skills.json"),
            ),
            ("PLANE_RELEASES_PUBLIC_URL", root.join("not-a-url")),
            ("PLANE_API_BASE_URL", root.join("env-plane.example.test")),
            ("PLANE_API_KEY", root.join("env-token")),
            ("PLANE_WORKSPACE_SLUG", root.join("env-workspace")),
            ("CODEX_HOME", root.join("env-codex")),
        ],
        ConfigOverrides {
            config_path: Some(config_path.clone()),
            ..ConfigOverrides::default()
        },
    );

    assert_eq!(config.config_path, config_path);
    assert_eq!(config.plane_home, config_dir.join("file-home"));
    assert_eq!(config.state_dir, config_dir.join("file-state"));
    assert_eq!(
        config.skills_state_path,
        config_dir.join("file-state/skills.custom.json")
    );
    assert_eq!(config.releases_public_url, "https://mirror.example.test");
    assert_eq!(
        config.api_base_url.as_deref(),
        Some("https://plane.example.test")
    );
    assert_eq!(config.api_key.as_deref(), Some("file-token"));
    assert_eq!(config.workspace_slug.as_deref(), Some("file-workspace"));
    assert_eq!(config.codex_home, Some(config_dir.join("codex-config")));
    assert!(config.codex_home_explicit);
}

#[test]
fn args_override_config_and_env_paths() {
    let root = unique_test_dir();
    let config_path = root.join("plane.toml");
    fs::create_dir_all(&root).expect("test root");
    fs::write(
        &config_path,
        r#"
home = "file-home"
state_dir = "file-state"
skills_state_path = "file-state/skills.json"
"#,
    )
    .expect("write config");

    let arg_home = root.join("arg-home");
    let arg_state = root.join("arg-state");
    let arg_skills = root.join("arg-skills.json");
    let config = resolve(
        &root,
        [
            ("HOME", root.join("home")),
            ("PLANE_HOME", root.join("env-home")),
            ("PLANE_STATE_DIR", root.join("env-state")),
            (
                "PLANE_SKILLS_STATE_PATH",
                root.join("env-state/skills.json"),
            ),
        ],
        ConfigOverrides {
            config_path: Some(config_path),
            plane_home: Some(arg_home.clone()),
            state_dir: Some(arg_state.clone()),
            skills_state_path: Some(arg_skills.clone()),
            ..ConfigOverrides::default()
        },
    );

    assert_eq!(config.plane_home, arg_home);
    assert_eq!(config.state_dir, arg_state);
    assert_eq!(config.skills_state_path, arg_skills);
}

#[test]
fn args_override_config_and_env_api_values() {
    let root = unique_test_dir();
    let config_path = root.join("plane.toml");
    fs::create_dir_all(&root).expect("test root");
    fs::write(
        &config_path,
        r#"
api_base_url = "https://file-plane.example.test"
api_key = "file-token"
workspace_slug = "file-workspace"
"#,
    )
    .expect("write config");

    let config = resolve(
        &root,
        [
            ("HOME", root.join("home").into_os_string()),
            (
                "PLANE_API_BASE_URL",
                "https://env-plane.example.test".into(),
            ),
            ("PLANE_API_KEY", "env-token".into()),
            ("PLANE_WORKSPACE_SLUG", "env-workspace".into()),
        ],
        ConfigOverrides {
            config_path: Some(config_path),
            api_base_url: Some("https://arg-plane.example.test".to_string()),
            api_key: Some("arg-token".to_string()),
            workspace_slug: Some("arg-workspace".to_string()),
            ..ConfigOverrides::default()
        },
    );

    assert_eq!(
        config.api_base_url.as_deref(),
        Some("https://arg-plane.example.test")
    );
    assert_eq!(config.api_key.as_deref(), Some("arg-token"));
    assert_eq!(config.workspace_slug.as_deref(), Some("arg-workspace"));
}

#[test]
fn empty_config_api_values_fall_back_to_env() {
    let root = unique_test_dir();
    let config_path = root.join("plane.toml");
    fs::create_dir_all(&root).expect("test root");
    fs::write(
        &config_path,
        r#"
api_base_url = ""
api_key = "   "
workspace_slug = ""
"#,
    )
    .expect("write config");

    let config = resolve(
        &root,
        [
            ("HOME", root.join("home").into_os_string()),
            ("PLANE_API_KEY", "env-token".into()),
        ],
        ConfigOverrides {
            config_path: Some(config_path),
            ..ConfigOverrides::default()
        },
    );

    assert_eq!(config.api_base_url, None);
    assert_eq!(config.api_key.as_deref(), Some("env-token"));
    assert_eq!(config.workspace_slug, None);
}

#[test]
fn config_state_path_beats_state_derived_from_arg_home() {
    let root = unique_test_dir();
    let config_path = root.join("plane.toml");
    fs::create_dir_all(&root).expect("test root");
    fs::write(&config_path, r#"state_dir = "configured-state""#).expect("write config");
    let arg_home = root.join("arg-home");
    let config = resolve(
        &root,
        [("HOME", root.join("home"))],
        ConfigOverrides {
            config_path: Some(config_path),
            plane_home: Some(arg_home.clone()),
            ..ConfigOverrides::default()
        },
    );

    assert_eq!(config.plane_home, arg_home);
    assert_eq!(config.state_dir, root.join("configured-state"));
}

#[test]
fn explicit_missing_config_file_errors() {
    let root = unique_test_dir();
    let error = PlaneConfig::resolve(
        ConfigOverrides {
            config_path: Some(root.join("missing.toml")),
            ..ConfigOverrides::default()
        },
        ConfigEnv::new(root.clone(), [("HOME", root.join("home"))]),
    )
    .expect_err("missing explicit config should fail");

    assert!(error.contains("failed to read config"));
}

#[test]
fn tilde_paths_expand_against_user_home() {
    let root = unique_test_dir();
    let home = root.join("home");
    let config = resolve(
        &root,
        [
            ("HOME", home.clone()),
            ("PLANE_HOME", PathBuf::from("~/.plane-dev")),
        ],
        ConfigOverrides::default(),
    );

    assert_eq!(config.plane_home, home.join(".plane-dev"));
    assert_eq!(config.config_path, home.join(".plane-dev/plane.toml"));
}

fn resolve<I, K, V>(root: &Path, vars: I, overrides: ConfigOverrides) -> PlaneConfig
where
    I: IntoIterator<Item = (K, V)>,
    K: Into<String>,
    V: Into<std::ffi::OsString>,
{
    PlaneConfig::resolve(overrides, ConfigEnv::new(root.to_path_buf(), vars)).expect("config")
}

fn unique_test_dir() -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "plane-cli-config-test-{}-{nanos}",
        std::process::id()
    ))
}
