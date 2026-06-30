//! Generic project-scoped CRUD shared by the simple resources
//! (states, labels, cycles, modules, estimates, intake-issues).
//!
//! These endpoints all live under
//! `workspaces/{ws}/projects/{project}/{segment}/` and follow the same
//! list/get/create/update/delete shape, so they share one implementation and
//! render loosely from `serde_json::Value` (id + name) instead of a typed model
//! per resource.

use super::{
    as_query_refs, collect_list, parse_data_object, pretty_json, query_pairs, render_json,
    require_workspace,
};
use crate::core::app::AppState;
use crate::core::request::Client;
use serde_json::Value;

pub struct ListOptions {
    pub all: bool,
    pub fields: Option<String>,
    pub expand: Option<String>,
    pub json: bool,
}

pub struct GetOptions {
    pub fields: Option<String>,
    pub expand: Option<String>,
    pub json: bool,
}

pub struct CreateOptions {
    pub name: String,
    pub data: Option<String>,
    pub dry_run: bool,
    pub json: bool,
}

pub struct UpdateOptions {
    pub name: Option<String>,
    pub data: Option<String>,
    pub dry_run: bool,
    pub json: bool,
}

fn collection_path(workspace: &str, project: &str, segment: &str) -> String {
    format!("workspaces/{workspace}/projects/{project}/{segment}/")
}

fn item_path(workspace: &str, project: &str, segment: &str, id: &str) -> String {
    format!("workspaces/{workspace}/projects/{project}/{segment}/{id}/")
}

pub fn list(
    state: &AppState,
    project: &str,
    segment: &str,
    options: ListOptions,
) -> Result<String, String> {
    let workspace = require_workspace(state)?;
    let client = Client::from_state(state).map_err(|error| error.to_string())?;
    let path = collection_path(&workspace, project, segment);
    let base = query_pairs(&options.fields, &options.expand);
    if options.json {
        let value = client
            .get(&path, &as_query_refs(&base))
            .map_err(|error| error.to_string())?;
        return render_json(&value);
    }
    let items: Vec<Value> = collect_list(&client, &path, &base, options.all)?;
    Ok(render_records(&items))
}

pub fn get(
    state: &AppState,
    project: &str,
    segment: &str,
    id: &str,
    options: GetOptions,
) -> Result<String, String> {
    let workspace = require_workspace(state)?;
    let client = Client::from_state(state).map_err(|error| error.to_string())?;
    let pairs = query_pairs(&options.fields, &options.expand);
    let value = client
        .get(
            &item_path(&workspace, project, segment, id),
            &as_query_refs(&pairs),
        )
        .map_err(|error| error.to_string())?;
    if options.json {
        return render_json(&value);
    }
    Ok(render_record(&value))
}

pub fn create(
    state: &AppState,
    project: &str,
    segment: &str,
    options: CreateOptions,
) -> Result<String, String> {
    let workspace = require_workspace(state)?;
    let mut body = parse_data_object(&options.data)?;
    body.as_object_mut()
        .expect("data is an object")
        .insert("name".to_string(), Value::String(options.name.clone()));
    let path = collection_path(&workspace, project, segment);
    if options.dry_run {
        return Ok(format!(
            "DRY RUN POST /api/v1/{path}\n{}\n",
            pretty_json(&body)?
        ));
    }
    let client = Client::from_state(state).map_err(|error| error.to_string())?;
    let value = client
        .post(&path, &body)
        .map_err(|error| error.to_string())?;
    if options.json {
        return render_json(&value);
    }
    Ok(format!("created {}", render_record(&value)))
}

pub fn update(
    state: &AppState,
    project: &str,
    segment: &str,
    id: &str,
    options: UpdateOptions,
) -> Result<String, String> {
    let workspace = require_workspace(state)?;
    let mut body = parse_data_object(&options.data)?;
    if let Some(name) = &options.name {
        body.as_object_mut()
            .expect("data is an object")
            .insert("name".to_string(), Value::String(name.clone()));
    }
    let path = item_path(&workspace, project, segment, id);
    if options.dry_run {
        return Ok(format!(
            "DRY RUN PATCH /api/v1/{path}\n{}\n",
            pretty_json(&body)?
        ));
    }
    let client = Client::from_state(state).map_err(|error| error.to_string())?;
    let value = client
        .patch(&path, &body)
        .map_err(|error| error.to_string())?;
    if options.json {
        return render_json(&value);
    }
    Ok(format!("updated {}", render_record(&value)))
}

pub fn delete(
    state: &AppState,
    project: &str,
    segment: &str,
    id: &str,
    dry_run: bool,
) -> Result<String, String> {
    let workspace = require_workspace(state)?;
    let path = item_path(&workspace, project, segment, id);
    if dry_run {
        return Ok(format!("DRY RUN DELETE /api/v1/{path}\n"));
    }
    let client = Client::from_state(state).map_err(|error| error.to_string())?;
    client.delete(&path).map_err(|error| error.to_string())?;
    Ok(format!("deleted {segment} {id}\n"))
}

fn render_records(items: &[Value]) -> String {
    if items.is_empty() {
        return "no results\n".to_string();
    }
    let mut out = String::new();
    for item in items {
        out.push_str(&render_record(item));
    }
    out
}

fn render_record(value: &Value) -> String {
    let id = field(value, "id").unwrap_or("<no id>");
    let name = field(value, "name")
        .or_else(|| field(value, "display_name"))
        .unwrap_or("");
    format!("{id}  {name}\n")
}

fn field<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
}
