//! Work item comment commands.
//!
//! Comments are a work-item sub-resource
//! (`workspaces/{ws}/projects/{project}/work-items/{issue_id}/comments/`). They
//! reuse the generic sub-resource layer for list/get/delete, and add authoring
//! ergonomics for create/update: the body is written as Markdown (converted to
//! HTML) or raw HTML and sent as `comment_html`, mirroring `page`.
//!
//! `--work-item` accepts either a UUID or a human identifier such as `OPEND-7`.
//! Identifiers are resolved to a UUID via the workspace-level
//! `work-items/<project_identifier>-<issue_identifier>/` lookup, because the
//! comments endpoint is keyed by the work item's UUID.

use super::page::{self, BodyArgs};
use super::{generic, require_workspace};
use crate::core::app::AppState;
use crate::core::request::Client;
use serde_json::{json, Value};

pub struct ListOptions<'a> {
    pub project: &'a str,
    pub work_item: &'a str,
    pub all: bool,
    pub fields: Option<String>,
    pub expand: Option<String>,
    pub json: bool,
}

pub struct GetOptions<'a> {
    pub project: &'a str,
    pub work_item: &'a str,
    pub id: &'a str,
    pub fields: Option<String>,
    pub expand: Option<String>,
    pub json: bool,
}

pub struct CreateOptions<'a> {
    pub project: &'a str,
    pub work_item: &'a str,
    pub body: BodyArgs<'a>,
    pub data: Option<String>,
    pub dry_run: bool,
    pub json: bool,
}

pub struct UpdateOptions<'a> {
    pub project: &'a str,
    pub work_item: &'a str,
    pub id: &'a str,
    pub body: BodyArgs<'a>,
    pub data: Option<String>,
    pub dry_run: bool,
    pub json: bool,
}

pub struct DeleteOptions<'a> {
    pub project: &'a str,
    pub work_item: &'a str,
    pub id: &'a str,
    pub dry_run: bool,
}

/// Whether a string is a canonical 8-4-4-4-12 hex UUID.
fn looks_like_uuid(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() != 36 {
        return false;
    }
    bytes.iter().enumerate().all(|(index, byte)| {
        if matches!(index, 8 | 13 | 18 | 23) {
            *byte == b'-'
        } else {
            byte.is_ascii_hexdigit()
        }
    })
}

/// Resolve `--work-item` to the work item UUID the comments endpoint needs.
/// A UUID is used as-is; anything else is treated as a `PROJECT-N` identifier
/// and resolved through the workspace-level lookup.
fn resolve_work_item_id(
    state: &AppState,
    workspace: &str,
    work_item: &str,
) -> Result<String, String> {
    if looks_like_uuid(work_item) {
        return Ok(work_item.to_string());
    }
    let client = Client::from_state(state).map_err(|error| error.to_string())?;
    let path = format!("workspaces/{workspace}/work-items/{work_item}/");
    let value = client
        .get(&path, &[])
        .map_err(|error| format!("could not resolve work item '{work_item}': {error}"))?;
    value
        .get("id")
        .and_then(Value::as_str)
        .map(ToString::to_string)
        .ok_or_else(|| format!("work item '{work_item}' lookup returned no id"))
}

fn collection_path(workspace: &str, project: &str, issue_id: &str) -> String {
    format!("workspaces/{workspace}/projects/{project}/work-items/{issue_id}/comments/")
}

/// Merge the resolved comment HTML into the base `--data` object, returning a
/// JSON string for the generic write layer.
fn build_comment_data(
    data: &Option<String>,
    html: Option<String>,
) -> Result<Option<String>, String> {
    let mut object = match data {
        Some(raw) => serde_json::from_str::<Value>(raw)
            .map_err(|error| format!("--data is not valid JSON: {error}"))?,
        None => json!({}),
    };
    let map = object
        .as_object_mut()
        .ok_or_else(|| "--data must be a JSON object".to_string())?;
    if let Some(html) = html {
        map.insert("comment_html".to_string(), Value::String(html));
    }
    if map.is_empty() {
        return Ok(None);
    }
    serde_json::to_string(&object)
        .map(Some)
        .map_err(|error| error.to_string())
}

