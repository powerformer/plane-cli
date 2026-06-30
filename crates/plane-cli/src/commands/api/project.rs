use super::{
    as_query_refs, collect_list, parse_data_object, pretty_json, query_pairs, render_json,
    require_workspace,
};
use crate::core::app::AppState;
use crate::core::model::project::Project;
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
    pub identifier: String,
    pub data: Option<String>,
    pub dry_run: bool,
    pub json: bool,
}

pub fn list(state: &AppState, options: ListOptions) -> Result<String, String> {
    let workspace = require_workspace(state)?;
    let client = Client::from_state(state).map_err(|error| error.to_string())?;
    let path = format!("workspaces/{workspace}/projects/");
    let base = query_pairs(&options.fields, &options.expand);
    if options.json {
        let value = client
            .get(&path, &as_query_refs(&base))
            .map_err(|error| error.to_string())?;
        return render_json(&value);
    }
    let projects: Vec<Project> = collect_list(&client, &path, &base, options.all)?;
    Ok(render_list(&projects))
}

pub fn get(state: &AppState, id: &str, options: GetOptions) -> Result<String, String> {
    let workspace = require_workspace(state)?;
    let client = Client::from_state(state).map_err(|error| error.to_string())?;
    let pairs = query_pairs(&options.fields, &options.expand);
    let value = client
        .get(
            &format!("workspaces/{workspace}/projects/{id}/"),
            &as_query_refs(&pairs),
        )
        .map_err(|error| error.to_string())?;
    if options.json {
        return render_json(&value);
    }
    let project: Project = serde_json::from_value(value)
        .map_err(|error| format!("failed to parse project: {error}"))?;
    Ok(render_one(&project))
}

pub fn create(state: &AppState, options: CreateOptions) -> Result<String, String> {
    let workspace = require_workspace(state)?;
    let mut body = parse_data_object(&options.data)?;
    let object = body.as_object_mut().expect("data is an object");
    object.insert("name".to_string(), Value::String(options.name.clone()));
    object.insert(
        "identifier".to_string(),
        Value::String(options.identifier.clone()),
    );
    let path = format!("workspaces/{workspace}/projects/");
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
    let project: Project = serde_json::from_value(value)
        .map_err(|error| format!("failed to parse project: {error}"))?;
    Ok(format!("created {}", render_one(&project)))
}

pub struct UpdateOptions {
    pub id: String,
    pub name: Option<String>,
    pub data: Option<String>,
    pub dry_run: bool,
    pub json: bool,
}

pub fn update(state: &AppState, options: UpdateOptions) -> Result<String, String> {
    let workspace = require_workspace(state)?;
    let mut body = parse_data_object(&options.data)?;
    if let Some(name) = &options.name {
        body.as_object_mut()
            .expect("data is an object")
            .insert("name".to_string(), Value::String(name.clone()));
    }
    let path = format!("workspaces/{workspace}/projects/{}/", options.id);
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
    let project: Project = serde_json::from_value(value)
        .map_err(|error| format!("failed to parse project: {error}"))?;
    Ok(format!("updated {}", render_one(&project)))
}

pub fn delete(state: &AppState, id: &str, dry_run: bool) -> Result<String, String> {
    let workspace = require_workspace(state)?;
    let path = format!("workspaces/{workspace}/projects/{id}/");
    if dry_run {
        return Ok(format!("DRY RUN DELETE /api/v1/{path}\n"));
    }
    let client = Client::from_state(state).map_err(|error| error.to_string())?;
    client.delete(&path).map_err(|error| error.to_string())?;
    Ok(format!("deleted project {id}\n"))
}

pub fn archive(state: &AppState, id: &str, dry_run: bool) -> Result<String, String> {
    let workspace = require_workspace(state)?;
    let path = format!("workspaces/{workspace}/projects/{id}/archive/");
    if dry_run {
        return Ok(format!("DRY RUN POST /api/v1/{path}\n"));
    }
    let client = Client::from_state(state).map_err(|error| error.to_string())?;
    client
        .post(&path, &serde_json::json!({}))
        .map_err(|error| error.to_string())?;
    Ok(format!("archived project {id}\n"))
}

pub fn unarchive(state: &AppState, id: &str, dry_run: bool) -> Result<String, String> {
    let workspace = require_workspace(state)?;
    let path = format!("workspaces/{workspace}/projects/{id}/archive/");
    if dry_run {
        return Ok(format!("DRY RUN DELETE /api/v1/{path}\n"));
    }
    let client = Client::from_state(state).map_err(|error| error.to_string())?;
    client.delete(&path).map_err(|error| error.to_string())?;
    Ok(format!("unarchived project {id}\n"))
}

pub fn summary(state: &AppState, id: &str) -> Result<String, String> {
    let workspace = require_workspace(state)?;
    let client = Client::from_state(state).map_err(|error| error.to_string())?;
    let value = client
        .get(
            &format!("workspaces/{workspace}/projects/{id}/summary/"),
            &[],
        )
        .map_err(|error| error.to_string())?;
    render_json(&value)
}

fn render_list(projects: &[Project]) -> String {
    if projects.is_empty() {
        return "no projects\n".to_string();
    }
    let mut out = String::new();
    for project in projects {
        out.push_str(&render_row(project));
    }
    out
}

fn render_row(project: &Project) -> String {
    let id = project.id.as_deref().unwrap_or("<no id>");
    let name = project.name.as_deref().unwrap_or("<no name>");
    let identifier = project.identifier.as_deref().unwrap_or("-");
    format!("{id}  {name}  [{identifier}]\n")
}

fn render_one(project: &Project) -> String {
    let mut out = render_row(project);
    if let Some(description) = project.description.as_deref() {
        if !description.trim().is_empty() {
            out.push_str(&format!("  {}\n", description.trim()));
        }
    }
    out
}
