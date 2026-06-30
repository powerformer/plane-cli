use super::{render_json, require_workspace};
use crate::core::app::AppState;
use crate::core::model::common::Paginated;
use crate::core::model::project::Project;
use crate::core::request::Client;

pub fn list(state: &AppState, json: bool) -> Result<String, String> {
    let workspace = require_workspace(state)?;
    let client = Client::from_state(state).map_err(|error| error.to_string())?;
    let value = client
        .get(&format!("workspaces/{workspace}/projects/"), &[])
        .map_err(|error| error.to_string())?;
    if json {
        return render_json(&value);
    }
    let page: Paginated<Project> = serde_json::from_value(value)
        .map_err(|error| format!("failed to parse projects: {error}"))?;
    Ok(render_list(&page))
}

pub fn get(state: &AppState, id: &str, json: bool) -> Result<String, String> {
    let workspace = require_workspace(state)?;
    let client = Client::from_state(state).map_err(|error| error.to_string())?;
    let value = client
        .get(&format!("workspaces/{workspace}/projects/{id}/"), &[])
        .map_err(|error| error.to_string())?;
    if json {
        return render_json(&value);
    }
    let project: Project = serde_json::from_value(value)
        .map_err(|error| format!("failed to parse project: {error}"))?;
    Ok(render_one(&project))
}

fn render_list(page: &Paginated<Project>) -> String {
    if page.results.is_empty() {
        return "no projects\n".to_string();
    }
    let mut out = String::new();
    for project in &page.results {
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
