use crate::config::{ConfigOverrides, PlaneConfig};

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
}

pub fn build_version() -> &'static str {
    option_env!("PLANE_BUILD_VERSION").unwrap_or(env!("CARGO_PKG_VERSION"))
}
