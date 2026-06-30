use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaneConfig {
    pub workspace_root: PathBuf,
    pub plane_home: PathBuf,
    pub state_dir: PathBuf,
    pub skills_state_path: PathBuf,
    pub releases_public_url: String,
}

impl Default for PlaneConfig {
    fn default() -> Self {
        let plane_home = default_plane_home();
        let state_dir = plane_home.join("state");
        let skills_state_path = state_dir.join("skills.json");
        Self {
            workspace_root: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            plane_home,
            state_dir,
            skills_state_path,
            releases_public_url: std::env::var("PLANE_RELEASES_PUBLIC_URL")
                .unwrap_or_else(|_| "https://releases.plane.powerformer.net".to_string()),
        }
    }
}

fn default_plane_home() -> PathBuf {
    if let Some(value) = std::env::var_os("PLANE_HOME") {
        return PathBuf::from(value);
    }
    if cfg!(windows) {
        if let Some(value) = std::env::var_os("LOCALAPPDATA") {
            return PathBuf::from(value).join("plane");
        }
    }
    home_dir()
        .map(|home| home.join(".local").join("share").join("plane"))
        .unwrap_or_else(|| PathBuf::from(".").join(".plane"))
}

pub fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("USERPROFILE").map(PathBuf::from))
}
