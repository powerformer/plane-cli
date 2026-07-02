use super::{
    as_query_refs, collect_list, list_json, parse_data_object, pretty_json, query_pairs, reference,
    render_json, require_workspace, workspace_client,
};
use crate::core::app::AppState;
use crate::core::model::work_item::WorkItem;
use crate::core::request::MultipartFile;
use serde_json::Value;
use std::path::{Path, PathBuf};

pub struct ListOptions {
    pub project: String,
    pub all: bool,
    pub fields: Option<String>,
    pub expand: Option<String>,
    pub json: bool,
}

pub struct GetOptions {
    pub project: Option<String>,
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
    pub project: Option<String>,
    pub id: String,
    pub name: Option<String>,
    pub data: Option<String>,
    pub dry_run: bool,
    pub json: bool,
}

pub struct DeleteOptions {
    pub project: Option<String>,
    pub id: String,
    pub dry_run: bool,
}

pub struct AttachOptions {
    pub item: String,
    pub file: PathBuf,
    pub content_type: Option<String>,
    pub name: Option<String>,
    pub json: bool,
}

pub fn list(state: &AppState, options: ListOptions) -> Result<String, String> {
    let project = reference::resolve_project(state, &options.project)?;
    let (workspace, client) = workspace_client(state)?;
    let path = format!("workspaces/{workspace}/projects/{project}/work-items/");
    let base = query_pairs(&options.fields, &options.expand);
    if options.json {
        return list_json(&client, &path, &base, options.all);
    }
    let items: Vec<WorkItem> = collect_list(&client, &path, &base, options.all)?;
    Ok(render_list(&items))
}

pub fn get(state: &AppState, options: GetOptions) -> Result<String, String> {
    let resolved = reference::resolve_work_item(state, options.project.as_deref(), &options.id)?;
    let (workspace, client) = workspace_client(state)?;
    let pairs = query_pairs(&options.fields, &options.expand);
    let value = client
        .get(
            &format!(
                "workspaces/{workspace}/projects/{}/work-items/{}/",
                resolved.project, resolved.id
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
    let project = reference::resolve_project(state, &options.project)?;
    let workspace = require_workspace(state)?;
    let mut body = parse_data_object(&options.data)?;
    body.as_object_mut()
        .expect("data is an object")
        .insert("name".to_string(), Value::String(options.name.clone()));
    let path = format!("workspaces/{workspace}/projects/{project}/work-items/");
    if options.dry_run {
        return Ok(format!(
            "DRY RUN POST /api/v1/{path}\n{}\n",
            pretty_json(&body)?
        ));
    }
    let (_, client) = workspace_client(state)?;
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
    let resolved = reference::resolve_work_item(state, options.project.as_deref(), &options.id)?;
    let workspace = require_workspace(state)?;
    let mut body = parse_data_object(&options.data)?;
    if let Some(name) = &options.name {
        body.as_object_mut()
            .expect("data is an object")
            .insert("name".to_string(), Value::String(name.clone()));
    }
    let path = format!(
        "workspaces/{workspace}/projects/{}/work-items/{}/",
        resolved.project, resolved.id
    );
    if options.dry_run {
        return Ok(format!(
            "DRY RUN PATCH /api/v1/{path}\n{}\n",
            pretty_json(&body)?
        ));
    }
    let (_, client) = workspace_client(state)?;
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
    let resolved = reference::resolve_work_item(state, options.project.as_deref(), &options.id)?;
    let workspace = require_workspace(state)?;
    let path = format!(
        "workspaces/{workspace}/projects/{}/work-items/{}/",
        resolved.project, resolved.id
    );
    if options.dry_run {
        return Ok(format!("DRY RUN DELETE /api/v1/{path}\n"));
    }
    let (_, client) = workspace_client(state)?;
    client.delete(&path).map_err(|error| error.to_string())?;
    Ok(format!("deleted work item {}\n", options.id))
}

/// Attach a local file to a work item by `KEY-SEQ`. The bytes are streamed to
/// the server-proxied upload endpoint in one request; the CLI never touches S3.
pub fn attach(state: &AppState, options: AttachOptions) -> Result<String, String> {
    let (key, seq) = reference::parse_work_item_ref(&options.item).ok_or_else(|| {
        format!(
            "--item must be <KEY>-<SEQ>, e.g. PLANECLI-8 (got `{}`)",
            options.item
        )
    })?;
    let bytes = std::fs::read(&options.file)
        .map_err(|error| format!("failed to read {}: {error}", options.file.display()))?;
    let filename = options
        .name
        .clone()
        .or_else(|| {
            options
                .file
                .file_name()
                .and_then(|name| name.to_str())
                .map(String::from)
        })
        .ok_or_else(|| "could not determine file name; pass --name".to_string())?;
    let content_type = options
        .content_type
        .clone()
        .unwrap_or_else(|| guess_content_type(&options.file));

    // Resolve KEY-SEQ to the issue and its project (the upload URL is scoped by
    // both). The by-identifier endpoint is workspace-scoped.
    let resolved = reference::fetch_work_item_by_identifier(state, &key, &seq)?;
    let (workspace, client) = workspace_client(state)?;

    let file_part = MultipartFile {
        field: "asset",
        filename: &filename,
        content_type: &content_type,
        bytes: &bytes,
    };
    let response = client
        .post_multipart(
            &format!(
                "workspaces/{workspace}/projects/{}/work-items/{}/attachments/upload/",
                resolved.project, resolved.id
            ),
            &[],
            &file_part,
        )
        .map_err(|error| error.to_string())?;

    if options.json {
        return render_json(&response);
    }
    let attachment_id = response.get("id").and_then(Value::as_str).unwrap_or("?");
    Ok(format!(
        "attached {filename} ({} bytes, {content_type}) to {key}-{seq} (attachment {attachment_id})\n",
        bytes.len()
    ))
}

/// Best-effort MIME type from the file extension, biased toward types the server
/// accepts (`ATTACHMENT_MIME_TYPES`). Unknown extensions fall back to
/// `application/octet-stream`; pass `--type` to override.
fn guess_content_type(path: &Path) -> String {
    let ext = path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or_default()
        .to_lowercase();
    match ext.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "svg" => "image/svg+xml",
        "pdf" => "application/pdf",
        "md" | "markdown" => "text/markdown",
        // Common textual dev artifacts (logs, configs, data) the server accepts
        // as text/plain.
        "txt" | "log" | "json" | "yaml" | "yml" | "toml" | "ini" | "csv" => "text/plain",
        _ => "application/octet-stream",
    }
    .to_string()
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn guess_content_type_maps_common_and_defaults() {
        assert_eq!(guess_content_type(Path::new("shot.PNG")), "image/png");
        assert_eq!(guess_content_type(Path::new("build.log")), "text/plain");
        assert_eq!(guess_content_type(Path::new("data.json")), "text/plain");
        assert_eq!(guess_content_type(Path::new("notes.md")), "text/markdown");
        assert_eq!(
            guess_content_type(Path::new("archive.xyz")),
            "application/octet-stream"
        );
    }
}
