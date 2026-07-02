use crate::core::app::AppState;
use crate::core::error::ApiError;
use serde_json::Value;
use std::io::Read;
use tracing::debug;

const USER_AGENT: &str = "plane-cli";

/// Default Plane backend used when no api_base_url is configured, so api_key is
/// the only setting required for routine use.
pub const DEFAULT_API_BASE_URL: &str = "https://plane.powerformer.net";

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
            .unwrap_or_else(|| normalize_api_base_url(DEFAULT_API_BASE_URL));
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

    pub fn post(&self, path: &str, body: &Value) -> Result<Value, ApiError> {
        self.send("POST", path, &[], Some(body))
    }

    pub fn patch(&self, path: &str, body: &Value) -> Result<Value, ApiError> {
        self.send("PATCH", path, &[], Some(body))
    }

    pub fn put(&self, path: &str, body: &Value) -> Result<Value, ApiError> {
        self.send("PUT", path, &[], Some(body))
    }

    pub fn delete(&self, path: &str) -> Result<Value, ApiError> {
        self.send("DELETE", path, &[], None)
    }

    /// POST a `multipart/form-data` body: any number of text `fields` plus one
    /// file part. Used to stream file bytes straight to the API (e.g. work-item
    /// attachment upload), so the client never talks to object storage.
    pub fn post_multipart(
        &self,
        path: &str,
        fields: &[(&str, &str)],
        file: &MultipartFile,
    ) -> Result<Value, ApiError> {
        let url = endpoint(&self.base_url, path);
        debug!(method = "POST", url = %url, "calling Plane API (multipart)");
        let boundary = multipart_boundary();
        let body = build_multipart_body(&boundary, fields, file);
        let response = ureq::post(&url)
            .set("User-Agent", &self.user_agent)
            .set("Accept", "application/json")
            .set("X-API-Key", &self.api_key)
            .set(
                "Content-Type",
                &format!("multipart/form-data; boundary={boundary}"),
            )
            .send_bytes(&body);
        read_response(response, url)
    }

    fn send(
        &self,
        method: &str,
        path: &str,
        query: &[(&str, &str)],
        body: Option<&Value>,
    ) -> Result<Value, ApiError> {
        let url = endpoint(&self.base_url, path);
        debug!(method, url = %url, "calling Plane API");
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
        read_response(response, url)
    }
}

/// A file part for a `multipart/form-data` upload.
pub struct MultipartFile<'a> {
    /// Form field name (e.g. `asset`).
    pub field: &'a str,
    /// File name reported to the server.
    pub filename: &'a str,
    /// MIME type of the bytes.
    pub content_type: &'a str,
    /// The raw file contents.
    pub bytes: &'a [u8],
}

/// Turn a ureq response result into a decoded JSON value (or a typed error),
/// shared by the JSON and multipart request paths.
fn read_response(
    response: Result<ureq::Response, ureq::Error>,
    url: String,
) -> Result<Value, ApiError> {
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

fn multipart_boundary() -> String {
    format!("planeCliBoundary{}", std::process::id())
}

/// Assemble a `multipart/form-data` body: each text field first, then the file
/// part last (some backends require the file to be the final part).
fn build_multipart_body(boundary: &str, fields: &[(&str, &str)], file: &MultipartFile) -> Vec<u8> {
    let mut body: Vec<u8> = Vec::new();
    for (name, value) in fields {
        body.extend_from_slice(
            format!("--{boundary}\r\nContent-Disposition: form-data; name=\"{name}\"\r\n\r\n{value}\r\n")
                .as_bytes(),
        );
    }
    body.extend_from_slice(
        format!(
            "--{boundary}\r\nContent-Disposition: form-data; name=\"{}\"; filename=\"{}\"\r\nContent-Type: {}\r\n\r\n",
            file.field, file.filename, file.content_type
        )
        .as_bytes(),
    );
    body.extend_from_slice(file.bytes);
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
    body
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

    #[test]
    fn multipart_body_puts_fields_then_file_last() {
        let file = MultipartFile {
            field: "asset",
            filename: "note.txt",
            content_type: "text/plain",
            bytes: b"hello",
        };
        let body = build_multipart_body("BOUND", &[("name", "note.txt")], &file);
        let text = String::from_utf8(body).expect("ascii body");
        assert_eq!(
            text,
            "--BOUND\r\nContent-Disposition: form-data; name=\"name\"\r\n\r\nnote.txt\r\n\
             --BOUND\r\nContent-Disposition: form-data; name=\"asset\"; filename=\"note.txt\"\r\n\
             Content-Type: text/plain\r\n\r\nhello\r\n--BOUND--\r\n"
        );
    }

    #[test]
    fn multipart_body_preserves_binary_file_bytes() {
        let raw = [0u8, 159, 146, 150, 255];
        let file = MultipartFile {
            field: "asset",
            filename: "blob.bin",
            content_type: "application/octet-stream",
            bytes: &raw,
        };
        let body = build_multipart_body("B", &[], &file);
        // The raw bytes survive verbatim between the header blank line and the
        // trailing boundary.
        let needle = b"\r\n\r\n";
        let start = body
            .windows(needle.len())
            .position(|w| w == needle)
            .expect("header terminator")
            + needle.len();
        assert_eq!(&body[start..start + raw.len()], &raw);
    }
}
