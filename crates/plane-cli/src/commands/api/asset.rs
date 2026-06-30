//! Asset upload: the one place that needs an S3 presigned multipart POST.
//!
//! Flow: POST `workspaces/{slug}/assets/` to presign (returns upload_data =
//! {url, fields} + asset_id), multipart-POST the file to S3, then PATCH
//! `.../assets/{id}/` with `is_uploaded: true`.

use super::{pretty_json, render_json, require_workspace};
use crate::core::app::AppState;
use crate::core::request::Client;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

pub struct UploadOptions {
    pub file: PathBuf,
    pub project: Option<String>,
    pub content_type: Option<String>,
    pub name: Option<String>,
    pub dry_run: bool,
    pub json: bool,
}

pub fn upload(state: &AppState, options: UploadOptions) -> Result<String, String> {
    let workspace = require_workspace(state)?;
    let bytes = std::fs::read(&options.file)
        .map_err(|error| format!("failed to read {}: {error}", options.file.display()))?;
    let name = options
        .name
        .clone()
        .or_else(|| {
            options
                .file
                .file_name()
                .and_then(|n| n.to_str())
                .map(String::from)
        })
        .ok_or_else(|| "could not determine file name; pass --name".to_string())?;
    let content_type = options
        .content_type
        .clone()
        .unwrap_or_else(|| guess_content_type(&options.file));
    let size = bytes.len();

    let mut meta = json!({ "name": name, "type": content_type, "size": size });
    if let Some(project) = &options.project {
        meta.as_object_mut()
            .expect("object")
            .insert("project_id".to_string(), Value::String(project.clone()));
    }

    if options.dry_run {
        return Ok(format!(
            "DRY RUN upload {} ({size} bytes, {content_type}) -> POST /api/v1/workspaces/{workspace}/assets/\n{}\n",
            options.file.display(),
            pretty_json(&meta)?
        ));
    }

    let client = Client::from_state(state).map_err(|error| error.to_string())?;
    // 1. presign
    let presign = client
        .post(&format!("workspaces/{workspace}/assets/"), &meta)
        .map_err(|error| error.to_string())?;
    let asset_id = presign
        .get("asset_id")
        .and_then(Value::as_str)
        .ok_or_else(|| format!("presign response missing asset_id: {presign}"))?
        .to_string();
    let upload_data = presign
        .get("upload_data")
        .ok_or_else(|| format!("presign response missing upload_data: {presign}"))?;
    let url = upload_data
        .get("url")
        .and_then(Value::as_str)
        .ok_or_else(|| "presign upload_data missing url".to_string())?;
    let fields = upload_data
        .get("fields")
        .and_then(Value::as_object)
        .ok_or_else(|| "presign upload_data missing fields".to_string())?;
    let field_pairs: Vec<(String, String)> = fields
        .iter()
        .map(|(k, v)| (k.clone(), v.as_str().unwrap_or_default().to_string()))
        .collect();

    // 2. S3 multipart upload (no auth, different host)
    s3_multipart_post(url, &field_pairs, &name, &content_type, &bytes)?;

    // 3. confirm
    client
        .patch(
            &format!("workspaces/{workspace}/assets/{asset_id}/"),
            &json!({ "is_uploaded": true }),
        )
        .map_err(|error| error.to_string())?;

    if options.json {
        return render_json(&presign);
    }
    Ok(format!(
        "uploaded asset {asset_id} ({name}, {size} bytes)\n"
    ))
}

fn s3_multipart_post(
    url: &str,
    fields: &[(String, String)],
    file_name: &str,
    file_type: &str,
    bytes: &[u8],
) -> Result<(), String> {
    let boundary = format!("planeCliBoundary{}", std::process::id());
    let mut body: Vec<u8> = Vec::new();
    for (key, value) in fields {
        body.extend_from_slice(
            format!(
                "--{boundary}\r\nContent-Disposition: form-data; name=\"{key}\"\r\n\r\n{value}\r\n"
            )
            .as_bytes(),
        );
    }
    // S3 requires the file part last.
    body.extend_from_slice(
        format!("--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"{file_name}\"\r\nContent-Type: {file_type}\r\n\r\n")
            .as_bytes(),
    );
    body.extend_from_slice(bytes);
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());

    let response = ureq::post(url)
        .set(
            "Content-Type",
            &format!("multipart/form-data; boundary={boundary}"),
        )
        .send_bytes(&body);
    match response {
        Ok(_) => Ok(()),
        Err(ureq::Error::Status(status, response)) => {
            let body = response.into_string().unwrap_or_default();
            Err(format!(
                "S3 upload returned HTTP {status}: {}",
                body.chars().take(300).collect::<String>()
            ))
        }
        Err(error) => Err(format!("S3 upload failed: {error}")),
    }
}

fn guess_content_type(path: &Path) -> String {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or_default()
        .to_lowercase();
    match ext.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "svg" => "image/svg+xml",
        "pdf" => "application/pdf",
        "txt" => "text/plain",
        "csv" => "text/csv",
        "json" => "application/json",
        "md" => "text/markdown",
        "zip" => "application/zip",
        _ => "application/octet-stream",
    }
    .to_string()
}
