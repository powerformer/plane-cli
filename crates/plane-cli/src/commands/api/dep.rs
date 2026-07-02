//! `plane dep` — a label-backed cross-project work-item dependency surface.
//!
//! Plane's native relations do not cross projects, so cross-project dependency
//! edges are stored as labels named `dep:<KEY>:<SEQ>` on the *dependent* item
//! (e.g. an item carrying `dep:PLANE:5` is blocked by PLANE-5). Labels are the
//! durable, queryable edge store; this module is the CRUD + query layer over
//! them. DAG diagnosis (`doctor`) is tracked separately.

use super::{collect_list, render_json, require_workspace};
use crate::core::app::AppState;
use crate::core::error::ApiError;
use crate::core::request::Client;
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};

const DEP_PREFIX: &str = "dep:";
/// Neutral color so dep edges read differently from category labels.
const DEP_LABEL_COLOR: &str = "#6b7280";

/// A resolved edge: the target reference `KEY-SEQ` and its name (None = dangling).
type DepEdge = (String, Option<String>);
/// One item's dependency row: its `IDENT-SEQ` and its outgoing edges.
type DepRow = (String, Vec<DepEdge>);

pub struct AddOptions {
    pub project: String,
    pub work_item: String,
    pub on: String,
    pub dry_run: bool,
}

pub struct RmOptions {
    pub project: String,
    pub work_item: String,
    pub on: String,
}

pub struct LsOptions {
    pub project: String,
    pub work_item: Option<String>,
    pub json: bool,
}

pub struct GcOptions {
    pub project: String,
    pub write: bool,
}

/// Add an edge: validate the target exists (forward references are rejected),
/// ensure the `dep:<KEY>:<SEQ>` label exists, and attach it to the item.
pub fn add(state: &AppState, mut options: AddOptions) -> Result<String, String> {
    options.project = super::reference::resolve_project(state, &options.project)?;
    let workspace = require_workspace(state)?;
    let (key, seq) = parse_on(&options.on)?;
    let client = Client::from_state(state).map_err(|error| error.to_string())?;

    let target = resolve_target(&client, &workspace, &key, &seq)?
        .ok_or_else(|| format!("dependency target {key}-{seq} does not exist"))?;
    let target_name = field_str(&target, "name").unwrap_or("").to_string();
    let label_name = dep_label_name(&key, &seq);

    if options.dry_run {
        return Ok(format!(
            "DRY RUN add dep {} -> {key}-{seq} ({target_name}); label {label_name}\n",
            options.work_item
        ));
    }

    let labels = list_labels(&client, &workspace, &options.project)?;
    let label_id = match find_label(&labels, &label_name) {
        Some(label) => field_str(label, "id")
            .ok_or_else(|| "label is missing id".to_string())?
            .to_string(),
        None => create_dep_label(&client, &workspace, &options.project, &label_name)?,
    };

    let item = get_work_item(&client, &workspace, &options.project, &options.work_item)?;
    let mut ids = label_ids(&item);
    if ids.iter().any(|id| id == &label_id) {
        return Ok(format!(
            "{} already depends on {key}-{seq}\n",
            options.work_item
        ));
    }
    ids.push(label_id);
    set_labels(
        &client,
        &workspace,
        &options.project,
        &options.work_item,
        &ids,
    )?;
    Ok(format!(
        "added dep {} -> {key}-{seq} ({target_name})\n",
        options.work_item
    ))
}

/// Remove an edge: detach the `dep:<KEY>:<SEQ>` label from the item. The label
/// object is kept; `gc` prunes it if it becomes orphaned.
pub fn rm(state: &AppState, mut options: RmOptions) -> Result<String, String> {
    options.project = super::reference::resolve_project(state, &options.project)?;
    let workspace = require_workspace(state)?;
    let (key, seq) = parse_on(&options.on)?;
    let label_name = dep_label_name(&key, &seq);
    let client = Client::from_state(state).map_err(|error| error.to_string())?;

    let labels = list_labels(&client, &workspace, &options.project)?;
    let label_id = match find_label(&labels, &label_name) {
        Some(label) => field_str(label, "id")
            .ok_or_else(|| "label is missing id".to_string())?
            .to_string(),
        None => {
            return Ok(format!(
                "no dep label {label_name} in project; nothing to remove\n"
            ))
        }
    };

    let item = get_work_item(&client, &workspace, &options.project, &options.work_item)?;
    let mut ids = label_ids(&item);
    let before = ids.len();
    ids.retain(|id| id != &label_id);
    if ids.len() == before {
        return Ok(format!(
            "{} does not depend on {key}-{seq}\n",
            options.work_item
        ));
    }
    set_labels(
        &client,
        &workspace,
        &options.project,
        &options.work_item,
        &ids,
    )?;
    Ok(format!(
        "removed dep {} -> {key}-{seq} (label kept; run `plane dep gc` to prune)\n",
        options.work_item
    ))
}

