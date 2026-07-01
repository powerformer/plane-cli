use super::{
    as_query_refs, collect_list, list_json, parse_data_object, pretty_json, query_pairs,
    render_json, require_workspace,
};
use crate::core::app::AppState;
use crate::core::error::ApiError;
use crate::core::model::work_item::WorkItem;
use crate::core::request::{Client, MultipartFile};
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

pub struct AttachOptions {
    pub item: String,
    pub file: PathBuf,
    pub content_type: Option<String>,
    pub name: Option<String>,
    pub json: bool,
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
        return list_json(&client, &path, &base, options.all);
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

/// Attach a local file to a work item by `KEY-SEQ`. The bytes are streamed to
/// the server-proxied upload endpoint in one request; the CLI never touches S3.
pub fn attach(state: &AppState, options: AttachOptions) -> Result<String, String> {
    let (key, seq) = parse_item_ref(&options.item)?;
    let workspace = require_workspace(state)?;
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

    let client = Client::from_state(state).map_err(|error| error.to_string())?;
    // Resolve KEY-SEQ to the issue and its project (the upload URL is scoped by
    // both). The by-identifier endpoint is workspace-scoped.
    let item = client
        .get(
            &format!("workspaces/{workspace}/work-items/{key}-{seq}/"),
            &[],
        )
        .map_err(|error| match error {
            ApiError::Http { status: 404, .. } => {
                format!("work item {key}-{seq} not found in workspace {workspace}")
            }
            other => other.to_string(),
        })?;
    let issue_id = item
        .get("id")
        .and_then(Value::as_str)
        .ok_or_else(|| format!("work item {key}-{seq} response is missing id"))?;
    let project_id = item
        .get("project")
        .and_then(Value::as_str)
        .ok_or_else(|| format!("work item {key}-{seq} response is missing project"))?;

    let file_part = MultipartFile {
        field: "asset",
        filename: &filename,
        content_type: &content_type,
        bytes: &bytes,
    };
    let response = client
        .post_multipart(
            &format!(
                "workspaces/{workspace}/projects/{project_id}/work-items/{issue_id}/attachments/upload/"
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

/// Parse `--item <KEY>-<SEQ>` (e.g. `PLANECLI-8`) into an uppercased project key
/// and numeric sequence.
fn parse_item_ref(item: &str) -> Result<(String, String), String> {
    let (key, seq) = item
        .rsplit_once('-')
        .ok_or_else(|| format!("--item must be <KEY>-<SEQ>, e.g. PLANECLI-8 (got `{item}`)"))?;
    let key = key.trim();
    let seq = seq.trim();
    if key.is_empty() || !key.chars().all(|c| c.is_ascii_alphanumeric()) {
        return Err(format!("invalid work item key in --item: `{item}`"));
    }
    if seq.is_empty() || !seq.chars().all(|c| c.is_ascii_digit()) {
        return Err(format!("sequence in --item must be a number: `{item}`"));
    }
    Ok((key.to_ascii_uppercase(), seq.to_string()))
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
    fn parse_item_ref_uppercases_key_and_keeps_seq() {
        assert_eq!(
            parse_item_ref("planecli-8").unwrap(),
            ("PLANECLI".to_string(), "8".to_string())
        );
        assert_eq!(
            parse_item_ref("PLANE-12").unwrap(),
            ("PLANE".to_string(), "12".to_string())
        );
    }

    #[test]
    fn parse_item_ref_rejects_bad_shapes() {
        assert!(parse_item_ref("PLANECLI").is_err()); // no seq
        assert!(parse_item_ref("PLANECLI-").is_err()); // empty seq
        assert!(parse_item_ref("PLANECLI-x").is_err()); // non-numeric seq
        assert!(parse_item_ref("-8").is_err()); // empty key
    }

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