pub fn list(state: &AppState, options: ListOptions) -> Result<String, String> {
    let workspace = require_workspace(state)?;
    let issue_id = resolve_work_item_id(state, &workspace, options.work_item)?;
    let collection = collection_path(&workspace, options.project, &issue_id);
    generic::list(
        state,
        &collection,
        generic::ListOptions {
            all: options.all,
            fields: options.fields,
            expand: options.expand,
            json: options.json,
        },
    )
}

pub fn get(state: &AppState, options: GetOptions) -> Result<String, String> {
    let workspace = require_workspace(state)?;
    let issue_id = resolve_work_item_id(state, &workspace, options.work_item)?;
    let collection = collection_path(&workspace, options.project, &issue_id);
    generic::get(
        state,
        &collection,
        options.id,
        generic::GetOptions {
            fields: options.fields,
            expand: options.expand,
            json: options.json,
        },
    )
}

pub fn create(state: &AppState, options: CreateOptions) -> Result<String, String> {
    let workspace = require_workspace(state)?;
    let html = page::resolve_html(&options.body)?;
    let data = build_comment_data(&options.data, html)?;
    if data.is_none() {
        return Err("a comment body is required; pass --from-file, --body, or --data".to_string());
    }
    let issue_id = resolve_work_item_id(state, &workspace, options.work_item)?;
    let collection = collection_path(&workspace, options.project, &issue_id);
    generic::create(
        state,
        &collection,
        generic::WriteOptions {
            data,
            dry_run: options.dry_run,
            json: options.json,
        },
    )
}

pub fn update(state: &AppState, options: UpdateOptions) -> Result<String, String> {
    let workspace = require_workspace(state)?;
    let html = page::resolve_html(&options.body)?;
    let data = build_comment_data(&options.data, html)?;
    if data.is_none() {
        return Err("nothing to update; pass --from-file, --body, or --data".to_string());
    }
    let issue_id = resolve_work_item_id(state, &workspace, options.work_item)?;
    let collection = collection_path(&workspace, options.project, &issue_id);
    generic::update(
        state,
        &collection,
        options.id,
        generic::WriteOptions {
            data,
            dry_run: options.dry_run,
            json: options.json,
        },
    )
}

pub fn delete(state: &AppState, options: DeleteOptions) -> Result<String, String> {
    let workspace = require_workspace(state)?;
    let issue_id = resolve_work_item_id(state, &workspace, options.work_item)?;
    let collection = collection_path(&workspace, options.project, &issue_id);
    generic::delete(state, &collection, options.id, options.dry_run)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uuid_detection() {
        assert!(looks_like_uuid("11111111-1111-1111-1111-111111111111"));
        assert!(!looks_like_uuid("OPEND-7"));
        assert!(!looks_like_uuid("11111111-1111-1111-1111-11111111111"));
        assert!(!looks_like_uuid("zzzzzzzz-1111-1111-1111-111111111111"));
    }

    #[test]
    fn build_comment_data_sets_comment_html() {
        let out = build_comment_data(&None, Some("<p>hi</p>".to_string()))
            .unwrap()
            .unwrap();
        let value: Value = serde_json::from_str(&out).unwrap();
        assert_eq!(value["comment_html"], "<p>hi</p>");
    }

    #[test]
    fn build_comment_data_merges_over_data() {
        let out = build_comment_data(
            &Some(r#"{"access":1}"#.to_string()),
            Some("<p>hi</p>".to_string()),
        )
        .unwrap()
        .unwrap();
        let value: Value = serde_json::from_str(&out).unwrap();
        assert_eq!(value["access"], 1);
        assert_eq!(value["comment_html"], "<p>hi</p>");
    }

    #[test]
    fn build_comment_data_empty_is_none() {
        assert!(build_comment_data(&None, None).unwrap().is_none());
    }
}