/// List edges for one item (or every item in the project) and resolve targets.
pub fn ls(state: &AppState, mut options: LsOptions) -> Result<String, String> {
    options.project = super::reference::resolve_project(state, &options.project)?;
    let workspace = require_workspace(state)?;
    let client = Client::from_state(state).map_err(|error| error.to_string())?;

    let labels = list_labels(&client, &workspace, &options.project)?;
    let id_to_name: HashMap<String, String> = labels
        .iter()
        .filter_map(|label| {
            Some((
                field_str(label, "id")?.to_string(),
                field_str(label, "name")?.to_string(),
            ))
        })
        .collect();
    let identifier = project_identifier(&client, &workspace, &options.project)?;

    let items = match &options.work_item {
        Some(id) => vec![get_work_item(&client, &workspace, &options.project, id)?],
        None => list_work_items(&client, &workspace, &options.project)?,
    };

    let mut resolved: HashMap<String, Option<String>> = HashMap::new();
    let mut rows: Vec<DepRow> = Vec::new();
    for item in &items {
        let source = format!(
            "{identifier}-{}",
            item.get("sequence_id").and_then(Value::as_i64).unwrap_or(0)
        );
        let mut edges = Vec::new();
        for label_id in label_ids(item) {
            let Some(name) = id_to_name.get(&label_id) else {
                continue;
            };
            let Some((key, seq)) = parse_dep_label(name) else {
                continue;
            };
            let reference = format!("{key}-{seq}");
            if !resolved.contains_key(&reference) {
                let target = resolve_target(&client, &workspace, &key, &seq)?;
                let name = target
                    .as_ref()
                    .and_then(|t| field_str(t, "name").map(String::from));
                resolved.insert(reference.clone(), name);
            }
            edges.push((
                reference.clone(),
                resolved.get(&reference).cloned().flatten(),
            ));
        }
        if !edges.is_empty() {
            rows.push((source, edges));
        }
    }

    if options.json {
        let value = json!(rows
            .iter()
            .map(|(source, edges)| json!({
                "work_item": source,
                "deps": edges.iter().map(|(reference, name)| json!({
                    "ref": reference,
                    "target": name,
                    "dangling": name.is_none(),
                })).collect::<Vec<_>>(),
            }))
            .collect::<Vec<_>>());
        return render_json(&value);
    }

    if rows.is_empty() {
        return Ok("no dependencies\n".to_string());
    }
    let mut out = String::new();
    for (source, edges) in rows {
        out.push_str(&format!("{source}\n"));
        for (reference, name) in edges {
            match name {
                Some(name) => out.push_str(&format!("  -> {reference}  {name}\n")),
                None => out.push_str(&format!("  -> {reference}  (dangling)\n")),
            }
        }
    }
    Ok(out)
}

/// Delete `dep:*` labels that no item carries. Dry run unless `--write`.
pub fn gc(state: &AppState, mut options: GcOptions) -> Result<String, String> {
    options.project = super::reference::resolve_project(state, &options.project)?;
    let workspace = require_workspace(state)?;
    let client = Client::from_state(state).map_err(|error| error.to_string())?;

    let labels = list_labels(&client, &workspace, &options.project)?;
    let dep_labels: Vec<(String, String)> = labels
        .iter()
        .filter_map(|label| {
            let name = field_str(label, "name")?;
            let id = field_str(label, "id")?;
            name.starts_with(DEP_PREFIX)
                .then(|| (id.to_string(), name.to_string()))
        })
        .collect();
    if dep_labels.is_empty() {
        return Ok("no dep labels in project\n".to_string());
    }

    let items = list_work_items(&client, &workspace, &options.project)?;
    let used: HashSet<String> = items.iter().flat_map(label_ids).collect();
    let orphans: Vec<(String, String)> = dep_labels
        .into_iter()
        .filter(|(id, _)| !used.contains(id))
        .collect();

    if orphans.is_empty() {
        return Ok("no orphan dep labels\n".to_string());
    }
    if !options.write {
        let mut out = String::from("orphan dep labels (dry run; pass --write to delete):\n");
        for (_, name) in &orphans {
            out.push_str(&format!("  {name}\n"));
        }
        return Ok(out);
    }
    let mut out = String::new();
    for (id, name) in &orphans {
        delete_label(&client, &workspace, &options.project, id)?;
        out.push_str(&format!("deleted {name}\n"));
    }
    Ok(out)
}

/// Parse `--on <KEY>:<SEQ>` into an uppercased project key and numeric sequence.
fn parse_on(on: &str) -> Result<(String, String), String> {
    let (key, seq) = on
        .split_once(':')
        .ok_or_else(|| format!("--on must be <project>:<seq>, e.g. PLANE:5 (got `{on}`)"))?;
    let key = key.trim();
    let seq = seq.trim();
    if key.is_empty() || !key.chars().all(|c| c.is_ascii_alphanumeric()) {
        return Err(format!("invalid project key in --on: `{on}`"));
    }
    if seq.is_empty() || !seq.chars().all(|c| c.is_ascii_digit()) {
        return Err(format!("sequence in --on must be a number: `{on}`"));
    }
    Ok((key.to_ascii_uppercase(), seq.to_string()))
}

fn dep_label_name(key: &str, seq: &str) -> String {
    format!("{DEP_PREFIX}{key}:{seq}")
}

