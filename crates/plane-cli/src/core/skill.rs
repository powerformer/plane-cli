use crate::core::app::AppState;
use chrono::{SecondsFormat, Utc};
use flate2::read::GzDecoder;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, OpenOptions},
    io::{Cursor, Read, Write},
    path::{Path, PathBuf},
    thread,
    time::{Duration, Instant},
};
use tar::Archive;
use tracing::{debug, info, warn};

const SKILL_NAME: &str = "plane-cli";
const MANAGED_BY: &str = "plane-cli";
const STATE_SCHEMA_VERSION: u32 = 1;
const SKILL_METADATA_SCHEMA_VERSION: u32 = 1;
const USER_AGENT: &str = "plane-cli";

#[derive(Debug, Clone)]
pub struct SkillInstallOptions {
    pub path: Option<PathBuf>,
    pub release_url: Option<String>,
    pub channel: String,
    pub version: Option<String>,
    pub dry_run: bool,
}

#[derive(Debug, Clone)]
pub struct SkillUpgradeOptions {
    pub release_url: Option<String>,
    pub channel: Option<String>,
    pub version: Option<String>,
    pub dry_run: bool,
}

#[derive(Debug, Clone)]
pub struct SkillUninstallOptions {
    pub dry_run: bool,
}

#[derive(Debug, Clone)]
struct InstallTarget {
    agent: String,
    path: PathBuf,
}

#[derive(Debug, Clone)]
struct DefaultAgentCandidate {
    agent: &'static str,
    presence_dir: PathBuf,
    skills_dir: PathBuf,
    allow_missing_presence: bool,
}

#[derive(Debug, Clone)]
struct ResolvedSkillRelease {
    release_url: String,
    channel: String,
    release_version: String,
    artifact_name: String,
    artifact_url: String,
    sha256: String,
}

