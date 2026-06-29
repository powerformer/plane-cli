use crate::config::PlaneConfig;

#[derive(Debug, Clone)]
pub struct AppState {
    pub config: PlaneConfig,
    pub version: &'static str,
}

impl AppState {
    pub fn from_env() -> Self {
        Self {
            config: PlaneConfig::default(),
            version: build_version(),
        }
    }
}

fn build_version() -> &'static str {
    option_env!("PLANE_BUILD_VERSION").unwrap_or(env!("CARGO_PKG_VERSION"))
}
