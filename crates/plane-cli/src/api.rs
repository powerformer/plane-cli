use crate::app::AppState;
use serde_json::Value;
use std::io::Read;
use tracing::{debug, info};

const USER_AGENT: &str = "plane-cli";

#[derive(Debug, Clone)]
pub struct ApiMeOptions {
    pub json: bool,
}

pub fn me(state: &AppState, options: ApiMeOptions) -> Result<String, String> {
    let api_base_url = require_api_base_url(state)?;
    let api_key = require_api_key(state)?;
    let url = endpoint_url(&api_base_url, "users/me/");

    info!(url = %url, "calling Plane API");
    let user = get_json(state, &url, &api_key)?;
    if options.json {
        return serde_json::to_string_pretty(&user)
            .map(|json| format!("{json}\n"))
            .map_err(|error| format!("failed to render API response JSON: {error}"));
    }
    Ok(render_me(&api_base_url, &user))
}

fn require_api_base_url(state: &AppState) -> Result<String, String> {
    state
        .config
        .api_base_url
        .as_deref()
        .map(normalize_api_base_url)
        .ok_or_else(|| {
            "api_base_url is required for Plane API commands; set --api-base-url, api_base_url in plane.toml, or PLANE_API_BASE_URL".to_string()
        })
}

fn require_api_key(state: &AppState) -> Result<String, String> {
    state.config.api_key.clone().ok_or_else(|| {
        "api_key is required for Plane API commands; set --api-key, api_key in plane.toml, or PLANE_API_KEY"
            .to_string()
    })
}

fn get_json(state: &AppState, url: &str, api_key: &str) -> Result<Value, String> {
    let user_agent = format!("{USER_AGENT}/{}", state.version);
    let response = ureq::get(url)
        .set("User-Agent", &user_agent)
        .set("Accept", "application/json")
        .set("X-API-Key", api_key)
        .call();

    let response = match response {
        Ok(response) => response,
        Err(ureq::Error::Status(status, response)) => {
            let body = read_response_body(response);
            return Err(format!(
                "Plane API GET {url} returned HTTP {status}: {}",
                summarize_body(&body)
            ));
        }
        Err(error) => return Err(format!("failed to GET {url}: {error}")),
    };

    let status = response.status();
    debug!(status, url, "Plane API response received");
    let body = read_response_body(response);
    serde_json::from_str(&body).map_err(|error| {
        format!(
            "Plane API GET {url} returned invalid JSON: {error}; body: {}",
            summarize_body(&body)
        )
    })
}

fn read_response_body(response: ureq::Response) -> String {
    let mut reader = response.into_reader();
    let mut body = String::new();
    if reader.read_to_string(&mut body).is_err() {
        return "<failed to read response body>".to_string();
    }
    body
}

fn summarize_body(body: &str) -> String {
    let body = body.trim();
    if body.is_empty() {
        return "<empty body>".to_string();
    }
    const LIMIT: usize = 500;
    if body.chars().count() <= LIMIT {
        body.to_string()
    } else {
        format!("{}...", body.chars().take(LIMIT).collect::<String>())
    }
}

fn render_me(api_base_url: &str, user: &Value) -> String {
    let id = string_field(user, "id").unwrap_or_else(|| "<unknown>".to_string());
    let email = string_field(user, "email").unwrap_or_else(|| "<unknown>".to_string());
    let display_name = string_field(user, "display_name")
        .or_else(|| string_field(user, "displayName"))
        .unwrap_or_else(|| {
            let first = string_field(user, "first_name").unwrap_or_default();
            let last = string_field(user, "last_name").unwrap_or_default();
            let full = format!("{first} {last}").trim().to_string();
            if full.is_empty() {
                "<unknown>".to_string()
            } else {
                full
            }
        });

    format!(
        "Plane API smoke ok\napi_base_url: {api_base_url}\nuser: {display_name} <{email}>\nid: {id}\n"
    )
}

fn string_field(user: &Value, field: &str) -> Option<String> {
    user.get(field)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn endpoint_url(api_base_url: &str, path: &str) -> String {
    format!(
        "{}/{}",
        normalize_api_base_url(api_base_url),
        path.trim_start_matches('/')
    )
}

fn normalize_api_base_url(raw: &str) -> String {
    let trimmed = raw.trim().trim_end_matches('/');
    if trimmed.ends_with("/api/v1") {
        trimmed.to_string()
    } else {
        format!("{trimmed}/api/v1")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn normalizes_server_url_to_api_v1_base() {
        assert_eq!(
            normalize_api_base_url("https://plane.example.test/"),
            "https://plane.example.test/api/v1"
        );
        assert_eq!(
            normalize_api_base_url("https://plane.example.test/api/v1/"),
            "https://plane.example.test/api/v1"
        );
    }

    #[test]
    fn endpoint_url_appends_relative_path() {
        assert_eq!(
            endpoint_url("https://plane.example.test", "/users/me/"),
            "https://plane.example.test/api/v1/users/me/"
        );
    }

    #[test]
    fn render_me_uses_stable_user_fields() {
        let output = render_me(
            "https://plane.example.test/api/v1",
            &json!({
                "id": "user-id",
                "display_name": "Ada Lovelace",
                "email": "ada@example.test"
            }),
        );

        assert!(output.contains("Plane API smoke ok"));
        assert!(output.contains("Ada Lovelace <ada@example.test>"));
        assert!(output.contains("id: user-id"));
    }
}
