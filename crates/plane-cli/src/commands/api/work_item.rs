use super::{
    as_query_refs, collect_list, parse_data_object, pretty_json, query_pairs, render_json,
    require_workspace,
};
use crate::core::app::AppState;
use crate::core::model::work_item::WorkItem;
use crate::core::request::Client;
use serde_json::Value;

pub struct ListOptions {
    pub project: String,
    pub all: bool,
    pub fields: Option<String>,
    pub expand: Option<String>,
    pub json: bool,
}

pub struct GetOptions {
    pub project: String,
    pub id: String,
    pub fields: Option<String>,
    pub expand: Option<String>,
    pub json: bool,
}

pub struct CreateOptions {
    pub project: String,
    pub name: String,
    pub data: Option<String>,
    pub dry_run: bool,
    pub json: bool,
}

pub struct UpdateOptions {
    pub project: String,
    pub id: String,
    pub name: Option<String>,
    pub data: Option<String>,
    pub dry_run: bool,
    pub json: bool,
}

pub struct DeleteOptions {
    pub project: String,
    pub id: String,
    pub dry_run: bool,
}

pub fn list(state: &AppState, options: ListOptions) -> Result<String, String> {
    let workspace = require_workspace(state)?;
    let client = Client::from_state(state).map_err(|error| error.to_string())?;
    let path = format!(
        "workspaces/{workspace}/projects/{}/work-items/",
        options.project
    );
    let base = query_pairs(&options.fields, &options.expand);
    if options.json {
        let value = client
            .get(&path, &as_query_refs(&base))
            .map_err(|error| error.to_string())?;
        return render_json(&value);
    }
    let items: Vec<WorkItem> = collect_list(&client, &path, &base, options.all)?;
    Ok(render_list(&items))
}

pub fn get(state: &AppState, options: GetOptions) -> Result<String, String> {
    let workspace = require_workspace(state)?;
    let client = Client::from_state(state).map_err(|error| error.to_string())?;
    let pairs = query_pairs(&options.fields, &options.expand);
    let value = client
        .get(
            &format!(
                "workspaces/{workspace}/projects/{}/work-items/{}/",
                options.project, options.id
            ),
            &as_query_refs(&pairs),
        )
        .map_err(|error| error.to_string())?;
    if options.json {
        return render_json(&value);
    }
    let item: WorkItem = serde_json::from_value(value)
        .map_err(|error| format!("failed to parse work item: {error}"))?;
    Ok(render_one(&item))
}

pub fn create(state: &AppState, options: CreateOptions) -> Result<String, String> {
    let workspace = require_workspace(state)?;
    let mut body = parse_data_object(&options.data)?;
    body.as_object_mut()
        .expect("data is an object")
        .insert("name".to_string(), Value::String(options.name.clone()));
    let path = format!(
        "workspaces/{workspace}/projects/{}/work-items/",
        options.project
    );
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
    let item: WorkItem = serde_json::from_value(value)
        .map_err(|error| format!("failed to parse work item: {error}"))?;
    Ok(format!("created {}", render_one(&item)))
}

pub fn update(state: &AppState, options: UpdateOptions) -> Result<String, String> {
    let workspace = require_workspace(state)?;
    let mut body = parse_data_object(&options.data)?;
    if let Some(name) = &options.name {
        body.as_object_mut()
            .expect("data is an object")
            .insert("name".to_string(), Value::String(name.clone()));
    }
    let path = format!(
        "workspaces/{workspace}/projects/{}/work-items/{}/",
        options.project, options.id
    );
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
    let item: WorkItem = serde_json::from_value(value)
        .map_err(|error| format!("failed to parse work item: {error}"))?;
    Ok(format!("updated {}", render_one(&item)))
}

pub fn delete(state: &AppState, options: DeleteOptions) -> Result<String, String> {
    let workspace = require_workspace(state)?;
    let path = format!(
        "workspaces/{workspace}/projects/{}/work-items/{}/",
        options.project, options.id
    );
    if options.dry_run {
        return Ok(format!("DRY RUN DELETE /api/v1/{path}\n"));
    }
    let client = Client::from_state(state).map_err(|error| error.to_string())?;
    client.delete(&path).map_err(|error| error.to_string())?;
    Ok(format!("deleted work item {}\n", options.id))
}

fn render_list(items: &[WorkItem]) -> String {
    if items.is_empty() {
        return "no work items\n".to_string();
    }
    let mut out = String::new();
    for item in items {
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
