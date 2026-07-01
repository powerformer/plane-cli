use super::{collect_list, list_json, pretty_json, query_pairs, render_json, require_workspace};
use crate::core::app::AppState;
use crate::core::request::Client;
use serde_json::{json, Value};

pub struct ListOptions {
    pub project: String,
    pub work_item: String,
    pub all: bool,
    pub fields: Option<String>,
    pub expand: Option<String>,
    pub json: bool,
}

pub struct LinkOptions {
    pub project: String,
    pub work_item: String,
    pub page_ids: Vec<String>,
    pub dry_run: bool,
    pub json: bool,
}

pub struct UnlinkOptions {
    pub project: String,
    pub work_item: String,
    pub page_id: String,
    pub dry_run: bool,
}

fn collection_path(workspace: &str, project: &str, work_item: &str) -> String {
    format!("workspaces/{workspace}/projects/{project}/work-items/{work_item}/pages/")
}

fn item_path(workspace: &str, project: &str, work_item: &str, page_id: &str) -> String {
    format!("workspaces/{workspace}/projects/{project}/work-items/{work_item}/pages/{page_id}/")
}

pub fn list(state: &AppState, options: ListOptions) -> Result<String, String> {
    let workspace = require_workspace(state)?;
    let client = Client::from_state(state).map_err(|error| error.to_string())?;
    let path = collection_path(&workspace, &options.project, &options.work_item);
    let base = query_pairs(&options.fields, &options.expand);
    if options.json {
        return list_json(&client, &path, &base, options.all);
    }
    let items: Vec<Value> = collect_list(&client, &path, &base, options.all)?;
    Ok(render_records(&items))
}

pub fn link(state: &AppState, options: LinkOptions) -> Result<String, String> {
    let workspace = require_workspace(state)?;
    let path = collection_path(&workspace, &options.project, &options.work_item);
    if options.dry_run {
        let mut out = String::new();
        for page_id in &options.page_ids {
            let body = json!({ "page_id": page_id });
            out.push_str(&format!(
                "DRY RUN POST /api/v1/{path}\n{}\n",
                pretty_json(&body)?
            ));
        }
        return Ok(out);
    }
    let client = Client::from_state(state).map_err(|error| error.to_string())?;
    let mut results = Vec::new();
    for page_id in &options.page_ids {
        let body = json!({ "page_id": page_id });
        let value = client
            .post(&path, &body)
            .map_err(|error| error.to_string())?;
        results.push(value);
    }
    if options.json {
        return render_json(&Value::Array(results));
    }
    Ok(render_link_results(&results))
}

pub fn unlink(state: &AppState, options: UnlinkOptions) -> Result<String, String> {
    let workspace = require_workspace(state)?;
    let path = item_path(
        &workspace,
        &options.project,
        &options.work_item,
        &options.page_id,
    );
    if options.dry_run {
        return Ok(format!("DRY RUN DELETE /api/v1/{path}\n"));
    }
    let client = Client::from_state(state).map_err(|error| error.to_string())?;
    client.delete(&path).map_err(|error| error.to_string())?;
    Ok(format!("unlinked page {}\n", options.page_id))
}

fn render_link_results(values: &[Value]) -> String {
    let mut out = String::new();
    for value in values {
        if let Some(items) = value.as_array() {
            out.push_str(&render_records(items));
            continue;
        }
        if let Some(items) = value.get("results").and_then(Value::as_array) {
            out.push_str(&render_records(items));
            continue;
        }
        out.push_str(&render_record(value));
    }
    out
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
        .or_else(|| field(value, "title"))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn link_results_handle_results_envelope() {
        let values = vec![json!({"results": [{"id": "p1", "name": "Doc"}]})];
        assert_eq!(render_link_results(&values), "p1  Doc\n");
    }

    #[test]
    fn empty_list_matches_existing_style() {
        assert_eq!(render_records(&[]), "no results\n");
    }
}
