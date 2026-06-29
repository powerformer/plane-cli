use crate::config::PlaneConfig;

#[test]
fn default_config_has_workspace_root() {
    let config = PlaneConfig::default();

    assert!(config.workspace_root.is_absolute() || config.workspace_root.as_os_str() == ".");
}