fn parse_dep_label(name: &str) -> Option<(String, String)> {
    let (key, seq) = name.strip_prefix(DEP_PREFIX)?.split_once(':')?;
    if key.is_empty() || seq.is_empty() {
        return None;
    }
    Some((key.to_string(), seq.to_string()))
}

fn resolve_target(
    client: &Client,
    workspace: &str,
    key: &str,
    seq: &str,
) -> Result<Option<Value>, String> {
    match client.get(
        &format!("workspaces/{workspace}/work-items/{key}-{seq}/"),
        &[],
    ) {
        Ok(value) => Ok(Some(value)),
        Err(ApiError::Http { status: 404, .. }) => Ok(None),
        Err(error) => Err(error.to_string()),
    }
}

fn get_work_item(
    client: &Client,
    workspace: &str,
    project: &str,
    id: &str,
) -> Result<Value, String> {
    client
        .get(
            &format!("workspaces/{workspace}/projects/{project}/work-items/{id}/"),
            &[],
        )
        .map_err(|error| error.to_string())
}

fn list_labels(client: &Client, workspace: &str, project: &str) -> Result<Vec<Value>, String> {
    collect_list::<Value>(
        client,
        &format!("workspaces/{workspace}/projects/{project}/labels/"),
        &[],
        true,
    )
}

fn list_work_items(client: &Client, workspace: &str, project: &str) -> Result<Vec<Value>, String> {
    collect_list::<Value>(
        client,
        &format!("workspaces/{workspace}/projects/{project}/work-items/"),
        &[],
        true,
    )
}

fn create_dep_label(
    client: &Client,
    workspace: &str,
    project: &str,
    name: &str,
) -> Result<String, String> {
    let created = client
        .post(
            &format!("workspaces/{workspace}/projects/{project}/labels/"),
            &json!({ "name": name, "color": DEP_LABEL_COLOR }),
        )
        .map_err(|error| error.to_string())?;
    field_str(&created, "id")
        .map(ToString::to_string)
        .ok_or_else(|| "label create response missing id".to_string())
}

fn set_labels(
    client: &Client,
    workspace: &str,
    project: &str,
    id: &str,
    label_ids: &[String],
) -> Result<(), String> {
    client
        .patch(
            &format!("workspaces/{workspace}/projects/{project}/work-items/{id}/"),
            &json!({ "labels": label_ids }),
        )
        .map(|_| ())
        .map_err(|error| error.to_string())
}

fn delete_label(
    client: &Client,
    workspace: &str,
    project: &str,
    label_id: &str,
) -> Result<(), String> {
    client
        .delete(&format!(
            "workspaces/{workspace}/projects/{project}/labels/{label_id}/"
        ))
        .map(|_| ())
        .map_err(|error| error.to_string())
}

fn project_identifier(client: &Client, workspace: &str, project: &str) -> Result<String, String> {
    let value = client
        .get(&format!("workspaces/{workspace}/projects/{project}/"), &[])
        .map_err(|error| error.to_string())?;
    Ok(field_str(&value, "identifier").unwrap_or("?").to_string())
}

fn find_label<'a>(labels: &'a [Value], name: &str) -> Option<&'a Value> {
    labels
        .iter()
        .find(|label| field_str(label, "name") == Some(name))
}

/// The work-item `labels` field is a list of label ids (as strings or objects).
fn label_ids(item: &Value) -> Vec<String> {
    item.get("labels")
        .and_then(Value::as_array)
        .map(|array| array.iter().filter_map(label_ref_id).collect())
        .unwrap_or_default()
}

fn label_ref_id(value: &Value) -> Option<String> {
    value
        .as_str()
        .map(String::from)
        .or_else(|| field_str(value, "id").map(String::from))
}

fn field_str<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_on_accepts_key_seq_and_uppercases() {
        assert_eq!(parse_on("PLANE:5").unwrap(), ("PLANE".into(), "5".into()));
        assert_eq!(
            parse_on(" plane : 12 ").unwrap(),
            ("PLANE".into(), "12".into())
        );
    }

    #[test]
    fn parse_on_rejects_malformed() {
        assert!(parse_on("PLANE-5").is_err());
        assert!(parse_on("PLANE:").is_err());
        assert!(parse_on(":5").is_err());
        assert!(parse_on("PLANE:abc").is_err());
        assert!(parse_on("pl ane:5").is_err());
    }

    #[test]
    fn dep_label_roundtrips() {
        let name = dep_label_name("PLANE", "5");
        assert_eq!(name, "dep:PLANE:5");
        assert_eq!(parse_dep_label(&name), Some(("PLANE".into(), "5".into())));
        assert_eq!(parse_dep_label("cli"), None);
        assert_eq!(parse_dep_label("dep:PLANE"), None);
    }

    #[test]
    fn label_ids_handles_strings_and_objects() {
        let item = json!({ "labels": ["a", { "id": "b" }, { "name": "x" }] });
        assert_eq!(label_ids(&item), vec!["a".to_string(), "b".to_string()]);
        assert!(label_ids(&json!({})).is_empty());
    }
}
