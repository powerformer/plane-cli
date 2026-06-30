use super::render_json;
use crate::core::app::AppState;
use crate::core::request::Client;
use serde_json::{json, Value};

pub struct RequestOptions {
    pub method: String,
    pub path: String,
    pub data: Option<String>,
}

/// Passthrough to an arbitrary `/api/v1`-relative path. The escape hatch for
/// endpoints the typed commands do not cover yet.
pub fn run(state: &AppState, options: RequestOptions) -> Result<String, String> {
    let client = Client::from_state(state).map_err(|error| error.to_string())?;
    let path = options.path.trim_start_matches('/');
    let value = match options.method.to_uppercase().as_str() {
        "GET" => client.get(path, &[]),
        "POST" => client.post(path, &parse_body(options.data.as_deref())?),
        "PATCH" => client.patch(path, &parse_body(options.data.as_deref())?),
        "PUT" => client.put(path, &parse_body(options.data.as_deref())?),
        "DELETE" => client.delete(path),
        other => return Err(format!("unsupported method for passthrough: {other}")),
    }
    .map_err(|error| error.to_string())?;
    render_json(&value)
}

fn parse_body(data: Option<&str>) -> Result<Value, String> {
    match data {
        Some(raw) => {
            serde_json::from_str(raw).map_err(|error| format!("--data is not valid JSON: {error}"))
        }
        None => Ok(json!({})),
    }
}
