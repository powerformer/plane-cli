use crate::config::{ConfigOverrides, PlaneConfig};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct AppState {
    pub config: PlaneConfig,
    pub version: &'static str,
}

impl AppState {
    pub fn from_env(overrides: ConfigOverrides) -> Result<Self, String> {
        Ok(Self {
            config: PlaneConfig::load(overrides)?,
            version: build_version(),
        })
    }

    #[allow(dead_code)]
    pub fn load_from_dev() -> Result<Self, String> {
        let config_path = std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(".local")
            .join(".plane")
            .join("plane.toml");
        Self::from_env(ConfigOverrides {
            config_path: Some(config_path),
            ..ConfigOverrides::default()
        })
    }
}

pub fn build_version() -> &'static str {
    option_env!("PLANE_BUILD_VERSION").unwrap_or(env!("CARGO_PKG_VERSION"))
}
