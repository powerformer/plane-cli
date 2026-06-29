use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaneConfig {
    pub workspace_root: PathBuf,
}

impl Default for PlaneConfig {
    fn default() -> Self {
        Self {
            workspace_root: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        }
    }
}
