//! Human-readable reference resolution shared by the API commands.
//!
//! Anywhere a command accepts a project or work-item UUID it also accepts the
//! form users copy out of Plane: a project identifier (e.g. `OPEND`) or a
//! work-item `<KEY>-<SEQ>` (e.g. `OPEND-372`). UUID-shaped input passes
//! through untouched, so scripted UUID flows never pay an extra request and
//! `--dry-run` with UUIDs stays offline.

use super::{collect_list, workspace_client};
use crate::core::app::AppState;
use crate::core::error::ApiError;
use serde_json::Value;

/// A work-item reference resolved to the concrete UUIDs API paths need.
pub struct ResolvedWorkItem {
    pub id: String,
    pub project: String,
}

/// True when `value` is shaped like a canonical hyphenated UUID
/// (8-4-4-4-12 hex groups).
pub fn is_uuid(value: &str) -> bool {
    let groups: Vec<&str> = value.split('-').collect();
    groups.len() == 5
        && [8usize, 4, 4, 4, 12]
            .iter()
            .zip(&groups)
            .all(|(len, group)| group.len() == *len && group.chars().all(|c| c.is_ascii_hexdigit()))
}

/// Parse `<KEY>-<SEQ>` (e.g. `OPEND-372`) into an uppercased project key and
/// numeric sequence. Returns None when the shape doesn't match.
pub fn parse_work_item_ref(reference: &str) -> Option<(String, String)> {
    let (key, seq) = reference.rsplit_once('-')?;
    let key = key.trim();
    let seq = seq.trim();
    if key.is_empty() || !key.chars().all(|c| c.is_ascii_alphanumeric()) {
        return None;
    }
    if seq.is_empty() || !seq.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    Some((key.to_ascii_uppercase(), seq.to_string()))
}

/// Fetch a work item through the workspace-scoped by-identifier endpoint and
/// return the UUIDs API paths need.
pub fn fetch_work_item_by_identifier(
    state: &AppState,
    key: &str,
    seq: &str,
) -> Result<ResolvedWorkItem, String> {
    let (workspace, client) = workspace_client(state)?;
    let item = client
        .get(
            &format!("workspaces/{workspace}/work-items/{key}-{seq}/"),
            &[],
        )
        .map_err(|error| match error {
            // The backend answers 403 (not 404) for a key that does not exist,
            // so both read as "not there" to the caller.
            ApiError::Http {
                status: 404 | 403, ..
            } => {
                format!(
                    "work item {key}-{seq} not found or not accessible in workspace {workspace}"
                )
            }
            other => other.to_string(),
        })?;
    let id = item
        .get("id")
        .and_then(Value::as_str)
        .ok_or_else(|| format!("work item {key}-{seq} response is missing id"))?;
    let project = item
        .get("project")
        .and_then(Value::as_str)
        .ok_or_else(|| format!("work item {key}-{seq} response is missing project"))?;
    Ok(ResolvedWorkItem {
        id: id.to_string(),
        project: project.to_string(),
    })
}

/// Resolve a work-item reference (UUID or `<KEY>-<SEQ>`) plus an optional
/// project reference into concrete UUIDs. A `<KEY>-<SEQ>` reference carries
/// its own project, so `--project` may be omitted; when both are given the
/// item's real project wins.
pub fn resolve_work_item(
    state: &AppState,
    project: Option<&str>,
    reference: &str,
) -> Result<ResolvedWorkItem, String> {
    let reference = reference.trim();
    if is_uuid(reference) {
        let project = project.ok_or_else(|| {
            "--project is required when the work item is a UUID; a <KEY>-<SEQ> identifier (e.g. OPEND-372) works without it"
                .to_string()
        })?;
        return Ok(ResolvedWorkItem {
            id: reference.to_string(),
            project: resolve_project(state, project)?,
        });
    }
    let (key, seq) = parse_work_item_ref(reference).ok_or_else(|| {
        format!(
            "work item must be a UUID or <KEY>-<SEQ> identifier, e.g. OPEND-372 (got `{reference}`)"
        )
    })?;
    fetch_work_item_by_identifier(state, &key, &seq)
}

/// Resolve a project reference to its UUID: UUID-shaped input passes through;
/// anything else is matched case-insensitively against the workspace's
/// project identifiers.
pub fn resolve_project(state: &AppState, reference: &str) -> Result<String, String> {
    let reference = reference.trim();
    if is_uuid(reference) {
        return Ok(reference.to_string());
    }
    let (workspace, client) = workspace_client(state)?;
    let projects: Vec<Value> = collect_list(
        &client,
        &format!("workspaces/{workspace}/projects/"),
        &[],
        true,
    )?;
    for project in &projects {
        let matched = project
            .get("identifier")
            .and_then(Value::as_str)
            .is_some_and(|identifier| identifier.eq_ignore_ascii_case(reference));
        if matched {
            if let Some(id) = project.get("id").and_then(Value::as_str) {
                return Ok(id.to_string());
            }
        }
    }
    Err(format!(
        "no project with identifier `{}` in workspace {workspace}",
        reference.to_ascii_uppercase()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_uuid_accepts_canonical_uuids() {
        assert!(is_uuid("e456060b-0433-4ceb-b62b-19cfa09651cb"));
        assert!(is_uuid("49832A02-3158-4FAF-BF2F-D0E39C40C7E6"));
    }

    #[test]
    fn is_uuid_rejects_non_uuid_shapes() {
        assert!(!is_uuid("OPEND-372"));
        assert!(!is_uuid("OPEND"));
        assert!(!is_uuid(""));
        assert!(!is_uuid("e456060b-0433-4ceb-b62b")); // missing group
        assert!(!is_uuid("g456060b-0433-4ceb-b62b-19cfa09651cb")); // non-hex
    }

    #[test]
    fn parse_work_item_ref_uppercases_key_and_keeps_seq() {
        assert_eq!(
            parse_work_item_ref("opend-372"),
            Some(("OPEND".to_string(), "372".to_string()))
        );
        assert_eq!(
            parse_work_item_ref("PLANE-12"),
            Some(("PLANE".to_string(), "12".to_string()))
        );
    }

    #[test]
    fn parse_work_item_ref_rejects_bad_shapes() {
        assert_eq!(parse_work_item_ref("PLANECLI"), None); // no seq
        assert_eq!(parse_work_item_ref("PLANECLI-"), None); // empty seq
        assert_eq!(parse_work_item_ref("PLANECLI-x"), None); // non-numeric seq
        assert_eq!(parse_work_item_ref("-8"), None); // empty key
                                                     // a UUID never parses as KEY-SEQ (the key keeps its hyphens)
        assert_eq!(
            parse_work_item_ref("e456060b-0433-4ceb-b62b-19cfa09651cb"),
            None
        );
    }
}
