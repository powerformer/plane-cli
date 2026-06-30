//! Best-effort "newer plane is available" check.
//!
//! The binary itself is installed and upgraded by the manager (`manage.sh`), not
//! by this CLI, so the most we do is *notice* a newer release and print the
//! `manage.sh upgrade` command. We never download or replace the binary here.
//!
//! Two entry points:
//! - [`passive_notice`]: throttled, opt-out-aware background-style check run after
//!   normal commands; returns a short stderr notice (or `None`). Never errors.
//! - [`run_check`]: the explicit `plane upgrade` command; always fetches and
//!   reports, surfacing network errors to the user.

use crate::core::app::AppState;
use crate::core::skill::infer_channel;
use serde::{Deserialize, Serialize};
use std::io::{IsTerminal, Read};
use std::path::{Path, PathBuf};
use std::time::Duration;

const USER_AGENT: &str = "plane-cli";
const CACHE_FILE: &str = "update-check.json";
/// Check the network at most this often during passive checks.
const CHECK_INTERVAL_SECS: i64 = 24 * 60 * 60;
const HTTP_TIMEOUT: Duration = Duration::from_secs(2);

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdateCache {
    #[serde(default)]
    last_checked_unix: i64,
    #[serde(default)]
    latest_version: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LatestMetadata {
    release_version: String,
}

/// Throttled, opt-out-aware check for use after a normal command. Returns the
/// stderr notice to show, or `None`. Best-effort: any error is swallowed.
pub fn passive_notice(state: &AppState) -> Option<String> {
    if !passive_enabled() {
        return None;
    }
    let channel = infer_channel(state.version);
    let mut cache = read_cache(&state.config.state_dir);
    let now = now_unix();
    if now.saturating_sub(cache.last_checked_unix) >= CHECK_INTERVAL_SECS {
        // Record the attempt regardless of outcome so an offline machine does
        // not re-hit the network on every command.
        cache.last_checked_unix = now;
        if let Ok(latest) = fetch_latest(&state.config.releases_public_url, &channel) {
            cache.latest_version = Some(latest);
        }
        let _ = write_cache(&state.config.state_dir, &cache);
    }
    let latest = cache.latest_version.as_deref()?;
    if is_newer(state.version, latest) {
        Some(format_notice(
            state.version,
            latest,
            &state.config.releases_public_url,
            &channel,
        ))
    } else {
        None
    }
}

/// Explicit `plane upgrade`: always fetch the channel's latest and report. The
/// command only prints how to upgrade; it does not modify anything.
pub fn run_check(state: &AppState) -> Result<String, String> {
    let channel = infer_channel(state.version);
    let latest = fetch_latest(&state.config.releases_public_url, &channel)?;
    let _ = write_cache(
        &state.config.state_dir,
        &UpdateCache {
            last_checked_unix: now_unix(),
            latest_version: Some(latest.clone()),
        },
    );
    let current = state.version;
    if is_newer(current, &latest) {
        Ok(format!(
            "plane {current} (latest {channel}: {latest})\n\nA newer release is available. Upgrade with:\n  {unix}\n\nWindows PowerShell:\n  {ps}\n",
            unix = upgrade_command_unix(&state.config.releases_public_url, &channel),
            ps = upgrade_command_powershell(&state.config.releases_public_url, &channel),
        ))
    } else {
        Ok(format!(
            "plane {current} is up to date (latest {channel}: {latest}).\n"
        ))
    }
}

/// Passive checks only run for an interactive human: a TTY, not CI, not opted
/// out.
fn passive_enabled() -> bool {
    if truthy_env("PLANE_NO_UPDATE_CHECK") {
        return false;
    }
    if std::env::var_os("CI").is_some_and(|value| !value.is_empty()) {
        return false;
    }
    std::io::stderr().is_terminal()
}

fn truthy_env(name: &str) -> bool {
    match std::env::var(name) {
        Ok(value) => {
            let value = value.trim().to_ascii_lowercase();
            !(value.is_empty() || value == "0" || value == "false")
        }
        Err(_) => false,
    }
}

fn fetch_latest(releases_public_url: &str, channel: &str) -> Result<String, String> {
    let url = format!(
        "{}/{channel}/latest/metadata.json",
        releases_public_url.trim_end_matches('/')
    );
    let bytes = http_get(&url)?;
    let metadata: LatestMetadata = serde_json::from_slice(&bytes)
        .map_err(|error| format!("release metadata is invalid JSON: {error}"))?;
    Ok(metadata.release_version)
}

fn http_get(url: &str) -> Result<Vec<u8>, String> {
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(HTTP_TIMEOUT)
        .timeout_read(HTTP_TIMEOUT)
        .build();
    let response = agent
        .get(url)
        .set("User-Agent", USER_AGENT)
        .call()
        .map_err(|error| format!("failed to GET {url}: {error}"))?;
    let mut bytes = Vec::new();
    response
        .into_reader()
        .read_to_end(&mut bytes)
        .map_err(|error| format!("failed to read {url}: {error}"))?;
    Ok(bytes)
}

/// True when `latest` is a strictly higher semver than `current`. Unparsable
/// versions (e.g. a `0.1.0` dev build vs nothing) compare as "not newer".
fn is_newer(current: &str, latest: &str) -> bool {
    match (parse_version(current), parse_version(latest)) {
        (Some(current), Some(latest)) => latest > current,
        _ => false,
    }
}

fn parse_version(value: &str) -> Option<semver::Version> {
    semver::Version::parse(value.trim().trim_start_matches('v')).ok()
}

fn format_notice(current: &str, latest: &str, releases_public_url: &str, channel: &str) -> String {
    format!(
        "\nplane: a newer version is available: {latest} (current {current})\n       upgrade: {cmd}\n",
        cmd = upgrade_command_unix(releases_public_url, channel),
    )
}

fn upgrade_command_unix(releases_public_url: &str, channel: &str) -> String {
    format!(
        "curl -fsSL {}/manage.sh | sh -s -- upgrade --channel {channel}",
        releases_public_url.trim_end_matches('/')
    )
}

fn upgrade_command_powershell(releases_public_url: &str, channel: &str) -> String {
    let url = releases_public_url.trim_end_matches('/');
    format!(
        "$m = Join-Path $env:TEMP \"plane-manage.ps1\"; iwr {url}/manage.ps1 -OutFile $m; pwsh -File $m upgrade --channel {channel}"
    )
}

fn cache_path(state_dir: &Path) -> PathBuf {
    state_dir.join(CACHE_FILE)
}

fn read_cache(state_dir: &Path) -> UpdateCache {
    match std::fs::read(cache_path(state_dir)) {
        Ok(bytes) => serde_json::from_slice(&bytes).unwrap_or_default(),
        Err(_) => UpdateCache::default(),
    }
}

fn write_cache(state_dir: &Path, cache: &UpdateCache) -> Result<(), String> {
    std::fs::create_dir_all(state_dir).map_err(|error| error.to_string())?;
    let json = serde_json::to_vec_pretty(cache).map_err(|error| error.to_string())?;
    std::fs::write(cache_path(state_dir), json).map_err(|error| error.to_string())
}

fn now_unix() -> i64 {
    chrono::Utc::now().timestamp()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_newer_compares_beta_numbers() {
        assert!(is_newer("v0.1.0-beta.13", "v0.1.0-beta.14"));
        assert!(!is_newer("v0.1.0-beta.14", "v0.1.0-beta.13"));
        assert!(!is_newer("v0.1.0-beta.13", "v0.1.0-beta.13"));
    }

    #[test]
    fn is_newer_handles_v_prefix_and_base_bump() {
        assert!(is_newer("0.1.0-beta.99", "0.1.0"));
        assert!(is_newer("v0.1.0", "v0.2.0"));
        assert!(!is_newer("v0.2.0", "v0.1.0"));
    }

    #[test]
    fn is_newer_is_false_for_unparsable_versions() {
        assert!(!is_newer("not-a-version", "v0.1.0"));
        assert!(!is_newer("v0.1.0", "garbage"));
    }

    #[test]
    fn notice_mentions_latest_and_upgrade_command() {
        let notice = format_notice(
            "v0.1.0-beta.13",
            "v0.1.0-beta.14",
            "https://releases.plane.powerformer.net",
            "beta",
        );
        assert!(notice.contains("v0.1.0-beta.14"));
        assert!(notice.contains("current v0.1.0-beta.13"));
        assert!(notice.contains(
            "curl -fsSL https://releases.plane.powerformer.net/manage.sh | sh -s -- upgrade --channel beta"
        ));
    }

    #[test]
    fn upgrade_command_trims_trailing_slash() {
        assert_eq!(
            upgrade_command_unix("https://releases.plane.powerformer.net/", "stable"),
            "curl -fsSL https://releases.plane.powerformer.net/manage.sh | sh -s -- upgrade --channel stable"
        );
    }

    #[test]
    fn cache_roundtrips_through_state_dir() {
        let dir = std::env::temp_dir().join(format!("plane-update-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(read_cache(&dir).last_checked_unix, 0);
        write_cache(
            &dir,
            &UpdateCache {
                last_checked_unix: 42,
                latest_version: Some("v0.1.0-beta.14".to_string()),
            },
        )
        .expect("write cache");
        let cache = read_cache(&dir);
        assert_eq!(cache.last_checked_unix, 42);
        assert_eq!(cache.latest_version.as_deref(), Some("v0.1.0-beta.14"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
