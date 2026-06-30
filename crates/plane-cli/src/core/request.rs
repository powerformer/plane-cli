use crate::core::app::AppState;
use crate::core::error::ApiError;
use serde_json::Value;
use std::io::Read;
use tracing::{debug, info};

const USER_AGENT: &str = "plane-cli";

/// Synchronous Plane `/api/v1` client over ureq with `X-API-Key` auth.
///
/// The token is held only to set the request header; it is never logged or
/// rendered.
pub struct Client {
    base_url: String,
    api_key: String,
    user_agent: String,
}

impl Client {
    /// Build from resolved config; errors if `api_base_url` or `api_key` are absent.
    pub fn from_state(state: &AppState) -> Result<Self, ApiError> {
        let base_url = state
            .config
            .api_base_url
            .as_deref()
            .map(normalize_api_base_url)
            .ok_or_else(|| {
                ApiError::Config(
                    "api_base_url is required for Plane API commands; set --api-base-url, api_base_url in plane.toml, or PLANE_API_BASE_URL"
                        .to_string(),
                )
            })?;
        let api_key = state.config.api_key.clone().ok_or_else(|| {
            ApiError::Config(
                "api_key is required for Plane API commands; set --api-key, api_key in plane.toml, or PLANE_API_KEY"
                    .to_string(),
            )
        })?;
        Ok(Self {
            base_url,
            api_key,
            user_agent: format!("{USER_AGENT}/{}", state.version),
        })
    }

    /// Normalized `/api/v1` base URL (token-free), suitable for display.
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    pub fn get(&self, path: &str, query: &[(&str, &str)]) -> Result<Value, ApiError> {
        self.send("GET", path, query, None)
    }

    fn send(
        &self,
        method: &str,
        path: &str,
        query: &[(&str, &str)],
        body: Option<&Value>,
    ) -> Result<Value, ApiError> {
        let url = endpoint(&self.base_url, path);
        info!(method, url = %url, "calling Plane API");
        let mut request = ureq::request(method, &url)
            .set("User-Agent", &self.user_agent)
            .set("Accept", "application/json")
            .set("X-API-Key", &self.api_key);
        for (key, value) in query {
            request = request.query(key, value);
        }
        let response = match body {
            Some(value) => request.send_json(value),
            None => request.call(),
        };
        let response = match response {
            Ok(response) => response,
            Err(ureq::Error::Status(status, response)) => {
                let body = read_body(response);
                return Err(ApiError::Http { status, url, body });
            }
            Err(error) => {
                return Err(ApiError::Transport {
                    url,
                    source: error.to_string(),
                });
            }
        };
        debug!(status = response.status(), url = %url, "Plane API response received");
        let text = read_body(response);
        if text.trim().is_empty() {
            return Ok(Value::Null);
        }
        serde_json::from_str(&text).map_err(|error| ApiError::Decode {
            url,
            source: error.to_string(),
            body: text,
        })
    }
}

fn endpoint(base_url: &str, path: &str) -> String {
    format!("{}/{}", base_url, path.trim_start_matches('/'))
}

fn read_body(response: ureq::Response) -> String {
    let mut reader = response.into_reader();
    let mut body = String::new();
    if reader.read_to_string(&mut body).is_err() {
        return "<failed to read response body>".to_string();
    }
    body
}

/// Accept either a Plane server root or an explicit `/api/v1` base and return the
/// `/api/v1` base without a trailing slash.
pub fn normalize_api_base_url(raw: &str) -> String {
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
    fn endpoint_appends_relative_path() {
        assert_eq!(
            endpoint(
                &normalize_api_base_url("https://plane.example.test"),
                "/users/me/"
            ),
            "https://plane.example.test/api/v1/users/me/"
        );
    }
}
