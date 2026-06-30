//! Generic path-based CRUD for nested / sub-resources whose create/update body
//! is supplied via `--data` (work-item comments/links/relations/activities,
//! members, cycle/module issue associations, estimate points, ...).
//!
//! The command layer builds the full collection path (workspace + parents +
//! segment) and this renders loosely from `serde_json::Value` (id + a label).

use super::{
    as_query_refs, collect_list, list_json, parse_data_object, pretty_json, query_pairs,
    render_json,
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

pub struct WriteOptions {
    pub data: Option<String>,
    pub dry_run: bool,
    pub json: bool,
}

pub fn list(state: &AppState, collection: &str, options: ListOptions) -> Result<String, String> {
    let client = Client::from_state(state).map_err(|error| error.to_string())?;
    let base = query_pairs(&options.fields, &options.expand);
    if options.json {
        return list_json(&client, collection, &base, options.all);
    }
    let items: Vec<Value> = collect_list(&client, collection, &base, options.all)?;
    Ok(render_records(&items))
}

pub fn get(
    state: &AppState,
    collection: &str,
    id: &str,
    options: GetOptions,
) -> Result<String, String> {
    let client = Client::from_state(state).map_err(|error| error.to_string())?;
    let pairs = query_pairs(&options.fields, &options.expand);
    let value = client
        .get(&format!("{collection}{id}/"), &as_query_refs(&pairs))
        .map_err(|error| error.to_string())?;
    if options.json {
        return render_json(&value);
    }
    Ok(render_record(&value))
}

pub fn create(state: &AppState, collection: &str, options: WriteOptions) -> Result<String, String> {
    let body = parse_data_object(&options.data)?;
    if options.dry_run {
        return Ok(format!(
            "DRY RUN POST /api/v1/{collection}\n{}\n",
            pretty_json(&body)?
        ));
    }
    let client = Client::from_state(state).map_err(|error| error.to_string())?;
    let value = client
        .post(collection, &body)
        .map_err(|error| error.to_string())?;
    if options.json {
        return render_json(&value);
    }
    Ok(format!("created {}", render_record(&value)))
}

pub fn update(
    state: &AppState,
    collection: &str,
    id: &str,
    options: WriteOptions,
) -> Result<String, String> {
    let body = parse_data_object(&options.data)?;
    let path = format!("{collection}{id}/");
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
    collection: &str,
    id: &str,
    dry_run: bool,
) -> Result<String, String> {
    let path = format!("{collection}{id}/");
    if dry_run {
        return Ok(format!("DRY RUN DELETE /api/v1/{path}\n"));
    }
    let client = Client::from_state(state).map_err(|error| error.to_string())?;
    client.delete(&path).map_err(|error| error.to_string())?;
    Ok(format!("deleted {id}\n"))
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
    let id = field(value, "id")
        .or_else(|| field(value, "member"))
        .unwrap_or("<no id>");
    // comments carry no name/title; fall back to their text so the row is not
    // an id with a blank label.
    let label = field(value, "name")
        .or_else(|| field(value, "title"))
        .or_else(|| field(value, "display_name"))
        .or_else(|| field(value, "url"))
        .or_else(|| field(value, "comment_stripped"))
        .map(summarize)
        .unwrap_or_default();
    format!("{id}  {label}\n")
}

/// Collapse a label to a single, length-bounded line for human output.
fn summarize(text: &str) -> String {
    let first_line = text.lines().next().unwrap_or("").trim();
    if first_line.chars().count() > 80 {
        let head: String = first_line.chars().take(79).collect();
        format!("{head}…")
    } else {
        first_line.to_string()
    }
}

fn field<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
}
