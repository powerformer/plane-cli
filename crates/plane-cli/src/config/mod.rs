use serde::Deserialize;
use std::{
    collections::BTreeMap,
    ffi::{OsStr, OsString},
    fs,
    path::{Component, Path, PathBuf},
};

const DEFAULT_RELEASES_PUBLIC_URL: &str = "https://releases.plane.powerformer.net";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaneConfig {
    pub workspace_root: PathBuf,
    pub config_path: PathBuf,
    pub plane_home: PathBuf,
    pub state_dir: PathBuf,
    pub skills_state_path: PathBuf,
    pub releases_public_url: String,
    pub user_home: Option<PathBuf>,
    pub codex_home: Option<PathBuf>,
    pub codex_home_explicit: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ConfigOverrides {
    pub config_path: Option<PathBuf>,
    pub plane_home: Option<PathBuf>,
    pub state_dir: Option<PathBuf>,
    pub skills_state_path: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct ConfigEnv {
    current_dir: PathBuf,
    vars: BTreeMap<String, OsString>,
}

impl ConfigEnv {
    pub fn from_process() -> Self {
        let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let vars = std::env::vars_os()
            .map(|(key, value)| (key.to_string_lossy().to_string(), value))
            .collect();
        Self { current_dir, vars }
    }

    #[allow(dead_code)]
    pub(crate) fn new<I, K, V>(current_dir: PathBuf, vars: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<OsString>,
    {
        Self {
            current_dir,
            vars: vars
                .into_iter()
                .map(|(key, value)| (key.into(), value.into()))
                .collect(),
        }
    }

    fn var_os(&self, name: &str) -> Option<OsString> {
        self.vars.get(name).cloned()
    }
}

impl PlaneConfig {
    pub fn load(overrides: ConfigOverrides) -> Result<Self, String> {
        Self::resolve(overrides, ConfigEnv::from_process())
    }

    pub(crate) fn resolve(overrides: ConfigOverrides, env: ConfigEnv) -> Result<Self, String> {
        let workspace_root = env.current_dir.clone();
        let user_home = process_home_dir(&env);

        let env_plane_home = env
            .var_os("PLANE_HOME")
            .map(PathBuf::from)
            .map(|path| normalize_env_path(&path, &env, user_home.as_deref()));
        let default_bootstrap_home = env_plane_home
            .clone()
            .unwrap_or_else(|| default_plane_home(&env, user_home.as_deref()));

        let explicit_config_path =
            overrides.config_path.is_some() || env.var_os("PLANE_CONFIG").is_some();
        let config_path = match overrides.config_path.as_ref() {
            Some(path) => normalize_cli_path(path, &env, user_home.as_deref()),
            None => match env.var_os("PLANE_CONFIG") {
                Some(path) => normalize_env_path(&PathBuf::from(path), &env, user_home.as_deref()),
                None => default_bootstrap_home.join("plane.toml"),
            },
        };

        let file = read_config_file(&config_path, explicit_config_path)?;
        let config_base = config_parent(&config_path, &workspace_root);

        let config_plane_home = file
            .home
            .as_ref()
            .map(|path| normalize_config_path(path, &config_base, user_home.as_deref()));
        let plane_home = overrides
            .plane_home
            .as_ref()
            .map(|path| normalize_cli_path(path, &env, user_home.as_deref()))
            .or(config_plane_home)
            .or(env_plane_home)
            .unwrap_or_else(|| default_plane_home(&env, user_home.as_deref()));
        ensure_non_empty_path("home", &plane_home)?;

        let config_state_dir = file
            .state_dir
            .as_ref()
            .map(|path| normalize_config_path(path, &config_base, user_home.as_deref()));
        let env_state_dir = env
            .var_os("PLANE_STATE_DIR")
            .map(PathBuf::from)
            .map(|path| normalize_env_path(&path, &env, user_home.as_deref()));
        let state_dir = overrides
            .state_dir
            .as_ref()
            .map(|path| normalize_cli_path(path, &env, user_home.as_deref()))
            .or(config_state_dir)
            .or(env_state_dir)
            .unwrap_or_else(|| plane_home.join("state"));
        ensure_non_empty_path("state_dir", &state_dir)?;

        let config_skills_state_path = file
            .skills_state_path
            .as_ref()
            .map(|path| normalize_config_path(path, &config_base, user_home.as_deref()));
        let env_skills_state_path = env
            .var_os("PLANE_SKILLS_STATE_PATH")
            .or_else(|| env.var_os("PLANE_SKILL_STATE_PATH"))
            .map(PathBuf::from)
            .map(|path| normalize_env_path(&path, &env, user_home.as_deref()));
        let skills_state_path = overrides
            .skills_state_path
            .as_ref()
            .map(|path| normalize_cli_path(path, &env, user_home.as_deref()))
            .or(config_skills_state_path)
            .or(env_skills_state_path)
            .unwrap_or_else(|| state_dir.join("skills.json"));
        ensure_non_empty_path("skills_state_path", &skills_state_path)?;

        let releases_public_url = file
            .releases_public_url
            .or_else(|| env.var_os("PLANE_RELEASES_PUBLIC_URL").map(os_to_string))
            .unwrap_or_else(|| DEFAULT_RELEASES_PUBLIC_URL.to_string());
        let releases_public_url = releases_public_url.trim();
        if releases_public_url.is_empty() {
            return Err("releases_public_url cannot be empty".to_string());
        }

        let config_codex_home = file
            .codex_home
            .as_ref()
            .map(|path| normalize_config_path(path, &config_base, user_home.as_deref()));
        let env_codex_home = env
            .var_os("CODEX_HOME")
            .map(PathBuf::from)
            .map(|path| normalize_env_path(&path, &env, user_home.as_deref()));
        let codex_home_explicit = config_codex_home.is_some() || env_codex_home.is_some();
        let codex_home = config_codex_home
            .or(env_codex_home)
            .or_else(|| user_home.as_ref().map(|home| home.join(".codex")));

        Ok(Self {
            workspace_root,
            config_path,
            plane_home,
            state_dir,
            skills_state_path,
            releases_public_url: releases_public_url.trim_end_matches('/').to_string(),
            user_home,
            codex_home,
            codex_home_explicit,
        })
    }
}

#[derive(Debug, Deserialize, Default)]
#[serde(default, deny_unknown_fields, rename_all = "snake_case")]
struct PlaneConfigFile {
    #[serde(alias = "plane_home")]
    home: Option<PathBuf>,
    state_dir: Option<PathBuf>,
    skills_state_path: Option<PathBuf>,
    #[serde(alias = "release_url")]
    releases_public_url: Option<String>,
    codex_home: Option<PathBuf>,
}

fn read_config_file(path: &Path, explicit: bool) -> Result<PlaneConfigFile, String> {
    match fs::read_to_string(path) {
        Ok(source) => toml::from_str(&source)
            .map_err(|error| format!("{} is invalid TOML: {error}", path.display())),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound && !explicit => {
            Ok(PlaneConfigFile::default())
        }
        Err(error) => Err(format!("failed to read config {}: {error}", path.display())),
    }
}

fn config_parent(config_path: &Path, workspace_root: &Path) -> PathBuf {
    config_path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| workspace_root.to_path_buf())
}

fn normalize_cli_path(path: &Path, env: &ConfigEnv, user_home: Option<&Path>) -> PathBuf {
    normalize_path(path, &env.current_dir, user_home)
}

fn normalize_env_path(path: &Path, env: &ConfigEnv, user_home: Option<&Path>) -> PathBuf {
    normalize_path(path, &env.current_dir, user_home)
}

fn normalize_config_path(path: &Path, config_base: &Path, user_home: Option<&Path>) -> PathBuf {
    normalize_path(path, config_base, user_home)
}

fn normalize_path(path: &Path, base: &Path, user_home: Option<&Path>) -> PathBuf {
    let expanded = expand_tilde(path, user_home);
    if expanded.is_absolute() {
        expanded
    } else {
        base.join(expanded)
    }
}

fn expand_tilde(path: &Path, user_home: Option<&Path>) -> PathBuf {
    let Some(home) = user_home else {
        return path.to_path_buf();
    };
    let mut components = path.components();
    let Some(Component::Normal(first)) = components.next() else {
        return path.to_path_buf();
    };
    if first != OsStr::new("~") {
        return path.to_path_buf();
    }
    let mut expanded = home.to_path_buf();
    for component in components {
        expanded.push(component.as_os_str());
    }
    expanded
}

fn default_plane_home(env: &ConfigEnv, user_home: Option<&Path>) -> PathBuf {
    user_home
        .map(|home| home.join(".plane"))
        .unwrap_or_else(|| env.current_dir.join(".plane"))
}

fn process_home_dir(env: &ConfigEnv) -> Option<PathBuf> {
    env.var_os("HOME")
        .or_else(|| env.var_os("USERPROFILE"))
        .map(PathBuf::from)
        .map(|path| normalize_path(&path, &env.current_dir, None))
}

fn ensure_non_empty_path(name: &str, path: &Path) -> Result<(), String> {
    if path.as_os_str().is_empty() {
        return Err(format!("{name} cannot be empty"));
    }
    Ok(())
}

fn os_to_string(value: OsString) -> String {
    value.to_string_lossy().to_string()
}
