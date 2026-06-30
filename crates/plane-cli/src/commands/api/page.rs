//! Page (document) commands.
//!
//! Pages are an ordinary project-scoped resource
//! (`workspaces/{ws}/projects/{project}/pages/`), so list/get/delete reuse the
//! generic `crud` layer. Create and edit add document ergonomics on top: the
//! body is written as Markdown (converted to HTML) or raw HTML, and `--access`
//! maps to Plane's numeric field. Plane stores the body as `description_html`
//! and the collaborative editor hydrates from it on first open.

use super::crud;
use super::{render_json, require_workspace};
use crate::core::app::AppState;
use crate::core::request::Client;
use pulldown_cmark::{html, Options, Parser};
use serde_json::{json, Value};
use std::path::Path;

#[derive(Debug, Clone, Copy)]
pub enum Access {
    Public,
    Private,
}

impl Access {
    fn code(self) -> i64 {
        match self {
            Access::Public => 0,
            Access::Private => 1,
        }
    }
}

/// Where the page body comes from on the command line.
pub struct BodyArgs<'a> {
    pub from_file: Option<&'a Path>,
    pub body: Option<&'a str>,
    pub format: Option<&'a str>,
}

pub struct CreateOptions<'a> {
    pub name: String,
    pub body: BodyArgs<'a>,
    pub access: Option<Access>,
    pub data: Option<String>,
    pub dry_run: bool,
    pub json: bool,
}

pub struct UpdateOptions<'a> {
    pub name: Option<String>,
    pub body: BodyArgs<'a>,
    pub access: Option<Access>,
    pub data: Option<String>,
    pub dry_run: bool,
    pub json: bool,
}

pub struct GetOptions {
    pub content: bool,
    pub fields: Option<String>,
    pub expand: Option<String>,
    pub json: bool,
}

/// Convert Markdown into the HTML Plane stores as `description_html`.
pub fn markdown_to_html(markdown: &str) -> String {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);
    options.insert(Options::ENABLE_FOOTNOTES);
    let parser = Parser::new_ext(markdown, options);
    let mut out = String::new();
    html::push_html(&mut out, parser);
    if out.trim().is_empty() {
        // Plane treats an empty document as `<p></p>`.
        return "<p></p>".to_string();
    }
    out
}

/// Resolve the body flags into HTML, or `None` when no body was supplied.
/// Markdown is the default; raw HTML is used for `.html`/`.htm` files or
/// `--format html`. Shared with other authored resources (e.g. comments).
pub(crate) fn resolve_html(args: &BodyArgs) -> Result<Option<String>, String> {
    let (raw, default_format) = match (args.from_file, args.body) {
        (Some(path), _) => {
            let content = std::fs::read_to_string(path)
                .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
            let is_html = path
                .extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext.eq_ignore_ascii_case("html") || ext.eq_ignore_ascii_case("htm"))
                .unwrap_or(false);
            (content, if is_html { "html" } else { "md" })
        }
        (None, Some(text)) => (text.to_string(), "md"),
        (None, None) => return Ok(None),
    };
    let format = args.format.unwrap_or(default_format);
    let html = if format == "html" {
        raw
    } else {
        markdown_to_html(&raw)
    };
    Ok(Some(html))
}

/// Merge the page body/access fields into the base `--data` object, returning a
/// JSON string the `crud` layer can post.
fn build_data(
    data: &Option<String>,
    html: Option<String>,
    access: Option<Access>,
) -> Result<Option<String>, String> {
    let mut object = match data {
        Some(raw) => serde_json::from_str::<Value>(raw)
            .map_err(|error| format!("--data is not valid JSON: {error}"))?,
        None => json!({}),
    };
    let map = object
        .as_object_mut()
        .ok_or_else(|| "--data must be a JSON object".to_string())?;
    if let Some(html) = html {
        map.insert("description_html".to_string(), Value::String(html));
    }
    if let Some(access) = access {
        map.insert("access".to_string(), json!(access.code()));
    }
    if map.is_empty() {
        return Ok(None);
    }
    serde_json::to_string(&object)
        .map(Some)
        .map_err(|error| error.to_string())
}

pub fn create(state: &AppState, project: &str, options: CreateOptions) -> Result<String, String> {
    let html = resolve_html(&options.body)?;
    let data = build_data(&options.data, html, options.access)?;
    crud::create(
        state,
        project,
        "pages",
        crud::CreateOptions {
            name: options.name,
            data,
            dry_run: options.dry_run,
            json: options.json,
        },
    )
}

pub fn update(
    state: &AppState,
    project: &str,
    id: &str,
    options: UpdateOptions,
) -> Result<String, String> {
    let html = resolve_html(&options.body)?;
    let data = build_data(&options.data, html, options.access)?;
    if options.name.is_none() && data.is_none() {
        return Err("nothing to update; pass --name, --from-file/--body, or --access".to_string());
    }
    crud::update(
        state,
        project,
        "pages",
        id,
        crud::UpdateOptions {
            name: options.name,
            data,
            dry_run: options.dry_run,
            json: options.json,
        },
    )
}

pub fn get(
    state: &AppState,
    project: &str,
    id: &str,
    options: GetOptions,
) -> Result<String, String> {
    if !options.content {
        return crud::get(
            state,
            project,
            "pages",
            id,
            crud::GetOptions {
                fields: options.fields,
                expand: options.expand,
                json: options.json,
            },
        );
    }
    // `--content` prints only the document body HTML.
    let workspace = require_workspace(state)?;
    let client = Client::from_state(state).map_err(|error| error.to_string())?;
    let path = format!("workspaces/{workspace}/projects/{project}/pages/{id}/");
    let value = client.get(&path, &[]).map_err(|error| error.to_string())?;
    if options.json {
        return render_json(&value);
    }
    let body = value
        .get("description_html")
        .and_then(Value::as_str)
        .unwrap_or("");
    Ok(format!("{body}\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn markdown_converts_blocks_and_inline() {
        let html = markdown_to_html("# Title\n\n- a\n- b\n\nHello **bold**.");
        assert!(html.contains("<h1>Title</h1>"));
        assert!(html.contains("<li>a</li>"));
        assert!(html.contains("<strong>bold</strong>"));
    }

    #[test]
    fn empty_markdown_is_empty_paragraph() {
        assert_eq!(markdown_to_html("  \n"), "<p></p>");
    }

    #[test]
    fn build_data_merges_html_and_access_over_data() {
        let out = build_data(
            &Some(r#"{"color":"red"}"#.to_string()),
            Some("<p>x</p>".to_string()),
            Some(Access::Private),
        )
        .unwrap()
        .unwrap();
        let value: Value = serde_json::from_str(&out).unwrap();
        assert_eq!(value["color"], "red");
        assert_eq!(value["description_html"], "<p>x</p>");
        assert_eq!(value["access"], 1);
    }

    #[test]
    fn build_data_empty_is_none() {
        assert!(build_data(&None, None, None).unwrap().is_none());
    }
}
