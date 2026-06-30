use super::{render_json, require_workspace};
use crate::core::app::AppState;
use crate::core::model::common::Paginated;
use crate::core::model::work_item::WorkItem;
use crate::core::request::Client;
use serde_json::{json, Value};

pub struct CreateOptions {
    pub project: String,
    pub name: String,
    pub data: Option<String>,
    pub dry_run: bool,
    pub json: bool,
}

pub fn list(state: &AppState, project: &str, json: bool) -> Result<String, String> {
    let workspace = require_workspace(state)?;
    let client = Client::from_state(state).map_err(|error| error.to_string())?;
    let value = client
        .get(
            &format!("workspaces/{workspace}/projects/{project}/work-items/"),
            &[],
        )
        .map_err(|error| error.to_string())?;
    if json {
        return render_json(&value);
    }
    let page: Paginated<WorkItem> = serde_json::from_value(value)
        .map_err(|error| format!("failed to parse work items: {error}"))?;
    Ok(render_list(&page))
}

pub fn get(state: &AppState, project: &str, id: &str, json: bool) -> Result<String, String> {
    let workspace = require_workspace(state)?;
    let client = Client::from_state(state).map_err(|error| error.to_string())?;
    let value = client
        .get(
            &format!("workspaces/{workspace}/projects/{project}/work-items/{id}/"),
            &[],
        )
        .map_err(|error| error.to_string())?;
    if json {
        return render_json(&value);
    }
    let item: WorkItem = serde_json::from_value(value)
        .map_err(|error| format!("failed to parse work item: {error}"))?;
    Ok(render_one(&item))
}

pub fn create(state: &AppState, options: CreateOptions) -> Result<String, String> {
    let workspace = require_workspace(state)?;
    let mut body = match options.data.as_deref() {
        Some(raw) => serde_json::from_str::<Value>(raw)
            .map_err(|error| format!("--data is not valid JSON: {error}"))?,
        None => json!({}),
    };
    let object = body
        .as_object_mut()
        .ok_or_else(|| "--data must be a JSON object".to_string())?;
    object.insert("name".to_string(), Value::String(options.name.clone()));

    let path = format!(
        "workspaces/{workspace}/projects/{}/work-items/",
        options.project
    );
    if options.dry_run {
        let pretty = serde_json::to_string_pretty(&body).map_err(|error| error.to_string())?;
        return Ok(format!("DRY RUN POST /api/v1/{path}\n{pretty}\n"));
    }

    let client = Client::from_state(state).map_err(|error| error.to_string())?;
    let value = client
        .post(&path, &body)
        .map_err(|error| error.to_string())?;
    if options.json {
        return render_json(&value);
    }
    let item: WorkItem = serde_json::from_value(value)
        .map_err(|error| format!("failed to parse work item: {error}"))?;
    Ok(format!("created {}", render_one(&item)))
}

fn render_list(page: &Paginated<WorkItem>) -> String {
    if page.results.is_empty() {
        return "no work items\n".to_string();
    }
    let mut out = String::new();
    for item in &page.results {
        out.push_str(&render_one(item));
    }
    out
}

fn render_one(item: &WorkItem) -> String {
    let id = item.id.as_deref().unwrap_or("<no id>");
    let name = item.name.as_deref().unwrap_or("<no name>");
    let seq = item
        .sequence_id
        .map(|seq| format!("#{seq}"))
        .unwrap_or_default();
    let priority = item.priority.as_deref().unwrap_or("");
    format!("{id}  {seq}  {name}  {priority}\n")
}