#[derive(Debug, Clone)]
struct SkillArchive {
    release: ResolvedSkillRelease,
    bytes: Vec<u8>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct SkillState {
    schema_version: u32,
    installations: Vec<SkillInstallation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SkillInstallation {
    agent: String,
    path: PathBuf,
    binary_version: String,
    skill_version: String,
    channel: String,
    release_url: String,
    source_artifact: String,
    source_sha256: String,
    installed_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct InstalledSkillMetadata {
    schema_version: u32,
    name: String,
    managed_by: String,
    binary_version: String,
    skill_version: String,
    source: InstalledSkillSource,
    managed: InstalledSkillManaged,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct InstalledSkillSource {
    release_url: String,
    channel: String,
    artifact: String,
    sha256: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct InstalledSkillManaged {
    agent: String,
    install_path: PathBuf,
    installed_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReleaseMetadata {
    release_version: String,
    artifacts: ReleaseArtifacts,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReleaseArtifacts {
    skill_tar_gz: Option<ReleaseArtifact>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReleaseArtifact {
    name: String,
    url: String,
    sha256: String,
}

pub fn install(state: &AppState, options: SkillInstallOptions) -> Result<String, String> {
    let _lock = StateLock::acquire(&state.config.state_dir)?;
    let mut skill_state = read_state(&state.config.skills_state_path)?;
    let targets = install_targets(state, options.path.as_deref(), options.dry_run)?;
    if targets.is_empty() {
        return Err("no default agent skill directories were found; pass --path <dir> to install to an explicit final skill directory".to_string());
    }

    let release = resolve_release(
        state,
        &options.channel,
        options.version.as_deref(),
        options.release_url.as_deref(),
    )?;
    if options.dry_run {
        return Ok(render_install_plan(
            "install",
            &targets,
            &release,
            &skill_state,
        ));
    }
    let archive = download_skill_archive(release)?;
    let mut installed = Vec::new();
    for target in targets {
        install_one(state, &archive, &target, &mut skill_state)?;
        installed.push(target);
    }
    write_state(&state.config.skills_state_path, &skill_state)?;
    Ok(render_installed("installed", &installed, &archive.release))
}

pub fn upgrade(state: &AppState, options: SkillUpgradeOptions) -> Result<String, String> {
    let _lock = StateLock::acquire(&state.config.state_dir)?;
    let mut skill_state = read_state(&state.config.skills_state_path)?;
    if skill_state.installations.is_empty() {
        return Err(
            "no managed skill installations found; run `plane skill install` first".to_string(),
        );
    }

    let channel = options
        .channel
        .clone()
        .or_else(|| {
            skill_state
                .installations
                .first()
                .map(|item| item.channel.clone())
        })
        .unwrap_or_else(|| "stable".to_string());
    let release = resolve_release(
        state,
        &channel,
        options.version.as_deref(),
        options.release_url.as_deref(),
    )?;
    let targets = skill_state
        .installations
        .iter()
        .map(|item| InstallTarget {
            agent: item.agent.clone(),
            path: item.path.clone(),
        })
        .collect::<Vec<_>>();

    if options.dry_run {
        return Ok(render_install_plan(
            "upgrade",
            &targets,
            &release,
            &skill_state,
        ));
    }
    let archive = download_skill_archive(release)?;
    for target in &targets {
        install_one(state, &archive, target, &mut skill_state)?;
    }
    write_state(&state.config.skills_state_path, &skill_state)?;
    Ok(render_installed("upgraded", &targets, &archive.release))
}

pub fn uninstall(state: &AppState, options: SkillUninstallOptions) -> Result<String, String> {
    let _lock = StateLock::acquire(&state.config.state_dir)?;
    let mut skill_state = read_state(&state.config.skills_state_path)?;
    if skill_state.installations.is_empty() {
        return Err("no managed skill installations found".to_string());
    }

    let mut removed = Vec::new();
    let mut retained = Vec::new();
    for installation in skill_state.installations.iter() {
        if options.dry_run {
            removed.push(InstallTarget {
                agent: installation.agent.clone(),
                path: installation.path.clone(),
            });
            continue;
        }
        if !installation.path.exists() {
            warn!(path = %installation.path.display(), "managed skill path already missing");
            removed.push(InstallTarget {
                agent: installation.agent.clone(),
                path: installation.path.clone(),
            });
            continue;
        }
        ensure_existing_path_is_managed(&installation.path)?;
        info!(path = %installation.path.display(), "removing managed skill");
        fs::remove_dir_all(&installation.path).map_err(|error| {
            format!("failed to remove {}: {error}", installation.path.display())
        })?;
        removed.push(InstallTarget {
            agent: installation.agent.clone(),
            path: installation.path.clone(),
        });
    }

    if !options.dry_run {
        skill_state.installations.clear();
        skill_state.installations.append(&mut retained);
        write_state(&state.config.skills_state_path, &skill_state)?;
    }

    let verb = if options.dry_run {
        "would remove"
    } else {
        "removed"
    };
    Ok(render_targets(verb, &removed))
}

pub fn list(state: &AppState) -> Result<String, String> {
    let skill_state = read_state(&state.config.skills_state_path)?;
    if skill_state.installations.is_empty() {
        return Ok("no managed skill installations\n".to_string());
    }
    let mut output = String::from("managed skill installations\n");
    for item in skill_state.installations {
        output.push_str(&format!(
            "- {} {} at {} (binary {}, skill {})\n",
            item.agent,
            item.channel,
            item.path.display(),
            item.binary_version,
            item.skill_version
        ));
    }
    Ok(output)
}

fn install_targets(
    state: &AppState,
    path: Option<&Path>,
    dry_run: bool,
) -> Result<Vec<InstallTarget>, String> {
    if let Some(path) = path {
        let install_path = final_install_path(path, dry_run)?;
        return Ok(vec![InstallTarget {
            agent: "custom".to_string(),
            path: install_path,
        }]);
    }

    let Some(home) = state.config.user_home.as_deref() else {
        return Ok(Vec::new());
    };

    let mut targets = Vec::new();
    for candidate in default_agent_candidates(
        home,
        state.config.codex_home.as_deref(),
        state.config.codex_home_explicit,
    ) {
        match default_target_for_candidate(&candidate, dry_run)? {
            Some(target) => {
                if !targets
                    .iter()
                    .any(|existing: &InstallTarget| paths_equal(&existing.path, &target.path))
                {
                    targets.push(target);
                }
            }
            None => {
                debug!(
                    agent = candidate.agent,
                    path = %candidate.presence_dir.display(),
                    "agent home directory not found"
                );
            }
        }
    }
    Ok(targets)
}

fn default_agent_candidates(
    home: &Path,
    codex_home: Option<&Path>,
    codex_home_explicit: bool,
) -> Vec<DefaultAgentCandidate> {
    default_agent_candidates_for_home(home, codex_home.map(Path::to_path_buf), codex_home_explicit)
}

fn default_agent_candidates_for_home(
    home: &Path,
    codex_home_override: Option<PathBuf>,
    codex_home_explicit: bool,
) -> Vec<DefaultAgentCandidate> {
    let codex_home = codex_home_override
        .clone()
        .unwrap_or_else(|| home.join(".codex"));
    vec![
        DefaultAgentCandidate {
            agent: "claude-code",
            presence_dir: home.join(".claude"),
            skills_dir: home.join(".claude").join("skills"),
            allow_missing_presence: false,
        },
        DefaultAgentCandidate {
            agent: "codex",
            presence_dir: codex_home.clone(),
            skills_dir: codex_home.join("skills"),
            allow_missing_presence: codex_home_explicit,
        },
        DefaultAgentCandidate {
            agent: "codex",
            presence_dir: home.join(".agents").join("skills"),
            skills_dir: home.join(".agents").join("skills"),
            allow_missing_presence: false,
        },
        DefaultAgentCandidate {
            agent: "opencode",
            presence_dir: home.join(".config").join("opencode"),
            skills_dir: home.join(".config").join("opencode").join("skills"),
            allow_missing_presence: false,
        },
    ]
}

fn default_target_for_candidate(
    candidate: &DefaultAgentCandidate,
    dry_run: bool,
) -> Result<Option<InstallTarget>, String> {
    if !candidate.allow_missing_presence
        && !candidate.presence_dir.is_dir()
        && !candidate.skills_dir.is_dir()
    {
        return Ok(None);
    }

    let skills_dir = if candidate.skills_dir.is_dir() {
        candidate.skills_dir.canonicalize().map_err(|error| {
            format!(
                "failed to canonicalize {}: {error}",
                candidate.skills_dir.display()
            )
        })?
    } else if dry_run {
        absolute_path(&candidate.skills_dir)?
    } else {
        fs::create_dir_all(&candidate.skills_dir).map_err(|error| {
            format!(
                "failed to create {}: {error}",
                candidate.skills_dir.display()
            )
        })?;
        candidate.skills_dir.canonicalize().map_err(|error| {
            format!(
                "failed to canonicalize {}: {error}",
                candidate.skills_dir.display()
            )
        })?
    };

    Ok(Some(InstallTarget {
        agent: candidate.agent.to_string(),
        path: skills_dir.join(SKILL_NAME),
    }))
}

fn final_install_path(path: &Path, dry_run: bool) -> Result<PathBuf, String> {
    if path.exists() {
        return path
            .canonicalize()
            .map_err(|error| format!("failed to canonicalize {}: {error}", path.display()));
    }
    let absolute = absolute_path(path)?;
    let parent = absolute
        .parent()
        .ok_or_else(|| format!("install path has no parent: {}", absolute.display()))?;
    if !dry_run {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    }
    if parent.exists() {
        let parent = parent
            .canonicalize()
            .map_err(|error| format!("failed to canonicalize {}: {error}", parent.display()))?;
        let file_name = absolute.file_name().ok_or_else(|| {
            format!(
                "install path has no final component: {}",
                absolute.display()
            )
        })?;
        return Ok(parent.join(file_name));
    }
    Ok(absolute)
}

fn absolute_path(path: &Path) -> Result<PathBuf, String> {
    if path.is_absolute() {
        return Ok(path.to_path_buf());
    }
    std::env::current_dir()
        .map_err(|error| format!("failed to read current directory: {error}"))
        .map(|current_dir| current_dir.join(path))
}

fn resolve_release(
    state: &AppState,
    channel: &str,
    version: Option<&str>,
    release_url: Option<&str>,
) -> Result<ResolvedSkillRelease, String> {
    validate_channel(channel)?;
    let release_url = release_url
        .unwrap_or(&state.config.releases_public_url)
        .trim_end_matches('/')
        .to_string();
    let metadata_url = match version {
        Some(version) => format!(
            "{release_url}/{channel}/versions/{}/metadata.json",
            normalize_version(version)
        ),
        None => format!("{release_url}/{channel}/latest/metadata.json"),
    };
    info!(url = %metadata_url, "resolving skill release metadata");
    let metadata_bytes = download_url(&metadata_url)?;
    let metadata = serde_json::from_slice::<ReleaseMetadata>(&metadata_bytes)
        .map_err(|error| format!("release metadata is invalid JSON: {error}"))?;
    let artifact = metadata
        .artifacts
        .skill_tar_gz
        .ok_or_else(|| "release metadata does not include artifacts.skillTarGz".to_string())?;
    if artifact.name != "plane-cli.tar.gz" {
        return Err(format!("unexpected skill artifact name: {}", artifact.name));
    }
    Ok(ResolvedSkillRelease {
        release_url,
        channel: channel.to_string(),
        release_version: metadata.release_version,
        artifact_name: artifact.name,
        artifact_url: artifact.url,
        sha256: artifact.sha256,
    })
}

fn download_skill_archive(release: ResolvedSkillRelease) -> Result<SkillArchive, String> {
    info!(url = %release.artifact_url, "downloading skill artifact");
    let bytes = download_url(&release.artifact_url)?;
    let digest = format!("{:x}", Sha256::digest(&bytes));
    if digest != release.sha256 {
        return Err(format!(
            "skill artifact sha256 mismatch: expected {} got {}",
            release.sha256, digest
        ));
    }
    Ok(SkillArchive { release, bytes })
}

fn download_url(url: &str) -> Result<Vec<u8>, String> {
    let response = ureq::get(url)
        .set("User-Agent", USER_AGENT)
        .call()
        .map_err(|error| format!("failed to GET {url}: {error}"))?;
    let mut reader = response.into_reader();
    let mut bytes = Vec::new();
    reader
        .read_to_end(&mut bytes)
        .map_err(|error| format!("failed to read {url}: {error}"))?;
    Ok(bytes)
}

fn install_one(
    state: &AppState,
    archive: &SkillArchive,
    target: &InstallTarget,
    skill_state: &mut SkillState,
) -> Result<(), String> {
    validate_target_name(&target.path)?;
    let existing_index = skill_state
        .installations
        .iter()
        .position(|item| paths_equal(&item.path, &target.path));
    if target.path.exists() {
        if existing_index.is_none() {
            return Err(format!(
                "refusing to overwrite unmanaged skill path: {}",
                target.path.display()
            ));
        }
        ensure_existing_path_is_managed(&target.path)?;
    }

    let parent = target
        .path
        .parent()
        .ok_or_else(|| format!("install path has no parent: {}", target.path.display()))?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    let staging_root = parent.join(format!(".plane-cli-install-{}", std::process::id()));
    if staging_root.exists() {
        fs::remove_dir_all(&staging_root).map_err(|error| {
            format!(
                "failed to remove stale staging dir {}: {error}",
                staging_root.display()
            )
        })?;
    }
    fs::create_dir_all(&staging_root)
        .map_err(|error| format!("failed to create {}: {error}", staging_root.display()))?;

    let unpack_result =
        unpack_skill_archive(&archive.bytes, &staging_root).and_then(|skill_root| {
            write_install_metadata(state, archive, target, &skill_root).map(|_| skill_root)
        });
    let skill_root = match unpack_result {
        Ok(value) => value,
        Err(error) => {
            let _ = fs::remove_dir_all(&staging_root);
            return Err(error);
        }
    };

    if target.path.exists() {
        fs::remove_dir_all(&target.path)
            .map_err(|error| format!("failed to replace {}: {error}", target.path.display()))?;
    }
    info!(agent = %target.agent, path = %target.path.display(), "installing skill");
    fs::rename(&skill_root, &target.path).map_err(|error| {
        format!(
            "failed to move {} to {}: {error}",
            skill_root.display(),
            target.path.display()
        )
    })?;
    let _ = fs::remove_dir_all(&staging_root);

    let installed_at = now_rfc3339();
    let installation = SkillInstallation {
        agent: target.agent.clone(),
        path: target.path.clone(),
        binary_version: state.version.to_string(),
        skill_version: archive.release.release_version.clone(),
        channel: archive.release.channel.clone(),
        release_url: archive.release.release_url.clone(),
        source_artifact: archive.release.artifact_name.clone(),
        source_sha256: archive.release.sha256.clone(),
        installed_at,
    };
    match existing_index {
        Some(index) => skill_state.installations[index] = installation,
        None => skill_state.installations.push(installation),
    }
    Ok(())
}

fn unpack_skill_archive(bytes: &[u8], staging_root: &Path) -> Result<PathBuf, String> {
    let decoder = GzDecoder::new(Cursor::new(bytes));
    let mut archive = Archive::new(decoder);
    archive
        .unpack(staging_root)
        .map_err(|error| format!("failed to unpack skill archive: {error}"))?;
    let skill_root = staging_root.join(SKILL_NAME);
    if !skill_root.join("SKILL.md").is_file() {
        return Err("skill archive is missing plane-cli/SKILL.md".to_string());
    }
    Ok(skill_root)
}

fn write_install_metadata(
    state: &AppState,
    archive: &SkillArchive,
    target: &InstallTarget,
    skill_root: &Path,
) -> Result<(), String> {
    let installed_at = now_rfc3339();
    let metadata = InstalledSkillMetadata {
        schema_version: SKILL_METADATA_SCHEMA_VERSION,
        name: SKILL_NAME.to_string(),
        managed_by: MANAGED_BY.to_string(),
        binary_version: state.version.to_string(),
        skill_version: archive.release.release_version.clone(),
        source: InstalledSkillSource {
            release_url: archive.release.release_url.clone(),
            channel: archive.release.channel.clone(),
            artifact: archive.release.artifact_name.clone(),
            sha256: archive.release.sha256.clone(),
        },
        managed: InstalledSkillManaged {
            agent: target.agent.clone(),
            install_path: target.path.clone(),
            installed_at,
        },
    };
    let bytes = serde_json::to_vec_pretty(&metadata)
        .map_err(|error| format!("failed to encode skill metadata: {error}"))?;
    fs::write(
        skill_root.join("metadata.json"),
        [bytes, b"\n".to_vec()].concat(),
    )
    .map_err(|error| format!("failed to write skill metadata: {error}"))
}

fn ensure_existing_path_is_managed(path: &Path) -> Result<(), String> {
    let metadata_path = path.join("metadata.json");
    let bytes = fs::read(&metadata_path)
        .map_err(|error| format!("failed to read {}: {error}", metadata_path.display()))?;
    let metadata = serde_json::from_slice::<InstalledSkillMetadata>(&bytes)
        .map_err(|error| format!("{} is invalid JSON: {error}", metadata_path.display()))?;
    if metadata.name != SKILL_NAME || metadata.managed_by != MANAGED_BY {
        return Err(format!(
            "refusing to overwrite unmanaged skill path: {}",
            path.display()
        ));
    }
    Ok(())
}

fn read_state(path: &Path) -> Result<SkillState, String> {
    if !path.exists() {
        return Ok(SkillState {
            schema_version: STATE_SCHEMA_VERSION,
            installations: Vec::new(),
        });
    }
    let bytes =
        fs::read(path).map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    let mut state = serde_json::from_slice::<SkillState>(&bytes)
        .map_err(|error| format!("{} is invalid JSON: {error}", path.display()))?;
    if state.schema_version == 0 {
        state.schema_version = STATE_SCHEMA_VERSION;
    }
    if state.schema_version != STATE_SCHEMA_VERSION {
        return Err(format!(
            "unsupported skill state schema version: {}",
            state.schema_version
        ));
    }
    Ok(state)
}

fn write_state(path: &Path, state: &SkillState) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    }
    let tmp_path = path.with_extension("json.tmp");
    let bytes = serde_json::to_vec_pretty(state)
        .map_err(|error| format!("failed to encode skill state: {error}"))?;
    fs::write(&tmp_path, [bytes, b"\n".to_vec()].concat())
        .map_err(|error| format!("failed to write {}: {error}", tmp_path.display()))?;
    fs::rename(&tmp_path, path).map_err(|error| {
        format!(
            "failed to move {} to {}: {error}",
            tmp_path.display(),
            path.display()
        )
    })
}

fn render_install_plan(
    action: &str,
    targets: &[InstallTarget],
    release: &ResolvedSkillRelease,
    state: &SkillState,
) -> String {
    let mut output = format!(
        "would {action} plane-cli skill {} from {}\n",
        release.release_version, release.artifact_url
    );
    for target in targets {
        let managed = state
            .installations
            .iter()
            .any(|item| paths_equal(&item.path, &target.path));
        output.push_str(&format!(
            "- {} at {} ({})\n",
            target.agent,
            target.path.display(),
            if managed { "managed" } else { "new" }
        ));
    }
    output
}

fn render_installed(
    action: &str,
    targets: &[InstallTarget],
    release: &ResolvedSkillRelease,
) -> String {
    let mut output = format!("{action} plane-cli skill {}\n", release.release_version);
    for target in targets {
        output.push_str(&format!(
            "- {} at {}\n",
            target.agent,
            target.path.display()
        ));
    }
    output
}

fn render_targets(verb: &str, targets: &[InstallTarget]) -> String {
    let mut output = format!("{verb} plane-cli skill\n");
    for target in targets {
        output.push_str(&format!(
            "- {} at {}\n",
            target.agent,
            target.path.display()
        ));
    }
    output
}

fn validate_channel(channel: &str) -> Result<(), String> {
    match channel {
        "stable" | "beta" => Ok(()),
        _ => Err(format!("unsupported channel: {channel}")),
    }
}

fn normalize_version(value: &str) -> String {
    format!("v{}", value.trim_start_matches('v'))
}

fn validate_target_name(path: &Path) -> Result<(), String> {
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| format!("install path has no final component: {}", path.display()))?;
    if name != SKILL_NAME {
        return Err(format!(
            "--path must point to the final plane-cli skill directory, ending in {SKILL_NAME}"
        ));
    }
    Ok(())
}

fn paths_equal(left: &Path, right: &Path) -> bool {
    left == right
}

fn now_rfc3339() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true)
}

struct StateLock {
    path: PathBuf,
}

impl StateLock {
    fn acquire(state_dir: &Path) -> Result<Self, String> {
        fs::create_dir_all(state_dir)
            .map_err(|error| format!("failed to create {}: {error}", state_dir.display()))?;
        let path = state_dir.join("skills.lock");
        let started = Instant::now();
        loop {
            match OpenOptions::new().write(true).create_new(true).open(&path) {
                Ok(mut file) => {
                    writeln!(file, "pid={}", std::process::id()).ok();
                    return Ok(Self { path });
                }
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                    if started.elapsed() > Duration::from_secs(5) {
                        return Err(format!(
                            "timed out waiting for skill state lock: {}",
                            path.display()
                        ));
                    }
                    thread::sleep(Duration::from_millis(50));
                }
                Err(error) => {
                    return Err(format!("failed to create {}: {error}", path.display()));
                }
            }
        }
    }
}

impl Drop for StateLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn default_candidates_use_real_codex_home() {
        let home = PathBuf::from("/home/example");
        let candidates = default_agent_candidates_for_home(&home, None, false);

        assert!(candidates
            .iter()
            .any(|item| item.agent == "codex" && item.skills_dir == home.join(".codex/skills")));
        assert!(candidates.iter().any(|item| {
            item.agent == "claude-code" && item.skills_dir == home.join(".claude/skills")
        }));
    }

    #[test]
    fn default_candidates_honor_codex_home_override() {
        let home = PathBuf::from("/home/example");
        let codex_home = PathBuf::from("/tmp/custom-codex");
        let candidates = default_agent_candidates_for_home(&home, Some(codex_home.clone()), true);

        assert!(candidates.iter().any(|item| {
            item.agent == "codex"
                && item.skills_dir == codex_home.join("skills")
                && item.allow_missing_presence
        }));
    }

    #[test]
    fn default_target_creates_missing_skills_dir_when_agent_home_exists() {
        let root = unique_test_dir();
        let agent_home = root.join(".claude");
        let skills_dir = agent_home.join("skills");
        fs::create_dir_all(&agent_home).expect("create agent home");
        let candidate = DefaultAgentCandidate {
            agent: "claude-code",
            presence_dir: agent_home,
            skills_dir: skills_dir.clone(),
            allow_missing_presence: false,
        };

        let target = default_target_for_candidate(&candidate, false)
            .expect("candidate target")
            .expect("target");

        assert!(skills_dir.is_dir());
        assert_eq!(target.agent, "claude-code");
        assert_eq!(
            target.path,
            skills_dir
                .canonicalize()
                .expect("canonical skills dir")
                .join(SKILL_NAME)
        );
        let _ = fs::remove_dir_all(root);
    }

    fn unique_test_dir() -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        std::env::temp_dir().join(format!("plane-cli-test-{}-{nanos}", std::process::id()))
    }
}
