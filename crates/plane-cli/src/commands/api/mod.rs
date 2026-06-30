pub mod crud;
pub mod generic;
pub mod me;
pub mod project;
pub mod request;
pub mod work_item;

pub use me::ApiMeOptions;

use crate::core::app::AppState;
use crate::core::model::common::Paginated;
use crate::core::request::Client;
use serde_json::{json, Value};

/// Resolve the workspace slug (config default or --workspace) for
/// workspace-scoped commands.
pub(crate) fn require_workspace(state: &AppState) -> Result<String, String> {
    state
        .config
        .workspace_slug
        .as_deref()
        .map(str::trim)
        .filter(|slug| !slug.is_empty())
        .map(ToString::to_string)
        .ok_or_else(|| {
            "workspace is required; set --workspace, workspace_slug in plane.toml, or PLANE_WORKSPACE_SLUG"
                .to_string()
        })
}

/// Render any JSON value as the pretty-printed `--json` output.
pub(crate) fn render_json(value: &Value) -> Result<String, String> {
    serde_json::to_string_pretty(value)
        .map(|json| format!("{json}\n"))
        .map_err(|error| error.to_string())
}

/// Pretty-print a JSON value without a trailing newline (for dry-run bodies).
pub(crate) fn pretty_json(value: &Value) -> Result<String, String> {
    serde_json::to_string_pretty(value).map_err(|error| error.to_string())
}

/// Build the `fields`/`expand` query pairs shared by list/get.
pub(crate) fn query_pairs(
    fields: &Option<String>,
    expand: &Option<String>,
) -> Vec<(String, String)> {
    let mut pairs = Vec::new();
    if let Some(fields) = fields {
        pairs.push(("fields".to_string(), fields.clone()));
    }
    if let Some(expand) = expand {
        pairs.push(("expand".to_string(), expand.clone()));
    }
    pairs
}

/// Borrow owned query pairs as the `&[(&str, &str)]` the client expects.
pub(crate) fn as_query_refs(pairs: &[(String, String)]) -> Vec<(&str, &str)> {
    pairs
        .iter()
        .map(|(k, v)| (k.as_str(), v.as_str()))
        .collect()
}

/// Parse the optional `--data` JSON, requiring an object; defaults to `{}`.
pub(crate) fn parse_data_object(data: &Option<String>) -> Result<Value, String> {
    let value = match data {
        Some(raw) => serde_json::from_str::<Value>(raw)
            .map_err(|error| format!("--data is not valid JSON: {error}"))?,
        None => json!({}),
    };
    if !value.is_object() {
        return Err("--data must be a JSON object".to_string());
    }
    Ok(value)
}

/// GET a list endpoint and collect results, optionally following cursor pages.
pub(crate) fn collect_list<T: serde::de::DeserializeOwned>(
    client: &Client,
    path: &str,
    base: &[(String, String)],
    all: bool,
) -> Result<Vec<T>, String> {
    let mut out = Vec::new();
    let mut cursor: Option<String> = None;
    loop {
        let mut pairs = as_query_refs(base);
        if let Some(cursor) = &cursor {
            pairs.push(("cursor", cursor.as_str()));
        }
        let value = client
            .get(path, &pairs)
            .map_err(|error| error.to_string())?;
        // Some endpoints (e.g. project members) return a bare JSON array instead
        // of a cursor-paginated envelope; accept both.
        if value.is_array() {
            let items: Vec<T> = serde_json::from_value(value)
                .map_err(|error| format!("failed to parse list: {error}"))?;
            out.extend(items);
            break;
        }
        let page: Paginated<T> = serde_json::from_value(value)
            .map_err(|error| format!("failed to parse list: {error}"))?;
        out.extend(page.results);
        match (all, page.next_page_results, page.next_cursor) {
            (true, true, Some(next)) => cursor = Some(next),
            _ => break,
        }
    }
    Ok(out)
}
